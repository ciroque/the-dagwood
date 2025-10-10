# DAGwood WIT Interface Versions

This directory contains versioned releases of the DAGwood processor WIT interface.

## Version History

### v1.0.0 (Current)
- **Release Date**: 2025-10-09
- **Status**: ✅ Stable
- **Breaking Changes**: Initial release
- **Features**:
  - Structured error handling (`ProcessingError`, `AllocationError`)
  - Memory management functions (`allocate`, `deallocate`, `process`)
  - Type-safe pointer operations with `u32` and `u64` types
  - Complete sandboxing (no imports allowed)

#### Interface Summary
```wit
interface processor {
    variant allocation-error { ... }
    variant processing-error { ... }
    
    process: func(input-ptr: u32, input-len: u64, output-len-ptr: u32) -> result<u32, processing-error>;
    allocate: func(size: u64) -> result<u32, allocation-error>;
    deallocate: func(ptr: u32, size: u64);
}
```

#### Compatible Components
- `hello_wasm` v1.0.0

#### Migration Notes
- Migrated from C-style exports to WIT bindings
- Replaced null pointer errors with structured error types
- Added input size validation (10MB limit)

## Usage

### For Component Authors
```bash
# Copy WIT file to your component
cp wit/versions/v1.0.0/dagwood-processor.wit your-component/wit/

# Add to Cargo.toml
[dependencies]
wit-bindgen = "0.30"
```

### For DAGwood Runtime
The runtime automatically detects WIT vs C-style components and handles both interfaces.

## Compatibility Matrix

| WIT Version | DAGwood Runtime | Component Model | Status |
|-------------|-----------------|-----------------|---------|
| v1.0.0      | ≥ 0.1.0        | Core WASM      | ✅ Stable |

## Future Versions

### Planned v1.1.0
- Binary data support (beyond UTF-8 strings)
- Streaming interface for large payloads
- Enhanced metadata support

### Planned v2.0.0 (Breaking)
- Full Component Model migration
- Rich type system (records, variants, resources)
- Async processing support
- Multi-input/multi-output processors
