# WASM Module Architecture

This document describes the architectural decisions and design patterns used in the hello_world WASM module implementation.

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

2. **`process_with_length(input_ptr: *const u8, input_len: usize, output_len: *mut usize)`**
   - Explicit length parameter
   - More efficient (no strlen needed)
   - Better for binary data
   - Returns output length via pointer parameter

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

This WASM module demonstrates production-ready systems programming patterns for safely bridging between Rust's memory safety and the raw pointer world of WASM linear memory.
