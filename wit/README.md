# DAGwood WASM Component Interface (WIT)

This directory contains the WebAssembly Interface Types (WIT) specification for DAGwood processor components.

## 🚀 Quick Start

### For Component Authors
```bash
# Create a new component with WIT dependencies
./scripts/setup-wit-deps.sh my_processor v1.0.0

# Or manually copy the latest WIT interface
cp wit/latest.wit your-component/wit/dagwood-processor.wit
```

### For DAGwood Users
The WIT interface is automatically handled by the DAGwood runtime. Components using WIT bindings provide better error messages and type safety compared to C-style exports.

## 📁 Directory Structure

```
wit/
├── README.md              # This file
├── wit-deps.toml          # Global WIT dependencies
├── latest.wit             # Symlink to latest stable version
├── versions/              # Versioned WIT releases
│   ├── VERSIONS.md        # Version history and compatibility
│   └── v1.0.0/           # Specific version directory
│       └── dagwood-processor.wit
└── examples/              # Example implementations
    └── migration-example.rs
```

## Overview

The `dagwood-processor.wit` file defines a formal contract for implementing WASM processors that can be executed within the DAGwood workflow orchestration system. This WIT specification enables:

- **Type Safety**: Compile-time verification of component interfaces
- **Language Interoperability**: Generate bindings for multiple languages (Rust, C, JavaScript, Python, etc.)
- **Tooling Integration**: IDE support, documentation generation, and testing tools
- **Future Compatibility**: Migration path to WASM Component Model

## Current vs Future Architecture

### Current Implementation (Core WASM)
- Uses C-style exports: `process()`, `allocate()`, `deallocate()`
- Manual memory management with raw pointers
- String-based interface with UTF-8 validation
- Direct wasmtime integration

### Future Implementation (Component Model)
- Uses WIT-generated bindings with automatic serialization
- Managed memory with garbage collection
- Rich type system with records, variants, and enums
- Standard component runtime (wasmtime, wasm-tools)

## Using This WIT File

### 1. Generate Language Bindings

```bash
# Install wit-bindgen
cargo install wit-bindgen-cli

# Generate Rust bindings
wit-bindgen rust wit/dagwood-processor.wit --out-dir src/bindings

# Generate JavaScript bindings  
wit-bindgen js wit/dagwood-processor.wit --out-dir js/bindings

# Generate Python bindings
wit-bindgen py wit/dagwood-processor.wit --out-dir python/bindings
```

### 2. Implement Processor Component (Rust Example)

```rust
// src/lib.rs
use bindings::exports::dagwood::processor::processor::{
    Guest, ProcessorError, ProcessorIntent, ProcessingResult, ProcessorInfo
};

struct MyProcessor;

impl Guest for MyProcessor {
    fn process(input: String) -> Result<ProcessingResult, ProcessorError> {
        // Your processing logic here
        let output = format!("{}-processed", input);
        
        Ok(ProcessingResult {
            output,
            metadata: Some(ExecutionMetadata {
                processing_time_ms: Some(42),
                input_size_bytes: Some(input.len() as u64),
                output_size_bytes: Some(output.len() as u64),
                custom_fields: vec![
                    ("algorithm".to_string(), "simple-append".to_string())
                ],
            })
        })
    }
    
    fn get_intent() -> ProcessorIntent {
        ProcessorIntent::Transform
    }
    
    fn get_info() -> ProcessorInfo {
        ProcessorInfo {
            name: "my-processor".to_string(),
            version: "1.0.0".to_string(),
            description: "Example processor implementation".to_string(),
            supported_formats: vec!["text/plain".to_string()],
        }
    }
}

bindings::export!(MyProcessor with_types_in bindings);
```

### 3. Build Component

```bash
# Add to Cargo.toml
[dependencies]
wit-bindgen = "0.16"

[lib]
crate-type = ["cdylib"]

# Build WASM component
cargo build --target wasm32-wasi --release

# Convert to component (requires wasm-tools)
wasm-tools component new target/wasm32-wasi/release/my_processor.wasm \
    -o my_processor.wasm
```

## Migration Strategy

### Phase 1: Parallel Implementation (Current)
- Keep existing C-style interface for backward compatibility
- Implement WIT-based components alongside current system
- Validate WIT interface with real-world processors

### Phase 2: Gradual Migration
- Add component model support to DAGwood runtime
- Provide migration tools for existing processors
- Support both interfaces during transition period

### Phase 3: Full Component Model
- Deprecate C-style interface
- Full migration to WASM Component Model
- Enhanced tooling and developer experience

## Interface Design Decisions

### String-Based Interface
- **Rationale**: Simplifies initial implementation and testing
- **Trade-off**: Less efficient than binary formats but more debuggable
- **Future**: Can extend to support binary data via `list<u8>` types

### Error Handling
- **Structured Errors**: Variant type enables proper error categorization
- **Error Context**: String messages provide debugging information
- **Recovery Strategy**: DAGwood runtime can make informed decisions

### Metadata System
- **Optional Metadata**: Processors can provide execution information
- **Extensible Fields**: Custom key-value pairs for processor-specific data
- **Performance Metrics**: Built-in fields for common performance data

### Processor Intent
- **Architectural Clarity**: Transform vs Analyze classification
- **Optimization Enablement**: Runtime can optimize based on intent
- **Future Extensions**: Can add more intent types as needed

## Tooling and Development

### Recommended Tools
- **wit-bindgen**: Generate language bindings from WIT files
- **wasm-tools**: Component model utilities and validation
- **wasmtime**: Runtime for testing components
- **cargo-component**: Rust-specific component build tool

### Testing Strategy
- **Unit Tests**: Test individual processor functions
- **Integration Tests**: Test with DAGwood runtime
- **Compatibility Tests**: Validate WIT interface compliance
- **Performance Tests**: Benchmark component vs core WASM

## Security Considerations

### Sandboxing
- Components inherit WASM's security model
- No access to host system resources
- Memory isolation between components

### Resource Limits
- Execution time bounded by fuel consumption
- Memory usage limited by WASM linear memory
- Input size validation prevents DoS attacks

### Validation
- WIT interface provides compile-time type checking
- Runtime validation of component exports
- Structured error handling prevents information leakage

## Future Enhancements

### Rich Type System
- Support for binary data (`list<u8>`)
- Structured input/output types (JSON, protobuf)
- Stream processing interfaces

### Advanced Features
- Async processing support
- Multi-input/multi-output processors
- State management and persistence

### Ecosystem Integration
- Package registry for processor components
- Versioning and compatibility management
- Automated testing and validation tools

## Contributing

When modifying the WIT interface:

1. **Backward Compatibility**: Ensure changes don't break existing implementations
2. **Documentation**: Update this README and inline documentation
3. **Testing**: Validate changes with real processor implementations
4. **Versioning**: Follow semantic versioning for interface changes

## Resources

- [WebAssembly Component Model](https://github.com/WebAssembly/component-model)
- [WIT Specification](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md)
- [wit-bindgen Documentation](https://github.com/bytecodealliance/wit-bindgen)
- [wasm-tools](https://github.com/bytecodealliance/wasm-tools)
