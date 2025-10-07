use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

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
    
    let input_bytes = input_cstr.to_bytes();
    
    let mut output_len = 0;
    let result_ptr = process_with_length(
        input_bytes.as_ptr(),
        input_bytes.len(),
        &mut output_len as *mut usize
    );
    
    if result_ptr.is_null() {
        return ptr::null_mut();
    }
    
    // For C compatibility, we can just cast since we know it's null-terminated
    result_ptr as *mut c_char
}

/// Core process function that implements the main business logic.
/// 
/// # Arguments
/// 
/// * `input_ptr` - Pointer to the input string data in WASM memory
/// * `input_len` - Length of the input string in bytes
/// * `output_len` - Output parameter that will receive the length of the output (excluding null terminator)
/// 
/// # Returns
/// 
/// Returns a pointer to the output string in WASM memory. The string is null-terminated
/// for C compatibility. The host MUST call `deallocate()` to free this memory.
#[no_mangle]
pub extern "C" fn process_with_length(
    input_ptr: *const u8, 
    input_len: usize,
    output_len: *mut usize
) -> *mut u8 {
    // Safety: We assume the input pointer and length are valid
    let input_slice = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };
    
    // Convert to Rust string
    let input_str = match std::str::from_utf8(input_slice) {
        Ok(s) => s,
        Err(_) => {
            unsafe { *output_len = 0; }
            return ptr::null_mut();
        }
    };
    
    // Calculate output
    let output = format!("{}-wasm", input_str);
    let output_bytes = output.as_bytes();
    let output_len_val = output_bytes.len();
    
    // Allocate memory (including space for null terminator)
    let result = allocate(output_len_val + 1);
    if result.is_null() {
        unsafe { *output_len = 0; }
        return ptr::null_mut();
    }
    
    unsafe {
        // Copy the output bytes
        std::ptr::copy_nonoverlapping(output_bytes.as_ptr(), result, output_len_val);
        // Null-terminate
        *result.add(output_len_val) = 0;
        // Set output length
        *output_len = output_len_val;
    }
    
    result
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
    if !ptr.is_null() {
        unsafe {
            let _ = Vec::from_raw_parts(ptr, 0, size);
            // Vec will be dropped and memory freed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::ptr;
    use std::slice;

    // Helper function to create a test string
    fn create_test_string(s: &str) -> (CString, *const c_char) {
        let cstr = CString::new(s).unwrap();
        let ptr = cstr.as_ptr();
        (cstr, ptr)
    }

    #[test]
    fn test_process_basic() {
        let (_input, input_ptr) = create_test_string("test");
        let output_ptr = unsafe { process(input_ptr) };
        assert!(!output_ptr.is_null(), "Process returned null pointer");
        
        // Convert back to Rust string for assertion
        let output_cstr = unsafe { CStr::from_ptr(output_ptr) };
        assert_eq!(output_cstr.to_str().unwrap(), "test-wasm");
        
        // Clean up
        unsafe { deallocate(output_ptr as *mut u8, output_cstr.to_bytes().len() + 1) };
    }

    #[test]
    fn test_process_empty_string() {
        let (_input, input_ptr) = create_test_string("");
        let output_ptr = unsafe { process(input_ptr) };
        assert!(!output_ptr.is_null(), "Process returned null pointer");
        
        let output_cstr = unsafe { CStr::from_ptr(output_ptr) };
        assert_eq!(output_cstr.to_str().unwrap(), "-wasm");
        
        // Clean up
        unsafe { deallocate(output_ptr as *mut u8, output_cstr.to_bytes().len() + 1) };
    }

    #[test]
    fn test_process_with_length_basic() {
        let input = "hello";
        let mut output_len = 0;
        let output_ptr = process_with_length(
            input.as_ptr(),
            input.len(),
            &mut output_len as *mut usize
        );
        
        assert!(!output_ptr.is_null(), "process_with_length returned null");
        assert_eq!(output_len, input.len() + 5); // "hello-wasm" is 10 bytes
        
        // Convert back to string
        let output_slice = unsafe { 
            slice::from_raw_parts(output_ptr, output_len) 
        };
        let output_str = std::str::from_utf8(output_slice).unwrap();
        assert_eq!(output_str, "hello-wasm");
        
        // Clean up
        unsafe { deallocate(output_ptr, output_len + 1) };
    }

    #[test]
    fn test_process_with_length_empty_string() {
        let input = "";
        let mut output_len = 0;
        let output_ptr = process_with_length(
            input.as_ptr(),
            input.len(),
            &mut output_len as *mut usize
        );
        
        assert!(!output_ptr.is_null(), "process_with_length returned null");
        assert_eq!(output_len, 5); // "-wasm" is 5 bytes
        
        // Convert back to string
        let output_slice = unsafe { 
            slice::from_raw_parts(output_ptr, output_len) 
        };
        let output_str = std::str::from_utf8(output_slice).unwrap();
        assert_eq!(output_str, "-wasm");
        
        // Clean up
        unsafe { deallocate(output_ptr, output_len + 1) };
    }

    #[test]
    fn test_process_with_length_invalid_utf8() {
        let invalid_utf8 = &[0xC3, 0x28]; // Invalid UTF-8 sequence
        let mut output_len = 0;
        let output_ptr = process_with_length(
            invalid_utf8.as_ptr(),
            invalid_utf8.len(),
            &mut output_len as *mut usize
        );
        
        assert!(output_ptr.is_null(), "Expected null for invalid UTF-8 input");
        assert_eq!(output_len, 0);
    }

    #[test]
    fn test_process_null_pointer() {
        let output_ptr = unsafe { process(ptr::null()) };
        assert!(output_ptr.is_null(), "Expected null for null input");
    }

    #[test]
    fn test_allocate_and_deallocate() {
        let size = 1024;
        let ptr = allocate(size);
        assert!(!ptr.is_null(), "Allocation failed");
        
        // Write some data to make sure it's usable memory
        unsafe {
            for i in 0..size {
                *ptr.add(i) = (i % 256) as u8;
            }
        }
        
        // Read back and verify
        unsafe {
            for i in 0..size {
                assert_eq!(*ptr.add(i), (i % 256) as u8);
            }
        }
        
        // Free the memory
        deallocate(ptr, size);
    }

    #[test]
    fn test_memory_isolation() {
        // Test that different allocations don't interfere
        let ptr1 = allocate(10);
        let ptr2 = allocate(10);
        
        assert_ne!(ptr1, ptr2, "Allocations should return different pointers");
        
        // Write to first allocation
        unsafe { *ptr1 = 42 };
        
        // Second allocation should still be zeroed
        unsafe { assert_eq!(*ptr2, 0) };
        
        deallocate(ptr1, 10);
        deallocate(ptr2, 10);
    }
}
