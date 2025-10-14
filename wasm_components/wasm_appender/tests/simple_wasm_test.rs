use wasmtime::*;

/// Simple integration test that validates the WASM module loads and exports the expected functions
/// This test works with wasmtime 37.0 API

const APPEND_STRING: &str = "::WASM";

#[test]
fn test_wasm_module_loads_and_functions_exist() {
    // Create engine and load module
    let engine = Engine::default();
    let module = Module::from_file(&engine, "../wasm_appender.wasm")
        .expect("Should be able to load WASM module");
    
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])
        .expect("Should be able to instantiate WASM module");
    
    // Verify the module exports the expected functions
    let _process = instance.get_typed_func::<(i32, i32, i32), i32>(&mut store, "process")
        .expect("process function should be exported");
    let _allocate = instance.get_typed_func::<i32, i32>(&mut store, "allocate")
        .expect("allocate function should be exported");
    let _deallocate = instance.get_typed_func::<(i32, i32), ()>(&mut store, "deallocate")
        .expect("deallocate function should be exported");
    let _memory = instance.get_memory(&mut store, "memory")
        .expect("memory should be exported");
    
    println!("✅ WASM module loads successfully and exports all required functions");
}

#[test]
fn test_wasm_basic_functionality() {
    // Create engine and load module
    let engine = Engine::default();
    let module = Module::from_file(&engine, "../wasm_appender.wasm")
        .expect("Should be able to load WASM module");
    
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])
        .expect("Should be able to instantiate WASM module");
    
    // Get function handles
    let process = instance.get_typed_func::<(i32, i32, i32), i32>(&mut store, "process")
        .expect("process function should be exported");
    let allocate = instance.get_typed_func::<i32, i32>(&mut store, "allocate")
        .expect("allocate function should be exported");
    let deallocate = instance.get_typed_func::<(i32, i32), ()>(&mut store, "deallocate")
        .expect("deallocate function should be exported");
    let memory = instance.get_memory(&mut store, "memory")
        .expect("memory should be exported");
    
    let input = "hello";
    let expected_output = format!("{}{}", input, APPEND_STRING);
    
    // Allocate memory for input
    let input_len = input.len() as i32;
    let input_ptr = allocate.call(&mut store, input_len)
        .expect("Should be able to allocate input memory");
    
    // Write input to WASM memory
    {
        let memory_data = memory.data_mut(&mut store);
        let input_bytes = input.as_bytes();
        memory_data[input_ptr as usize..(input_ptr as usize + input_bytes.len())]
            .copy_from_slice(input_bytes);
    }
    
    // Allocate space for output length
    let output_len_ptr = allocate.call(&mut store, 4)
        .expect("Should be able to allocate output length memory");
    
    // Call the process function
    let output_ptr = process.call(&mut store, (input_ptr, input_len, output_len_ptr))
        .expect("Process function should execute successfully");
    
    // Read the output length
    let output_len = {
        let memory_data = memory.data(&store);
        i32::from_le_bytes([
            memory_data[output_len_ptr as usize],
            memory_data[output_len_ptr as usize + 1],
            memory_data[output_len_ptr as usize + 2],
            memory_data[output_len_ptr as usize + 3],
        ])
    };
    
    assert_ne!(output_ptr, 0, "Process should return non-null pointer");
    assert_eq!(output_len, expected_output.len() as i32, "Output length should match expected");
    
    // Read the output string
    let output_str = {
        let memory_data = memory.data(&store);
        let output_bytes = &memory_data[output_ptr as usize..(output_ptr as usize + output_len as usize)];
        std::str::from_utf8(output_bytes)
            .expect("Output should be valid UTF-8")
            .to_string()
    };
    
    assert_eq!(output_str, expected_output, "Output should match expected result");
    
    // Clean up allocated memory
    deallocate.call(&mut store, (input_ptr, input_len))
        .expect("Should be able to deallocate input memory");
    deallocate.call(&mut store, (output_len_ptr, 4))
        .expect("Should be able to deallocate output length memory");
    deallocate.call(&mut store, (output_ptr, output_len))
        .expect("Should be able to deallocate output memory");
    
    println!("✅ WASM process function works correctly: '{}' -> '{}'", input, output_str);
}

#[test]
fn test_wasm_empty_string() {
    // Create engine and load module
    let engine = Engine::default();
    let module = Module::from_file(&engine, "../wasm_appender.wasm")
        .expect("Should be able to load WASM module");
    
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])
        .expect("Should be able to instantiate WASM module");
    
    // Get function handles
    let process = instance.get_typed_func::<(i32, i32, i32), i32>(&mut store, "process")
        .expect("process function should be exported");
    let allocate = instance.get_typed_func::<i32, i32>(&mut store, "allocate")
        .expect("allocate function should be exported");
    let deallocate = instance.get_typed_func::<(i32, i32), ()>(&mut store, "deallocate")
        .expect("deallocate function should be exported");
    let memory = instance.get_memory(&mut store, "memory")
        .expect("memory should be exported");
    
    // Allocate space for output length
    let output_len_ptr = allocate.call(&mut store, 4)
        .expect("Should be able to allocate output length memory");
    
    // Call process with empty string (null pointer, 0 length)
    let output_ptr = process.call(&mut store, (0, 0, output_len_ptr))
        .expect("Process function should handle empty input");
    
    // Read the output length
    let output_len = {
        let memory_data = memory.data(&store);
        i32::from_le_bytes([
            memory_data[output_len_ptr as usize],
            memory_data[output_len_ptr as usize + 1],
            memory_data[output_len_ptr as usize + 2],
            memory_data[output_len_ptr as usize + 3],
        ])
    };
    
    // The WASM module returns null pointer for empty input (input_len <= 0)
    assert_eq!(output_ptr, 0, "Process should return null pointer for empty input");
    assert_eq!(output_len, 0, "Output length should be 0 for empty input");
    
    // Clean up (only the output length pointer, no output to deallocate)
    deallocate.call(&mut store, (output_len_ptr, 4))
        .expect("Should be able to deallocate output length memory");
    
    println!("✅ Empty string processing works correctly: returns null pointer as expected");
}
