# Getting Started Guide

Ready to dive into The DAGwood project? This guide will help you set up your development environment, run your first pipelines, and start contributing to this exciting Rust-based pipeline orchestration system.

## Quick Start

### Prerequisites

Ensure you have the following installed:

```bash
# Rust toolchain (latest stable)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# WASM target for building WASM modules
rustup target add wasm32-unknown-unknown

# mdBook for viewing documentation
cargo install mdbook

# Optional: WASM optimization tools
cargo install wasm-pack
```

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/your-org/the-dagwood.git
cd the-dagwood

# Build the project
cargo build --release

# Run tests to verify everything works
cargo test

# Run the interactive demo
cargo run --release -- --demo-mode
```

## Your First Pipeline

### 1. Create a Simple Configuration

Create a file called `my-first-pipeline.yaml`:

```yaml
strategy: work_queue
failure_strategy: fail_fast

executor_options:
  max_concurrency: 2

processors:
  - id: greeting
    backend: local
    impl: change_text_case_upper
    depends_on: []
    options: {}

  - id: enthusiasm
    backend: local
    impl: prefix_suffix_adder
    depends_on: [greeting]
    options:
      prefix: "🎉 "
      suffix: " 🚀"
```

### 2. Run Your Pipeline

```bash
cargo run --release -- my-first-pipeline.yaml "hello dagwood"
```

Expected output:
```
🚀 DAGwood Execution Strategy Demo
═══════════════════════════════════
Input: "hello dagwood"
Config files: ["my-first-pipeline.yaml"]

📋 Configuration: my-first-pipeline.yaml
🔧 Strategy: WorkQueue
⚙️  Max Concurrency: 2
🛡️  Failure Strategy: FailFast

📊 Execution Results:
⏱️  Execution Time: ~2ms
🔢 Processors Executed: 2

🔄 Processor Chain:
  1. greeting → "HELLO DAGWOOD"
  2. enthusiasm → "🎉 HELLO DAGWOOD 🚀"

🎯 Final Transformation:
   Input:  "hello dagwood"
   Output: "🎉 HELLO DAGWOOD 🚀"
```

Congratulations! You've just run your first DAGwood pipeline! 🎉

## Understanding the Components

### Configuration Structure

Every DAGwood pipeline is defined by a YAML configuration:

```yaml
# Execution strategy selection
strategy: work_queue  # Options: work_queue, level, reactive

# Error handling behavior
failure_strategy: fail_fast  # Options: fail_fast, continue_on_error, best_effort

# Executor configuration
executor_options:
  max_concurrency: 4  # Maximum parallel processors

# Processor definitions
processors:
  - id: unique_processor_name
    backend: local  # Backend type: local, wasm
    impl: processor_implementation  # Specific processor to use
    depends_on: [list_of_dependencies]  # Dependency processors
    options:  # Processor-specific configuration
      key: value
```

### Available Processors

The local backend provides several built-in processors:

#### Text Transformation
```yaml
# Change text case
- impl: change_text_case_upper    # HELLO WORLD
- impl: change_text_case_lower    # hello world
- impl: change_text_case_proper   # Hello World
- impl: change_text_case_title    # Hello World

# Reverse text
- impl: reverse_text              # "hello" → "olleh"

# Add prefix/suffix
- impl: prefix_suffix_adder
  options:
    prefix: ">>> "
    suffix: " <<<"
```

#### Text Analysis
```yaml
# Count tokens
- impl: token_counter
  options:
    count_type: "words"     # Options: words, characters, lines

# Analyze word frequency
- impl: word_frequency_analyzer   # Returns JSON with word counts
```

### Execution Strategies

Choose the right strategy for your pipeline:

#### Work Queue (Default)
```yaml
strategy: work_queue
```
- **Best for**: Irregular DAGs, maximum parallelism
- **Algorithm**: Dependency counting with priority queue
- **Parallelism**: Maximum - executes processors as soon as dependencies complete

#### Level-by-Level
```yaml
strategy: level
```
- **Best for**: Regular DAGs, predictable execution
- **Algorithm**: Topological level computation
- **Parallelism**: Within levels only - waits for entire level completion

#### Reactive
```yaml
strategy: reactive
```
- **Best for**: Real-time pipelines, I/O-bound processors
- **Algorithm**: Event-driven execution with immediate response
- **Parallelism**: Maximum responsiveness - processors react instantly to dependency completion

## Building Your Own Processors

### Local Processor Development

Create a new processor by implementing the `Processor` trait:

```rust
// src/backends/local/processors/my_processor.rs
use crate::traits::processor::{Processor, ProcessorIntent};
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, processor_response::Outcome};
use async_trait::async_trait;

pub struct MyProcessor {
    config: String,
}

impl MyProcessor {
    pub fn new(config: String) -> Self {
        MyProcessor { config }
    }
}

#[async_trait]
impl Processor for MyProcessor {
    async fn process(&self, input: ProcessorRequest) -> Result<ProcessorResponse, ProcessorError> {
        // Convert input bytes to string
        let input_text = String::from_utf8_lossy(&input.payload);
        
        // Your processing logic here
        let output = format!("Processed: {}", input_text);
        
        // Return response
        Ok(ProcessorResponse {
            outcome: Some(Outcome::NextPayload(output.into_bytes())),
            metadata: None, // Add metadata if needed
        })
    }
    
    fn declared_intent(&self) -> ProcessorIntent {
        ProcessorIntent::Transform  // or ProcessorIntent::Analyze
    }
}
```

### Register Your Processor

Add your processor to the factory:

```rust
// src/backends/local/factory.rs
impl LocalProcessorFactory {
    pub fn create_processor(config: &ProcessorConfig) -> Result<Box<dyn Processor>, ProcessorError> {
        let impl_name = config.impl_.as_deref().unwrap_or("stub");
        
        match impl_name {
            // ... existing processors
            "my_processor" => {
                let config_value = config.options.get("config")
                    .unwrap_or(&"default".to_string())
                    .clone();
                Ok(Box::new(MyProcessor::new(config_value)))
            },
            // ... rest of match
        }
    }
}
```

### Use Your Processor

```yaml
processors:
  - id: my_custom_step
    backend: local
    impl: my_processor
    depends_on: []
    options:
      config: "my configuration"
```

## WASM Processor Development

### Create a WASM Module

```rust
// wasm_modules/my_module/src/lib.rs
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn process(input_ptr: *const c_char) -> *mut c_char {
    let input = unsafe {
        if input_ptr.is_null() {
            return std::ptr::null_mut();
        }
        CStr::from_ptr(input_ptr).to_string_lossy().into_owned()
    };
    
    // Your WASM processing logic
    let output = format!("WASM processed: {}", input);
    
    match CString::new(output) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn allocate(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[no_mangle]
pub extern "C" fn deallocate(ptr: *mut u8, size: usize) {
    unsafe {
        Vec::from_raw_parts(ptr, 0, size);
    }
}
```

### Build the WASM Module

```bash
cd wasm_modules/my_module
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/my_module.wasm ../my_module.wasm
```

### Use Your WASM Processor

```yaml
processors:
  - id: wasm_step
    backend: wasm
    module: wasm_modules/my_module.wasm
    depends_on: []
    options:
      intent: transform
```

## Advanced Pipelines

### Diamond Dependency Pattern

Create pipelines with parallel processing:

```yaml
processors:
  # Entry point
  - id: input_processor
    backend: local
    impl: change_text_case_lower
    depends_on: []

  # Parallel processing
  - id: analysis_a
    backend: local
    impl: token_counter
    depends_on: [input_processor]
    options:
      count_type: "words"

  - id: analysis_b
    backend: local
    impl: word_frequency_analyzer
    depends_on: [input_processor]

  # Convergence point
  - id: final_processor
    backend: local
    impl: prefix_suffix_adder
    depends_on: [analysis_a, analysis_b]
    options:
      prefix: "Analysis: "
      suffix: " [Complete]"
```

### Multi-Backend Pipelines

Combine local and WASM processors:

```yaml
processors:
  - id: local_prep
    backend: local
    impl: change_text_case_upper
    depends_on: []

  - id: wasm_processing
    backend: wasm
    module: wasm_modules/hello_world.wasm
    depends_on: [local_prep]
    options:
      intent: transform

  - id: local_finalize
    backend: local
    impl: prefix_suffix_adder
    depends_on: [wasm_processing]
    options:
      prefix: "🦀 "
      suffix: " ✨"
```

## Debugging and Troubleshooting

### Common Issues

#### Compilation Errors
```bash
# Clean and rebuild
cargo clean
cargo build --release

# Check for missing dependencies
cargo check
```

#### WASM Module Issues
```bash
# Verify WASM module exists
ls -la wasm_modules/

# Rebuild WASM module
cd wasm_modules/hello_world
cargo build --target wasm32-unknown-unknown --release
```

#### Configuration Errors
```bash
# Validate configuration syntax
# DAGwood will show detailed error messages for invalid configs
cargo run --release -- invalid-config.yaml "test"
```

### Debugging Tips

#### Enable Detailed Logging
```rust
// Add to your main.rs for debugging
env_logger::init();
```

#### Use the Demo Mode
```bash
# Interactive demo with explanations
cargo run --release -- --demo-mode
```

#### Examine Test Cases
```bash
# Run specific tests
cargo test work_queue
cargo test integration_tests
cargo test wasm
```

## Performance Optimization

### Choosing the Right Strategy

```yaml
# For irregular DAGs with high parallelism potential
strategy: work_queue

# For regular, layered DAGs
strategy: level

# Adjust concurrency based on your system
executor_options:
  max_concurrency: 8  # Usually 1-2x CPU cores
```

### Processor Optimization

#### Transform vs Analyze Intent
```rust
// Transform processors modify data
fn declared_intent(&self) -> ProcessorIntent {
    ProcessorIntent::Transform
}

// Analyze processors only add metadata
fn declared_intent(&self) -> ProcessorIntent {
    ProcessorIntent::Analyze
}
```

#### Efficient Memory Usage
```rust
// Avoid unnecessary clones
let input_text = String::from_utf8_lossy(&input.payload);

// Use references when possible
fn process_text(text: &str) -> String {
    // Processing logic
}
```

## Next Steps

### Explore the Codebase

1. **Start with the demo**: `docs/demo/src/`
2. **Examine processors**: `src/backends/local/processors/`
3. **Study executors**: `src/engine/`
4. **Understand WASM integration**: `src/backends/wasm/`

### Join the Community

1. **GitHub Discussions**: Ask questions and share ideas
2. **Issues**: Report bugs or request features
3. **Pull Requests**: Contribute improvements
4. **Documentation**: Help improve guides and examples

### Learning Resources

1. **Rust Book**: https://doc.rust-lang.org/book/
2. **Async Rust**: https://rust-lang.github.io/async-book/
3. **WASM Book**: https://rustwasm.github.io/docs/book/
4. **Tokio Tutorial**: https://tokio.rs/tokio/tutorial

### Contribution Ideas

#### Beginner-Friendly
- Add new local processors
- Improve documentation and examples
- Write additional test cases
- Create configuration templates

#### Intermediate
- Implement the Reactive executor
- Add WASI support to WASM backend
- Create performance benchmarks
- Build CLI tools and utilities

#### Advanced
- Design the Hybrid executor
- Implement distributed execution
- Add machine learning optimization
- Contribute to research and algorithms

---

> 🎯 **Success Path**: Start with the interactive demo, experiment with configurations, build your own processors, and gradually explore the advanced features. The DAGwood project is designed to be both a learning platform and a production-ready system - enjoy the journey!
