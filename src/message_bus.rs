use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell, Ref};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::marker::PhantomData;

#[derive(Clone)]
pub struct MessageBus {
	inner: Rc<MessageBusInner>,
}

impl MessageBus {
	pub fn new() -> Self {
		MessageBus {
			inner: Rc::new(MessageBusInner::default())
		}
	}

	pub fn subscribe<T: 'static>(&self) -> Subscription<T> {
		let type_id = TypeId::of::<T>();

		let current_message_count = self.inner.message_queues.borrow()
			.get(&type_id)
			.map_or(0, |queue| queue.current_message_count());

		let subscription_inner = Rc::new(SubscriptionInner {
			// Make sure this subscription doesn't see any messages already in the queue
			next_unread_message_index: Cell::new(current_message_count),
		});

		let mut subscription_lists = self.inner.subscription_lists.borrow_mut();

		let subscription_list = subscription_lists.entry(type_id)
			.or_insert_with(Default::default);

		subscription_list.subscribers.push(Rc::downgrade(&subscription_inner));

		Subscription {
			inner: subscription_inner,
			_phantom: PhantomData,
		}
	}

	pub fn messages_available<T: 'static>(&self, subscription: &Subscription<T>) -> bool {
		let type_id = TypeId::of::<T>();
		let next_unread_message_index = subscription.inner.next_unread_message_index.get();

		self.inner.message_queues.borrow()
			.get(&type_id)
			.map_or(false, |queue| next_unread_message_index < queue.current_message_count())
	}

	pub fn poll<T: 'static>(&self, subscription: &Subscription<T>) -> Ref<'_, [T]> {
		let type_id = TypeId::of::<T>();
		let next_unread_message_index = subscription.inner.next_unread_message_index.get();

		Ref::map(self.inner.message_queues.borrow(), move |message_queues| {
			message_queues.get(&type_id)
				.map(|queue| {
					let queue = queue.to_concrete();

					subscription.inner.next_unread_message_index.set(queue.messages.len());
					&queue.messages[next_unread_message_index..]
				})
				.unwrap_or(&[])
		})
	}

	pub fn poll_consume<T: 'static>(&self, subscription: &Subscription<T>) -> impl Iterator<Item=T> + '_ {
		let type_id = TypeId::of::<T>();

		let mut message_queues = self.inner.message_queues.borrow_mut();

		let subscription_inner = subscription.inner.clone();

		std::iter::from_fn(move || {
			let next_unread_message_index = subscription_inner.next_unread_message_index.get();

			let messages = message_queues.get_mut(&type_id)
				.map(|queue| &mut queue.to_concrete_mut().messages);

			if let Some(messages) = messages
				&& next_unread_message_index < messages.len()
			{
				// No need to adjust next_unread_message_index since `remove` will shift every later message down.
				Some(messages.remove(next_unread_message_index))
			}
			else {
				None
			}
		})
	}

	// TODO(pat.m): this will panic while polling
	pub fn emit<T: 'static>(&self, message: T) {
		let type_id = TypeId::of::<T>();

		let mut message_queues = self.inner.message_queues.borrow_mut();
		let message_queue = message_queues.entry(type_id)
			.or_insert_with(|| Box::new(TypedMessageQueue::<T>::default()));

		message_queue.to_concrete_mut().messages.push(message);
	}

	pub fn garbage_collect(&self) {
		let mut message_queues = self.inner.message_queues.borrow_mut();
		let mut subscription_lists = self.inner.subscription_lists.borrow_mut();

		for (type_id, queue) in message_queues.iter_mut() {
			let subscription_list = subscription_lists.get_mut(&type_id);
			queue.garbage_collect(subscription_list);
		}
	}
}

#[derive(Default)]
struct MessageBusInner {
	message_queues: RefCell<HashMap<TypeId, Box<dyn MessageQueue>>>,
	subscription_lists: RefCell<HashMap<TypeId, SubscriptionList>>,
}



trait MessageQueue {
	fn as_any(&self) -> &dyn Any;
	fn as_any_mut(&mut self) -> &mut dyn Any;

	fn current_message_count(&self) -> usize;

	fn garbage_collect(&mut self, subscription_list: Option<&mut SubscriptionList>);
}

impl dyn MessageQueue {
	fn to_concrete<T: Any>(&self) -> &TypedMessageQueue<T> {
		self.as_any()
			.downcast_ref()
			.unwrap()
	}

	fn to_concrete_mut<T: Any>(&mut self) -> &mut TypedMessageQueue<T> {
		self.as_any_mut()
			.downcast_mut()
			.unwrap()
	}
}

impl<T: Any> MessageQueue for TypedMessageQueue<T> {
	fn as_any(&self) -> &dyn Any { self as &dyn Any }
	fn as_any_mut(&mut self) -> &mut dyn Any { self as &mut dyn Any }

	fn current_message_count(&self) -> usize {
		self.messages.len()
	}

	fn garbage_collect(&mut self, subscription_list: Option<&mut SubscriptionList>) {
		let Some(subscription_list) = subscription_list else {
			self.messages.clear();
			return;
		};

		subscription_list.subscribers.retain(|subscriber| subscriber.strong_count() > 0);
		if subscription_list.subscribers.is_empty() {
			self.messages.clear();
			return;
		}

		let minimum_unread_index = subscription_list.subscribers.iter()
			.flat_map(|subscriber| subscriber.upgrade().map(|inner| inner.next_unread_message_index.get()))
			.min()
			.unwrap_or(0);

		if minimum_unread_index != 0 {
			let to_drop = minimum_unread_index;
			self.messages.drain(..to_drop);

			for sub in subscription_list.subscribers.iter() {
				let Some(sub) = sub.upgrade() else { continue };
				let new_seq = sub.next_unread_message_index.get() - minimum_unread_index;
				sub.next_unread_message_index.set(new_seq);
			}
		}
	}
}


struct TypedMessageQueue<T: 'static> {
	messages: Vec<T>,
}

impl<T: 'static> Default for TypedMessageQueue<T> {
	fn default() -> Self {
		Self { messages: Vec::default() }
	}
}



pub struct Subscription<T: 'static> {
	inner: Rc<SubscriptionInner>,
	_phantom: PhantomData<&'static T>,
}

struct SubscriptionInner {
	next_unread_message_index: Cell<usize>,
}

#[derive(Default)]
struct SubscriptionList {
	subscribers: Vec<Weak<SubscriptionInner>>,
}


impl<T: 'static> std::fmt::Debug for Subscription<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Subscription({})", std::any::type_name::<T>())
	}
}