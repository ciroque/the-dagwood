// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WASM Execution Engine and Memory Management
//!
//! This module handles the pure execution of WASM modules with:
//! - WASM module instantiation and execution
//! - Memory management and data marshaling
//! - Fuel-based resource protection
//! - Error handling and fallback strategies
//!
//! ## Responsibilities
//! - Instantiate WASM modules with configured linkers
//! - Execute WASM functions with input/output marshaling
//! - Manage WASM linear memory and allocations
//! - Handle execution errors and timeouts
//! - Provide execution metadata and performance metrics

use crate::backends::wasm::error::{WasmError, WasmResult};
use crate::backends::wasm::capability_manager::WasiSetup;
use crate::backends::wasm::module_loader::{LoadedModule, WasmArtifact};
use std::ffi::{CStr, CString};
use wasmtime::*;
use wasmtime::component::{Component, Linker as ComponentLinker, bindgen};

/// Fuel level for WASM execution (100M instructions)
/// This provides computational budget to prevent infinite loops
const FUEL_LEVEL: u64 = 100_000_000;

/// Maximum input size for WASM processing (1MB)
const MAX_INPUT_SIZE: usize = 1024 * 1024;

// Generate bindings for the DAGwood WIT interface
bindgen!({
    world: "dagwood-component",
    path: "wit/dagwood-processor.wit",
    async: false,
});

/// Execution result with performance metadata
#[derive(Debug)]
pub struct ExecutionResult {
    pub output: String,
    pub fuel_consumed: u64,
    pub input_size: usize,
    pub output_size: usize,
    pub execution_time_ms: u64,
}

/// WASM Executor - handles pure execution and memory management
pub struct WasmExecutor;

impl WasmExecutor {
    /// Execute a WASM module with the given input
    pub fn execute(
        loaded_module: &LoadedModule,
        wasi_setup: WasiSetup,
        input: &str,
    ) -> WasmResult<ExecutionResult> {
        let start_time = std::time::Instant::now();
        let input_size = input.len();

        // Validate input size
        if input_size > MAX_INPUT_SIZE {
            return Err(WasmError::ValidationError(format!(
                "Input too large: {} bytes (max: {} bytes)",
                input_size,
                MAX_INPUT_SIZE
            )));
        }

        // Create store with WASI context
        let mut store = Store::new(&loaded_module.engine, wasi_setup.store_data);

        // Set fuel limit for security and resource protection
        store.set_fuel(FUEL_LEVEL)?;

        // Execute based on artifact type
        let output = match &loaded_module.artifact {
            WasmArtifact::Module(module) => {
                // Instantiate and execute core WASM module
                let instance = wasi_setup.linker.instantiate(&mut store, module)?;
                Self::execute_c_style(&mut store, &instance, input)?
            }
            WasmArtifact::Component(component) => {
                // Execute WIT component using generated bindings
                Self::execute_wit_component(&mut store, component, input)?
            }
        };

        // Calculate execution metrics
        let fuel_consumed = FUEL_LEVEL - store.get_fuel().unwrap_or(0);
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        let output_size = output.len();

        Ok(ExecutionResult {
            output,
            fuel_consumed,
            input_size,
            output_size,
            execution_time_ms,
        })
    }

    /// Execute C-style WASM module (process, allocate, deallocate functions)
    fn execute_c_style(
        store: &mut Store<()>,
        instance: &Instance,
        input: &str,
    ) -> WasmResult<String> {
        // Get the module's memory
        let memory = instance
            .get_memory(&mut *store, "memory")
            .ok_or_else(|| WasmError::ValidationError("WASM module must export 'memory'".to_string()))?;

        // Get required functions
        let process_func = instance
            .get_typed_func::<(i32, i32, i32), i32>(&mut *store, "process")
            .map_err(|_| WasmError::ValidationError("WASM module must export 'process' function with signature (i32, i32, i32) -> i32".to_string()))?;

        let allocate_func = instance
            .get_typed_func::<i32, i32>(&mut *store, "allocate")
            .map_err(|_| WasmError::ValidationError("WASM module must export 'allocate' function with signature (i32) -> i32".to_string()))?;

        let deallocate_func = instance
            .get_typed_func::<(i32, i32), ()>(&mut *store, "deallocate")
            .map_err(|_| WasmError::ValidationError("WASM module must export 'deallocate' function with signature (i32, i32) -> ()".to_string()))?;

        // Convert input to C string
        let input_cstring = CString::new(input)
            .map_err(|_| WasmError::ValidationError("Input contains null bytes".to_string()))?;
        let input_bytes = input_cstring.as_bytes_with_nul();

        // Allocate memory in WASM for input
        let input_ptr = allocate_func.call(&mut *store, input_bytes.len() as i32)?;
        if input_ptr == 0 {
            return Err(WasmError::MemoryError("Failed to allocate input memory".to_string()));
        }

        // Write input to WASM memory
        memory.write(&mut *store, input_ptr as usize, input_bytes)
            .map_err(|e| WasmError::MemoryError(format!("Failed to write input to WASM memory: {}", e)))?;

        // Allocate memory for output length
        let output_len_ptr = allocate_func.call(&mut *store, 4)?; // 4 bytes for i32
        if output_len_ptr == 0 {
            // Clean up input memory
            deallocate_func.call(&mut *store, (input_ptr, input_bytes.len() as i32))?;
            return Err(WasmError::MemoryError("Failed to allocate output length memory".to_string()));
        }

        // Call the process function
        let result_ptr = process_func.call(&mut *store, (input_ptr, input_bytes.len() as i32, output_len_ptr))?;

        // Clean up input memory
        deallocate_func.call(&mut *store, (input_ptr, input_bytes.len() as i32))?;

        if result_ptr == 0 {
            // Clean up output length memory
            deallocate_func.call(&mut *store, (output_len_ptr, 4))?;
            return Err(WasmError::ExecutionError(anyhow::anyhow!("WASM process function returned null pointer")));
        }

        // Read output length
        let mut output_len_bytes = [0u8; 4];
        memory.read(&mut *store, output_len_ptr as usize, &mut output_len_bytes)
            .map_err(|e| WasmError::MemoryError(format!("Failed to read output length: {}", e)))?;
        let output_len = i32::from_le_bytes(output_len_bytes) as usize;

        // Clean up output length memory
        deallocate_func.call(&mut *store, (output_len_ptr, 4))?;

        if output_len == 0 {
            // Clean up result memory
            deallocate_func.call(&mut *store, (result_ptr, 1))?; // Minimal cleanup
            return Ok(String::new());
        }

        // Read output data
        let mut output_bytes = vec![0u8; output_len];
        memory.read(&mut *store, result_ptr as usize, &mut output_bytes)
            .map_err(|e| WasmError::MemoryError(format!("Failed to read output data: {}", e)))?;

        // Clean up result memory
        deallocate_func.call(&mut *store, (result_ptr, output_len as i32))?;

        // Convert output to string (remove null terminator if present)
        // Ensure null terminator exists
        if output_bytes.last() != Some(&0) {
            output_bytes.push(0);
        }
        
        let output_cstr = CStr::from_bytes_with_nul(&output_bytes)
            .map_err(|e| WasmError::StringError(format!("Invalid C string in output: {}", e)))?;

        let output = output_cstr.to_str()
            .map_err(|e| WasmError::StringError(format!("Invalid UTF-8 in output: {}", e)))?
            .to_string();

        Ok(output)
    }

    /// Execute WIT component using generated bindings
    fn execute_wit_component(
        store: &mut Store<()>,
        component: &Component,
        input: &str,
    ) -> WasmResult<String> {
        // Create component linker
        let linker = ComponentLinker::new(store.engine());
        
        // Instantiate the component
        let instance = linker.instantiate(&mut *store, component)
            .map_err(|e| WasmError::ExecutionError(e.into()))?;
        
        // Get the component interface
        let _dagwood_component = DagwoodComponent::new(store, &instance)
            .map_err(|e| WasmError::ExecutionError(e.into()))?;
        
        // For Phase 2.1: Simplified WIT execution
        // This is a placeholder implementation that demonstrates WIT component loading
        // Full implementation will handle:
        // 1. Memory allocation in component linear memory
        // 2. Calling the actual process function with proper WIT types
        // 3. Handling WIT result<T, E> return types
        // 4. Memory cleanup and deallocation
        
        let output = format!("{}-wit-processed", input);
        
        tracing::info!("WIT component execution successful (placeholder - component loaded and instantiated)");
        Ok(output)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::wasm::capability_manager::CapabilityManager;
    use crate::backends::wasm::module_loader::WasmModuleLoader;
    use tempfile::NamedTempFile;
    use std::io::Write;

    fn create_test_wasm_module() -> Vec<u8> {
        // Create a simple WASM module that appends "-processed" to input
        // This is a simpler version that should work correctly
        wat::parse_str(r#"
            (module
                (memory (export "memory") 1)
                
                (func $allocate (export "allocate") (param $size i32) (result i32)
                    ;; Simple bump allocator starting at offset 1000
                    (i32.const 1000)
                )
                
                (func $deallocate (export "deallocate") (param $ptr i32) (param $size i32)
                    ;; No-op deallocator for testing
                )
                
                (func $process (export "process") (param $input_ptr i32) (param $input_len i32) (param $output_len_ptr i32) (result i32)
                    ;; For testing, just return a simple fixed string "test-processed"
                    ;; Store the string at offset 2000
                    (i32.store8 (i32.const 2000) (i32.const 116)) ;; 't'
                    (i32.store8 (i32.const 2001) (i32.const 101)) ;; 'e'
                    (i32.store8 (i32.const 2002) (i32.const 115)) ;; 's'
                    (i32.store8 (i32.const 2003) (i32.const 116)) ;; 't'
                    (i32.store8 (i32.const 2004) (i32.const 45))  ;; '-'
                    (i32.store8 (i32.const 2005) (i32.const 112)) ;; 'p'
                    (i32.store8 (i32.const 2006) (i32.const 114)) ;; 'r'
                    (i32.store8 (i32.const 2007) (i32.const 111)) ;; 'o'
                    (i32.store8 (i32.const 2008) (i32.const 99))  ;; 'c'
                    (i32.store8 (i32.const 2009) (i32.const 101)) ;; 'e'
                    (i32.store8 (i32.const 2010) (i32.const 115)) ;; 's'
                    (i32.store8 (i32.const 2011) (i32.const 115)) ;; 's'
                    (i32.store8 (i32.const 2012) (i32.const 101)) ;; 'e'
                    (i32.store8 (i32.const 2013) (i32.const 100)) ;; 'd'
                    (i32.store8 (i32.const 2014) (i32.const 0))   ;; null terminator
                    
                    ;; Store output length (15 bytes including null terminator)
                    (i32.store (local.get $output_len_ptr) (i32.const 15))
                    
                    ;; Return output pointer
                    (i32.const 2000)
                )
            )
        "#).unwrap()
    }

    #[test]
    fn test_execute_c_style_success() {
        // Create test WASM module
        let wasm_bytes = create_test_wasm_module();
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(&wasm_bytes).unwrap();

        // Load module
        let loaded_module = WasmModuleLoader::load_module(temp_file.path()).unwrap();
        
        // Create WASI setup
        let requirements = CapabilityManager::analyze_capabilities(&loaded_module);
        let wasi_setup = CapabilityManager::create_wasi_setup(&loaded_module.engine, &requirements).unwrap();

        // Execute
        let result = WasmExecutor::execute(&loaded_module, wasi_setup, "test").unwrap();

        assert_eq!(result.output, "test-processed");
        assert_eq!(result.input_size, 4);
        assert_eq!(result.output_size, 14);
        assert!(result.fuel_consumed > 0);
        // execution_time_ms is u64, so always >= 0
    }

    #[test]
    fn test_input_size_validation() {
        let wasm_bytes = create_test_wasm_module();
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(&wasm_bytes).unwrap();

        let loaded_module = WasmModuleLoader::load_module(temp_file.path()).unwrap();
        let requirements = CapabilityManager::analyze_capabilities(&loaded_module);
        let wasi_setup = CapabilityManager::create_wasi_setup(&loaded_module.engine, &requirements).unwrap();

        // Create input that's too large
        let large_input = "x".repeat(MAX_INPUT_SIZE + 1);
        let result = WasmExecutor::execute(&loaded_module, wasi_setup, &large_input);

        assert!(result.is_err());
        if let Err(WasmError::ValidationError(msg)) = result {
            assert!(msg.contains("Input too large"));
        } else {
            panic!("Expected ValidationError for oversized input");
        }
    }

}
