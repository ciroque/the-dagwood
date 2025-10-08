# RUSTME: WASM Module Implementation

This directory contains a WebAssembly (WASM) module written in Rust that demonstrates advanced Rust concepts for systems programming, FFI (Foreign Function Interface), and memory management across language boundaries.

## Rust Concepts Demonstrated

### Beginner Level

#### **1. External Function Interface (`extern "C"`)**
```rust
#[no_mangle]
pub extern "C" fn process(input_ptr: *const c_char) -> *mut c_char
```
- **`extern "C"`**: Uses C calling convention for compatibility with other languages
- **`#[no_mangle]`**: Prevents Rust from changing the function name during compilation
- **Purpose**: Makes Rust functions callable from WASM host environments

#### **2. Raw Pointers**
```rust
*const c_char  // Immutable raw pointer to C-style char
*mut c_char    // Mutable raw pointer to C-style char
*const u8      // Immutable raw pointer to byte
```
- **Why needed**: WASM linear memory is accessed via raw pointers
- **Safety**: All pointer operations require `unsafe` blocks
- **Memory model**: Direct memory access without Rust's ownership system

#### **3. C String Interoperability**
```rust
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
```
- **`CStr`**: Borrowed reference to null-terminated C string
- **`CString`**: Owned null-terminated C string
- **`c_char`**: Platform-specific character type for C compatibility

### Intermediate Level

#### **4. Unsafe Code Blocks**
```rust
let input_cstr = unsafe { CStr::from_ptr(input_ptr) };
let result_slice = unsafe {
    let mut len = 0;
    while *result_ptr.add(len) != 0 {
        len += 1;
    }
    std::slice::from_raw_parts(result_ptr, len)
};
```
- **Why unsafe**: Raw pointer dereferencing can cause segfaults
- **Assumptions**: We assume input pointers are valid and properly aligned
- **Responsibility**: Programmer must ensure memory safety manually

#### **5. Memory Layout and Pointer Arithmetic**
```rust
while *result_ptr.add(len) != 0 {  // Find null terminator
    len += 1;
}
```
- **`ptr.add(offset)`**: Safe pointer arithmetic (checks for overflow)
- **Null termination**: C strings end with `\0` byte
- **Linear search**: Manual iteration to find string length

#### **6. Global Allocator Customization**
```rust
use wee_alloc::WeeAlloc;

// Use wee_alloc as the global allocator (safer for WASM)
#[global_allocator]
static ALLOC: WeeAlloc = WeeAlloc::INIT;
```
- **`#[global_allocator]`**: Rust attribute to override default allocator
- **`static`**: Global lifetime for allocator instance
- **WASM-specific**: `wee_alloc` is optimized for WebAssembly environments
- **Memory efficiency**: Smaller code size than default allocator

#### **7. Error Handling with Null Pointers**
```rust
if result_ptr.is_null() {
    return std::ptr::null_mut();
}
```
- **Null as error**: C convention for indicating failure
- **`std::ptr::null_mut()`**: Creates a null mutable pointer
- **Early return**: Fail-fast error handling pattern

### Advanced Level

#### **1. Cross-Language Memory Management**
```rust
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

#[no_mangle]
pub extern "C" fn deallocate(ptr: *mut u8, size: usize) {
    if !ptr.is_null() && size > 0 {
        unsafe {
            // Reconstruct Vec from raw parts and let it drop
            let _ = Vec::from_raw_parts(ptr, size, size);
        }
    }
}
```
- **`std::mem::forget()`**: Prevents Rust from freeing memory automatically
- **Caller responsibility**: Host must call `deallocate()` to free memory
- **Memory leak prevention**: Careful ownership transfer across language boundaries

#### **2. Adapter Pattern Implementation**
```rust
pub extern "C" fn process(input_ptr: *const c_char) -> *mut c_char {
    // Convert C-string to bytes
    let input_bytes = input_cstr.to_bytes();
    
    // Delegate to core implementation
    let mut output_len = 0;
    let result_ptr = process_with_length(input_bytes.as_ptr(), input_bytes.len(), &mut output_len);
    
    // Convert result back to C-string format
    // ... conversion logic
}
```
- **Separation of concerns**: Interface adaptation vs. business logic
- **Code reuse**: Single implementation with multiple interfaces
- **Type conversion**: Bridging between different data representations

#### **3. Manual Memory Management**
```rust
// Allocate memory (including space for null terminator)
let result = allocate(output_len_val + 1);
unsafe {
    // Copy the output bytes
    std::ptr::copy_nonoverlapping(output_bytes.as_ptr(), result, output_len_val);
    // Null-terminate
    *result.add(output_len_val) = 0;
}
```
- **Custom allocator**: Uses module's own `allocate()` function
- **Manual copying**: Direct memory copy with `copy_nonoverlapping`
- **Manual null termination**: Add `\0` byte for C compatibility

This WASM module demonstrates advanced Rust language features for systems programming, FFI (Foreign Function Interface), and memory management across language boundaries in WebAssembly environments.
