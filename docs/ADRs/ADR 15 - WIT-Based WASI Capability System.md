# ADR 15: WIT-Based WASI Capability System

## Status
Accepted

## Context

Following the successful completion of Phase 1 (basic WASI validation), we need to implement a comprehensive capability system for WASM components that:

1. **Enables Modern WASM Languages**: Support Grain, AssemblyScript, and other languages requiring WASI
2. **Maintains Security**: Explicit capability declarations with Principle of Least Privilege
3. **Supports Component Model**: Leverage WebAssembly Component Model and WIT for type safety
4. **Future-Proofs Architecture**: Align with WASI Preview 2 and emerging standards

### Current State
- Phase 0: Interface clarification (`processing-node` terminology)
- Phase 1: Basic WASI validation (allows specific functions, but no runtime provision)
- Existing WasmProcessor: Monolithic implementation violating Single Responsibility Principle

### Requirements
- **Security by Default**: Base components have zero capabilities
- **Explicit Capability Requests**: Components declare exactly what they need
- **WIT-Based Architecture**: Use WebAssembly Interface Types for contracts
- **Component Model Alignment**: Modern signatures, structured errors, automatic serialization
- **Ecosystem Growth**: Support component author namespaces, not vendor lock-in

## Decision

### Architecture: WIT-Based Capability System

#### 1. Base WIT World (Zero Imports)
```wit
package dagwood:base@1.0.0;

interface processing-node {
    /// Process input data and return transformed output
    process: func(input: string) -> result<string, processing-error>;
    
    variant processing-error {
        invalid-input(string),
        processing-failed(string),
        input-too-large(u64),
    }
}

world dagwood-component {
    export processing-node;
    // NO IMPORTS = Complete isolation by default
}
```

#### 2. Component-Specific WIT Files
Each WASM component declares its own capabilities:

```
wasm_components/
├── grain_rle/wit/
│   └── component.wit     # package grain:rle@1.0.0
├── file_processor/wit/
│   └── component.wit     # package myorg:file-processor@1.0.0
```

Example component WIT:
```wit
package grain:rle@1.0.0;

world rle-processor {
    include dagwood:base@1.0.0/dagwood-component;
    import wasi:clocks/monotonic-clock@0.2.0;  // Explicit capability request
}
```

#### 3. Host Implementation Strategy
- **WIT Introspection**: Parse component WIT to discover capability requests
- **Dynamic Provisioning**: Only provide WASI functions that are declared
- **Automatic Validation**: wasmtime fails instantiation if imports aren't satisfied
- **Component Model**: Use wasmtime 25.0+ Component Model APIs (not Preview 1)

#### 4. Namespace Strategy
- **Component Author Namespaces**: `grain:rle`, `myorg:processor` (not `dagwood-components:*`)
- **Ecosystem Growth**: Encourages portable, reusable components
- **Platform Positioning**: DAGwood as platform, not silo

### Implementation Phases

#### Phase 2: WIT-Based Capability Declarations ✅ (This ADR)
- Create base WIT world with zero imports
- Update existing components to use new WIT architecture
- Implement WIT introspection in WasmProcessor
- Refactor WasmProcessor to follow Single Responsibility Principle

#### Phase 3: Full Capability Enforcement (Future)
- Runtime capability checking and advanced capabilities
- Filesystem, network, environment variable access
- DAGwood-specific capabilities (metadata access, logging)

### WasmProcessor Refactoring

Current WasmProcessor violates SRP by handling:
- Module loading and validation
- WIT parsing and capability introspection  
- WASI context creation and linker setup
- WASM execution and memory management
- Error handling and fallback strategies

**Proposed Decomposition**:
1. **WasmModuleLoader**: Module loading, validation, WIT parsing
2. **CapabilityManager**: WIT introspection, WASI context creation
3. **WasmExecutor**: Pure execution engine, memory management
4. **WasmProcessor**: Orchestrates the above, implements Processor trait

## Consequences

### Positive
- **Security by Default**: Zero imports base ensures complete sandboxing
- **Explicit Capabilities**: Clear audit trail of component requirements
- **Component Model Benefits**: Type safety, structured errors, automatic serialization
- **Ecosystem Growth**: Portable components, author ownership
- **Future-Proof**: Aligns with WASI Preview 2 and Component Model evolution
- **Better Architecture**: SRP-compliant WasmProcessor decomposition

### Negative
- **Implementation Complexity**: WIT parsing, Component Model APIs
- **Migration Effort**: Existing components need WIT file updates
- **Tooling Dependencies**: Requires wit-component, wit-parser crates

### Risks and Mitigations
- **wasmtime API Changes**: Pin to specific version, comprehensive testing
- **WIT Tooling Immaturity**: Use stable crates, fallback strategies
- **Component Migration**: Maintain backward compatibility during transition

## Deferred Decisions (Future ADRs)

### Policy-Based Capability Restrictions
- **Current**: Allow all requested capabilities
- **Future**: Configurable policies (e.g., deny network access in production)
- **Rationale**: Not needed for current development phase, adds complexity

### Advanced Capability Types
- **Filesystem Access**: Scoped read/write permissions
- **Network Access**: Specific endpoint restrictions
- **DAGwood Integration**: Metadata access, structured logging
- **Resource Limits**: Memory, CPU, execution time constraints

### WIT Repository Integration
- **Current**: Local WIT files with versioning
- **Future**: External WIT repository when tooling matures
- **Rationale**: Current WIT repository options are insufficient

### Component Discovery and Registry
- **Current**: Manual component configuration in YAML
- **Future**: Component registry, automatic discovery
- **Rationale**: Premature for current scale

## References
- [WebAssembly Component Model](https://github.com/WebAssembly/component-model)
- [WASI Preview 2](https://github.com/WebAssembly/WASI/tree/main/wasip2)
- [WIT Specification](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md)
- Phase 0: ADR 1 - Language Choice (Rust)
- Phase 1: WASI Implementation Plan (docs/wasi-implementation.md)

## Implementation Notes
- Use wasmtime 25.0+ Component Model APIs (not Preview 1)
- Maintain existing versioning strategy for base WIT
- Follow package naming conventions for future WIT repository migration
- Comprehensive testing with real components (Grain RLE, etc.)
