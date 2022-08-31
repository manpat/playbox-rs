use toybox::prelude::*;

use std::cell::Cell;
use std::future::{Future, IntoFuture};
use std::task::{Context, Poll, Wake};
use std::pin::Pin;
use std::sync::Arc;

thread_local! {
	static CURRENT_ENGINE_PTR: Cell<*mut toybox::Engine> = Cell::new(std::ptr::null_mut());
}




// #[must_not_suspend]
/// Holds temporary ownership of the engine through CURRENT_ENGINE_PTR.
pub struct EngineRef {
	_priv: ()
}

impl std::ops::Deref for EngineRef {
	type Target = toybox::Engine;

	fn deref(&self) -> &toybox::Engine {
		CURRENT_ENGINE_PTR.with(|engine_ptr| {
			unsafe {
				&*engine_ptr.get()
			}
		})
	}
}

impl std::ops::DerefMut for EngineRef {
	fn deref_mut(&mut self) -> &mut toybox::Engine {
		CURRENT_ENGINE_PTR.with(|engine_ptr| {
			unsafe {
				&mut *engine_ptr.get()
			}
		})
	}
}

impl std::ops::Drop for EngineRef {
	fn drop(&mut self) {
		CURRENT_ENGINE_PTR.with(|engine_ptr| {
			engine_ptr.set(std::ptr::null_mut())
		})
	}
}



pub struct NextFrameFuture(Option<()>);

impl Future for NextFrameFuture {
	type Output = EngineRef;

	fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
		CURRENT_ENGINE_PTR.with(|engine_ptr| {
			// Ensure we suspend the first frame we're polled, and consume the engine ptr.
			if let Some(_) = self.0.take() {
				engine_ptr.set(std::ptr::null_mut());
				Poll::Pending

			} else {
				assert!(!engine_ptr.get().is_null(), "CURRENT_ENGINE_PTR unexpectedly null");

				Poll::Ready(EngineRef {
					_priv: ()
				})
			}
		})
	}
}


pub struct NextFrame;

impl IntoFuture for NextFrame {
	type Output = EngineRef;
	type IntoFuture = NextFrameFuture;

	fn into_future(self) -> NextFrameFuture {
		NextFrameFuture(Some(()))
	}
}




struct NullWaker;

impl Wake for NullWaker {
	fn wake(self: Arc<Self>) {}
	fn wake_by_ref(self: &Arc<Self>) {}
}


pub fn run_main_loop<F>(engine: &mut toybox::Engine, mut future: F) -> Result<(), Box<dyn Error>>
	where F: Future<Output=Result<(), Box<dyn Error>>>
{
	// Pin the future so it can be polled.
	let mut future = unsafe {
		Pin::new_unchecked(&mut future)
	};

	// Create a new context to be passed to the future.
	let waker = Arc::new(NullWaker).into();

	// Run the future to completion.
	loop {
		engine.process_events();

		if engine.should_quit() {
			break
		}

		// Set up context for the future - which will either consume it when polled, or complete.
		CURRENT_ENGINE_PTR.with(|engine_ptr| {
			engine_ptr.set(engine);
		});

		let mut context = Context::from_waker(&waker);

		if let Poll::Ready(result) = future.as_mut().poll(&mut context) {
			result?;
			break
		}

		// If the future has not completed, then it must have consumed its context.
		CURRENT_ENGINE_PTR.with(|engine_ptr| {
			assert!(engine_ptr.get().is_null(), "EngineRef held across suspend point");
		});

		engine.end_frame();
	}

	engine.end_frame();

	Ok(())
}