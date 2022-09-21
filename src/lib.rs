use std::future::Future;
use std::io::Bytes;
use wasmer::{Function, ImportObject, Memory, MemoryType};

struct AsyncifyInstance { }

pub fn index_of_array(source: &[u8], search: &[u8]) -> i32 {
    let mut run = true;
    let mut start = 0;

    while run {
        let mut iterator = source.iter();
        iterator.advance_by(start).unwrap();

        let idx = iterator.position(|&r| r == search[0]).unwrap();

        if idx < start {
            run = false;
            continue;
        }

        let sub_buff = source[idx..(idx + search.len())];

        let mut retry = false;
        let mut i = 0;

        while i < search.len() && !retry {
            if sub_buff[i] != search[i] {
                retry = true;
            }
            i += 1;
        }

        if retry {
            start = idx + i;
            continue;
        } else {
            idx
        }
    }

    -1
}

struct SharedInvokeState {
    method: String,
    args: [u8],
    invoke_result: [u8],
    invoke_error: String,
    env: [u8]
}

impl SharedInvokeState {
    new
}

struct InstanceConfig {
    module: [u8],
    imports: ImportObject,
    required_exports: Option<[String]>
}

impl AsyncifyInstance {
    pub fn new() -> Self {
        Self { }
    }

    fn _wrap_import_fn<F: Future>(&self, function: F) -> Function {

    }

    pub fn create_memory(module: &[u8]) -> Memory {
        let env_memory_import_signature: [u8; 11] = [
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

        let sig_idx = index_of_array(module, &env_memory_import_signature);

        if sig_idx < 0 {
            panic!(
                "Unable to find Wasm memory import section. " +
                "Modules must import memory from the \"env\" module's " +
                "memory field like so:\n" +
                "(import \"env\" \"memory\" (memory (;0;) #))"
            )
        }

        let module_vec: Vec<u8> = module.into_vec();

        match module_vec.get(sig_idx + env_memory_import_signature.len() + 1) {
            None => panic!("No initial memory number found, this should never happen..."),
            Some(memory_initial_limits) => Memory::new(memory_initial_limits, MemoryType::new(1, None, false)).unwrap()
        }
    }

    pub fn create_instance(config: InstanceConfig) {
        let instance = AsyncifyInstance::new();

        instance.
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
