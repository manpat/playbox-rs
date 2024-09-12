use super::*;


#[derive(Clone)] struct EmptyMessage;
#[derive(Clone)] struct EmptyMessage2;


#[test]
fn basic_scenario() {
	let bus = MessageBus::new();

	bus.emit(EmptyMessage);

	let subscription = bus.subscribe::<EmptyMessage>();
	assert!(!bus.peek_any(&subscription), "New subscriptions shouldn't see prior messages");
	assert_eq!(bus.poll(&subscription).count(), 0, "New subscriptions shouldn't see prior messages");

	bus.emit(EmptyMessage);
	bus.emit(EmptyMessage);

	assert!(bus.peek_any(&subscription), "Existing subscriptions should see new messages");
	assert_eq!(bus.poll(&subscription).count(), 2, "Subscriptions should see _all_ new messages");

	assert!(!bus.peek_any(&subscription), "Subscriptions should only see each message once");
	assert_eq!(bus.poll(&subscription).count(), 0, "Subscriptions should only see each message once");

	bus.emit(EmptyMessage);
	bus.emit(EmptyMessage);

	bus.garbage_collect();

	assert!(bus.peek_any(&subscription), "Existing subscriptions should still see new messages after garbage_collect");
	assert_eq!(bus.poll(&subscription).count(), 2, "Subscriptions should still see _all_ new messages after garbage_collect");
}


#[test]
fn subscribe_same_while_polling() {
	let bus = MessageBus::new();
	let subscription = bus.subscribe::<EmptyMessage>();

	bus.emit(EmptyMessage);

	for _ in bus.poll(&subscription) {
		let _same_message_type = bus.subscribe::<EmptyMessage>();
	}
}

#[test]
fn subscribe_different_while_polling() {
	let bus = MessageBus::new();
	let subscription = bus.subscribe::<EmptyMessage>();

	bus.emit(EmptyMessage);

	for _ in bus.poll(&subscription) {
		let _different_message_type = bus.subscribe::<EmptyMessage2>();
	}
}


#[test]
fn emit_different_message_while_polling() {
	let bus = MessageBus::new();
	let subscription = bus.subscribe::<EmptyMessage>();

	bus.emit(EmptyMessage);

	for _ in bus.poll(&subscription) {
		bus.emit(EmptyMessage2);
	}
}


#[test]
fn emit_same_message_while_polling() {
	let bus = MessageBus::new();
	let subscription = bus.subscribe::<u32>();

	bus.emit(1u32);

	let mut hits = 0;

	for value in bus.poll(&subscription).take(5) {
		for _ in 0..100 {
			bus.emit(5u32);
		}

		hits += value;
	}

	assert_eq!(hits, 4*5 + 1, "poll should see all messages emitted during loop");
}


#[test]
fn poll_different_while_polling() {
	let bus = MessageBus::new();
	let subscription = bus.subscribe::<EmptyMessage>();
	let subscription2 = bus.subscribe::<EmptyMessage2>();

	bus.emit(EmptyMessage);
	bus.emit(EmptyMessage2);

	let mut hits = 0;

	for _ in bus.poll(&subscription).take(5) {
		hits += 1;

		for _ in bus.poll(&subscription2).take(5) {
			hits += 10;
		}
	}

	assert_eq!(hits, 11, "Polling for different message types should be fine");
}


#[test]
fn poll_consume_different_while_polling() {
	let bus = MessageBus::new();
	let subscription = bus.subscribe::<u32>();
	let subscription2 = bus.subscribe::<u16>();

	bus.emit(2u32);
	bus.emit(3u16);

	let mut hits = 0;

	for value in bus.poll(&subscription).take(5) {
		for value in bus.poll_consume(&subscription2).take(5) {
			hits += value as u32 * 10;
		}

		hits += value;
	}

	assert_eq!(hits, 32, "Polling for different message types should be fine");
}


#[test]
fn poll_same_while_polling() {
	let bus = MessageBus::new();
	let subscription = bus.subscribe::<EmptyMessage>();
	let subscription2 = bus.subscribe::<EmptyMessage>();

	bus.emit(EmptyMessage);
	bus.emit(EmptyMessage);

	for _ in bus.poll(&subscription).take(5) {
		for _ in bus.poll(&subscription2).take(5) {
		}
	}
}


#[test]
fn poll_consume_same_while_polling() {
	let bus = MessageBus::new();
	let subscription = bus.subscribe::<u32>();
	let subscription2 = bus.subscribe::<u32>();

	bus.emit(1u32);
	bus.emit(10u32);
	bus.emit(100u32);
	bus.emit(1000u32);

	let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
		for value in bus.poll(&subscription) {
			let mut hits = 0;
			let mut consume_hits = 0;

			for value in bus.poll_consume(&subscription2).take(2) {
				consume_hits += value;
			}

			hits += value;

			assert_eq!(consume_hits, 1111, "inner poll_consume should see every value once");
			assert_eq!(hits, 101, "poll_consume in the middle of a poll should truncate remaining messages seen by poll");
		}
	}));

	assert!(result.is_err(), "poll_consume while another poll of the same message type is active should panic");

}

#[test]
fn multiple_same_subs() {
	let bus = MessageBus::new();

	bus.emit(EmptyMessage);
	let subscription_1 = bus.subscribe::<EmptyMessage>();

	bus.emit(EmptyMessage);
	let subscription_2 = bus.subscribe::<EmptyMessage>();

	bus.emit(EmptyMessage);

	assert_eq!(bus.poll(&subscription_1).count(), 2, "New subscriptions shouldn't see prior messages");
	assert_eq!(bus.poll(&subscription_2).count(), 1, "New subscriptions shouldn't see prior messages");

	bus.emit(EmptyMessage);
	bus.emit(EmptyMessage);

	assert_eq!(bus.poll(&subscription_1).count(), 2, "Subscriptions should see _all_ new messages");
	assert_eq!(bus.poll(&subscription_1).count(), 0, "Subscriptions should only see each message once");

	bus.emit(EmptyMessage);
	bus.emit(EmptyMessage);

	bus.garbage_collect();

	assert!(bus.peek_any(&subscription_1), "Existing subscriptions should still see new messages after garbage_collect");
	assert_eq!(bus.poll(&subscription_1).count(), 2, "Subscriptions should still see _all_ new messages after garbage_collect");

	assert!(bus.peek_any(&subscription_2));
	assert_eq!(bus.poll(&subscription_2).count(), 4);
}