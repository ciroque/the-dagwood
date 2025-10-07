use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// C-string compatibility wrapper for the core process function.
/// 
/// This function provides a C-style interface by wrapping the core `process_with_length()`
/// implementation. It handles conversion between null-terminated C strings and the
/// pointer+length interface used by the core logic.
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
/// The returned string is allocated using the module's `allocate()` function.
/// The caller is responsible for freeing this memory using `deallocate()`.
#[no_mangle]
pub extern "C" fn process(input_ptr: *const c_char) -> *mut c_char {
    // Safety: We assume the input pointer is valid and points to a null-terminated string
    let input_cstr = unsafe { CStr::from_ptr(input_ptr) };
    
    // Get the bytes from the C string
    let input_bytes = input_cstr.to_bytes();
    
    // Delegate to the core implementation
    let result_ptr = process_with_length(input_bytes.as_ptr(), input_bytes.len() as i32);
    
    // Just cast the result - it's already null-terminated and ready for C
    // No need to copy again, avoiding double allocation
    result_ptr as *mut c_char
}

/// Core process function that implements the main business logic.
/// 
/// This is the canonical implementation that takes a pointer and length for the input string.
/// The `process()` function is a thin wrapper around this for C-string compatibility.
/// 
/// # Arguments
/// 
/// * `input_ptr` - Pointer to the input string data in WASM memory
/// * `input_len` - Length of the input string in bytes
/// 
/// # Returns
/// 
/// Returns a pointer to the output string in WASM memory. The string is null-terminated
/// for easy reading by the host. The host MUST call `deallocate()` to free this memory.
#[no_mangle]
pub extern "C" fn process_with_length(input_ptr: *const u8, input_len: i32) -> *mut u8 {
    // Safety: We assume the input pointer and length are valid
    let input_slice = unsafe { std::slice::from_raw_parts(input_ptr, input_len as usize) };
    
    // Convert to Rust string
    let input_str = match std::str::from_utf8(input_slice) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(), // Return null on invalid UTF-8
    };
    
    // Append "-wasm" to the input
    let output = format!("{}-wasm", input_str);
    let output_bytes = output.into_bytes();
    
    // Use our own allocator for consistent memory management
    let result = allocate(output_bytes.len() + 1);
    if result.is_null() {
        return std::ptr::null_mut();
    }
    
    unsafe {
        // Copy the output bytes
        std::ptr::copy_nonoverlapping(output_bytes.as_ptr(), result, output_bytes.len());
        // Null-terminate
        *result.add(output_bytes.len()) = 0;
    }
    
    result // Host can deallocate with our deallocate() function
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
