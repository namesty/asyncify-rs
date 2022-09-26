use wasmer::{Function, ImportObject, Instance, Module, Store, Value, MemoryError, Memory, ImportType, MemoryType};

pub mod macros;
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
    pub result: Option<Box<[Value]>>,
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
    Unwinding,
}

fn index_of_array(source: &[u8], search: &[u8]) -> Option<usize> {
    let mut run = true;
    let mut start = 0;

    while run {
      let mut iterator = source.iter();

      while start > 0 {
        iterator.next();
        start -= 1;
      }

      let idx = iterator.position(|&r| r == search[0]);

      if idx.is_none() {
        run = false;
        continue;
      }

      let sub_buff = &source.clone()[idx.unwrap()..idx.unwrap() + search.len()];

      let mut retry = false;
      let mut i = 0;

      while i < search.len() && !retry {
        if sub_buff[i] != search[i] {
          retry = true;
        }

        i += 1;
      }

      if retry {
        start = idx.unwrap() + 1;
        continue;
      } else {
        return Some(idx.unwrap());
      }
    }

    None
}

impl AsyncifyWasmerInstance {
  const DATA_ADDR: u32 = 16;
  const DATA_START: u32 = AsyncifyWasmerInstance::DATA_ADDR + 8;
  const DATA_END: u32= 1024;
  const ENV_MEMORY_IMPORT_SIG: [u8; 11] = [
    0x65,
    0x6e,
    0x76,
    0x06,
    0x6d,
    0x65,
    0x6d,
    0x6f,
    0x72,
    0x79,
    0x02,
  ];

    pub fn create_memory(store: &Store, module: &[u8]) -> wasmer::Memory {
      let sig_idx = index_of_array(module, &AsyncifyWasmerInstance::ENV_MEMORY_IMPORT_SIG);
      
      if sig_idx.is_none() {
        panic!(r#"
          Unable to find Wasm memory import section.
          Modules must import memory from the "env" module's
          "memory" field like so:
          (import "env" "memory" (memory (;0;) #))
        "#);
      }
  
      // Extract the initial memory page-range size
      let memory_initial_limits = module.get(sig_idx.unwrap() + &AsyncifyWasmerInstance::ENV_MEMORY_IMPORT_SIG.len() + 1);
      
      let memory = if memory_initial_limits.is_none() {
        panic!("No initial memory number found, this should never happen...");
      } else {
        wasmer::Memory::new(
          store,
          wasmer::MemoryType::new(
            memory_initial_limits.unwrap().clone() as u32,
            Some(AsyncifyWasmerInstance::DATA_END),
            false,
          ),
        ).unwrap()
      };

      unsafe {
        let memory_data_slice = *memory.data_unchecked_mut();
        let i32_array = memory_data_slice.iter_mut();
        let offset_start = AsyncifyWasmerInstance::DATA_ADDR as usize;
        let offset_end = (AsyncifyWasmerInstance::DATA_ADDR + 2) as usize;
        memory_data_slice[offset_start..offset_end].copy_from_slice([AsyncifyWasmerInstance::DATA_START, AsyncifyWasmerInstance::DATA_END].as_ref());
      }

      memory
      
    }

    pub fn new(module: &Module) -> Self {
        let import_object = ImportObject::new();
        let instance = Instance::new(&module, &import_object).unwrap();

        Self {
            async_import_call: Option::None,
            asyncify_exports: AsyncifyExports {
                asyncify_get_state: Box::new(
                    instance
                        .exports
                        .get_function("asyncify_get_state")
                        .unwrap()
                        .clone(),
                ),
                asyncify_start_unwind: Box::new(
                    instance
                        .exports
                        .get_function("asyncify_start_unwind")
                        .unwrap()
                        .clone(),
                ),
                asyncify_stop_rewind: Box::new(
                    instance
                        .exports
                        .get_function("asyncify_stop_rewind")
                        .unwrap()
                        .clone(),
                ),
                asyncify_start_rewind: Box::new(
                    instance
                        .exports
                        .get_function("asyncify_start_rewind")
                        .unwrap()
                        .clone(),
                ),
                asyncify_stop_unwind: Box::new(
                    instance
                        .exports
                        .get_function("asyncify_stop_unwind")
                        .unwrap()
                        .clone(),
                ),
            },
            instance,
        }
    }

    fn call_wrapped_import(&mut self, function: &Function, args: &[Value]) -> Option<Box<[Value]>> {
        if self._get_asyncify_state() == AsyncifyState::Rewinding {
            self.asyncify_exports
                .asyncify_stop_rewind
                .call(&[])
                .unwrap();
            let async_import_call = self.async_import_call.as_mut().unwrap();

            return Option::Some(async_import_call.result.as_ref().unwrap().clone());
        }

        // Paso 2: dentro del export, se ejecuta el import. Y llama a start unwind
        self.asyncify_exports
            .asyncify_start_unwind
            .call(&[])
            .unwrap();

        // Paso se almacena la llamada al actual import, sin ejecutarla.
        self.async_import_call = Some(Box::new(AsyncImportCall {
            function: function.clone(),
            args: args.to_vec().into_boxed_slice(),
            result: Option::None,
        }));

        Option::None
    }

    pub fn call_export(&mut self, function: &Function, args: &[Value]) -> Box<[Value]> {
        self._assert_none_state();

        // Paso 1: llamar al export
        let mut result = function.call(args).unwrap();

        // Paso 4: termina la primera llamada al export, con el import habiendo iniciado el unwind.
        while self._get_asyncify_state() == AsyncifyState::Unwinding {
            // Paso 4 (cont): Se llama a stop unwind
            self.asyncify_exports
                .asyncify_stop_unwind
                .call(&[])
                .unwrap();

            // Paso 5: se llama al actual async import
            let async_import_call = self.async_import_call.as_mut().unwrap();
            let async_import_call_function = &async_import_call.function.clone();
            let async_import_call_args = &async_import_call.args.clone();
            let import_result = self
                .call_wrapped_import(async_import_call_function, async_import_call_args)
                .unwrap();
            self.async_import_call.as_mut().unwrap().result = Option::Some(import_result);

            self._assert_none_state();
            self.asyncify_exports.asyncify_start_rewind.call(
                // TODO: pass the return value
                &[],
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
        match self
            .asyncify_exports
            .asyncify_get_state
            .call(&[])
            .unwrap()
            .first()
            .unwrap()
            .i32()
            .unwrap()
        {
            0 => AsyncifyState::None,
            1 => AsyncifyState::Rewinding,
            2 => AsyncifyState::Unwinding,
            _ => panic!("Invalid asyncify state"),
        }
    }
}

pub fn main() {
    let module_wat = r#"
    (module
      (memory 1 1)
      (import "env" "before" (func $before))
      (import "env" "sleep" (func $sleep (param i32)))
      (import "env" "after" (func $after))
      (export "memory" (memory 0))
      (export "main" (func $main))
      (func $main
        (call $before)
        (call $sleep (i32.const 2000))
        (call $after)
      )
    )
    "#;
    let store = Store::default();
    let module = Module::new(&store, &module_wat).unwrap();

    let mut instance = AsyncifyWasmerInstance::new(&module);
    instance.call_export(
        &instance
            .instance
            .exports
            .get_function("main")
            .unwrap()
            .clone(),
        &[],
    );

    let export_memory = instance.instance.exports.get_memory("memory");

    let import_memory = if export_memory.is_err() {
        let imported_memories: Vec<ImportType<MemoryType>> = module
            .imports()
            .memories()
            .collect();

        let first_memory_type = imported_memories.first().unwrap().ty().clone();

        Memory::new(&store, first_memory_type).unwrap()
      } else {
        export_memory.unwrap()
      };
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        crate::main();
    }
}
