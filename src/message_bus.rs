use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell, Ref};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::marker::PhantomData;

#[derive(Clone)]
pub struct MessageBus {
	inner: Rc<RefCell<Inner>>,
}

impl MessageBus {
	pub fn new() -> Self {
		MessageBus {
			inner: Rc::new(RefCell::new(Inner::default()))
		}
	}

	pub fn subscribe<T: 'static>(&self) -> Subscription<T> {
		let mut inner = self.inner.borrow_mut();
		let type_id = TypeId::of::<T>();

		let message_bus = inner.typed_busses.entry(type_id)
			.or_insert_with(|| Box::new(TypedMessageBusConcrete::<T> {
				subscribers: Vec::new(),
				messages: Vec::new(),
			}));

		let typed_message_bus = message_bus.to_concrete_mut::<T>();

		let subscription_inner = Rc::new(SubscriptionInner {
			// Make sure this subscription doesn't see any messages already in the queue
			seen_message_seq: Cell::new(typed_message_bus.messages.len() as u32),
			_phantom: PhantomData,
		});

		typed_message_bus.subscribers.push(Rc::downgrade(&subscription_inner));

		Subscription {
			inner: subscription_inner
		}
	}

	pub fn poll<T: 'static>(&self, subscription: &Subscription<T>) -> Ref<'_, [T]> {
		let type_id = TypeId::of::<T>();
		let last_seen_message_seq = subscription.inner.seen_message_seq.get() as usize;

		Ref::map(self.inner.borrow(), move |inner| {
			inner.typed_busses.get(&type_id)
				.map(|bus| {
					let bus = bus.to_concrete();

					subscription.inner.seen_message_seq.set(bus.messages.len() as u32);
					&bus.messages[last_seen_message_seq..]
				})
				.unwrap_or(&[])
		})
	}

	pub fn emit<T: 'static>(&self, message: T) {
		let type_id = TypeId::of::<T>();

		if let Some(bus) = self.inner.borrow_mut().typed_busses.get_mut(&type_id) {
			let bus = bus.to_concrete_mut();
			bus.messages.push(message);
		}
	}

	pub fn garbage_collect(&self) {
		let mut inner = self.inner.borrow_mut();
		for (_, bus) in inner.typed_busses.iter_mut() {
			bus.garbage_collect();
		}
	}
}

#[derive(Default)]
struct Inner {
	typed_busses: HashMap<TypeId, Box<dyn TypedMessageBus>>,
}



trait TypedMessageBus {
	fn as_any(&self) -> &dyn Any;
	fn as_any_mut(&mut self) -> &mut dyn Any;

	fn garbage_collect(&mut self);
}

impl dyn TypedMessageBus {
	fn to_concrete<T: Any>(&self) -> &TypedMessageBusConcrete<T> {
		self.as_any()
			.downcast_ref()
			.unwrap()
	}

	fn to_concrete_mut<T: Any>(&mut self) -> &mut TypedMessageBusConcrete<T> {
		self.as_any_mut()
			.downcast_mut()
			.unwrap()
	}
}

impl<T: Any> TypedMessageBus for TypedMessageBusConcrete<T> {
	fn as_any(&self) -> &dyn Any { self as &dyn Any }
	fn as_any_mut(&mut self) -> &mut dyn Any { self as &mut dyn Any }

	fn garbage_collect(&mut self) {
		self.subscribers.retain(|subscriber| subscriber.strong_count() > 0);
		if self.subscribers.is_empty() {
			self.messages.clear();
			return;
		}

		let minimum_message_seq = self.subscribers.iter()
			.flat_map(|subscriber| subscriber.upgrade().map(|inner| inner.seen_message_seq.get()))
			.min()
			.unwrap_or(0);

		if minimum_message_seq != 0 {
			let to_drop = minimum_message_seq as usize;
			self.messages.drain(..to_drop);

			for sub in self.subscribers.iter() {
				let Some(sub) = sub.upgrade() else { continue };
				let new_seq = sub.seen_message_seq.get() - minimum_message_seq;
				sub.seen_message_seq.set(new_seq);
			}
		}
	}
}


#[derive(Default)]
struct TypedMessageBusConcrete<T: 'static> {
	subscribers: Vec<Weak<SubscriptionInner<T>>>,
	messages: Vec<T>,
}


pub struct Subscription<T> {
	inner: Rc<SubscriptionInner<T>>,
}

struct SubscriptionInner<T> {
	seen_message_seq: Cell<u32>,
	_phantom: PhantomData<*const T>,
}