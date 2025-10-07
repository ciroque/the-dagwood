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

#### **6. Error Handling with Null Pointers**
```rust
if result_ptr.is_null() {
    return std::ptr::null_mut();
}
```
- **Null as error**: C convention for indicating failure
- **`std::ptr::null_mut()`**: Creates a null mutable pointer
- **Early return**: Fail-fast error handling pattern

### Advanced Level

#### **7. Cross-Language Memory Management**
```rust
// Allocate memory that caller must free
match CString::new(output) {
    Ok(c_string) => c_string.into_raw(),  // Transfer ownership
    Err(_) => std::ptr::null_mut(),
}
```
- **`into_raw()`**: Transfers ownership to caller, prevents Rust from freeing memory
- **Caller responsibility**: Host must call appropriate deallocation function
- **Memory leak prevention**: Careful ownership transfer across language boundaries

#### **8. Adapter Pattern Implementation**
```rust
pub extern "C" fn process(input_ptr: *const c_char) -> *mut c_char {
    // Convert C-string to bytes
    let input_bytes = input_cstr.to_bytes();
    
    // Delegate to core implementation
    let result_ptr = process_with_length(input_bytes.as_ptr(), input_bytes.len() as i32);
    
    // Convert result back to C-string format
    // ... conversion logic
}
```
- **Separation of concerns**: Interface adaptation vs. business logic
- **Code reuse**: Single implementation with multiple interfaces
- **Type conversion**: Bridging between different data representations

#### **9. Static Memory Management**
```rust
let mut result = Vec::with_capacity(output_bytes.len() + 1);
result.extend_from_slice(&output_bytes);
result.push(0); // Null terminator
result.leak().as_ptr()
```
- **`Vec::leak()`**: Intentionally leak memory to transfer ownership
- **Capacity optimization**: Pre-allocate exact size needed
- **Manual null termination**: Add `\0` byte for C compatibility

## WASM-Specific Concepts

### **Linear Memory Model**
- WASM has a single, contiguous memory space
- All data access goes through byte offsets (pointers)
- No garbage collector - manual memory management required

### **Host-Guest Communication**
- **Host**: The DAGwood WASM processor (Rust/wasmtime)
- **Guest**: This WASM module (compiled Rust)
- **Interface**: Function calls with pointer/length parameters

### **Compilation Target**
```toml
[lib]
crate-type = ["cdylib"]  # Create C-compatible dynamic library

# Compile with:
cargo build --target wasm32-unknown-unknown --release
```

## Architecture Decisions

### **Why Two Function Interfaces?**

1. **`process(input_ptr: *const c_char)`**
   - C-style null-terminated strings
   - Compatible with traditional C libraries
   - Requires string length calculation

2. **`process_with_length(input_ptr: *const u8, input_len: i32)`**
   - Explicit length parameter
   - More efficient (no strlen needed)
   - Better for binary data

### **Why `process()` Delegates to `process_with_length()`?**
- **Single source of truth**: Business logic only in one place
- **DRY principle**: Avoid code duplication
- **Easier maintenance**: Changes only needed in core function
- **Performance**: Length-based interface is more efficient

## Safety Considerations

### **Assumptions Made**
1. Input pointers are valid and properly aligned
2. Input strings are valid UTF-8
3. Caller will properly free returned memory
4. WASM linear memory is accessible

### **Error Handling Strategy**
- Return null pointers on any error
- Validate UTF-8 encoding before processing
- Use `match` expressions for safe error propagation
- Fail fast rather than undefined behavior

## Testing Strategy

### **Unit Testing Challenges**
- Cannot easily test `extern "C"` functions in Rust tests
- Pointer-based interfaces require integration testing
- Memory management testing needs host environment

### **Integration Testing**
- Test through DAGwood WASM processor
- Verify memory allocation/deallocation
- Test error conditions (invalid UTF-8, null pointers)

## Performance Considerations

### **Memory Efficiency**
- Pre-allocate vectors with known capacity
- Minimize string conversions
- Reuse allocations where possible

### **CPU Efficiency**
- Avoid unnecessary UTF-8 validation
- Use pointer arithmetic instead of string operations
- Delegate to most efficient implementation

## Common Pitfalls

1. **Memory Leaks**: Forgetting to free returned pointers
2. **Use After Free**: Accessing freed WASM memory
3. **Buffer Overruns**: Not validating pointer bounds
4. **UTF-8 Violations**: Assuming all byte sequences are valid strings
5. **Null Pointer Dereference**: Not checking for null before use

This WASM module demonstrates production-ready systems programming in Rust, showcasing how to safely bridge between Rust's memory safety and the raw pointer world of WASM linear memory.
