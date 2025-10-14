use wasmtime::*;
use std::path::Path;

// Test infrastructure moved inline
struct WasmTestRunner {
    engine: Engine,
    module: Module,
}

impl WasmTestRunner {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map_err(|_| "CARGO_MANIFEST_DIR not set")?;
        let wasm_path = format!("{}/../wasm_appender.wasm", manifest_dir);
        
        if !Path::new(&wasm_path).exists() {
            return Err(format!(
                "WASM module not found at {}. Run 'make build' first to compile the module.",
                wasm_path
            ).into());
        }
        
        let engine = Engine::default();
        let module = Module::from_file(&engine, &wasm_path)
            .map_err(|e| format!("Failed to load WASM module from {}: {}", wasm_path, e))?;
        
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
    if ptr + 3 >= memory_data.len() {
        return Err(format!(
            "Cannot read i32 at offset {}: would read beyond memory bounds (memory size: {})",
            ptr, memory_data.len()
        ).into());
    }
    
    Ok(i32::from_le_bytes([
        memory_data[ptr],
        memory_data[ptr + 1],
        memory_data[ptr + 2],
        memory_data[ptr + 3],
    ]))
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
        let _process = instance.get_typed_func::<(i32, i32, i32), i32>(&mut *store, "process")
            .expect("process function should be exported");
        let _allocate = instance.get_typed_func::<i32, i32>(&mut *store, "allocate")
            .expect("allocate function should be exported");
        let _deallocate = instance.get_typed_func::<(i32, i32), ()>(&mut *store, "deallocate")
            .expect("deallocate function should be exported");
        let _memory = instance.get_memory(&mut *store, "memory")
            .expect("memory should be exported");
        
        println!("✅ All expected functions and memory are exported");
        Ok(())
    }).expect("WASM module should load and export required functions");
}

#[test]
fn test_wasm_process_short_string() {
    let runner = WasmTestRunner::new().expect("Should be able to create WASM test runner");
    
    runner.run_test(|store, instance| {
        // Get function handles
        let process = instance.get_typed_func::<(i32, i32, i32), i32>(&mut *store, "process")
            .expect("process function should be exported");
        let allocate = instance.get_typed_func::<i32, i32>(&mut *store, "allocate")
            .expect("allocate function should be exported");
        let deallocate = instance.get_typed_func::<(i32, i32), ()>(&mut *store, "deallocate")
            .expect("deallocate function should be exported");
        let memory = instance.get_memory(&mut *store, "memory")
            .expect("memory should be exported");
        
        let input = "hello";
        let expected_output = format!("{}{}", input, APPEND_STRING);
        
        // Allocate and write input to WASM memory
        let (input_ptr, input_len) = write_input_to_memory(&mut *store, &memory, input, &allocate)
            .expect("Should be able to write input to memory");
        
        // Allocate space for output length
        let output_len_ptr = allocate.call(&mut *store, 4)
            .expect("Should be able to allocate output length memory");
        
        // Call the process function
        let output_ptr = process.call(&mut *store, (input_ptr, input_len, output_len_ptr))
            .expect("Process function should execute successfully");
        
        // Read the output length
        let output_len = {
            let memory_data = memory.data(&mut *store);
            read_i32_le(memory_data, output_len_ptr as usize)
                .expect("Should be able to read output length from valid memory location")
        };
        
        assert_ne!(output_ptr, 0, "Process should return non-null pointer");
        assert_eq!(output_len, expected_output.len() as i32, "Output length should match expected");
        
        // Read the output string
        let output_str = {
            let memory_data = memory.data(&mut *store);
            let output_bytes = &memory_data[output_ptr as usize..(output_ptr as usize + output_len as usize)];
            std::str::from_utf8(output_bytes)
                .expect("Output should be valid UTF-8")
                .to_string()
        };
        
        assert_eq!(output_str, expected_output, "Output should match expected result");
        
        // Clean up allocated memory
        deallocate.call(&mut *store, (input_ptr, input_len))
            .expect("Should be able to deallocate input memory");
        deallocate.call(&mut *store, (output_len_ptr, 4))
            .expect("Should be able to deallocate output length memory");
        deallocate.call(&mut *store, (output_ptr, output_len))
            .expect("Should be able to deallocate output memory");
        
        println!("✅ WASM process function works correctly: '{}' -> '{}'", input, output_str);
        Ok(())
    }).expect("WASM process function should work correctly");
}

#[test]
fn test_wasm_process_longer_string() {
    let runner = WasmTestRunner::new().expect("Failed to create WASM test runner");
    
    runner.run_test(|store, instance| {
        let process = instance.get_typed_func::<(i32, i32, i32), i32>(&mut *store, "process")?;
        let allocate = instance.get_typed_func::<i32, i32>(&mut *store, "allocate")?;
        let deallocate = instance.get_typed_func::<(i32, i32), ()>(&mut *store, "deallocate")?;
        let memory = instance.get_memory(&mut *store, "memory")
            .expect("WASM module should export memory");
        
        let input = "hello world";
        let expected_output = format!("{}{}", input, APPEND_STRING);
        
        // Allocate and write input to WASM memory
        let (input_ptr, input_len) = write_input_to_memory(&mut *store, &memory, input, &allocate)?;
        
        // Allocate space for output length
        let output_len_ptr = allocate.call(&mut *store, 4)?; // i32 = 4 bytes
        
        // Call the process function
        let output_ptr = process.call(&mut *store, (input_ptr, input_len, output_len_ptr))?;
        
        // Read the output length
        let output_len = {
            let memory_data = memory.data(&mut *store);
            read_i32_le(memory_data, output_len_ptr as usize)
                .map_err(|e| format!("Failed to read output length: {}", e))?
        };
        
        assert_ne!(output_ptr, 0, "Process should return non-null pointer");
        assert_eq!(output_len, expected_output.len() as i32, "Output length should match expected");
        
        // Read the output string
        let output_str = {
            let memory_data = memory.data(&mut *store);
            let output_bytes = &memory_data[output_ptr as usize..(output_ptr as usize + output_len as usize)];
            std::str::from_utf8(output_bytes)
                .expect("Output should be valid UTF-8")
                .to_string()
        };
        
        assert_eq!(output_str, expected_output, "Output should match expected result");
        
        // Clean up allocated memory
        deallocate.call(&mut *store, (input_ptr, input_len))?;
        deallocate.call(&mut *store, (output_len_ptr, 4))?;
        deallocate.call(&mut *store, (output_ptr, output_len))?;
        
        println!("✅ Process function works correctly: '{}' -> '{}'", input, output_str);
        Ok(())
    }).expect("WASM process function should work correctly");
}

#[test]
fn test_wasm_process_empty_string() {
    let runner = WasmTestRunner::new().expect("Should be able to create WASM test runner");
    
    runner.run_test(|store, instance| {
        // Get function handles
        let process = instance.get_typed_func::<(i32, i32, i32), i32>(&mut *store, "process")
            .expect("process function should be exported");
        let allocate = instance.get_typed_func::<i32, i32>(&mut *store, "allocate")
            .expect("allocate function should be exported");
        let deallocate = instance.get_typed_func::<(i32, i32), ()>(&mut *store, "deallocate")
            .expect("deallocate function should be exported");
        let memory = instance.get_memory(&mut *store, "memory")
            .expect("memory should be exported");
        
        // Allocate space for output length (even for empty input)
        let output_len_ptr = allocate.call(&mut *store, 4)
            .expect("Should be able to allocate output length memory");
        
        // Call process with empty string (null pointer, 0 length)
        let output_ptr = process.call(&mut *store, (0, 0, output_len_ptr))
            .expect("Process function should handle empty input");
        
        // Read the output length
        let output_len = {
            let memory_data = memory.data(&mut *store);
            read_i32_le(memory_data, output_len_ptr as usize)
                .expect("Should be able to read output length from valid memory location")
        };
        
        // The WASM module returns null pointer for empty input (input_len <= 0)
        assert_eq!(output_ptr, 0, "Process should return null pointer for empty input");
        assert_eq!(output_len, 0, "Output length should be 0 for empty input");
        
        // Clean up (only the output length pointer, no output to deallocate)
        deallocate.call(&mut *store, (output_len_ptr, 4))
            .expect("Should be able to deallocate output length memory");
        
        println!("✅ Empty string processing works correctly: returns null pointer as expected");
        Ok(())
    }).expect("WASM process function should handle empty strings");
}
