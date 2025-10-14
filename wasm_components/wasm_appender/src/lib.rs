// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WASM Appender Component - C-style exports
//! 
//! This component demonstrates C-style WASM exports for DAGwood processor integration.
//! It exports the standard functions expected by the DAGwood WASM backend:
//! - process: Main processing function
//! - allocate: Memory allocation
//! - deallocate: Memory deallocation

use wee_alloc::WeeAlloc;

// Use wee_alloc as the global allocator (safer for WASM)
#[global_allocator]
static ALLOC: WeeAlloc = WeeAlloc::INIT;

const APPEND_STRING: &str = "::WASM";

/// Primary WASM processing function with explicit length parameters.
/// 
/// This is the main WASM interface optimized for linear memory operations.
/// It takes input data with explicit length and returns output data with 
/// length via output parameter. This approach is more efficient and supports
/// binary data without null terminator limitations.
/// 
/// # Arguments
/// 
/// * `input_ptr`  - Pointer to the input data in WASM memory
/// * `input_len`  - Length of the input data in bytes
/// * `output_len`  - Output parameter that will receive the length of the output
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
    input_ptr: i32, 
    input_len: i32,
    output_len_ptr: i32
) -> i32 {
    // Handle null pointer or invalid length
    if input_ptr == 0 || input_len <= 0 || output_len_ptr == 0 {
        return 0;
    }
    
    // Safety: We assume the input pointer and length are valid
    let input_slice = unsafe { std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize) };
    
    // Remove null terminator if present (DAGwood sends C-strings with null terminator)
    let input_slice = if let Some(&0) = input_slice.last() {
        &input_slice[..input_slice.len() - 1]
    } else {
        input_slice
    };
    
    // Convert to Rust string
    let input_str = match std::str::from_utf8(input_slice) {
        Ok(s) => s,
        Err(_) => {
            unsafe { *(output_len_ptr as *mut i32) = 0; }
            return 0;
        }
    };
    
    // Calculate output
    let output = format!("{}{}", input_str, APPEND_STRING);
    let output_bytes = output.as_bytes();
    let output_len_val = output_bytes.len();
    
    // Allocate memory for the result (no null terminator needed!)
    let result = allocate(output_len_val as i32);
    if result == 0 {
        unsafe { *(output_len_ptr as *mut i32) = 0; }
        return 0;
    }
    
    unsafe {
        // Copy the output bytes
        std::ptr::copy_nonoverlapping(output_bytes.as_ptr(), result as *mut u8, output_len_val);
        // Set output length
        *(output_len_ptr as *mut i32) = output_len_val as i32;
    }
    
    result
}

/// Memory allocator function for WASM module.
/// 
/// This function allows the host to allocate memory in the WASM module's
/// linear memory space. Uses wee_alloc for safer WASM allocation.
#[no_mangle]
pub extern "C" fn allocate(size: i32) -> i32 {
    if size <= 0 {
        return 0;
    }
    
    // Use Vec with wee_alloc - should be safe in WASM
    let mut vec = Vec::with_capacity(size as usize);
    vec.resize(size as usize, 0);
    let ptr = vec.as_mut_ptr();
    std::mem::forget(vec); // Prevent Vec from being dropped
    ptr as i32
}

/// Memory deallocator function for WASM module.
/// 
/// This function allows the host to free memory that was allocated by
/// the WASM module. Works with wee_alloc.
#[no_mangle]
pub extern "C" fn deallocate(ptr: i32, size: i32) {
    if ptr != 0 && size > 0 {
        unsafe {
            // Reconstruct Vec from raw parts and let it drop
            let _ = Vec::from_raw_parts(ptr as *mut u8, size as usize, size as usize);
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
        let mut output_len: i32 = 0;
        let output_ptr = process(
            input.as_ptr() as i32,
            input.len() as i32,
            &mut output_len as *mut i32 as i32
        );
        
        assert_ne!(output_ptr, 0, "Process returned null pointer");
        assert_eq!(output_len, 9); // "test-wasm" is 9 bytes
        
        // Convert back to string
        let output_slice = unsafe { 
            slice::from_raw_parts(output_ptr as *const u8, output_len as usize) 
        };
        let output_str = std::str::from_utf8(output_slice).unwrap();
        assert_eq!(output_str, APPEND_STRING);
        
        // Clean up (no null terminator!)
        unsafe { deallocate(output_ptr, output_len as i32) };
    }

    #[test]
    fn test_process_empty_string() {
        let input = "";
        let mut output_len: i32 = 0;
        let output_ptr = process(
            input.as_ptr() as i32,
            input.len() as i32,
            &mut output_len as *mut i32 as i32
        );
        
        assert_ne!(output_ptr, 0, "Process returned null pointer");
        assert_eq!(output_len, 5);
        
        // Convert back to string
        let output_slice = unsafe { 
            slice::from_raw_parts(output_ptr as *const u8, output_len as usize) 
        };
        let output_str = std::str::from_utf8(output_slice).unwrap();
        assert_eq!(output_str, APPEND_STRING);
        
        // Clean up
        unsafe { deallocate(output_ptr, output_len as i32) };
    }

    #[test]
    fn test_process_invalid_utf8() {
        let invalid_utf8 = &[0xC3, 0x28]; // Invalid UTF-8 sequence
        let mut output_len: i32 = 0;
        let output_ptr = process(
            invalid_utf8.as_ptr() as i32,
            invalid_utf8.len() as i32,
            &mut output_len as *mut i32 as i32
        );
        
        assert_eq!(output_ptr, 0, "Expected null for invalid UTF-8 input");
        assert_eq!(output_len, 0);
    }

    #[test]
    fn test_allocate_and_deallocate() {
        let size = 1024;
        let ptr = allocate(size as i32);
        assert_ne!(ptr, 0, "Allocation failed");
        
        // Write some data to make sure it's usable memory
        unsafe {
            let ptr = ptr as *mut u8;
            for i in 0..size as usize {
                *ptr.add(i) = (i % 256) as u8;
            }
        }
        
        // Read back and verify
        unsafe {
            let ptr = ptr as *mut u8;
            for i in 0..size as usize {
                assert_eq!(*ptr.add(i), (i % 256) as u8);
            }
        }
        
        // Clean up
        deallocate(ptr, size as i32);
    }

    #[test]
    fn test_memory_isolation() {
        // Test that different allocations don't interfere
        let ptr1 = allocate(10 as i32);
        let ptr2 = allocate(10 as i32);
        
        assert_ne!(ptr1, 0, "First allocation failed");
        assert_ne!(ptr2, 0, "Second allocation failed");
        assert_ne!(ptr1, ptr2, "Allocations should return different pointers");
        
        // Write to first allocation
        unsafe {
            let ptr1 = ptr1 as *mut u8;
            *ptr1 = 42;
        }
        
        // Second allocation should still be zeroed
        unsafe {
            let ptr2 = ptr2 as *mut u8;
            assert_eq!(*ptr2, 0);
        }
        
        deallocate(ptr1, 10);
        deallocate(ptr2, 10);
    }
}
