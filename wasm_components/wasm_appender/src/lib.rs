// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Hello WASM Component using WIT bindings
//! 
//! This component demonstrates the use of WebAssembly Interface Types (WIT)
//! for creating DAGwood processor components. It replaces the manual C-style
//! exports with generated bindings that provide type safety and structured
//! error handling.

// Generate bindings from the WIT file
wit_bindgen::generate!({
    world: "dagwood-component",
    path: "wit",
});

use exports::dagwood::component::processing_node::{
    Guest, AllocationError, ProcessingError
};

/// Hello WASM component implementation using WIT bindings
/// 
/// This struct implements the Guest trait generated from our WIT specification.
/// It provides the same functionality as the old C-style exports but with
/// better type safety and structured error handling.
struct WasmAppenderComponent;

impl Guest for WasmAppenderComponent {
    /// Process input data and return transformed output
    /// 
    /// This function takes input data via pointers and returns a pointer to
    /// the output data. It maintains the same low-level interface as the
    /// C-style version but with structured error handling.
    fn process(input_ptr: u32, input_len: u64, output_len_ptr: u32) -> Result<u32, ProcessingError> {
        // Validate input size (10MB limit)
        if input_len > 10_000_000 {
            return Err(ProcessingError::InputTooLarge(input_len));
        }
        
        // Read input from WASM linear memory
        let input_slice = unsafe { 
            std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize) 
        };
        
        // Convert to UTF-8 string
        let input_str = match std::str::from_utf8(input_slice) {
            Ok(s) => s,
            Err(e) => {
                return Err(ProcessingError::InvalidInput(
                    format!("Invalid UTF-8 input: {}", e)
                ));
            }
        };
        
        // Process the input (same logic as before)
        let output = format!("{}-wasm", input_str);
        let output_bytes = output.as_bytes();
        let output_len_val = output_bytes.len();
        
        // Allocate memory for the result
        let result_ptr = match Self::allocate(output_len_val as u64) {
            Ok(ptr) => ptr,
            Err(AllocationError::OutOfMemory) => {
                return Err(ProcessingError::ProcessingFailed(
                    "Failed to allocate memory for output".to_string()
                ));
            }
            Err(AllocationError::InvalidSize(size)) => {
                return Err(ProcessingError::ProcessingFailed(
                    format!("Invalid allocation size: {}", size)
                ));
            }
            Err(AllocationError::MemoryCorruption) => {
                return Err(ProcessingError::ProcessingFailed(
                    "Memory corruption detected".to_string()
                ));
            }
        };
        
        // Copy output data to allocated memory
        unsafe {
            std::ptr::copy_nonoverlapping(
                output_bytes.as_ptr(),
                result_ptr as *mut u8,
                output_len_val
            );
            
            // Write output length to the provided pointer
            *(output_len_ptr as *mut u64) = output_len_val as u64;
        }
        
        Ok(result_ptr)
    }
    
    /// Allocate memory in WASM linear memory
    /// 
    /// This function allocates memory and returns a pointer to it.
    /// Uses structured error handling instead of null pointer returns.
    fn allocate(size: u64) -> Result<u32, AllocationError> {
        // Validate size
        if size == 0 {
            return Err(AllocationError::InvalidSize(size));
        }
        
        if size > 10_000_000 {  // 10MB limit
            return Err(AllocationError::InvalidSize(size));
        }
        
        // Allocate memory using Vec (standard Rust allocation)
        let mut vec = Vec::with_capacity(size as usize);
        vec.resize(size as usize, 0);
        let ptr = vec.as_mut_ptr();
        
        // Prevent Vec from being dropped (transfer ownership to caller)
        std::mem::forget(vec);
        
        Ok(ptr as u32)
    }
    
    /// Deallocate memory in WASM linear memory
    /// 
    /// This function frees memory that was previously allocated.
    /// Note: No error handling for deallocate to match C-style void return.
    fn deallocate(ptr: u32, size: u64) {
        // Handle null pointer (safe to ignore)
        if ptr == 0 {
            return;
        }
        
        // Validate size (just return on invalid size)
        if size == 0 {
            return;
        }
        
        // Reconstruct Vec from raw parts and let it drop (frees memory)
        unsafe {
            let _ = Vec::from_raw_parts(ptr as *mut u8, size as usize, size as usize);
        }
    }
}

// Export the component using WIT bindings
export!(WasmAppenderComponent);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_zero_size() {
        // Test that zero-size allocation returns error
        let result = WasmAppenderComponent::allocate(0);
        assert!(matches!(result, Err(AllocationError::InvalidSize(0))));
    }

    #[test]
    fn test_allocate_too_large() {
        // Test that oversized allocation returns error
        let result = WasmAppenderComponent::allocate(20_000_000); // > 10MB limit
        assert!(matches!(result, Err(AllocationError::InvalidSize(_))));
    }

    #[test]
    fn test_deallocate_null_pointer() {
        // Test that deallocating null pointer is safe
        WasmAppenderComponent::deallocate(0, 100); // Should not panic
    }

    #[test]
    fn test_deallocate_zero_size() {
        // Test that zero-size deallocation is safe
        WasmAppenderComponent::deallocate(100, 0); // Should not panic
    }
}
