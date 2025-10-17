// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::super::processing_node::{
    ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor,
};
use wasmtime::*;

pub struct CStyleNodeExecutor {
    module: Arc<Module>,
    engine: Arc<Engine>,
}

impl CStyleNodeExecutor {
    pub fn new(module: Module, engine: Engine) -> Result<Self, ProcessingNodeError> {
        Ok(Self {
            module: Arc::new(module),
            engine: Arc::new(engine),
        })
    }
}

impl ProcessingNodeExecutor for CStyleNodeExecutor {
    fn execute(&self, input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError> {
        let mut store = Store::new(&self.engine, ());

        store
            .set_fuel(100_000_000)
            .map_err(|e| ProcessingNodeError::RuntimeError(e.to_string()))?;

        let instance = Instance::new(&mut store, &self.module, &[])
            .map_err(|e| ProcessingNodeError::RuntimeError(e.to_string()))?;

        self.execute_c_style_process(&mut store, &instance, input)
    }

    fn artifact_type(&self) -> &'static str {
        "C-Style"
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "c-style".to_string(),
            "sandboxed".to_string(),
            "no-system-access".to_string(),
        ]
    }

    fn execution_metadata(&self) -> ExecutionMetadata {
        ExecutionMetadata {
            module_path: "".to_string(), // Path no longer stored in executor
            artifact_type: self.artifact_type().to_string(),
            import_count: 0, // Import tracking removed (will be added back via factory if needed)
            capabilities: self.capabilities(),
        }
    }
}

impl CStyleNodeExecutor {
    fn execute_c_style_process(
        &self,
        store: &mut Store<()>,
        instance: &Instance,
        input: &[u8],
    ) -> Result<Vec<u8>, ProcessingNodeError> {
        let memory = instance.get_memory(&mut *store, "memory").ok_or_else(|| {
            ProcessingNodeError::ValidationError("WASM module must export 'memory'".to_string())
        })?;

        let process_func = instance
            .get_typed_func::<(i32, i32, i32), i32>(&mut *store, "process")
            .map_err(|_| ProcessingNodeError::ValidationError("WASM module must export 'process' function with signature (i32, i32, i32) -> i32".to_string()))?;

        let allocate_func = instance
            .get_typed_func::<i32, i32>(&mut *store, "allocate")
            .map_err(|_| {
                ProcessingNodeError::ValidationError(
                    "WASM module must export 'allocate' function with signature (i32) -> i32"
                        .to_string(),
                )
            })?;

        let deallocate_func = instance
            .get_typed_func::<(i32, i32), ()>(&mut *store, "deallocate")
            .map_err(|_| {
                ProcessingNodeError::ValidationError(
                    "WASM module must export 'deallocate' function with signature (i32, i32) -> ()"
                        .to_string(),
                )
            })?;

        // Use input bytes directly - no string conversion needed
        let input_bytes = input;

        // Allocate memory in WASM for input
        let input_ptr = allocate_func
            .call(&mut *store, input_bytes.len() as i32)
            .map_err(|e| {
                ProcessingNodeError::RuntimeError(format!(
                    "Failed to call allocate function: {}",
                    e
                ))
            })?;
        if input_ptr == 0 {
            return Err(ProcessingNodeError::RuntimeError(
                "Failed to allocate input memory".to_string(),
            ));
        }

        // Write input to WASM memory
        memory
            .write(&mut *store, input_ptr as usize, input_bytes)
            .map_err(|e| {
                ProcessingNodeError::RuntimeError(format!(
                    "Failed to write input to WASM memory: {}",
                    e
                ))
            })?;

        // Allocate memory for output length
        let output_len_ptr = allocate_func.call(&mut *store, 4).map_err(|e| {
            ProcessingNodeError::RuntimeError(format!(
                "Failed to allocate output length memory: {}",
                e
            ))
        })?; // 4 bytes for i32
        if output_len_ptr == 0 {
            // Clean up input memory
            let _ = deallocate_func.call(&mut *store, (input_ptr, input_bytes.len() as i32));
            return Err(ProcessingNodeError::RuntimeError(
                "Failed to allocate output length memory".to_string(),
            ));
        }

        // Call the process function
        let result_ptr = process_func
            .call(
                &mut *store,
                (input_ptr, input_bytes.len() as i32, output_len_ptr),
            )
            .map_err(|e| {
                // Clean up allocated memory
                let _ = deallocate_func.call(&mut *store, (input_ptr, input_bytes.len() as i32));
                let _ = deallocate_func.call(&mut *store, (output_len_ptr, 4));
                ProcessingNodeError::RuntimeError(format!("Failed to call process function: {}", e))
            })?;

        // Clean up input memory
        deallocate_func
            .call(&mut *store, (input_ptr, input_bytes.len() as i32))
            .map_err(|e| {
                ProcessingNodeError::RuntimeError(format!(
                    "Failed to deallocate input memory: {}",
                    e
                ))
            })?;

        if result_ptr == 0 {
            // Clean up output length memory
            let _ = deallocate_func.call(&mut *store, (output_len_ptr, 4));
            return Err(ProcessingNodeError::RuntimeError(
                "WASM process function returned null pointer".to_string(),
            ));
        }

        // Read output length
        let mut output_len_bytes = [0u8; 4];
        memory
            .read(&mut *store, output_len_ptr as usize, &mut output_len_bytes)
            .map_err(|e| {
                ProcessingNodeError::RuntimeError(format!("Failed to read output length: {}", e))
            })?;
        let output_len = i32::from_le_bytes(output_len_bytes) as usize;

        // Clean up output length memory
        deallocate_func
            .call(&mut *store, (output_len_ptr, 4))
            .map_err(|e| {
                ProcessingNodeError::RuntimeError(format!(
                    "Failed to deallocate output length memory: {}",
                    e
                ))
            })?;

        if output_len == 0 {
            // Clean up result memory
            let _ = deallocate_func.call(&mut *store, (result_ptr, 1)); // Minimal cleanup
            return Ok(Vec::new());
        }

        // Read output data
        let mut output_bytes = vec![0u8; output_len];
        memory
            .read(&mut *store, result_ptr as usize, &mut output_bytes)
            .map_err(|e| {
                ProcessingNodeError::RuntimeError(format!("Failed to read output data: {}", e))
            })?;

        // Clean up result memory
        deallocate_func
            .call(&mut *store, (result_ptr, output_len as i32))
            .map_err(|e| {
                ProcessingNodeError::RuntimeError(format!(
                    "Failed to deallocate result memory: {}",
                    e
                ))
            })?;

        // Return the raw output bytes - let the caller handle any necessary conversion
        Ok(output_bytes)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use crate::backends::wasm::create_executor;

    #[test]
    fn test_cstyle_executor_with_wasm_appender() {
        // Path to the built wasm_appender component
        let wasm_path = Path::new("wasm_components/wasm_appender.wasm");

        // Skip test if the WASM file doesn't exist
        if !wasm_path.exists() {
            println!("Skipping test: wasm_appender.wasm not found. Run 'cd wasm_components/wasm_appender && ./build.sh' to build it.");
            return;
        }

        // Load the WASM module using the new ADR-17 flow
        use crate::backends::wasm::{load_wasm_bytes, wasm_encoding};

        let bytes = load_wasm_bytes(wasm_path).expect("Failed to load wasm_appender module");
        let encoding = wasm_encoding(&bytes).expect("Failed to detect encoding");
        let executor = create_executor(&bytes, encoding)
            .expect("Failed to create CStyleNodeExecutor");

        // Test input
        let input = b"hello";

        // Execute the module (synchronously)
        let result = executor
            .execute(input)
            .expect("Failed to execute WASM module");

        // Convert result back to string for verification
        let output = String::from_utf8(result).expect("Output is not valid UTF-8");

        // The wasm_appender should append "::WASM" to the input
        assert_eq!(output, "hello::WASM");

        // Verify metadata
        let metadata = executor.execution_metadata();
        assert_eq!(metadata.artifact_type, "C-Style");
        assert!(metadata.capabilities.contains(&"c-style".to_string()));
        assert!(metadata.capabilities.contains(&"sandboxed".to_string()));
    }
}
