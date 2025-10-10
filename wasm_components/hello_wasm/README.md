# Hello World WASM Module

This directory contains a WebAssembly module written in Rust that demonstrates basic WASM functionality for The DAGwood project.

## Building the WASM Module

### Prerequisites
- Rust toolchain with `wasm32-unknown-unknown` target
- wasmtime CLI tool for testing

Install the WASM target if not already available:
```bash
rustup target add wasm32-unknown-unknown
```

### Compilation Commands

Build the WASM module in release mode:
```bash
cargo build --target wasm32-unknown-unknown --release
```

Copy the compiled WASM file to the expected location:
```bash
cp target/wasm32-unknown-unknown/release/hello_wasm.wasm ../hello.wasm
```

Combined build and copy:
```bash
cargo build --target wasm32-unknown-unknown --release && cp target/wasm32-unknown-unknown/release/hello_wasm.wasm ../hello.wasm
```

## Testing with Wasmtime

### Test the allocate function
```bash
cd ..
wasmtime --invoke allocate hello.wasm 51
```
Expected output: A memory pointer (e.g., `1114120`)

### Test the process function
```bash
# Note: This requires setting up memory and string pointers, which is complex from CLI
# The process function is best tested through the DAGwood processor
```

### Test the deallocate function
```bash
wasmtime --invoke deallocate hello.wasm 1114120 51
```
Expected output: No output (void function)

## Module Interface

The WASM module exports the following functions:

- `allocate(size: usize) -> *mut u8` - Allocates memory in WASM linear memory
- `deallocate(ptr: *mut u8, size: usize)` - Frees previously allocated memory  
- `process(input_ptr: *const u8, input_len: usize, output_len: *mut usize) -> *mut u8` - Main processing function

## Integration with DAGwood

The compiled WASM module (`hello.wasm`) is used by the DAGwood WASM processor backend. The module implements a simple text transformation that appends "-wasm" to the input string.

Example DAGwood configuration:
```yaml
processors:
  - id: wasm_hello_world
    type: wasm
    module: wasm_components/hello.wasm
    options:
      intent: transform
```

## Troubleshooting

If you encounter compilation errors:
1. Ensure the `wasm32-unknown-unknown` target is installed
2. Check that all dependencies are compatible with WASM compilation
3. Verify that no `std` features requiring system calls are used

If wasmtime testing fails:
1. Ensure wasmtime is installed: `cargo install wasmtime-cli`
2. Check that the WASM file exists and is not corrupted
3. Verify function signatures match the expected interface
