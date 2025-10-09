// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use std::ptr;
use wee_alloc::WeeAlloc;

// Use wee_alloc as the global allocator (safer for WASM)
#[global_allocator]
static ALLOC: WeeAlloc = WeeAlloc::INIT;

/// Primary WASM processing function with explicit length parameters.
/// 
/// This is the main WASM interface optimized for linear memory operations.
/// It takes input data with explicit length and returns output data with 
/// length via output parameter. This approach is more efficient and supports
/// binary data without null terminator limitations.
/// 
/// # Arguments
/// 
/// * `input_ptr` - Pointer to the input data in WASM memory
/// * `input_len` - Length of the input data in bytes
/// * `output_len` - Output parameter that will receive the length of the output
/// 
/// # Returns
/// 
/// Returns a pointer to the output data in WASM memory. The host MUST call 
/// `deallocate()` to free this memory using the returned length.
/// 
/// # Safety
/// 
/// This function is unsafe because it deals with raw pointers from WASM memory.
/// The caller must ensure that:
/// - `input_ptr` points to valid memory of at least `input_len` bytes
/// - `output_len` points to a valid usize location
/// - The returned pointer is properly freed by the caller
#[no_mangle]
pub extern "C" fn process(
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
    
    // Allocate memory for the result (no null terminator needed!)
    let result = allocate(output_len_val);
    if result.is_null() {
        unsafe { *output_len = 0; }
        return ptr::null_mut();
    }
    
    unsafe {
        // Copy the output bytes
        std::ptr::copy_nonoverlapping(output_bytes.as_ptr(), result, output_len_val);
        // Set output length
        *output_len = output_len_val;
    }
    
    result
}


/// Memory allocator function for WASM module.
/// 
/// This function allows the host to allocate memory in the WASM module's
/// linear memory space. Uses wee_alloc for safer WASM allocation.
#[no_mangle]
pub extern "C" fn allocate(size: usize) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }
    
    // Use Vec with wee_alloc - should be safe in WASM
    let mut vec = Vec::with_capacity(size);
    vec.resize(size, 0);
    let ptr = vec.as_mut_ptr();
    std::mem::forget(vec); // Prevent Vec from being dropped
    ptr
}

/// Memory deallocator function for WASM module.
/// 
/// This function allows the host to free memory that was allocated by
/// the WASM module. Works with wee_alloc.
#[no_mangle]
pub extern "C" fn deallocate(ptr: *mut u8, size: usize) {
    if !ptr.is_null() && size > 0 {
        unsafe {
            // Reconstruct Vec from raw parts and let it drop
            let _ = Vec::from_raw_parts(ptr, size, size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::slice;

    #[test]
    fn test_process_basic() {
        let input = "test";
        let mut output_len = 0;
        let output_ptr = process(
            input.as_ptr(),
            input.len(),
            &mut output_len as *mut usize
        );
        
        assert!(!output_ptr.is_null(), "Process returned null pointer");
        assert_eq!(output_len, 9); // "test-wasm" is 9 bytes
        
        // Convert back to string
        let output_slice = unsafe { 
            slice::from_raw_parts(output_ptr, output_len) 
        };
        let output_str = std::str::from_utf8(output_slice).unwrap();
        assert_eq!(output_str, "test-wasm");
        
        // Clean up (no null terminator!)
        unsafe { deallocate(output_ptr, output_len) };
    }

    #[test]
    fn test_process_empty_string() {
        let input = "";
        let mut output_len = 0;
        let output_ptr = process(
            input.as_ptr(),
            input.len(),
            &mut output_len as *mut usize
        );
        
        assert!(!output_ptr.is_null(), "Process returned null pointer");
        assert_eq!(output_len, 5); // "-wasm" is 5 bytes
        
        // Convert back to string
        let output_slice = unsafe { 
            slice::from_raw_parts(output_ptr, output_len) 
        };
        let output_str = std::str::from_utf8(output_slice).unwrap();
        assert_eq!(output_str, "-wasm");
        
        // Clean up
        unsafe { deallocate(output_ptr, output_len) };
    }

    #[test]
    fn test_process_invalid_utf8() {
        let invalid_utf8 = &[0xC3, 0x28]; // Invalid UTF-8 sequence
        let mut output_len: usize = 0;
        let output_ptr = process(
            invalid_utf8.as_ptr(),
            invalid_utf8.len(),
            &mut output_len as *mut usize
        );
        
        assert!(output_ptr.is_null(), "Expected null for invalid UTF-8 input");
        assert_eq!(output_len, 0);
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
