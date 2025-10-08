# WASM Integration: Sandboxed Processing

The fourth demonstration introduces WebAssembly (WASM) processors, showcasing cutting-edge sandboxing technology and multi-language support within the Rust-based DAG execution system.

## What You'll Learn

- **WASM module loading and execution** with wasmtime
- **Memory management** across WASM boundaries
- **Security sandboxing** and isolation patterns
- **Multi-backend processor** architecture

## Configuration Overview

```yaml
# Demo 4: WASM Integration - Sandboxed Processing
# This demonstrates WASM processor integration with security sandboxing

strategy: work_queue
failure_strategy: fail_fast

executor_options:
  max_concurrency: 2

processors:
  # Local processor prepares input
  - id: prepare_input
    backend: local
    impl: change_text_case_lower
    depends_on: []
    options: {}

  # WASM processor provides sandboxed execution
  - id: wasm_hello_world
    backend: wasm
    module: wasm_modules/hello_world.wasm
    depends_on: [prepare_input]
    options:
      intent: transform

  # Local processor adds final formatting
  - id: final_format
    backend: local
    impl: prefix_suffix_adder
    depends_on: [wasm_hello_world]
    options:
      prefix: "🦀 Rust + WASM: "
      suffix: " ✨"
```

### Multi-Backend Architecture

This configuration demonstrates seamless integration between:
- **Local backend**: Native Rust processors
- **WASM backend**: Sandboxed WASM modules
- **Mixed execution**: Local → WASM → Local pipeline

## Rust Concepts in Action

### 1. WASM Runtime Integration

The WASM processor uses wasmtime for secure execution:

```rust
// From src/backends/wasm/processor.rs
use wasmtime::{Engine, Module, Store, Instance, Caller, Linker};

pub struct WasmProcessor {
    engine: Engine,
    module: Module,
    module_path: String,
}

impl WasmProcessor {
    pub fn new(module_path: &str) -> Result<Self, WasmError> {
        let engine = Engine::default();
        let module_bytes = std::fs::read(module_path)?;
        let module = Module::new(&engine, &module_bytes)?;
        
        Ok(WasmProcessor {
            engine,
            module,
            module_path: module_path.to_string(),
        })
    }
}
```

**Key Rust features**:
- **Error propagation**: `?` operator for clean error handling
- **Ownership**: Module bytes are owned by the processor
- **Resource management**: Engine and Module are automatically cleaned up

### 2. Memory Management Across Boundaries

WASM modules must manage their own memory, with careful coordination:

```rust
// WASM module interface (C-style for WASM compatibility)
#[no_mangle]
pub extern "C" fn process(input_ptr: *const c_char) -> *mut c_char {
    // Convert C string to Rust String
    let input = unsafe {
        CStr::from_ptr(input_ptr).to_string_lossy().into_owned()
    };
    
    // Process the input
    let output = format!("{}-wasm", input);
    
    // Convert back to C string (caller must free!)
    let c_string = CString::new(output).unwrap();
    c_string.into_raw()
}

#[no_mangle]
pub extern "C" fn allocate(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf); // Prevent Rust from deallocating
    ptr
}

#[no_mangle]
pub extern "C" fn deallocate(ptr: *mut u8, size: usize) {
    unsafe {
        Vec::from_raw_parts(ptr, 0, size); // Reconstruct Vec to deallocate
    }
}
```

**Memory safety patterns**:
- **Explicit allocation**: WASM module controls its memory
- **Careful ownership transfer**: `into_raw()` and `from_raw_parts()`
- **Resource cleanup**: Proper deallocation prevents leaks

### 3. Secure Sandboxing

The wasmtime runtime provides complete isolation:

```rust
// Host function linking (controlled capabilities)
let mut linker = Linker::new(&engine);

// Only expose specific host functions
linker.func_wrap("env", "host_log", |caller: Caller<'_, ()>, ptr: i32, len: i32| {
    // Controlled logging capability
    // WASM module cannot access arbitrary host functions
})?;

// Create isolated store for this execution
let mut store = Store::new(&engine, ());
let instance = linker.instantiate(&mut store, &module)?;
```

**Security benefits**:
- **No file system access**: WASM cannot read/write files
- **No network access**: Complete network isolation
- **Controlled host interaction**: Only explicitly linked functions available
- **Memory isolation**: WASM linear memory is separate from host

### 4. Cross-Language Execution

The same WASM interface could be implemented in multiple languages:

```rust
// Rust implementation (current example)
#[no_mangle]
pub extern "C" fn process(input_ptr: *const c_char) -> *mut c_char {
    // Rust processing logic
}
```

```c
// Hypothetical C implementation
char* process(const char* input) {
    // C processing logic
    char* output = malloc(strlen(input) + 6);
    sprintf(output, "%s-wasm", input);
    return output;
}
```

```typescript
// Hypothetical AssemblyScript implementation
export function process(input: string): string {
    return input + "-wasm";
}
```

## Expected Output

```
📋 Configuration: docs/demo/configs/04-wasm-integration.yaml
🔧 Strategy: WorkQueue
⚙️  Max Concurrency: 2
🛡️  Failure Strategy: FailFast

📊 Execution Results:
⏱️  Execution Time: ~5ms
🔢 Processors Executed: 3

🔄 Processor Chain:
  1. prepare_input → "hello world"
  2. wasm_hello_world → "hello world-wasm"
     📝 Metadata: 3 entries (e.g., module_path)
  3. final_format → "🦀 Rust + WASM: hello world-wasm ✨"

🎯 Final Transformation:
   Input:  "hello world"
   Output: "🦀 Rust + WASM: hello world-wasm ✨"
   
   Pipeline Metadata:
   wasm_hello_world:
      • module_path: wasm_modules/hello_world.wasm
      • input_length: 11
      • output_length: 16
```

## Architecture Deep Dive

### WASM Module Compilation

The hello_world WASM module is compiled from Rust:

```toml
# wasm_modules/hello_world/Cargo.toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
```

```bash
# Compilation process
cd wasm_modules/hello_world
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/hello_world.wasm ../hello_world.wasm
```

### Factory Pattern Integration

The WASM backend integrates seamlessly with the processor factory:

```rust
// From src/backends/wasm/factory.rs
impl ProcessorFactory for WasmProcessorFactory {
    fn create_processor(&self, config: &ProcessorConfig) -> Result<Box<dyn Processor>, ProcessorError> {
        let module_path = config.module.as_ref()
            .ok_or_else(|| ProcessorError::ConfigurationError {
                message: "WASM processor requires 'module' field".to_string()
            })?;
            
        let processor = WasmProcessor::new(module_path)?;
        Ok(Box::new(processor))
    }
}
```

### Performance Characteristics

WASM execution has different performance characteristics:

- **Startup cost**: Module loading and instantiation (~1-2ms)
- **Execution speed**: Near-native performance for compute-intensive tasks
- **Memory overhead**: Separate linear memory space
- **Security overhead**: Sandboxing adds minimal runtime cost

## Security Analysis

### Threat Model

WASM processors provide defense against:

- **Malicious code execution**: Complete sandboxing prevents host compromise
- **Resource exhaustion**: Memory and CPU limits can be enforced
- **Data exfiltration**: No network or file system access
- **Side-channel attacks**: Isolated execution environment

### Trust Boundaries

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Local Proc    │    │   WASM Proc     │    │   Local Proc    │
│   (Trusted)     │───▶│  (Sandboxed)    │───▶│   (Trusted)     │
│                 │    │                 │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
        │                        │                        │
        ▼                        ▼                        ▼
   Host Memory              WASM Memory               Host Memory
```

## Try It Yourself

### Building the WASM Module

```bash
# Install WASM target
rustup target add wasm32-unknown-unknown

# Build the module
cd wasm_modules/hello_world
cargo build --target wasm32-unknown-unknown --release
```

### Experimenting with WASM

1. **Modify the WASM logic**: Change the `-wasm` suffix to something else
2. **Add computation**: Implement a more complex algorithm in WASM
3. **Test isolation**: Try to access host resources (it should fail!)

## What's Next?

In the final demo, the exploration moves to a **complex multi-backend workflow** that combines everything learned:
- Multiple execution strategies
- Mixed local and WASM processors
- Advanced error handling
- Production-ready patterns

---

> 🔒 **Security Insight**: WASM represents the future of secure code execution. By combining Rust's memory safety with WASM's sandboxing, both performance and security are achieved - essential for processing untrusted code in production environments!
