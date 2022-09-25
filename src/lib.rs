use std::collections::HashMap;
use wasmer::{Module, Store, Instance, ImportObject, Function, Value, RuntimeError};

pub mod macros;

struct AsyncifyExports {
  pub asyncify_get_state: Box<Function>,
  pub asyncify_start_unwind: Box<Function>,
  pub asyncify_stop_rewind: Box<Function>,
  pub asyncify_start_rewind: Box<Function>,
  pub asyncify_stop_unwind: Box<Function>,
}

struct AsyncifyWasmerInstance {
  pub asyncify_exports: AsyncifyExports,
  pub instance: Instance,
  pub return_value: Option<Box<[Value]>>,
}

#[derive(PartialEq)]
enum AsyncifyState {
  None,
  Rewinding,
  Unwinding
}

impl AsyncifyWasmerInstance {
  pub fn new(module: Module) -> Self {
    let import_object = ImportObject::new();
    let instance = Instance::new(&module, &import_object).unwrap();

    Self {
      return_value: Option::None,
      asyncify_exports: AsyncifyExports {
        asyncify_get_state: Box::new(instance.exports.get_function("asyncify_get_state").unwrap().clone()),
        asyncify_start_unwind: Box::new(instance.exports.get_function("asyncify_start_unwind").unwrap().clone()),
        asyncify_stop_rewind: Box::new(instance.exports.get_function("asyncify_stop_rewind").unwrap().clone()),
        asyncify_start_rewind: Box::new(instance.exports.get_function("asyncify_start_rewind").unwrap().clone()),
        asyncify_stop_unwind: Box::new(instance.exports.get_function("asyncify_stop_unwind").unwrap().clone()),
      },
      instance,
    }
  }

  fn _wrap_exports(&self) -> HashMap<String, impl Fn(&[Value]) -> Box<[Value]> + '_> {
    let mut aux = HashMap::new();
    for (name, func) in self.instance.exports.iter().functions() {
      if !name.starts_with("asyncify_") {
        let wrapped = | args: &[Value] | {
          self._assert_none_state();
    
          let mut result = func.call(args);
          
          while self._get_asyncify_state() == AsyncifyState::Unwinding {
            self.asyncify_exports.asyncify_stop_unwind.call(&[]).unwrap();
    
            // self.return_value = self.return_value;
            self._assert_none_state();
            self.asyncify_exports.asyncify_start_rewind.call(
              // TODO: pass the return value
              &[]
            );
            result = func.call(&[]);
          }
    
          self._assert_none_state();
    
          result.unwrap().clone()
        };

        aux.insert(name.to_string(), Box::new(wrapped));
      }
    }
    
    aux
  }

  fn _assert_none_state(&self) {
    let state = self._get_asyncify_state();
    if state != AsyncifyState::None {
      panic!("Asyncify state is not None");
    }
  }

  fn _get_asyncify_state(&self) -> AsyncifyState {
    match self.asyncify_exports.asyncify_get_state.call(&[]).unwrap().first().unwrap().i32().unwrap() {
      0 => AsyncifyState::None,
      1 => AsyncifyState::Rewinding,
      2 => AsyncifyState::Unwinding,
      _ => panic!("Invalid asyncify state")
    }
  }

}

pub fn main() {
  let module_wat = r#"
    (module
      (type $t0 (func (param i32) (result i32)))
      (func $add_one (export "add_one") (type $t0) (param $p0 i32) (result i32)
        get_local $p0
        i32.const 1
        i32.add))
    "#;
    let store = Store::default();
    let module = Module::new(&store, &module_wat).unwrap();

    let instance = AsyncifyWasmerInstance::new(module);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        crate::main();
    }
}
