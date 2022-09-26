// use std::{
//   future::Future,
//   pin::{Pin, self,},
//   sync::{Arc, Mutex},
//   task::{Context, Poll, Waker},
//   thread,
//   time::Duration,
// };
// use futures::future::BoxFuture;

// #[derive(PartialEq)]
// enum AsyncifyState {
//   None,
//   Rewinding,
//   Unwinding
// }

// #[pin_project::pin_project]
// struct ImportFuture {
//   shared_state: Arc<Mutex<SharedState>>,
//   function: wasmer::Function,
//   #[pin]
//   result: Option<BoxFuture<'static, wasmer::Value>>,
// }

// struct AsyncifyExports {
//   pub asyncify_get_state: Box<wasmer::Function>,
//   pub asyncify_start_unwind: Box<wasmer::Function>,
//   pub asyncify_stop_rewind: Box<wasmer::Function>,
//   pub asyncify_start_rewind: Box<wasmer::Function>,
//   pub asyncify_stop_unwind: Box<wasmer::Function>,
// }


// struct SharedState {
//   waker: Option<Waker>,
//   state: AsyncifyState,
//   asyncify_exports: AsyncifyExports,
// }

// impl Future for ImportFuture {
//   type Output = Box<[wasmer::Value]>;

//   fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
//     let mut shared_state = self.shared_state.lock().unwrap();
//     let mut this = self.project();

    

//     if shared_state.state == AsyncifyState::Unwinding {
//       shared_state.asyncify_exports.asyncify_stop_unwind.call(&[]).unwrap();
//       shared_state.result = shared_state.result;
//       // Poll::Ready(shared_state.result.take().unwrap())
//     } else {
//       shared_state.waker = Some(cx.waker().clone());
//       Poll::Pending
//     }
//   }
// }

// impl ImportFuture {
//   pub fn new(function: wasmer::Function) -> Self {
//     let shared_state = Arc::new(Mutex::new(SharedState {
//       result: Option::None,
//       state: AsyncifyState::None,
//       waker: None,
//     }));

//     // let thread_shared_state = shared_state.clone();
//     // thread::spawn(move || {
//     //     thread::sleep(duration);
//     //     let mut shared_state = thread_shared_state.lock().unwrap();
//     //     // Signal that the timer has completed and wake up the last
//     //     // task on which the future was polled, if one exists.
//     //     shared_state.completed = true;
//     //     if let Some(waker) = shared_state.waker.take() {
//     //         waker.wake()
//     //     }
//     // });

//     Self {
//       function,
//       shared_state
//     }
//   }
// }