use wasmer::{Module, Store, Instance, ImportObject, Function, Value};

pub mod macros;
pub mod asyncify;

struct AsyncifyExports {
  pub asyncify_get_state: Box<Function>,
  pub asyncify_start_unwind: Box<Function>,
  pub asyncify_stop_rewind: Box<Function>,
  pub asyncify_start_rewind: Box<Function>,
  pub asyncify_stop_unwind: Box<Function>,
}

struct AsyncImportCall {
  pub function: Function,
  pub args: Box<[Value]>,
  pub result: Option<Box<[Value]>>
}

struct AsyncifyWasmerInstance {
  pub asyncify_exports: AsyncifyExports,
  pub instance: Instance,
  pub async_import_call: Option<Box<AsyncImportCall>>,
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
      async_import_call: Option::None,
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

  fn call_wrapped_import(&mut self, function: &Function, args: &[Value]) -> Option<Box<[Value]>> {
    if self._get_asyncify_state() == AsyncifyState::Rewinding {
      self.asyncify_exports.asyncify_stop_rewind.call(&[]).unwrap();
      let async_import_call = self.async_import_call.as_mut().unwrap();

      return Option::Some(async_import_call.result.as_ref().unwrap().clone());
    }

    // Paso 2: dentro del export, se ejecuta el import. Y llama a start unwind
    self.asyncify_exports.asyncify_start_unwind.call(&[]).unwrap();

    // Paso se almacena la llamada al actual import, sin ejecutarla.
    self.async_import_call = Some(Box::new(AsyncImportCall {
      function: function.clone(),
      args: args.to_vec().into_boxed_slice(),
      result: Option::None
    }));

    Option::None
  }

  fn call_wrapped_export(&mut self, function: &Function, args: &[Value]) -> Box<[Value]> {
    self._assert_none_state();

    // Paso 1: llamar al export
    let mut result = function.call(args).unwrap();
    
    // Paso 4: termina la primera llamada al export, con el import habiendo iniciado el unwind. 
    while self._get_asyncify_state() == AsyncifyState::Unwinding {
      // Paso 4 (cont): Se llama a stop unwind
      self.asyncify_exports.asyncify_stop_unwind.call(&[]).unwrap();

      // Paso 5: se llama al actual async import
      let async_import_call = self.async_import_call.as_mut().unwrap();
      let import_result = async_import_call.function.call(&async_import_call.args).unwrap();
      self.async_import_call.as_mut().unwrap().result = Option::Some(import_result);

      self._assert_none_state();
      self.asyncify_exports.asyncify_start_rewind.call(
        // TODO: pass the return value
        &[]
      );
      result = function.call(args).unwrap();
    }

    self._assert_none_state();

    result
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
