use super::*;


struct EmptyMessage;
struct EmptyMessage2;


#[test]
fn basic_scenario() {
	let bus = MessageBus::new();

	bus.emit(EmptyMessage);

	let subscription = bus.subscribe::<EmptyMessage>();
	assert!(!bus.messages_available(&subscription), "New subscriptions shouldn't see prior messages");
	assert_eq!(bus.poll_consume(&subscription).count(), 0, "New subscriptions shouldn't see prior messages");

	bus.emit(EmptyMessage);
	bus.emit(EmptyMessage);

	assert!(bus.messages_available(&subscription), "Existing subscriptions should see new messages");
	assert_eq!(bus.poll_consume(&subscription).count(), 2, "Subscriptions should see _all_ new messages");

	assert!(!bus.messages_available(&subscription), "Subscriptions should only see each message once");
	assert_eq!(bus.poll_consume(&subscription).count(), 0, "Subscriptions should only see each message once");

	bus.emit(EmptyMessage);
	bus.emit(EmptyMessage);

	bus.garbage_collect();

	assert!(bus.messages_available(&subscription), "Existing subscriptions should still see new messages after garbage_collect");
	assert_eq!(bus.poll_consume(&subscription).count(), 2, "Subscriptions should still see _all_ new messages after garbage_collect");
}


#[test]
fn subscribe_while_polling() {
	let bus = MessageBus::new();
	let subscription = bus.subscribe::<EmptyMessage>();

	bus.emit(EmptyMessage);

	for _ in bus.poll_consume(&subscription) {
		let _different_message_type = bus.subscribe::<EmptyMessage2>();
		let _same_message_type = bus.subscribe::<EmptyMessage>();
	}
}


#[test]
fn emit_different_message_while_polling() {
	let bus = MessageBus::new();
	let subscription = bus.subscribe::<EmptyMessage>();

	bus.emit(EmptyMessage);

	for _ in bus.poll_consume(&subscription) {
		bus.emit(EmptyMessage2);
	}
}


#[test]
fn emit_same_message_while_polling() {
	let bus = MessageBus::new();
	let subscription = bus.subscribe::<EmptyMessage>();

	bus.emit(EmptyMessage);

	let mut hits = 0;

	for _ in bus.poll_consume(&subscription).take(5) {
		bus.emit(EmptyMessage);
		hits += 1;
	}

	assert_eq!(hits, 5, "poll should see all messages emitted during loop");
}


// Known broken
#[test]
fn poll_different_while_polling() {
	let bus = MessageBus::new();
	let subscription = bus.subscribe::<EmptyMessage>();
	let subscription2 = bus.subscribe::<EmptyMessage2>();

	bus.emit(EmptyMessage);
	bus.emit(EmptyMessage2);

	let mut hits = 0;

	for _ in bus.poll_consume(&subscription).take(5) {
		hits += 1;

		for _ in bus.poll_consume(&subscription2).take(5) {
			hits += 10;
		}
	}

	assert_eq!(hits, 11, "Polling for different message types should be fine");
}


#[test]
fn poll_same_while_polling() {
	let bus = MessageBus::new();
	let subscription = bus.subscribe::<EmptyMessage>();
	let subscription2 = bus.subscribe::<EmptyMessage>();

	bus.emit(EmptyMessage);
	bus.emit(EmptyMessage);

	for _ in bus.poll_consume(&subscription).take(5) {
		for _ in bus.poll_consume(&subscription2).take(5) {
		}
	}
}


// Known broken
#[test]
fn multiple_same_subs() {
	let bus = MessageBus::new();

	bus.emit(EmptyMessage);
	let subscription_1 = bus.subscribe::<EmptyMessage>();

	bus.emit(EmptyMessage);
	let subscription_2 = bus.subscribe::<EmptyMessage>();

	bus.emit(EmptyMessage);

	assert_eq!(bus.poll_consume(&subscription_1).count(), 2, "New subscriptions shouldn't see prior messages");
	assert_eq!(bus.poll_consume(&subscription_2).count(), 1, "New subscriptions shouldn't see prior messages");

	bus.emit(EmptyMessage);
	bus.emit(EmptyMessage);

	assert_eq!(bus.poll_consume(&subscription_1).count(), 2, "Subscriptions should see _all_ new messages");
	assert_eq!(bus.poll_consume(&subscription_1).count(), 0, "Subscriptions should only see each message once");

	bus.emit(EmptyMessage);
	bus.emit(EmptyMessage);

	bus.garbage_collect();

	assert!(bus.messages_available(&subscription_1), "Existing subscriptions should still see new messages after garbage_collect");
	assert_eq!(bus.poll_consume(&subscription_1).count(), 2, "Subscriptions should still see _all_ new messages after garbage_collect");

	assert!(bus.messages_available(&subscription_2));
	assert_eq!(bus.poll_consume(&subscription_2).count(), 4);
}