use toybox::common::*;

use slotmap::SlotMap;

use tokio::runtime::Runtime;
use tokio::sync::{oneshot, mpsc, watch};

use std::future::Future;


slotmap::new_key_type! {
	pub struct ObjectKey;
}


pub struct World {
	pub objects: SlotMap<ObjectKey, Object>,

	runtime: Runtime,
	world_cmds_rx: mpsc::Receiver<(ObjectKey, WorldCommand)>,

	interactive_objects: Vec<(ObjectKey, oneshot::Sender<()>)>,
	waiting_for_frame: Vec<oneshot::Sender<()>>,

	comms: Comms,
}

impl World {
	pub fn new() -> World {
		let (world_cmds_tx, world_cmds_rx) = mpsc::channel(100);

		World {
			objects: SlotMap::with_key(),
			runtime: Runtime::new().unwrap(),

			world_cmds_rx,

			interactive_objects: Vec::new(),
			waiting_for_frame: Vec::new(),

			comms: Comms {
				world_cmds_tx,
			}
		}
	}

	pub fn new_object(&mut self, pos: Vec3, size: Vec2, color: impl Into<Color>) -> ObjectKey {
		self.objects.insert(Object {
			pos,
			size,
			color: color.into(),

			actor: None,
		})
	}

	pub fn attach_actor<F, A>(&mut self, key: ObjectKey, make_actor: F)
		where F: FnOnce(ActorContext) -> A
			, A: Future<Output=()> + Send + 'static
	{
		let object = self.objects.get_mut(key).expect("Invalid object");
		object.drop_actor();

		let actor_ctx = ActorContext {
			me: key,
			comms: self.comms.clone(),
		};

		let handle = self.runtime.spawn(make_actor(actor_ctx));
		object.actor = Some(handle);
	}

	pub fn update(&mut self) {
		for tx in self.waiting_for_frame.drain(..) {
			let _ = tx.send(());
		}

		while let Ok((src_key, cmd)) = self.world_cmds_rx.try_recv() {
			use WorldCommand::*;

			match cmd {
				SetPos(new_pos) => {
					if let Some(object) = self.objects.get_mut(src_key) {
						object.pos = new_pos;
					}
				}

				GetPos(key, tx) => {
					if let Some(object) = self.objects.get(key) {
						let _ = tx.send(object.pos);

					} else if let Some(object) = self.objects.get_mut(src_key) {
						println!("actor {src_key:?} tried to get position of non-existent object {key:?} - terminated");
						object.drop_actor();
					}
				}

				MakeInteractive(tx) => {
					self.interactive_objects.push((src_key, tx));
				}

				WaitForFrame(tx) => {
					self.waiting_for_frame.push(tx);
				}
			}
		}

		// TODO(pat.m):  wait for all objects to settle

		self.interactive_objects.retain(|(obj, tx)| !tx.is_closed() && self.objects.contains_key(*obj));
	}

	pub fn is_interactive(&self, needle: ObjectKey) -> bool {
		self.interactive_objects.iter()
			.any(|(obj, _)| *obj == needle)
	}

	pub fn nearest_interactive(&self, pos: Vec3, dir: Vec3) -> Option<ObjectKey> {
		self.interactive_objects.iter()
			.map(|&(obj, _)| (obj, (self.objects[obj].pos - pos).dot(dir)))
			.filter(|&(_, d)| d > 0.0 && d < 0.8)
			.min_by(|(_, a), (_, b)| a.total_cmp(b))
			.map(|(obj, _)| obj)
	}

	pub fn interact(&mut self, needle: ObjectKey) {
		if let Some((_, tx)) = self.interactive_objects.iter()
			.position(|(obj, _)| *obj == needle)
			.map(|index| self.interactive_objects.remove(index))
		{
			// We don't care if this fails
			let _ = tx.send(());
		}
	}
}



pub struct Object {
	pub pos: Vec3,
	pub size: Vec2,
	pub color: Color,

	actor: Option<tokio::task::JoinHandle<()>>,
}

impl Object {
	pub fn drop_actor(&mut self) {
		if let Some(actor) = self.actor.take() {
			actor.abort();
		}
	}
}

impl Drop for Object {
	fn drop(&mut self) {
		self.drop_actor();
	}
}



pub enum WorldCommand {
	SetPos(Vec3),
	GetPos(ObjectKey, oneshot::Sender<Vec3>),

	MakeInteractive(oneshot::Sender<()>),
	WaitForFrame(oneshot::Sender<()>),
}


#[derive(Clone)]
struct Comms {
	world_cmds_tx: mpsc::Sender<(ObjectKey, WorldCommand)>,
}

pub struct ActorContext {
	me: ObjectKey,
	comms: Comms,
}

impl ActorContext {
	pub fn key(&self) -> ObjectKey {
		self.me
	}

	pub async fn send(&self, cmd: WorldCommand) {
		self.comms.world_cmds_tx.send((self.me, cmd)).await.unwrap()
	}

	pub async fn set_pos(&self, pos: Vec3) {
		self.send(WorldCommand::SetPos(pos)).await;
	}

	pub async fn get_pos(&self, object: ObjectKey) -> Vec3 {
		let (tx, rx) = oneshot::channel();
		self.send(WorldCommand::GetPos(object, tx)).await;
		rx.await.unwrap()
	}

	pub async fn frame(&self) {
		let (tx, rx) = oneshot::channel();
		self.send(WorldCommand::WaitForFrame(tx)).await;
		rx.await.unwrap()
	}

	pub async fn interact(&self) {
		let (tx, rx) = oneshot::channel();
		self.send(WorldCommand::MakeInteractive(tx)).await;
		rx.await.unwrap()
	}
}





pub fn make_test_world() -> World {
	let mut world = World::new();

	world.new_object(Vec3::new(-1.0, 0.0, -1.0), Vec2::splat(1.0), Color::white());
	world.new_object(Vec3::new( 2.0, 0.0, -2.0), Vec2::new(1.0, 2.0), (0.2, 0.5, 0.8));

	let obj_0 = world.new_object(Vec3::new( 0.0, 0.0, -3.0), Vec2::new(1.0, 0.8), (0.8, 0.2, 0.8));
	let target_obj = world.new_object(Vec3::new( -3.0, 0.0, 2.0), Vec2::new(0.2, 0.5), (0.8, 0.9, 0.4));

	world.attach_actor(obj_0, |ctx| async move {
		let mut pos = ctx.get_pos(ctx.key()).await;

		loop {
			ctx.frame().await;

			let target = ctx.get_pos(target_obj).await;

			let diff = target - pos;
			if diff.length() > 0.5 {
				pos += diff * 1.0/60.0;
				ctx.set_pos(pos).await;
			}
		}
	});

	world.attach_actor(target_obj, |ctx| async move {
		use std::time::Duration;

		async fn move_to(ctx: &ActorContext, target: Vec3) {
			let mut pos = ctx.get_pos(ctx.key()).await;

			loop {
				let diff = target - pos;
				if diff.length() < 0.1 {
					break
				}

				pos += diff.normalize() * 2.0/60.0;
				ctx.set_pos(pos).await;
				ctx.frame().await;
			}
		}

		tokio::select! {
			_ = ctx.interact() => {}
			_ = tokio::time::sleep(Duration::from_secs(3)) => {}
		}

		loop {
			move_to(&ctx, Vec3::new(3.0, 0.0, 1.0)).await;
			ctx.interact().await;

			move_to(&ctx, Vec3::new(-1.0, 0.0, 2.0)).await;
			ctx.interact().await;

			move_to(&ctx, Vec3::new(-3.0, 0.0, 0.0)).await;
			ctx.interact().await;
		}

	});

	world
}