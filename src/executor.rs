use toybox::prelude::*;

use std::cell::Cell;
use std::future::Future;
use std::task::{Context, Poll, Wake};
use std::pin::Pin;
use std::sync::Arc;



fn exchange_global_engine_ptr(new_ptr: *mut toybox::Engine) -> *mut toybox::Engine {
	thread_local! {
		/// Pointer to the engine during execution of a future.
		/// Whoever has this pointer has exclusive access to the engine.
		/// Will be null outside of the executor context and when claimed by an EngineRef.
		static CURRENT_ENGINE_PTR: Cell<*mut toybox::Engine> = Cell::new(std::ptr::null_mut());
	}

	CURRENT_ENGINE_PTR.with(|engine_ptr| engine_ptr.replace(new_ptr))
}





/// Holds temporary ownership of the engine through CURRENT_ENGINE_PTR.
#[must_not_suspend]
pub struct EngineRef {
	engine_ptr: *mut toybox::Engine
}

impl EngineRef {
	fn new() -> EngineRef {
		let engine_ptr = exchange_global_engine_ptr(std::ptr::null_mut());
		assert!(!engine_ptr.is_null(), "CURRENT_ENGINE_PTR unexpectedly null. Either not in executor context or EngineRef already exists.");
		EngineRef { engine_ptr }
	}
}

impl std::ops::Deref for EngineRef {
	type Target = toybox::Engine;

	fn deref(&self) -> &toybox::Engine {
		unsafe {
			&*self.engine_ptr
		}
	}
}

impl std::ops::DerefMut for EngineRef {
	fn deref_mut(&mut self) -> &mut toybox::Engine {
		unsafe {
			&mut *self.engine_ptr
		}
	}
}

impl std::ops::Drop for EngineRef {
	fn drop(&mut self) {
		let prev_value = exchange_global_engine_ptr(self.engine_ptr);
		assert!(prev_value.is_null(), "CURRENT_ENGINE_PTR is non-null while dropping EngineRef");
	}
}



struct YieldToExecutorFuture(Option<()>);

impl Future for YieldToExecutorFuture {
	type Output = EngineRef;

	fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
		// Ensure we suspend the first frame we're polled, and consume the engine ptr.
		if let Some(_) = self.0.take() {
			Poll::Pending
		} else {
			Poll::Ready(EngineRef::new())
		}
	}
}



// TODO(pat.m): could this also create a new resource scope?
pub async fn start_loop() -> EngineRef {
	EngineRef::new()
}



pub async fn next_frame(engine_ref: EngineRef) -> EngineRef {
	drop(engine_ref);
	YieldToExecutorFuture(Some(())).await
}




struct NullWaker;

impl Wake for NullWaker {
	fn wake(self: Arc<Self>) {}
	fn wake_by_ref(self: &Arc<Self>) {}
}


pub fn run_main_loop<F>(engine: &mut toybox::Engine, future: F) -> Result<(), Box<dyn Error>>
	where F: Future<Output=Result<(), Box<dyn Error>>>
{
	use tracing::{info_span, Instrument};

	let mut future = future.instrument(info_span!("executor::run_main_loop"));

	// Pin the future so it can be polled.
	let mut future = unsafe {
		Pin::new_unchecked(&mut future)
	};

	// Create a new context to be passed to the future.
	let waker = Arc::new(NullWaker).into();

	// Run the future to completion.
	'main: loop {
		engine.process_events();

		if engine.should_quit() {
			break 'main
		}

		// Pass exclusive access to the engine to the future through a global channel.
		exchange_global_engine_ptr(engine);

		if let Poll::Ready(result) = future.as_mut().poll(&mut Context::from_waker(&waker)) {
			result?;
			break 'main
		}

		// Reclaim exclusive access from the global channel. Assuming futures are wellbehaved,
		// this will be the same engine ptr passed to exchange_global_engine_ptr above.
		// If it is null then an EngineRef is likely still holding onto exclusive access.
		let prev_engine_ptr = exchange_global_engine_ptr(std::ptr::null_mut());
		assert!(!prev_engine_ptr.is_null(), "EngineRef held across suspend point");

		engine.end_frame();
	}

	engine.end_frame();

	Ok(())
}