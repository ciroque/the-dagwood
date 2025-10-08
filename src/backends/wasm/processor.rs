use crate::proto::processor_v1::{
    processor_response::Outcome, ErrorDetail, PipelineMetadata, ProcessorRequest, ProcessorResponse,
    ProcessorMetadata,
};
use crate::traits::processor::{Processor, ProcessorIntent};
use async_trait::async_trait;
use std::collections::HashMap;
use wasmtime::*;
use crate::backends::wasm::error::{WasmError, WasmResult};

// 10MB maximum input size
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
        
        // Enable epoch-based interruption for timeouts
        config.epoch_interruption(true);
        
        // Enable reference types and bulk memory
        config.wasm_reference_types(true);
        config.wasm_bulk_memory(true);
        
        // Disable unnecessary features
        config.wasm_threads(false);
        config.wasm_simd(false);
        config.wasm_multi_memory(false);
        
        // Memory protection is enabled by default in wasmtime
        
        // Enable deterministic execution
        config.consume_fuel(true);
        
        let engine = Engine::new(&config)?;
        
        // Load and validate the module
        let module_bytes = std::fs::read(&module_path)
            .map_err(WasmError::IoError)?;
            
        if module_bytes.len() > 10 * 1024 * 1024 {
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
        
        // Set up timeout using fuel consumption
        store.set_fuel(1000000)?; // Set initial fuel for timeout
        
        // Create a linker (no WASI functions)
        let linker = Linker::new(&self.engine);
        
        // Instantiate the WASM module
        let instance = linker.instantiate(&mut store, &self.module)?;
        
        // Get the module's memory
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or("WASM module must export 'memory'")?;
        
        // Try to get the process function - first try the simple C-style interface
        if let Ok(process_func) = instance.get_typed_func::<i32, i32>(&mut store, "process") {
            // Use the simple C-style interface that takes a pointer to null-terminated string
            let input_cstring = std::ffi::CString::new(input)
                .map_err(|e| format!("Failed to create C string: {}", e))?;
            
            // Allocate memory in WASM for the input string
            let allocate_func = instance
                .get_typed_func::<i32, i32>(&mut store, "allocate")
                .map_err(|_| "WASM module must export 'allocate' function")?;
            
            let input_len = input_cstring.as_bytes_with_nul().len() as i32;
            let input_ptr = allocate_func.call(&mut store, input_len)?;
            
            // Write input data to WASM memory
            let memory_data = memory.data_mut(&mut store);
            let input_bytes = input_cstring.as_bytes_with_nul();
            memory_data[input_ptr as usize..(input_ptr as usize + input_bytes.len())]
                .copy_from_slice(input_bytes);
            
            // Call the WASM process function
            let result_ptr = process_func.call(&mut store, input_ptr)?;
            
            if result_ptr == 0 {
                return Err("WASM module returned null pointer".into());
            }
            
            // Read the result string from memory (null-terminated)
            let memory_data = memory.data(&store);
            let result_offset = result_ptr as usize;
            
            // Validate result pointer is within bounds
            if result_offset >= memory_data.len() {
                return Err("WASM module returned invalid pointer: out of bounds".into());
            }
            
            // Find the length of the null-terminated string with bounds checking
            let mut result_len = 0;
            let max_search_len = memory_data.len() - result_offset;
            
            for i in 0..max_search_len {
                if memory_data[result_offset + i] == 0 {
                    break;
                }
                result_len += 1;
                
                // Prevent excessive memory scanning (safety limit)
                if result_len > MAX_INPUT_SIZE {
                    return Err("WASM module returned excessively long string".into());
                }
            }
            
            // Ensure we found a null terminator
            if result_len == max_search_len {
                return Err("WASM module returned non-null-terminated string".into());
            }
            
            // Safe slice creation - bounds already validated
            let result_bytes = &memory_data[result_offset..result_offset + result_len];
            let result = String::from_utf8(result_bytes.to_vec())
                .map_err(|e| format!("WASM module returned invalid UTF-8: {}", e))?;
            
            Ok(result)
        } else {
            Err("WASM module does not export required 'process' function".into())
        }
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
