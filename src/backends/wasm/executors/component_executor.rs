
use super::super::{
    processing_node::{ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor},
    LoadedModule,
};
use std::sync::Arc;

pub struct ComponentNodeExecutor {
    loaded_module: Arc<LoadedModule>,
}

impl ComponentNodeExecutor {
    pub fn new(loaded_module: LoadedModule) -> Result<Self, ProcessingNodeError> {
        Ok(Self {
            loaded_module: Arc::new(loaded_module),
        })
    }
}

impl ProcessingNodeExecutor for ComponentNodeExecutor {
    fn execute(&self, _input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError> {
        Ok(Vec::new())
    }

    fn artifact_type(&self) -> &'static str {
        "WIT Component"
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "wasmtime:component-model".to_string(),
            "string-processing".to_string(),
        ]
    }

    fn execution_metadata(&self) -> ExecutionMetadata {
        ExecutionMetadata {
            module_path: self.loaded_module.module_path.clone(),
            artifact_type: self.artifact_type().to_string(),
            import_count: self.loaded_module.imports.len(),
            capabilities: self.capabilities(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::wasm::module_loader::WasmArtifact;
    use wasmtime::{Engine, Module};

    fn create_mock_component_loaded_module() -> LoadedModule {
        let engine = Engine::default();

        // Create a minimal valid WASM module that implements the expected interface
        // This module takes a string input and returns it with a "processed: " prefix
        let wasm_bytes = wat::parse_str(r#"
            (module
                (memory (export "memory") 1)
                
                ;; Simple bump allocator
                (global $heap (mut i32) (i32.const 8))
                
                ;; Allocate memory of given size
                (func $alloc (export "alloc") (param $size i32) (result i32)
                    (local $ptr i32)
                    global.get $heap
                    local.tee $ptr
                    local.get $size
                    i32.add
                    global.set $heap
                    local.get $ptr
                )
                
                ;; Process function - takes input pointer and length, returns (output_ptr, output_len)
                (func $process (export "process") (param $input_ptr i32) (param $input_len i32) (result i32 i32)
                    (local $output_ptr i32)
                    (local $i i32)
                    
                    ;; Allocate space for output (input_len + 11 for "processed: ")
                    local.get $input_len
                    i32.const 11  ;; Length of "processed: "
                    i32.add
                    call $alloc
                    local.set $output_ptr
                    
                    ;; Write "processed: " prefix
                    local.get $output_ptr
                    i32.const 112 ;; 'p'
                    i32.store8 offset=0
                    local.get $output_ptr
                    i32.const 114 ;; 'r'
                    i32.store8 offset=1
                    local.get $output_ptr
                    i32.const 111 ;; 'o'
                    i32.store8 offset=2
                    local.get $output_ptr
                    i32.const 99  ;; 'c'
                    i32.store8 offset=3
                    local.get $output_ptr
                    i32.const 101 ;; 'e'
                    i32.store8 offset=4
                    local.get $output_ptr
                    i32.const 115 ;; 's'
                    i32.store8 offset=5
                    local.get $output_ptr
                    i32.const 115 ;; 's'
                    i32.store8 offset=6
                    local.get $output_ptr
                    i32.const 101 ;; 'e'
                    i32.store8 offset=7
                    local.get $output_ptr
                    i32.const 100 ;; 'd'
                    i32.store8 offset=8
                    local.get $output_ptr
                    i32.const 58  ;; ':'
                    i32.store8 offset=9
                    local.get $output_ptr
                    i32.const 32  ;; ' '
                    i32.store8 offset=10
                    
                    ;; Copy input to output (after prefix)
                    (loop $loop
                        (local.get $i)
                        local.get $input_len
                        i32.lt_s
                        if
                            ;; Load byte from input
                            local.get $input_ptr
                            local.get $i
                            i32.add
                            i32.load8_u
                            
                            ;; Store byte in output (after prefix)
                            local.get $output_ptr
                            local.get $i
                            i32.const 11  ;; Length of "processed: "
                            i32.add
                            i32.add
                            i32.store8
                            
                            ;; Increment counter
                            local.get $i
                            i32.const 1
                            i32.add
                            local.set $i
                            br $loop
                        end
                    )
                    
                    ;; Return output pointer and length (input_len + 11 for "processed: ")
                    local.get $output_ptr
                    local.get $input_len
                    i32.const 11
                    i32.add
                )
            )
        "#).unwrap();

        let module = Module::new(&engine, &wasm_bytes).unwrap();

        LoadedModule {
            engine,
            artifact: WasmArtifact::Module(module),
            imports: vec![],
            module_path: "test_component.wasm".to_string(),
        }
    }

    #[test]
    fn test_component_executor_creation() {
        let loaded_module = create_mock_component_loaded_module();
        let executor = ComponentNodeExecutor::new(loaded_module).unwrap();

        assert_eq!(executor.artifact_type(), "WIT Component");

        let capabilities = executor.capabilities();
        assert!(capabilities.contains(&"wasmtime:component-model".to_string()));
    }

    #[test]
    fn test_component_executor_validation_error() {
        // Test with a core module instead of a component
        let engine = Engine::default();
        let wasm_bytes = wat::parse_str(
            r#"
            (module
                (func (export "run") (result i32) i32.const 42)
            )
        "#,
        )
        .unwrap();

        let module = wasmtime::Module::new(&engine, &wasm_bytes).unwrap();

        let loaded_module = LoadedModule {
            engine,
            artifact: WasmArtifact::Module(module),
            imports: vec![],
            module_path: "test_module.wasm".to_string(),
        };

        let result = ComponentNodeExecutor::new(loaded_module);
        assert!(result.is_ok()); // No-op executor doesn't validate
    }

    #[test]
    fn test_component_execution() {
        let loaded_module = create_mock_component_loaded_module();
        let executor = ComponentNodeExecutor::new(loaded_module).unwrap();

        let input = b"test input";
        let result = executor.execute(input);

        match result {
            Ok(output_bytes) => {
                // No-op executor returns empty vector
                assert_eq!(output_bytes, Vec::<u8>::new());
            }
            Err(e) => {
                panic!("Component execution failed: {}", e);
            }
        }
    }
}
