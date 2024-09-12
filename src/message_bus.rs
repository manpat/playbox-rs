use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell, Ref};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::marker::PhantomData;

#[cfg(test)]
mod test;


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
		let store = self.inner.get_store::<T>();

		Subscription {
			inner: store.new_subscriber(),
			_phantom: PhantomData,
		}
	}

	pub fn peek_count<T: 'static>(&self, subscription: &Subscription<T>) -> usize {
		let next_unread_message_index = subscription.inner.next_unread_message_index.get();
		self.inner.get_store::<T>().queued_message_count().saturating_sub(next_unread_message_index)
	}

	pub fn peek_any<T: 'static>(&self, subscription: &Subscription<T>) -> bool {
		self.peek_count(subscription) > 0
	}

	pub fn count<T: 'static>(&self, subscription: &Subscription<T>) -> usize {
		let next_unread_message_index = subscription.inner.next_unread_message_index.get();
		let available_messages = self.inner.get_store::<T>().queued_message_count();
		subscription.inner.next_unread_message_index.set(available_messages);
		available_messages.saturating_sub(next_unread_message_index)
	}

	pub fn any<T: 'static>(&self, subscription: &Subscription<T>) -> bool {
		self.count(subscription) > 0
	}

	pub fn poll<T: Clone + 'static>(&self, subscription: &Subscription<T>) -> impl Iterator<Item=T> + '_ {
		struct Iter<'s, T: Clone + 'static> {
			subscription: Rc<SubscriptionInner>,
			queue: *mut TypedMessageQueue<T>,

			_lock: MessageQueuePollLock,
			_phantom: PhantomData<&'s ()>,
		}

		impl<'s, T: Clone + 'static> Iterator for Iter<'s, T> {
			// TODO(pat.m): this currently ties the lifetime of the Item to MessageBus, where what we _really_ want
			// is to tie it to is the lifetime of the iterator.
			type Item = T;

			fn next(&mut self) -> Option<Self::Item> {
				let next_unread_message_index = self.subscription.next_unread_message_index.get();

				// SAFETY: There are only ever short lived references into messages, so this is guaranteed not to overlap.
				let messages = unsafe { &(*self.queue).messages };

				if let Some(message) = messages.get(next_unread_message_index) {
					self.subscription.next_unread_message_index.set(next_unread_message_index + 1);
					Some(message.clone())
				} else {
					None
				}
			}
		}

		let queue = self.inner.get_store::<T>().queue.cast::<TypedMessageQueue<T>>();

		Iter {
			subscription: subscription.inner.clone(),
			queue,
			_lock: MessageQueuePollLock::lock(queue),
			_phantom: PhantomData,
		}
	}

	pub fn poll_consume<T: 'static>(&self, subscription: &Subscription<T>) -> impl Iterator<Item=T> + '_ {
		struct Iter<'s, T: 'static> {
			subscription: Rc<SubscriptionInner>,
			queue: *mut TypedMessageQueue<T>,

			lock: MessageQueuePollLock,
			_phantom: PhantomData<&'s ()>,
		}

		impl<'s, T: 'static> Iterator for Iter<'s, T> {
			type Item = T;

			fn next(&mut self) -> Option<Self::Item> {
				let next_unread_message_index = self.subscription.next_unread_message_index.get();

				// TODO(pat.m): document
				assert!(self.lock.is_unique(), "Trying to poll consuming message bus iterator while also polling the same message type");

				// SAFETY: Because of the above assert, we can guarantee that there are no shared references into messages,
				// since messages is only ever borrowed temporarily or while the iterator returned from poll is alive.
				let messages = unsafe { &mut (*self.queue).messages };

				if next_unread_message_index < messages.len() {
					// NOTE: NO need to increment subscription.next_unread_message_index since removing this message
					// implicitly shifts all indices down by one anyway.
					Some(messages.remove(next_unread_message_index))
				} else {
					None
				}
			}
		}

		let queue = self.inner.get_store::<T>().queue.cast::<TypedMessageQueue<T>>();

		Iter {
			subscription: subscription.inner.clone(),
			queue,
			lock: MessageQueuePollLock::lock(queue),
			_phantom: PhantomData,
		}
	}

	pub fn emit<T: 'static>(&self, message: T) {
		let store = self.inner.get_store::<T>();
		
		unsafe {
			(*store.queue).to_concrete_mut().emit(message)
		}

	}

	pub fn garbage_collect(&self) {
		let mut stores = self.inner.stores.borrow_mut();
		for store in stores.iter_mut() {
			store.garbage_collect();
		}
	}
}

#[derive(Default)]
struct MessageBusInner {
	// Indexes into self.stores
	type_to_index: RefCell<HashMap<TypeId, usize>>,
	stores: RefCell<Vec<RawBusStore>>,
}

impl MessageBusInner {
	fn get_store<T: 'static>(&self) -> Ref<'_, RawBusStore> {
		let type_id = TypeId::of::<T>();
		let num_stores = self.stores.borrow().len();

		// Get type index
		let index = *self.type_to_index.borrow_mut()
			.entry(type_id).or_insert(num_stores);

		// Need to insert
		if index == num_stores {
			self.stores.try_borrow_mut()
				.expect("Trying to create new BusStore while store list is borrowed")
				.push(RawBusStore::new::<T>());
		}

		Ref::map(self.stores.borrow(), |stores| &stores[index])
	}
}


struct RawBusStore {
	// Borrows are always shortlived so this won't cause any problems
	subscriptions: RefCell<SubscriptionList>,
	queue: *mut dyn MessageQueue,
}

impl RawBusStore {
	pub fn new<T: 'static>() -> RawBusStore {
		let queue = Box::new(TypedMessageQueue::<T>::default());

		RawBusStore {
			subscriptions: Default::default(),
			queue: Box::into_raw(queue),
		}
	}

	fn new_subscriber(&self) -> Rc<SubscriptionInner> {
		let queued_message_count = self.queued_message_count();

		let subscription_inner = Rc::new(SubscriptionInner {
			// Make sure this subscription doesn't see any messages already in the queue
			next_unread_message_index: Cell::new(queued_message_count),
		});

		self.subscriptions.borrow_mut()
			.subscribers.push(Rc::downgrade(&subscription_inner));

		subscription_inner
	}

	fn queued_message_count(&self) -> usize {
		// SAFETY: Borrows of queue never escape the functions they exist in, so this is fine.
		unsafe { (*self.queue).queued_message_count() }
	}

	fn garbage_collect(&mut self) {
		let subscriptions = self.subscriptions.get_mut();

		// SAFETY: For this function to be called, both MessageBusInner::stores has to be borrowed mutably,
		// _and also_ self.queue.lock_for_poll must be false. So we can guarantee that both:
		// 	- Noone else is borrowing self or subscriptions.
		// 	- Noone has a pointer to queue that would attempt to read through it.
		unsafe {
			(*self.queue).garbage_collect(subscriptions);
		}
	}
}

impl Drop for RawBusStore {
	fn drop(&mut self) {
		// SAFETY: References into queue will always be tied to MessageBus, and so Drop cannot be called
		// while references are active.
		let _ = unsafe { Box::from_raw(self.queue) };
	}
}


trait MessageQueue {
	fn as_any_mut(&mut self) -> &mut dyn Any;

	fn queued_message_count(&self) -> usize;

	fn garbage_collect(&mut self, subscription_list: &mut SubscriptionList);
}

impl dyn MessageQueue {
	fn to_concrete_mut<T: Any>(&mut self) -> &mut TypedMessageQueue<T> {
		self.as_any_mut()
			.downcast_mut()
			.unwrap()
	}
}

impl<T: Any> MessageQueue for TypedMessageQueue<T> {
	fn as_any_mut(&mut self) -> &mut dyn Any { self as &mut dyn Any }

	fn queued_message_count(&self) -> usize {
		self.messages.len()
	}

	fn garbage_collect(&mut self, subscription_list: &mut SubscriptionList) {
		assert!(self.poll_lock == 0, "Trying to garbage collect a message queue while being polled");

		subscription_list.subscribers.retain(|subscriber| subscriber.strong_count() > 0);
		if subscription_list.subscribers.is_empty() {
			self.messages.clear();
			return;
		}

		let minimum_unread_index = subscription_list.subscribers.iter()
			.flat_map(|subscriber| subscriber.upgrade().map(|inner| inner.next_unread_message_index.get()))
			.min()
			.unwrap_or(0)
			.min(self.messages.len());

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
	poll_lock: u32,
}

impl<T: 'static> TypedMessageQueue<T> {
	fn emit(&mut self, message: T) {
		self.messages.push(message);
	}
}


impl<T: 'static> Default for TypedMessageQueue<T> {
	fn default() -> Self {
		Self { messages: Vec::default(), poll_lock: 0, }
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

impl std::fmt::Debug for MessageBus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "MessageBus{{...}}")
	}
}



struct MessageQueuePollLock {
	poll_lock: *mut u32,
}

impl MessageQueuePollLock {
	fn lock<T: 'static>(queue: *mut TypedMessageQueue<T>) -> MessageQueuePollLock {
		unsafe {
			let poll_lock = &raw mut (*queue).poll_lock;
			*poll_lock += 1;

			MessageQueuePollLock { poll_lock }
		}
	}

	fn is_unique(&self) -> bool {
		unsafe {
			self.poll_lock.read() == 1
		}
	}
}

impl Drop for MessageQueuePollLock {
	fn drop(&mut self) {
		unsafe {
			*self.poll_lock -= 1;
		}
	}
}