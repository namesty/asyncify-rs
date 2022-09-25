#[macro_export]
macro_rules! import {
    ($state:ident, fn $name:ident ($($arg:ident : $argtype:ty),*) -> $ret:ty = $body:expr) => {
      #[allow(unused_mut, unused_parens)]  
      pub fn $name($($arg: $argtype),*) -> $ret {
            state.get()
            println!("Hello, world!");
            $body
        }
    };
}

// if (this._getAsyncifyState() === AsyncifyState.Rewinding) {
//   this._wrappedExports.asyncify_stop_rewind();
//   return this._returnValue;
// }
// this._assertNoneState();