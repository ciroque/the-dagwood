// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! C-Style WASM executor for classic core modules with manual memory management.
//!
//! This executor implements the `ProcessingNodeExecutor` trait for classic WASM modules
//! that follow the C-style calling convention with explicit memory management functions.
//!
//! # Memory Management Convention
//!
//! C-Style modules must export three functions:
//! - **`process(input_ptr: i32, input_len: i32, output_len_ptr: i32) -> i32`**
//!   - Main processing function
//!   - Returns pointer to output data
//! - **`allocate(size: i32) -> i32`**
//!   - Allocates memory in WASM linear memory
//!   - Returns pointer to allocated memory
//! - **`deallocate(ptr: i32, size: i32)`**
//!   - Frees previously allocated memory
//!
//! # Execution Flow
//!
//! 1. Allocate input buffer in WASM memory
//! 2. Write input data to allocated buffer
//! 3. Allocate output length buffer (4 bytes for i32)
//! 4. Call `process()` with input pointer, length, and output length pointer
//! 5. Read output length from output length buffer
//! 6. Read output data from returned pointer
//! 7. Deallocate all buffers
//!
//! # Safety & Error Handling
//!
//! The executor performs comprehensive validation:
//! - Null pointer checks after each allocation
//! - Memory bounds validation
//! - Cleanup on error paths
//! - Fuel limits to prevent infinite loops
//!
//! # Use Cases
//!
//! - Legacy WASM modules compiled from C/C++/Rust
//! - Modules requiring maximum control over memory
//! - Simple processors without WASI dependencies
//! - Performance-critical code with minimal overhead

use std::sync::Arc;

use super::super::processing_node::{
    ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor,
};
use wasmtime::*;

/// Executor for C-Style WASM modules with manual memory management.
///
/// This executor handles classic WASM modules that export `process`, `allocate`,
/// and `deallocate` functions following C-style calling conventions.
///
/// # Thread Safety
/// - Uses `Arc` for shared ownership across threads
/// - Stateless execution (creates new Store per call)
/// - Safe for concurrent use
pub struct CStyleNodeExecutor {
    module: Arc<Module>,
    engine: Arc<Engine>,
    fuel_level: u64,
}

impl CStyleNodeExecutor {
    /// Create a new C-Style executor from a compiled module and engine.
    ///
    /// # Arguments
    /// * `module` - Compiled WASM module
    /// * `engine` - Configured Wasmtime engine
    /// * `fuel_level` - Maximum fuel (instruction count) for execution
    ///
    /// # Returns
    /// * `Ok(CStyleNodeExecutor)` - Ready-to-use executor
    /// * `Err(ProcessingNodeError)` - If initialization fails
    pub fn new(module: Module, engine: Engine, fuel_level: u64) -> Result<Self, ProcessingNodeError> {
        Ok(Self {
            module: Arc::new(module),
            engine: Arc::new(engine),
            fuel_level,
        })
    }
}

impl ProcessingNodeExecutor for CStyleNodeExecutor {
    fn execute(&self, input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError> {
        let mut store = Store::new(&self.engine, ());

        store
            .set_fuel(self.fuel_level)
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
            module_path: "".to_string(),
            artifact_type: self.artifact_type().to_string(),
            import_count: 0,
            capabilities: self.capabilities(),
        }
    }
}

impl CStyleNodeExecutor {
    /// Execute the C-Style process function with manual memory management.
    ///
    /// This internal method handles the complete memory management lifecycle:
    /// 1. Validates required exports (memory, process, allocate, deallocate)
    /// 2. Allocates input buffer and writes input data
    /// 3. Allocates output length buffer
    /// 4. Calls process function
    /// 5. Reads output data
    /// 6. Deallocates all buffers
    ///
    /// # Arguments
    /// * `store` - Wasmtime store with fuel configured
    /// * `instance` - Instantiated WASM module
    /// * `input` - Input data bytes
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - Processed output data
    /// * `Err(ProcessingNodeError)` - If validation, allocation, or execution fails
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

        let input_bytes = input;
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

        memory
            .write(&mut *store, input_ptr as usize, input_bytes)
            .map_err(|e| {
                ProcessingNodeError::RuntimeError(format!(
                    "Failed to write input to WASM memory: {}",
                    e
                ))
            })?;

        let output_len_ptr = allocate_func.call(&mut *store, 4).map_err(|e| {
            ProcessingNodeError::RuntimeError(format!(
                "Failed to allocate output length memory: {}",
                e
            ))
        })?;
        if output_len_ptr == 0 {
            let _ = deallocate_func.call(&mut *store, (input_ptr, input_bytes.len() as i32));
            return Err(ProcessingNodeError::RuntimeError(
                "Failed to allocate output length memory".to_string(),
            ));
        }

        let result_ptr = process_func
            .call(
                &mut *store,
                (input_ptr, input_bytes.len() as i32, output_len_ptr),
            )
            .map_err(|e| {
                let _ = deallocate_func.call(&mut *store, (input_ptr, input_bytes.len() as i32));
                let _ = deallocate_func.call(&mut *store, (output_len_ptr, 4));
                ProcessingNodeError::RuntimeError(format!("Failed to call process function: {}", e))
            })?;

        deallocate_func
            .call(&mut *store, (input_ptr, input_bytes.len() as i32))
            .map_err(|e| {
                ProcessingNodeError::RuntimeError(format!(
                    "Failed to deallocate input memory: {}",
                    e
                ))
            })?;

        if result_ptr == 0 {
            let _ = deallocate_func.call(&mut *store, (output_len_ptr, 4));
            return Err(ProcessingNodeError::RuntimeError(
                "WASM process function returned null pointer".to_string(),
            ));
        }

        let mut output_len_bytes = [0u8; 4];
        memory
            .read(&mut *store, output_len_ptr as usize, &mut output_len_bytes)
            .map_err(|e| {
                ProcessingNodeError::RuntimeError(format!("Failed to read output length: {}", e))
            })?;
        let output_len = i32::from_le_bytes(output_len_bytes) as usize;

        deallocate_func
            .call(&mut *store, (output_len_ptr, 4))
            .map_err(|e| {
                ProcessingNodeError::RuntimeError(format!(
                    "Failed to deallocate output length memory: {}",
                    e
                ))
            })?;

        if output_len == 0 {
            let _ = deallocate_func.call(&mut *store, (result_ptr, 1));
            return Ok(Vec::new());
        }

        let mut output_bytes = vec![0u8; output_len];
        memory
            .read(&mut *store, result_ptr as usize, &mut output_bytes)
            .map_err(|e| {
                ProcessingNodeError::RuntimeError(format!("Failed to read output data: {}", e))
            })?;

        deallocate_func
            .call(&mut *store, (result_ptr, output_len as i32))
            .map_err(|e| {
                ProcessingNodeError::RuntimeError(format!(
                    "Failed to deallocate result memory: {}",
                    e
                ))
            })?;

        Ok(output_bytes)
    }
}

#[cfg(test)]
mod tests {
    use crate::backends::wasm::create_executor;
    use std::path::Path;

    #[test]
    fn test_cstyle_executor_with_wasm_appender() {
        let wasm_path = Path::new("wasm_components/wasm_appender.wasm");

        if !wasm_path.exists() {
            println!("Skipping test: wasm_appender.wasm not found. Run 'cd wasm_components/wasm_appender && ./build.sh' to build it.");
            return;
        }

        use crate::backends::wasm::{detect_component_type, load_wasm_bytes};

        let bytes = load_wasm_bytes(wasm_path).expect("Failed to load wasm_appender module");
        let component_type =
            detect_component_type(&bytes).expect("Failed to detect component type");
        let executor =
            create_executor(&bytes, component_type, 100_000_000).expect("Failed to create CStyleNodeExecutor");

        let input = b"hello";

        let result = executor
            .execute(input)
            .expect("Failed to execute WASM module");

        let output = String::from_utf8(result).expect("Output is not valid UTF-8");

        assert_eq!(output, "hello::WASM");

        let metadata = executor.execution_metadata();
        assert_eq!(metadata.artifact_type, "C-Style");
        assert!(metadata.capabilities.contains(&"c-style".to_string()));
        assert!(metadata.capabilities.contains(&"sandboxed".to_string()));
    }
}
