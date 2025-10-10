# WASM Component Architecture

WebAssembly (WASM) integration in The DAGwood project represents a cutting-edge approach to secure, sandboxed processor execution. This chapter explores the architecture, security model, and implementation details of the WASM backend.

## WASM Integration Overview

### Why WASM for Workflow Orchestration?

Traditional workflow systems face several challenges when executing user-provided code:

- **Security**: Untrusted code can access host resources
- **Isolation**: Processor failures can crash the entire system  
- **Language Lock-in**: Limited to the host language ecosystem
- **Determinism**: Non-deterministic execution across environments

WASM solves these problems by providing:

```rust
// WASM benefits in practice
struct WasmBenefits {
    security: SecurityLevel::Complete,        // No host access by default
    isolation: IsolationLevel::ProcessLevel,  // Separate memory space
    languages: Vec<Language>,                 // Rust, C, Go, AssemblyScript, etc.
    determinism: bool,                        // true - same input = same output
    performance: PerformanceLevel::NearNative, // ~95% of native speed
}
```

## Architecture Components

### 1. WASM Runtime Integration

The DAGwood WASM backend uses wasmtime, the industry-standard WASM runtime:

```rust
// Core WASM processor structure
pub struct WasmProcessor {
    engine: Engine,           // WASM compilation engine
    module: Module,           // Compiled WASM module
    module_path: String,      // Path for debugging/metadata
}

impl WasmProcessor {
    pub fn new(module_path: &str) -> Result<Self, WasmError> {
        // 1. Create wasmtime engine with security configuration
        let mut config = Config::new();
        config.wasm_simd(true);           // Enable SIMD for performance
        config.wasm_bulk_memory(true);    // Enable bulk memory operations
        config.consume_fuel(true);        // Enable execution limits
        
        let engine = Engine::new(&config)?;
        
        // 2. Load and compile WASM module
        let module_bytes = std::fs::read(module_path)
            .map_err(|e| WasmError::ModuleLoadError { 
                path: module_path.to_string(), 
                source: e 
            })?;
            
        let module = Module::new(&engine, &module_bytes)
            .map_err(|e| WasmError::CompilationError { 
                path: module_path.to_string(), 
                source: e 
            })?;
        
        Ok(WasmProcessor {
            engine,
            module,
            module_path: module_path.to_string(),
        })
    }
}
```

### 2. Memory Management Architecture

WASM modules have their own linear memory space, requiring careful coordination:

```rust
// WASM memory management interface
#[repr(C)]
pub struct WasmInterface {
    // Required functions that WASM modules must export
    process_fn: extern "C" fn(*const c_char) -> *mut c_char,
    allocate_fn: extern "C" fn(usize) -> *mut u8,
    deallocate_fn: extern "C" fn(*mut u8, usize),
}

impl WasmProcessor {
    async fn execute_wasm(&self, input: &str) -> Result<String, WasmError> {
        // 1. Create isolated store for this execution
        let mut store = Store::new(&self.engine, ());
        
        // 2. Set resource limits
        store.limiter(|_| ResourceLimiter::new(
            memory_limit: 64 * 1024 * 1024,  // 64MB memory limit
            fuel_limit: 1_000_000,           // Execution time limit
        ));
        
        // 3. Instantiate module in isolated environment
        let instance = Instance::new(&mut store, &self.module, &[])?;
        
        // 4. Get required function exports
        let process_fn = instance.get_typed_func::<i32, i32>(&mut store, "process")?;
        let allocate_fn = instance.get_typed_func::<i32, i32>(&mut store, "allocate")?;
        let deallocate_fn = instance.get_typed_func::<(i32, i32), ()>(&mut store, "deallocate")?;
        
        // 5. Allocate input string in WASM memory
        let input_bytes = input.as_bytes();
        let input_len = input_bytes.len() as i32;
        let input_ptr = allocate_fn.call(&mut store, input_len + 1)?; // +1 for null terminator
        
        // 6. Copy input data to WASM memory
        let memory = instance.get_memory(&mut store, "memory")?;
        memory.write(&mut store, input_ptr as usize, input_bytes)?;
        memory.write(&mut store, (input_ptr + input_len) as usize, &[0])?; // null terminator
        
        // 7. Call WASM function
        let output_ptr = process_fn.call(&mut store, input_ptr)?;
        
        // 8. Read output from WASM memory
        let output = self.read_c_string_from_memory(&mut store, &memory, output_ptr)?;
        
        // 9. Clean up WASM memory
        deallocate_fn.call(&mut store, (input_ptr, input_len + 1))?;
        // Note: WASM module is responsible for deallocating output_ptr
        
        Ok(output)
    }
}
```

### 3. Security Sandboxing Model

WASM provides multiple layers of security isolation:

```rust
// Security layers in WASM execution
pub struct WasmSecurityModel {
    // Layer 1: Memory isolation
    memory_isolation: MemoryIsolation {
        linear_memory: true,        // WASM has its own memory space
        no_shared_memory: true,     // Cannot access host memory
        bounds_checking: true,      // All memory accesses are bounds-checked
    },
    
    // Layer 2: Capability-based security
    capabilities: HostCapabilities {
        file_system_access: false,  // No file system access by default
        network_access: false,      // No network access by default
        system_calls: false,        // No direct system calls
        host_functions: Vec::new(), // Only explicitly linked functions
    },
    
    // Layer 3: Resource limits
    resource_limits: ResourceLimits {
        memory_limit: 64 * 1024 * 1024,  // 64MB
        execution_time: Duration::from_secs(30),
        fuel_consumption: 1_000_000,
    },
    
    // Layer 4: Deterministic execution
    determinism: DeterminismGuarantees {
        no_random_sources: true,    // No access to random number generators
        no_time_sources: true,      // No access to system time
        reproducible_results: true, // Same input always produces same output
    },
}
```

## WASM Module Interface

### Standard Interface Contract

All WASM modules must implement a standard interface:

```rust
// Required exports from WASM modules
pub trait WasmModuleInterface {
    // Primary processing function
    fn process(input_ptr: *const c_char) -> *mut c_char;
    
    // Memory management functions
    fn allocate(size: usize) -> *mut u8;
    fn deallocate(ptr: *mut u8, size: usize);
    
    // Optional: metadata and introspection
    fn get_module_info() -> *const c_char;
    fn get_supported_formats() -> *const c_char;
}
```

### Example WASM Module Implementation

Here's how a WASM module is implemented in Rust:

```rust
// wasm_components/hello/src/lib.rs
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn process(input_ptr: *const c_char) -> *mut c_char {
    // 1. Convert C string to Rust String
    let input = unsafe {
        if input_ptr.is_null() {
            return std::ptr::null_mut();
        }
        
        match CStr::from_ptr(input_ptr).to_str() {
            Ok(s) => s.to_owned(),
            Err(_) => return std::ptr::null_mut(),
        }
    };
    
    // 2. Process the input (business logic)
    let output = format!("{}-wasm", input);
    
    // 3. Convert back to C string for return
    match CString::new(output) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
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
        // Reconstruct Vec to trigger proper deallocation
        let _ = Vec::from_raw_parts(ptr, 0, size);
    }
}

// Cargo.toml configuration for WASM compilation
/*
[lib]
crate-type = ["cdylib"]

[dependencies]
# Minimal dependencies for WASM
*/
```

### Compilation Process

```bash
# Build WASM module from Rust
cd wasm_components/hello_wasm
cargo build --target wasm32-unknown-unknown --release

# Copy to expected location
cp target/wasm32-unknown-unknown/release/hello_wasm.wasm ../hello.wasm

# Optional: Optimize WASM module
wasm-opt -Oz hello.wasm -o hello_wasm_optimized.wasm
```

## Multi-Language Support

### Language Ecosystem

WASM enables polyglot processor development:

```rust
// Supported languages for WASM processors
pub enum WasmLanguage {
    Rust {
        toolchain: "stable",
        target: "wasm32-unknown-unknown",
        features: vec!["memory-safe", "zero-cost-abstractions"],
    },
    C {
        compiler: "clang",
        target: "wasm32",
        features: vec!["manual-memory-management", "low-level-control"],
    },
    Go {
        compiler: "tinygo",
        target: "wasm",
        features: vec!["garbage-collected", "concurrent-safe"],
    },
    AssemblyScript {
        compiler: "asc",
        target: "wasm32",
        features: vec!["typescript-like", "web-optimized"],
    },
}
```

### Cross-Language Interface Example

```c
// C implementation of the same interface
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

__attribute__((export_name("process")))
char* process(const char* input) {
    if (!input) return NULL;
    
    size_t input_len = strlen(input);
    size_t output_len = input_len + 6; // "-wasm\0"
    
    char* output = malloc(output_len);
    if (!output) return NULL;
    
    snprintf(output, output_len, "%s-wasm", input);
    return output;
}

__attribute__((export_name("allocate")))
void* allocate(size_t size) {
    return malloc(size);
}

__attribute__((export_name("deallocate")))
void deallocate(void* ptr, size_t size) {
    free(ptr);
}
```

## Performance Characteristics

### Execution Performance

WASM provides near-native performance with safety guarantees:

```rust
// Performance comparison (hypothetical benchmarks)
struct PerformanceBenchmark {
    operation: &'static str,
    native_time: Duration,
    wasm_time: Duration,
    overhead_percent: f64,
}

let benchmarks = vec![
    PerformanceBenchmark {
        operation: "String processing",
        native_time: Duration::from_micros(100),
        wasm_time: Duration::from_micros(105),
        overhead_percent: 5.0,
    },
    PerformanceBenchmark {
        operation: "Mathematical computation",
        native_time: Duration::from_micros(50),
        wasm_time: Duration::from_micros(52),
        overhead_percent: 4.0,
    },
    PerformanceBenchmark {
        operation: "Memory allocation",
        native_time: Duration::from_micros(10),
        wasm_time: Duration::from_micros(15),
        overhead_percent: 50.0, // Higher overhead for memory operations
    },
];
```

### Memory Efficiency

```rust
// Memory usage patterns
struct WasmMemoryProfile {
    module_size: usize,           // Compiled WASM module size
    linear_memory: usize,         // WASM linear memory allocation
    host_overhead: usize,         // wasmtime runtime overhead
    total_footprint: usize,       // Total memory usage
}

impl WasmMemoryProfile {
    fn analyze_module(module_path: &str) -> Self {
        WasmMemoryProfile {
            module_size: 50 * 1024,      // ~50KB for simple modules
            linear_memory: 1024 * 1024,  // 1MB default linear memory
            host_overhead: 100 * 1024,   // ~100KB wasmtime overhead
            total_footprint: 1174 * 1024, // ~1.17MB total
        }
    }
}
```

## Advanced WASM Features

### WASI Integration (Future)

WebAssembly System Interface (WASI) will enable controlled system access:

```rust
// Planned WASI integration
pub struct WasiCapabilities {
    file_system: FileSystemAccess {
        read_only_directories: vec!["/tmp/dagwood/input"],
        write_directories: vec!["/tmp/dagwood/output"],
        forbidden_paths: vec!["/etc", "/home", "/root"],
    },
    network: NetworkAccess {
        allowed_domains: vec!["api.example.com"],
        forbidden_protocols: vec!["file://", "ftp://"],
    },
    environment: EnvironmentAccess {
        allowed_vars: vec!["DAGWOOD_CONFIG"],
        forbidden_vars: vec!["HOME", "PATH"],
    },
}
```

### Component Model (Future)

The WASM Component Model will enable more sophisticated interfaces:

```rust
// Planned Component Model integration
pub trait WasmComponent {
    type Input: Serialize + DeserializeOwned;
    type Output: Serialize + DeserializeOwned;
    type Error: Serialize + DeserializeOwned;
    
    async fn process(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;
    
    fn metadata(&self) -> ComponentMetadata;
    fn dependencies(&self) -> Vec<ComponentDependency>;
}
```

## Security Considerations

### Threat Model

WASM processors defend against various attack vectors:

```rust
pub enum SecurityThreat {
    // Memory safety threats
    BufferOverflow { mitigated_by: "WASM bounds checking" },
    UseAfterFree { mitigated_by: "WASM linear memory model" },
    
    // Resource exhaustion threats  
    InfiniteLoop { mitigated_by: "Fuel limits and timeouts" },
    MemoryExhaustion { mitigated_by: "Memory limits" },
    
    // Information disclosure threats
    MemoryLeakage { mitigated_by: "Isolated linear memory" },
    FileSystemAccess { mitigated_by: "No file system capabilities" },
    
    // Code injection threats
    DynamicCodeExecution { mitigated_by: "Static WASM validation" },
    HostFunctionAbuse { mitigated_by: "Explicit capability linking" },
}
```

### Security Best Practices

```rust
// Security configuration for production
impl WasmProcessor {
    pub fn new_secure(module_path: &str) -> Result<Self, WasmError> {
        let mut config = Config::new();
        
        // Enable security features
        config.consume_fuel(true);              // Execution limits
        config.epoch_interruption(true);       // Cooperative interruption
        config.max_wasm_stack(64 * 1024);     // Stack limit
        
        // Disable potentially unsafe features
        config.wasm_threads(false);            // No threading
        config.wasm_reference_types(false);    // No reference types
        config.wasm_multi_memory(false);       // Single memory space
        
        let engine = Engine::new(&config)?;
        // ... rest of initialization
    }
}
```

## Integration with DAG Execution

### Factory Integration

WASM processors integrate seamlessly with the processor factory:

```rust
// WASM processor factory
pub struct WasmProcessorFactory;

impl ProcessorFactory for WasmProcessorFactory {
    fn create_processor(&self, config: &ProcessorConfig) -> Result<Box<dyn Processor>, ProcessorError> {
        let module_path = config.module.as_ref()
            .ok_or_else(|| ProcessorError::ConfigurationError {
                message: "WASM processor requires 'module' field".to_string()
            })?;
            
        let processor = WasmProcessor::new(module_path)
            .map_err(|e| ProcessorError::CreationError { source: Box::new(e) })?;
            
        Ok(Box::new(processor))
    }
}
```

### Metadata Collection

WASM processors provide rich execution metadata:

```rust
impl Processor for WasmProcessor {
    async fn process(&self, input: ProcessorRequest) -> Result<ProcessorResponse, ProcessorError> {
        let start_time = Instant::now();
        let input_str = String::from_utf8_lossy(&input.payload);
        
        // Execute WASM module
        let output = self.execute_wasm(&input_str).await?;
        let execution_time = start_time.elapsed();
        
        // Collect execution metadata
        let mut metadata = HashMap::new();
        metadata.insert("module_path".to_string(), self.module_path.clone());
        metadata.insert("input_length".to_string(), input.payload.len().to_string());
        metadata.insert("output_length".to_string(), output.len().to_string());
        metadata.insert("execution_time_ms".to_string(), execution_time.as_millis().to_string());
        
        Ok(ProcessorResponse {
            outcome: Some(Outcome::NextPayload(output.into_bytes())),
            metadata: Some(PipelineMetadata {
                metadata: HashMap::from([
                    ("wasm_execution".to_string(), ProcessorMetadata { metadata })
                ])
            }),
        })
    }
}
```

---

> 🔒 **Security Philosophy**: WASM represents a paradigm shift in secure code execution. By providing strong isolation guarantees while maintaining near-native performance, it enables The DAGwood project to safely execute untrusted code in production environments - a capability that opens up entirely new possibilities for workflow orchestration systems.
