use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Simple WASM processor that appends "-wasm" to the input string.
/// 
/// This demonstrates the basic WASM processor interface for The DAGwood project.
/// The function receives a pointer to a null-terminated C string and returns
/// a pointer to a new null-terminated C string with "-wasm" appended.
/// 
/// # Safety
/// 
/// This function is unsafe because it deals with raw pointers from WASM memory.
/// The caller must ensure that:
/// - `input_ptr` points to a valid null-terminated string
/// - The returned pointer is properly freed by the caller
/// 
/// # Memory Management
/// 
/// The returned string is allocated using CString::into_raw(), which transfers
/// ownership to the caller. The caller is responsible for freeing this memory.
#[no_mangle]
pub extern "C" fn process(input_ptr: *const c_char) -> *mut c_char {
    // Safety: We assume the input pointer is valid and points to a null-terminated string
    let input_cstr = unsafe { CStr::from_ptr(input_ptr) };
    
    // Convert to Rust string
    let input_str = match input_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(), // Return null on invalid UTF-8
    };
    
    // Append "-wasm" to the input
    let output = format!("{}-wasm", input_str);
    
    // Convert back to C string and return pointer
    match CString::new(output) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => std::ptr::null_mut(), // Return null on string conversion error
    }
}

/// Alternative process function that matches the interface expected by WasmProcessor.
/// 
/// This version takes a pointer and length for the input string, which is more
/// compatible with the WASM memory model used in our WasmProcessor implementation.
/// 
/// # Arguments
/// 
/// * `input_ptr` - Pointer to the input string data in WASM memory
/// * `input_len` - Length of the input string in bytes
/// 
/// # Returns
/// 
/// Returns a pointer to the output string in WASM memory. The string is null-terminated
/// for easy reading by the host.
#[no_mangle]
pub extern "C" fn process_with_length(input_ptr: *const u8, input_len: i32) -> *const u8 {
    // Safety: We assume the input pointer and length are valid
    let input_slice = unsafe { std::slice::from_raw_parts(input_ptr, input_len as usize) };
    
    // Convert to Rust string
    let input_str = match std::str::from_utf8(input_slice) {
        Ok(s) => s,
        Err(_) => return std::ptr::null(), // Return null on invalid UTF-8
    };
    
    // Append "-wasm" to the input
    let output = format!("{}-wasm", input_str);
    
    // Convert to bytes and store in static memory (simple approach for demo)
    // In a real implementation, you'd want proper memory management
    let output_bytes = output.into_bytes();
    let mut result = Vec::with_capacity(output_bytes.len() + 1);
    result.extend_from_slice(&output_bytes);
    result.push(0); // Null terminator
    
    // Leak the memory so it persists after function return
    // The caller is responsible for managing this memory
    let boxed_slice = result.into_boxed_slice();
    Box::leak(boxed_slice).as_ptr()
}

/// Memory allocator function for WASM module.
/// 
/// This function allows the host to allocate memory in the WASM module's
/// linear memory space. This is useful for passing data to the module.
#[no_mangle]
pub extern "C" fn allocate(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf); // Prevent deallocation
    ptr
}

/// Memory deallocator function for WASM module.
/// 
/// This function allows the host to free memory that was allocated by
/// the WASM module. This helps prevent memory leaks.
#[no_mangle]
pub extern "C" fn deallocate(ptr: *mut u8, size: usize) {
    unsafe {
        let _ = Vec::from_raw_parts(ptr, 0, size);
        // Vec will be dropped and memory freed
    }
}
