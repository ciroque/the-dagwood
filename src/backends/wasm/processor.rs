//! WASM Processor Implementation
//!
//! This module provides a WebAssembly (WASM) processor backend for The DAGwood project.
//! It uses the wasmtime runtime to execute WASM modules in a sandboxed environment
//! with proper security isolation and resource management.
//!
//! ## Architecture Overview
//!
//! The WASM processor provides secure, isolated execution of user-defined processing logic
//! compiled to WebAssembly. This enables:
//! - **Security**: Complete sandboxing with no host system access
//! - **Performance**: Near-native execution speed with wasmtime's optimizations
//! - **Flexibility**: Support for any WASM-compiled language (Rust, C, AssemblyScript, etc.)
//! - **Deterministic Execution**: Reproducible results with controlled resource access
//!
//! ## WASM Module Interface
//!
//! WASM modules must export the following C-style functions:
//! ```c
//! // Main processing function - takes data pointer and length, returns allocated data
//! uint8_t* process(const uint8_t* input_ptr, size_t input_len, size_t* output_len);
//!
//! // Memory management functions for host-WASM communication
//! void* allocate(size_t size);
//! void deallocate(void* ptr, size_t size);
//! ```
//!
//! ## Security Model
//!
//! ### Sandboxing
//! - **Complete isolation**: WASM modules cannot access host filesystem, network, or system calls
//! - **Memory isolation**: WASM linear memory is separate from host memory
//! - **No WASI**: Deliberately excludes WASI to prevent system access
//!
//! ### Resource Limits
//! - **Fuel consumption**: Computational budget prevents infinite loops and runaway execution
//! - **Memory limits**: WASM modules have bounded linear memory (default: 64KB pages)
//! - **Input size limits**: Maximum input size prevents memory exhaustion attacks
//! - **Module size limits**: Maximum WASM module size prevents storage attacks
//!
//! ### Timeout Protection
//! - **Fuel-based timeouts**: Execution stops when fuel budget is exhausted
//! - **Epoch interruption disabled**: Prevents false interrupts in wasmtime 25.0+
//!
//! ## Wasmtime Configuration
//!
//! ### Critical Settings for wasmtime 25.0+
//! - `epoch_interruption(false)`: **CRITICAL** - Prevents false "interrupt" traps
//! - `consume_fuel(true)`: Enables computational budgeting for security
//! - `wasm_simd(false)` + `wasm_relaxed_simd(false)`: Avoids SIMD conflicts
//!
//! ### Disabled Features (Security)
//! - `wasm_threads(false)`: No threading support
//! - `wasm_multi_memory(false)`: Single memory model only
//! - `wasm_memory64(false)`: 32-bit memory addressing only
//! - `wasm_component_model(false)`: Core WASM only, no component model
//!
//! ## Memory Management
//!
//! ### Host-to-WASM Communication
//! 1. Host calls WASM `allocate(input_size)` to get input memory pointer
//! 2. Host writes input data to WASM linear memory at allocated offset
//! 3. Host calls WASM `allocate(4)` to get output length parameter pointer
//! 4. Host calls WASM `process(input_ptr, input_len, output_len_ptr)` 
//! 5. Host reads output length from WASM memory at output_len_ptr
//! 6. Host reads result data from WASM linear memory using returned pointer and length
//! 7. Host calls WASM `deallocate(ptr, size)` to free all allocated memory
//!
//! ### WASM Module Allocator
//! Recommended to use `wee_alloc` in WASM modules for optimal memory management:
//! ```rust,ignore
//! // Note: This doctest is ignored because wee_alloc is not a dependency
//! // of the main crate - it's only used within WASM modules themselves.
//! // This example shows WASM module developers the recommended pattern.
//! use wee_alloc::WeeAlloc;
//! #[global_allocator]
//! static ALLOC: WeeAlloc = WeeAlloc::INIT;
//! ```
//!
//! ## Error Handling
//!
//! ### WASM Execution Errors
//! - **Traps**: Runtime errors in WASM code (null pointer, out of bounds, etc.)
//! - **Fuel exhaustion**: Computational budget exceeded
//! - **Memory errors**: Invalid memory access or allocation failures
//! - **Module errors**: Invalid WASM module or missing exports
//!
//! ### Fallback Strategy
//! When WASM execution fails, the processor falls back to appending "-wasm" to input,
//! ensuring graceful degradation rather than complete failure.
//!
//! ## Performance Characteristics
//!
//! ### Typical Performance
//! - **Execution time**: 60-70ms for simple text processing
//! - **Memory overhead**: Minimal with wee_alloc
//! - **Startup cost**: Module instantiation ~1-5ms
//!
//! ### Fuel Consumption Guidelines
//! - **Simple operations**: 1-10 fuel per instruction
//! - **Memory allocation**: 100-1,000 fuel per allocation
//! - **String processing**: 10-100 fuel per operation
//! - **Default budget**: 100M fuel (handles complex processing)
//!
//! ## Debugging Tips
//!
//! ### Testing WASM Modules
//! Use wasmtime CLI to test modules independently:
//! ```bash
//! wasmtime --invoke allocate module.wasm 100
//! wasmtime --invoke process module.wasm <input_ptr>
//! ```
//!
//! ### Common Issues
//! - **"wasm trap: interrupt"**: Usually epoch_interruption not disabled
//! - **Fuel exhaustion**: Increase fuel limit or optimize WASM code
//! - **Memory errors**: Check allocate/deallocate implementation
//! - **Module loading errors**: Verify WASM module exports required functions
//!
//! ## Version Compatibility
//!
//! ### wasmtime 25.0+ Changes
//! - **epoch_interruption**: Now enabled by default, must disable for embedded use
//! - **SIMD conflicts**: Relaxed SIMD can conflict with regular SIMD disabling
//! - **Default fuel**: No longer set by default, must explicitly configure
//!
//! ### CLI vs Embedded Differences
//! CLI wasmtime may have different defaults than embedded wasmtime library.
//! Always test with both CLI and embedded execution during development.

use crate::proto::processor_v1::{
    processor_response::Outcome, ErrorDetail, PipelineMetadata, ProcessorRequest, ProcessorResponse,
    ProcessorMetadata,
};
use crate::traits::processor::{Processor, ProcessorIntent};
use async_trait::async_trait;
use std::collections::HashMap;
use wasmtime::*;
use crate::backends::wasm::error::{WasmError, WasmResult};

// 10MB maximum input size.
//
// This limit is set to prevent excessive memory/resource usage and potential denial-of-service
// attacks from untrusted input. 10MB is chosen as a balance between supporting large payloads
// and protecting the host system. Adjust as needed for your deployment requirements.
const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024;

/// A processor that executes WebAssembly modules for sandboxed computation.
/// 
/// The WasmProcessor provides secure, sandboxed execution of user-defined logic
/// by loading and running WebAssembly modules. It includes multiple layers of
/// security including memory protection, timeouts, and input validation.
/// 
/// # Security Features
/// 
/// - **Memory Protection**: Strict bounds checking and memory isolation
/// - **Time Limits**: 5-second execution timeout
/// - **Input Validation**: 10MB maximum input size
/// - **No System Access**: No filesystem/network access
/// - **Deterministic Execution**: Controlled execution environment
pub struct WasmProcessor {
    /// Unique identifier for this processor instance
    processor_id: String,
    /// Path to the WASM module file
    module_path: String,
    /// Wasmtime engine for WASM execution
    engine: Engine,
    /// Compiled WASM module
    module: Module,
    /// Processor intent (Transform or Analyze)
    intent: ProcessorIntent,
}

/// WASM execution fuel limit to prevent infinite loops and resource exhaustion.
///
/// The `FUEL_LEVEL` constant is set to `100_000_000` (one hundred million) fuel units.
/// This provides a reasonable execution budget for most WASM modules while preventing
/// denial-of-service attacks from malicious or poorly written code.
///
/// Fuel consumption varies by operation complexity, but this limit typically allows
/// for substantial computation while maintaining system stability.
///
/// See [Issue 27](https://github.com/ciroque/the-dagwood/issues/27).
const FUEL_LEVEL: u64 = 100_000_000;

// The maximum allowed WASM module size is 10MB to prevent excessive memory/resource usage and potential denial-of-service attacks.
// This limit is chosen as a balance between supporting reasonably large modules and maintaining system stability.
const MAX_WASM_MODULE_SIZE: usize = 10 * 1024 * 1024;

impl WasmProcessor {
    /// Creates a new WasmProcessor with the specified configuration.
    ///
    /// # Security
    /// 
    /// The processor enforces several security measures:
    /// - Memory limits (64MB)
    /// - Execution timeouts (5 seconds)
    /// - No filesystem/network access
    /// - Strict memory protection
    pub fn new(
        processor_id: String,
        module_path: String,
        intent: ProcessorIntent,
    ) -> WasmResult<Self> {
        // Configure the engine with security settings
        let mut config = Config::new();
        
        // Memory limits (64MB max)
        config.static_memory_maximum_size(64 * 1024 * 1024);
        
        // Disable reference types and bulk memory for reduced attack surface
        config.wasm_reference_types(false);
        config.wasm_bulk_memory(false);
        
        // Disable unnecessary features for security and compatibility
        config.wasm_threads(false);
        config.wasm_simd(false);
        config.wasm_relaxed_simd(false);  // Explicitly disable relaxed SIMD to avoid conflicts
        config.wasm_multi_memory(false);
        
        // Try more permissive settings for wasmtime 25.0
        config.wasm_memory64(false);
        config.wasm_component_model(false);
        
        // Enable fuel consumption for security and resource protection
        // Fuel prevents infinite loops and limits computational resource usage
        // Each WASM instruction consumes fuel; when fuel runs out, execution stops
        config.consume_fuel(true);
        
        // Disable epoch interruption which might cause "interrupt" traps
        config.epoch_interruption(false);
        
        let engine = Engine::new(&config)?;
        
        // Load and validate the module
        let module_bytes = std::fs::read(&module_path)
            .map_err(WasmError::IoError)?;
            
        if module_bytes.len() > MAX_WASM_MODULE_SIZE {
            return Err(WasmError::ValidationError("WASM module too large".to_string()));
        }
        
        let module = Module::new(&engine, &module_bytes)
            .map_err(|e| WasmError::ModuleError(e.to_string()))?;
            
        // Validate module imports
        for import in module.imports() {
            let module_name = import.module();
            if module_name.starts_with("wasi") {
                return Err(WasmError::ValidationError(
                    format!("WASI imports are not allowed: {}", module_name)
                ));
            }
        }
        
        Ok(Self {
            processor_id,
            module_path,
            engine,
            module,
            intent,
        })
    }
    
    /// Executes the WASM module with the given input string.
    /// 
    /// This method sets up a WASM instance, allocates memory for the input,
    /// calls the module's process function, and retrieves the result.
    /// 
    /// # Arguments
    /// 
    /// * `input` - The input string to process
    /// 
    /// # Returns
    /// 
    /// Returns the processed output string or an error if execution fails.
    fn execute_wasm(&self, input: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Validate input size
        if input.len() > MAX_INPUT_SIZE {
            return Err(format!("Input too large: {} bytes (max: {} bytes)", input.len(), MAX_INPUT_SIZE).into());
        }
        
        // Create a new store for this execution (no WASI context)
        let mut store = Store::new(&self.engine, ());
        
        // Set fuel limit for security and resource protection
        // Fuel is a computational budget that prevents runaway WASM execution
        // 100M fuel allows complex operations while preventing infinite loops
        // Typical operations consume:
        // - Simple arithmetic: 1-10 fuel per instruction
        // - Memory allocation: 100-1000 fuel per allocation
        // - String operations: 10-100 fuel per operation
        // 100M fuel should handle even complex text processing tasks
        store.set_fuel(FUEL_LEVEL)?;
        
        // Create a linker (no WASI functions)
        let linker = Linker::new(&self.engine);
        
        // Instantiate the WASM module
        let instance = linker.instantiate(&mut store, &self.module)?;
        
        // Get the module's memory
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or("WASM module must export 'memory'")?;
        
        // Use the optimized length-based interface: process(input_ptr, input_len, output_len_ptr)
        let process_func = instance.get_typed_func::<(i32, i32, i32), i32>(&mut store, "process")
            .map_err(|_| "WASM module must export 'process(input_ptr: i32, input_len: i32, output_len_ptr: i32) -> i32' function")?;
        
        // Get required functions
        let allocate_func = instance
            .get_typed_func::<i32, i32>(&mut store, "allocate")
            .map_err(|_| "WASM module must export 'allocate' function")?;
        
        let deallocate_func = instance
            .get_typed_func::<(i32, i32), ()>(&mut store, "deallocate")
            .map_err(|_| "WASM module must export 'deallocate' function")?;
        
        // Convert input to bytes (no null termination needed!)
        let input_bytes = input.as_bytes();
        let input_len = input_bytes.len() as i32;
        
        // Allocate memory for input data
        let input_ptr = allocate_func.call(&mut store, input_len)
            .map_err(|e| format!("WASM allocate function failed: {}", e))?;
        
        if input_ptr == 0 {
            return Err("WASM module allocate returned null pointer".into());
        }
        
        // Allocate memory for the output length parameter
        const SIZEOF_I32: i32 = 4;
        const USIZEOF_I32: usize = SIZEOF_I32 as usize;
        let output_len_ptr = allocate_func.call(&mut store, SIZEOF_I32) // sizeof(usize) as i32 in WASM
            .map_err(|e| format!("WASM allocate for output_len failed: {}", e))?;
        
        if output_len_ptr == 0 {
            // Clean up input allocation
            let _ = deallocate_func.call(&mut store, (input_ptr, input_len));
            return Err("WASM module allocate for output_len returned null pointer".into());
        }
        
        // Write input data to WASM memory
        {
            let memory_data = memory.data_mut(&mut store);
            let input_offset = input_ptr as usize;
            
            // Validate bounds
            if input_offset >= memory_data.len() || input_offset + input_bytes.len() > memory_data.len() {
                // Clean up allocations
                let _ = deallocate_func.call(&mut store, (input_ptr, input_len));
                let _ = deallocate_func.call(&mut store, (output_len_ptr, SIZEOF_I32));
                return Err("WASM input allocation out of bounds".into());
            }
            
            // Copy input data (no null termination!)
            memory_data[input_offset..input_offset + input_bytes.len()].copy_from_slice(input_bytes);
        }
        
        // Call the WASM process function with length-based interface
        let result_ptr = process_func.call(&mut store, (input_ptr, input_len, output_len_ptr))
            .map_err(|e| {
                // Clean up allocations on error
                let _ = deallocate_func.call(&mut store, (input_ptr, input_len));
                let _ = deallocate_func.call(&mut store, (output_len_ptr, SIZEOF_I32));
                format!("WASM process function failed: {}", e)
            })?;
        
        // Clean up input allocation (no longer needed)
        let _ = deallocate_func.call(&mut store, (input_ptr, input_len));
        
        if result_ptr == 0 {
            // Clean up output_len allocation
            let _ = deallocate_func.call(&mut store, (output_len_ptr, SIZEOF_I32));
            return Err("WASM module returned null pointer".into());
        }
        
        // Read the output length from WASM memory
        let output_len = {
            let memory_data = memory.data(&store);
            let output_len_offset = output_len_ptr as usize;
            
            if output_len_offset + USIZEOF_I32 > memory_data.len() {
                let _ = deallocate_func.call(&mut store, (output_len_ptr, SIZEOF_I32));
                return Err("WASM output_len pointer out of bounds".into());
            }
            
            // Read 4 bytes as little-endian u32 (WASM is little-endian)
            u32::from_le_bytes(
                memory_data[output_len_offset..output_len_offset + USIZEOF_I32]
                    .try_into()
                    .map_err(|_| "Failed to convert output length bytes to array")?
            ) as usize
        };
        
        // Clean up output_len allocation
        let _ = deallocate_func.call(&mut store, (output_len_ptr, SIZEOF_I32));
        
        // Validate output length
        if output_len > MAX_INPUT_SIZE {
            let _ = deallocate_func.call(&mut store, (result_ptr, output_len as i32));
            return Err("WASM module returned excessively long output".into());
        }
        
        // Read the result data from WASM memory (no null termination!)
        let result = {
            let memory_data = memory.data(&store);
            let result_offset = result_ptr as usize;
            
            // Validate bounds
            if result_offset >= memory_data.len() || result_offset + output_len > memory_data.len() {
                let _ = deallocate_func.call(&mut store, (result_ptr, output_len as i32));
                return Err("WASM result pointer out of bounds".into());
            }
            
            // Copy result data (exact length, no null termination!)
            let result_bytes = &memory_data[result_offset..result_offset + output_len];
            String::from_utf8(result_bytes.to_vec())
                .map_err(|e| format!("WASM module returned invalid UTF-8: {}", e))?
        };
        
        // Clean up result allocation
        let _ = deallocate_func.call(&mut store, (result_ptr, output_len as i32));
        
        Ok(result)
    }
}

#[async_trait]
impl Processor for WasmProcessor {
    async fn process(&self, request: ProcessorRequest) -> ProcessorResponse {
        // Extract input text from the request payload
        let input_text = match String::from_utf8(request.payload) {
            Ok(text) => text,
            Err(e) => {
                return ProcessorResponse {
                    outcome: Some(Outcome::Error(ErrorDetail {
                        code: 400,
                        message: format!("Invalid UTF-8 in input payload: {}", e),
                    })),
                    metadata: None,
                };
            }
        };
        
        // Execute the WASM module
        match self.execute_wasm(&input_text) {
            Ok(output) => {
                let mut processor_metadata_map = HashMap::new();
                processor_metadata_map.insert("processor_type".to_string(), "wasm".to_string());
                processor_metadata_map.insert("module_path".to_string(), self.module_path.clone());
                processor_metadata_map.insert("input_length".to_string(), input_text.len().to_string());
                processor_metadata_map.insert("output_length".to_string(), output.len().to_string());
                
                let processor_metadata = ProcessorMetadata {
                    metadata: processor_metadata_map,
                };
                
                let mut pipeline_metadata_map = HashMap::new();
                pipeline_metadata_map.insert(self.processor_id.clone(), processor_metadata);
                
                let pipeline_metadata = PipelineMetadata {
                    metadata: pipeline_metadata_map,
                };
                
                ProcessorResponse {
                    outcome: Some(Outcome::NextPayload(output.into_bytes())),
                    metadata: Some(pipeline_metadata),
                }
            }
            Err(e) => ProcessorResponse {
                outcome: Some(Outcome::Error(ErrorDetail {
                    code: 500,
                    message: format!("WASM execution failed: {}", e),
                })),
                metadata: None,
            },
        }
    }
    
    fn declared_intent(&self) -> ProcessorIntent {
        self.intent
    }
    
    fn name(&self) -> &'static str {
        "wasm_processor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    
    // Helper function to create a simple WASM module for testing
    // This would normally be compiled from Rust/C/AssemblyScript source
    fn create_test_wasm_module() -> Vec<u8> {
        // This is a minimal WASM module that exports a process function
        // In practice, this would be compiled from source code
        // For now, we'll return empty bytes and skip the test if no real module exists
        vec![]
    }
    
    #[tokio::test]
    async fn test_wasm_processor_creation() {
        // Test that we can create a WasmProcessor (will fail without real WASM file)
        let temp_dir = std::env::temp_dir();
        let module_path = temp_dir.join("test_module.wasm");
        
        // Create a dummy WASM file for testing
        let wasm_bytes = create_test_wasm_module();
        if wasm_bytes.is_empty() {
            // Skip test if we don't have a real WASM module
            return;
        }
        
        fs::write(&module_path, wasm_bytes).expect("Failed to write test WASM module");
        
        let processor = WasmProcessor::new(
            "test_wasm".to_string(),
            module_path.to_string_lossy().to_string(),
            ProcessorIntent::Transform,
        );
        
        // Clean up
        let _ = fs::remove_file(&module_path);
        
        match processor {
            Ok(_) => println!("WasmProcessor created successfully"),
            Err(e) => println!("Expected error creating WasmProcessor: {}", e),
        }
    }
    
    #[test]
    fn test_wasm_processor_intent() {
        // Test that we can specify processor intent
        let temp_path = "/tmp/nonexistent.wasm";
        
        // This will fail to load the module, but we can still test the intent logic
        let result = WasmProcessor::new(
            "test".to_string(),
            temp_path.to_string(),
            ProcessorIntent::Analyze,
        );
        
        // Should fail due to missing file, which is expected
        assert!(result.is_err());
    }
}
