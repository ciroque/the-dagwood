use wasmtime::*;
use std::path::PathBuf;

// Test infrastructure moved inline
struct WasmTestRunner {
    engine: Engine,
    module: Module,
}

impl WasmTestRunner {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map_err(|_| "CARGO_MANIFEST_DIR not set")?;
        let wasm_path = PathBuf::from(manifest_dir)
            .join("..")
            .join("wasm_appender.wasm");
        
        if !wasm_path.exists() {
            return Err(format!(
                "WASM module not found at {}. Run 'make wasm-build' at the repository root, or 'make -C wasm_components build-wasm_appender' to compile the module.",
                wasm_path.display()
            ).into());
        }
        
        let engine = Engine::default();
        let module = Module::from_file(&engine, &wasm_path)
            .map_err(|e| format!("Failed to load WASM module from {}: {}", wasm_path.display(), e))?;
        
        Ok(Self { engine, module })
    }
    
    fn run_test<F>(&self, test_fn: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnOnce(&mut Store<()>, &Instance) -> Result<(), Box<dyn std::error::Error>>,
    {
        let mut store = Store::new(&self.engine, ());
        let instance = Instance::new(&mut store, &self.module, &[])?;
        test_fn(&mut store, &instance)
    }
}

fn read_i32_le(memory_data: &[u8], ptr: usize) -> Result<i32, Box<dyn std::error::Error>> {
    // Use safe slice access to avoid manual arithmetic and make intent explicit
    let bytes = memory_data.get(ptr..ptr + 4)
        .ok_or_else(|| format!(
            "Cannot read i32 at offset {}: would read beyond memory bounds (memory size: {})",
            ptr, memory_data.len()
        ))?;
    
    // Convert slice to array safely
    let byte_array: [u8; 4] = bytes.try_into()
        .map_err(|_| "Failed to convert slice to 4-byte array")?;
    
    Ok(i32::from_le_bytes(byte_array))
}

fn write_input_to_memory<T>(
    store: &mut Store<T>,
    memory: &Memory,
    input: &str,
    allocate: &wasmtime::TypedFunc<i32, i32>
) -> Result<(i32, i32), Box<dyn std::error::Error>> {
    let input_len = input.len() as i32;
    let input_ptr = allocate.call(&mut *store, input_len)?;
    
    {
        let memory_data = memory.data_mut(&mut *store);
        let input_bytes = input.as_bytes();
        memory_data[input_ptr as usize..(input_ptr as usize + input_bytes.len())]
            .copy_from_slice(input_bytes);
    }
    
    Ok((input_ptr, input_len))
}

/// WASM function handles for easier test access
struct WasmFunctions {
    process: wasmtime::TypedFunc<(i32, i32, i32), i32>,
    allocate: wasmtime::TypedFunc<i32, i32>,
    deallocate: wasmtime::TypedFunc<(i32, i32), ()>,
    memory: Memory,
}

impl WasmFunctions {
    /// Extract all required WASM function handles from an instance
    fn from_instance(store: &mut Store<()>, instance: &Instance) -> Result<Self, Box<dyn std::error::Error>> {
        let process = instance.get_typed_func::<(i32, i32, i32), i32>(&mut *store, "process")?;
        let allocate = instance.get_typed_func::<i32, i32>(&mut *store, "allocate")?;
        let deallocate = instance.get_typed_func::<(i32, i32), ()>(&mut *store, "deallocate")?;
        let memory = instance.get_memory(&mut *store, "memory")
            .ok_or("WASM module should export memory")?;
        
        Ok(Self {
            process,
            allocate,
            deallocate,
            memory,
        })
    }
    
    /// Run the complete process workflow: allocate → process → read output → deallocate
    /// 
    /// This eliminates the repeated sequence across tests and centralizes memory handling.
    /// Returns None for empty input (null pointer case), otherwise returns the processed string.
    fn run_process(&self, store: &mut Store<()>, input: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        // Handle empty input case - call process with null pointer
        if input.is_empty() {
            // Allocate space for output length
            let output_len_ptr = self.allocate.call(&mut *store, 4)?;
            
            // Call process with empty string (null pointer, 0 length)
            let output_ptr = self.process.call(&mut *store, (0, 0, output_len_ptr))?;
            
            // Read the output length
            let output_len = {
                let memory_data = self.memory.data(&mut *store);
                read_i32_le(memory_data, output_len_ptr as usize)?
            };
            
            // Clean up output length pointer
            self.deallocate.call(&mut *store, (output_len_ptr, 4))?;
            
            // Empty input should return null pointer and 0 length
            if output_ptr == 0 && output_len == 0 {
                return Ok(None);
            } else {
                return Err("Expected null pointer and 0 length for empty input".into());
            }
        }
        
        // Normal processing for non-empty input
        // Allocate and write input to WASM memory
        let (input_ptr, input_len) = write_input_to_memory(&mut *store, &self.memory, input, &self.allocate)?;
        
        // Allocate space for output length
        let output_len_ptr = self.allocate.call(&mut *store, 4)?;
        
        // Call the process function
        let output_ptr = self.process.call(&mut *store, (input_ptr, input_len, output_len_ptr))?;
        
        // Read the output length
        let output_len = {
            let memory_data = self.memory.data(&mut *store);
            read_i32_le(memory_data, output_len_ptr as usize)?
        };
        
        // Always clean up input and output_len_ptr regardless of success/failure
        let cleanup_result = (|| {
            self.deallocate.call(&mut *store, (input_ptr, input_len))?;
            self.deallocate.call(&mut *store, (output_len_ptr, 4))?;
            Ok::<(), Box<dyn std::error::Error>>(())
        })();
        
        // Validate output after cleanup is scheduled
        if output_ptr == 0 {
            cleanup_result?; // Ensure cleanup completed
            return Err("Process returned null pointer for non-empty input".into());
        }
        
        // Read the output string
        let output_str = {
            let memory_data = self.memory.data(&mut *store);
            let output_bytes = &memory_data[output_ptr as usize..(output_ptr as usize + output_len as usize)];
            std::str::from_utf8(output_bytes)?.to_string()
        };
        
        // Complete cleanup (input and output_len_ptr already cleaned up above)
        cleanup_result?; // Ensure previous cleanup completed
        self.deallocate.call(&mut *store, (output_ptr, output_len))?;
        
        Ok(Some(output_str))
    }
}

/// Comprehensive WASM integration tests
/// 
/// This test suite validates the WASM module behavior in a proper WASM runtime context,
/// covering module loading, function exports, and various input scenarios.

const APPEND_STRING: &str = "::WASM";

#[test]
fn test_wasm_module_loads_and_exports() {
    let runner = WasmTestRunner::new().expect("Failed to create WASM test runner");
    
    runner.run_test(|store, instance| {
        // Verify the module exports the expected functions
        let _wasm_funcs = WasmFunctions::from_instance(&mut *store, instance)
            .expect("All required functions and memory should be exported");
        
        println!("✅ All expected functions and memory are exported");
        Ok(())
    }).expect("WASM module should load and export required functions");
}

#[test]
fn test_wasm_process_short_string() {
    let runner = WasmTestRunner::new().expect("Should be able to create WASM test runner");
    
    runner.run_test(|store, instance| {
        // Get function handles
        let wasm_funcs = WasmFunctions::from_instance(&mut *store, instance)
            .expect("All required functions and memory should be exported");
        
        let input = "hello";
        let expected_output = format!("{}{}", input, APPEND_STRING);
        
        // Use the centralized process helper
        let result = wasm_funcs.run_process(&mut *store, input)
            .expect("Process function should execute successfully");
        
        let output_str = result.expect("Should get output for non-empty input");
        assert_eq!(output_str, expected_output, "Output should match expected result");
        
        println!("✅ WASM process function works correctly: '{}' -> '{}'", input, output_str);
        Ok(())
    }).expect("WASM process function should work correctly");
}

#[test]
fn test_wasm_process_longer_string() {
    let runner = WasmTestRunner::new().expect("Failed to create WASM test runner");
    
    runner.run_test(|store, instance| {
        let wasm_funcs = WasmFunctions::from_instance(&mut *store, instance)?;
        
        let input = "hello world";
        let expected_output = format!("{}{}", input, APPEND_STRING);
        
        // Use the centralized process helper
        let result = wasm_funcs.run_process(&mut *store, input)?;
        
        let output_str = result.ok_or("Should get output for non-empty input")?;
        assert_eq!(output_str, expected_output, "Output should match expected result");
        
        println!("✅ Process function works correctly: '{}' -> '{}'", input, output_str);
        Ok(())
    }).expect("WASM process function should work correctly");
}

#[test]
fn test_wasm_process_empty_string() {
    let runner = WasmTestRunner::new().expect("Should be able to create WASM test runner");
    
    runner.run_test(|store, instance| {
        // Get function handles
        let wasm_funcs = WasmFunctions::from_instance(&mut *store, instance)
            .expect("All required functions and memory should be exported");
        
        // Use the centralized process helper for empty input
        let result = wasm_funcs.run_process(&mut *store, "")
            .expect("Process function should handle empty input");
        
        // Empty input should return None (null pointer case)
        assert!(result.is_none(), "Process should return None for empty input");
        
        println!("✅ Empty string processing works correctly: returns null pointer as expected");
        Ok(())
    }).expect("WASM process function should handle empty strings");
}
