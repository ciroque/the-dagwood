use wasmtime::*;

/// Integration tests that run the actual WASM module in wasmtime
/// These tests validate the WASM module behavior in a proper WASM runtime context

const APPEND_STRING: &str = "-wasm";

struct WasmTestRunner {
    engine: Engine,
    module: Module,
}

impl WasmTestRunner {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let engine = Engine::default();
        
        // Load the compiled WASM module
        let wasm_path = "../wasm_appender.wasm";
        let module = Module::from_file(&engine, wasm_path)
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

#[test]
fn test_wasm_module_loads() {
    let runner = WasmTestRunner::new().expect("Failed to create WASM test runner");
    
    runner.run_test(|store, instance| {
        // Verify the module exports the expected functions
        let _process = instance.get_typed_func::<(i32, i32, i32), i32>(store, "process")
            .expect("process function should be exported");
        let _allocate = instance.get_typed_func::<i32, i32>(store, "allocate")
            .expect("allocate function should be exported");
        let _deallocate = instance.get_typed_func::<(i32, i32), ()>(store, "deallocate")
            .expect("deallocate function should be exported");
        
        println!("✅ All expected functions are exported");
        Ok(())
    }).expect("WASM module should load and export required functions");
}

#[test]
fn test_wasm_process_hello_world() {
    let runner = WasmTestRunner::new().expect("Failed to create WASM test runner");
    
    runner.run_test(|store, instance| {
        let process = instance.get_typed_func::<(i32, i32, i32), i32>(store, "process")?;
        let allocate = instance.get_typed_func::<i32, i32>(store, "allocate")?;
        let deallocate = instance.get_typed_func::<(i32, i32), ()>(store, "deallocate")?;
        let memory = instance.get_memory(store, "memory")
            .expect("WASM module should export memory");
        
        let input = "hello world";
        let expected_output = format!("{}{}", input, APPEND_STRING);
        
        // Allocate memory for input string
        let input_len = input.len() as i32;
        let input_ptr = allocate.call(store, input_len)?;
        
        // Write input string to WASM memory
        let memory_data = memory.data_mut(store);
        let input_bytes = input.as_bytes();
        memory_data[input_ptr as usize..(input_ptr as usize + input_bytes.len())]
            .copy_from_slice(input_bytes);
        
        // Allocate space for output length
        let output_len_ptr = allocate.call(store, 4)?; // i32 = 4 bytes
        
        // Call the process function
        let output_ptr = process.call(store, (input_ptr, input_len, output_len_ptr))?;
        
        // Read the output length
        let memory_data = memory.data(store);
        let output_len = i32::from_le_bytes([
            memory_data[output_len_ptr as usize],
            memory_data[output_len_ptr as usize + 1],
            memory_data[output_len_ptr as usize + 2],
            memory_data[output_len_ptr as usize + 3],
        ]);
        
        assert_ne!(output_ptr, 0, "Process should return non-null pointer");
        assert_eq!(output_len, expected_output.len() as i32, "Output length should match expected");
        
        // Read the output string
        let output_bytes = &memory_data[output_ptr as usize..(output_ptr as usize + output_len as usize)];
        let output_str = std::str::from_utf8(output_bytes)
            .expect("Output should be valid UTF-8");
        
        assert_eq!(output_str, expected_output, "Output should match expected result");
        
        // Clean up allocated memory
        deallocate.call(store, (input_ptr, input_len))?;
        deallocate.call(store, (output_len_ptr, 4))?;
        deallocate.call(store, (output_ptr, output_len))?;
        
        println!("✅ Process function works correctly: '{}' -> '{}'", input, output_str);
        Ok(())
    }).expect("WASM process function should work correctly");
}

#[test]
fn test_wasm_process_empty_string() {
    let runner = WasmTestRunner::new().expect("Failed to create WASM test runner");
    
    runner.run_test(|store, instance| {
        let process = instance.get_typed_func::<(i32, i32, i32), i32>(store, "process")?;
        let allocate = instance.get_typed_func::<i32, i32>(store, "allocate")?;
        let deallocate = instance.get_typed_func::<(i32, i32), ()>(store, "deallocate")?;
        let memory = instance.get_memory(store, "memory")
            .expect("WASM module should export memory");
        
        let input = "";
        let expected_output = APPEND_STRING;
        
        // Allocate space for output length (even for empty input)
        let output_len_ptr = allocate.call(store, 4)?;
        
        // Call process with empty string (null pointer, 0 length)
        let output_ptr = process.call(store, (0, 0, output_len_ptr))?;
        
        // Read the output length
        let memory_data = memory.data(store);
        let output_len = i32::from_le_bytes([
            memory_data[output_len_ptr as usize],
            memory_data[output_len_ptr as usize + 1],
            memory_data[output_len_ptr as usize + 2],
            memory_data[output_len_ptr as usize + 3],
        ]);
        
        // The WASM module returns null pointer for empty input (input_len <= 0)
        assert_eq!(output_ptr, 0, "Process should return null pointer for empty input");
        assert_eq!(output_len, 0, "Output length should be 0 for empty input");
        
        // Clean up (only the output length pointer, no output to deallocate)
        deallocate.call(store, (output_len_ptr, 4))?;
        
        println!("✅ Empty string processing works correctly: returns null pointer as expected");
        Ok(())
    }).expect("WASM process function should handle empty strings");
}
