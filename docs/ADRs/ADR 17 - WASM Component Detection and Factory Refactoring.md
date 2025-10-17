# ADR 17 - WASM Component Detection and Factory Refactoring

**Status:** Accepted  
**Date:** 2025-01-17  
**Deciders:** Development Team  
**Supersedes:** Portions of module_loader.rs implementation

## Context

Following ADR 16's introduction of the Processing Node Strategy Pattern, the WASM backend's module loading and detection logic needs refactoring. The current `WasmModuleLoader` struct violates separation of concerns by:

1. **Reading file bytes** (I/O responsibility)
2. **Detecting component type** (binary analysis responsibility)
3. **Creating engines** (configuration responsibility)
4. **Parsing imports** (capability analysis responsibility)
5. **Instantiating modules/components** (factory responsibility)

Additionally, the current implementation has a critical bug: it creates an engine with `wasm_component_model(false)` then tries to parse as a Component, which always fails. This forces a fallback to Module parsing even for valid Component Model components.

## Problem

The monolithic `WasmModuleLoader::load_module()` makes it difficult to:
- Test component detection independently of file I/O
- Create engines with appropriate configurations for different component types
- Add new WASM encoding types without modifying loader logic
- Reason about responsibilities and dependencies

The current flow is:
```rust
WasmModuleLoader::load_module(path)
  ↓ creates engine with component_model=false
  ↓ tries Component::new() ← FAILS
  ↓ falls back to Module::new()
  ↓ returns LoadedModule
```

## Decision

We will refactor the WASM backend into **four separate, stateless modules** using pure functions with clear separation of concerns:

### Architecture Components

#### 1. **loader.rs** - File I/O and Validation
```rust
/// Loads WASM bytes from file and validates size
pub fn load_wasm_bytes<P: AsRef<Path>>(path: P) -> WasmResult<Vec<u8>>
```

**Responsibility:** Read file, validate size limits  
**No Dependencies:** Standalone I/O operations

#### 2. **detector.rs** - Binary Parsing and Type Detection
```rust
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WasmEncoding {
    ComponentModel,  // Modern Component Model (binary version 2+)
    Classic,         // Core WASM modules (version 1, no component section)
}

/// Determines WASM encoding by inspecting version header and custom sections
pub fn wasm_encoding(bytes: &[u8]) -> Result<WasmEncoding>
```

**Responsibility:** Parse WASM binary format, detect encoding type  
**Uses:** `wasmparser` crate for spec-compliant parsing  
**Rejects:** Legacy Preview 1 components (version 1 + "component" custom section)

#### 3. **capability_manager.rs** - Engine Configuration
```rust
/// Creates a Wasmtime engine configured for the given WASM encoding type
pub fn create_engine(encoding: WasmEncoding) -> WasmResult<Engine>
```

**Responsibility:** Configure engine features based on encoding type  
**Current:** Sets `wasm_component_model(true/false)` and security flags  
**Future:** WASI import validation and security policy enforcement

#### 4. **factory.rs** - Executor Creation and Orchestration
```rust
/// Creates the appropriate executor based on WASM encoding
pub fn create_executor(
    bytes: &[u8],
    encoding: WasmEncoding,
) -> WasmResult<Box<dyn ProcessingNodeExecutor>>
```

**Responsibility:** Orchestrate engine creation, module/component instantiation, and executor wrapping

### Integration Flow

```rust
// In WasmProcessor::from_config()
let bytes = load_wasm_bytes(module_path)?;           // loader.rs
let encoding = wasm_encoding(&bytes)?;                // detector.rs
let executor = create_executor(&bytes, encoding)?;    // factory.rs (uses capability_manager)
```

### Design Principles

1. **Stateless Functions**: No structs with methods, just pure functions
   - Consistent with user preference: "let's NEVER BE CLEVER"
   - Easier to test, reason about, and compose
   - No unnecessary state management

2. **Single Responsibility**: Each module has one clear job
   - loader: File I/O
   - detector: Binary analysis
   - capability_manager: Engine configuration
   - factory: Orchestration

3. **Spec-Compliant Detection**: Use `wasmparser` instead of trial-and-error
   - Inspects binary version header
   - Checks for "component" custom section
   - Explicitly rejects unsupported Legacy Preview 1

4. **Proper Engine Configuration**: Match engine features to encoding
   - ComponentModel → `wasm_component_model(true)`
   - Classic → `wasm_component_model(false)`

## Rationale

### Pure Functions Over Structs
**Why?**
- Zero state to manage
- Simpler mental model
- Easier unit testing
- Aligns with Rust idioms for utility functions

**Example:**
```rust
// Simple: Just call functions
let bytes = load_wasm_bytes(path)?;
let encoding = wasm_encoding(&bytes)?;

// Complex: Create and manage structs
let loader = WasmModuleLoader::new(config);
let detector = ComponentDetector::new();
```

### wasmparser for Detection
**Why not try Component::new() and fallback?**
- Trial-and-error creates wrong engine config
- Can't distinguish between "not a component" and "component parsing error"
- `wasmparser` gives definitive answers without instantiation overhead

**Benefits:**
- Fast: Just reads headers, no full parsing
- Accurate: Spec-compliant detection
- Clear errors: Can distinguish encoding types

### CapabilityManager Evolution
**Current Role:** Engine feature configuration
- Sets `wasm_component_model` flag based on encoding
- Configures security flags (fuel, no threads, etc.)

**Future Role:** Security policy enforcement
- Validate WASI imports against allowed list
- Component-level sandboxing
- Per-processor capability configuration

**Name Justification:**
- "Capability" encompasses both engine features (now) and runtime permissions (future)
- Natural place for security-related configuration

## Implementation Plan

### Phase 1: Create New Modules
1. ✅ Create `detector.rs` with `wasm_encoding()` implementation
2. ✅ Refactor `module_loader.rs` → `loader.rs` with just `load_wasm_bytes()`
3. ✅ Uncomment and simplify `capability_manager.rs` with `create_engine()`
4. ✅ Create `factory.rs` with `create_executor()`

### Phase 2: Integration
1. ✅ Update `processor.rs` to use new 3-function flow
2. ✅ Update `mod.rs` exports
3. ✅ Remove old `LoadedModule` struct and related types

### Phase 3: Testing
1. ✅ Unit tests for each module
2. ✅ Integration test with real Component Model WASM
3. ✅ Integration test with C-style WASM
4. ✅ Test Legacy Preview 1 rejection

## Consequences

### Positive
- **Clear Separation**: Each module has one responsibility
- **Testability**: Functions can be tested in isolation
- **Correct Behavior**: Engine configured properly for encoding type
- **Maintainability**: Easy to locate and modify specific functionality
- **Extensibility**: Adding new encoding types is straightforward
- **Simplicity**: No unnecessary structs or state management

### Negative
- **More Files**: 4 modules instead of 1 monolithic loader
- **Learning Curve**: Developers need to understand the flow across modules

### Neutral
- **Breaking Change**: Old `WasmModuleLoader::load_module()` API removed
  - Only used internally, no external API impact

### Risks & Mitigations

**Risk:** Over-fragmentation makes code hard to follow  
**Mitigation:** Each module is small, focused, and well-documented. Flow is linear: load → detect → create.

**Risk:** `wasmparser` dependency adds complexity  
**Mitigation:** `wasmparser` is maintained by Bytecode Alliance (same as Wasmtime). It's the canonical WASM parsing library.

## Alternatives Considered

### Alternative 1: Keep Monolithic Loader, Just Fix Engine Config
```rust
impl WasmModuleLoader {
    fn load_module(path) -> LoadedModule {
        // Try component engine first
        // Fallback to classic engine
    }
}
```

**Rejected:**
- Still mixes responsibilities
- Trial-and-error approach is fragile
- Doesn't address testability issues

### Alternative 2: Loader + Factory Only (No Separate Detector)
```rust
// factory.rs
fn create_executor(path) -> Box<dyn ProcessingNodeExecutor> {
    let bytes = load_wasm_bytes(path)?;
    // Inline detection logic here
}
```

**Rejected:**
- Factory would mix detection with orchestration
- Can't test detection independently
- Harder to reuse detection logic

### Alternative 3: Struct-Based Architecture
```rust
struct WasmLoader { config: LoaderConfig }
struct ComponentDetector { /* state */ }
struct CapabilityManager { security_config: SecurityConfig }
```

**Rejected:**
- Unnecessary state management
- More complex API
- Goes against user preference for simplicity

## References

- [WebAssembly Component Model Specification](https://github.com/WebAssembly/component-model)
- [wasmparser Documentation](https://docs.rs/wasmparser/)
- [Wasmtime Component Model Guide](https://docs.wasmtime.dev/api/wasmtime/component/)
- [ADR 16 - WASM Processing Node Strategy Pattern](./ADR%2016%20-%20WASM%20Processing%20Node%20Strategy%20Pattern.md)

## Status

**Accepted** - This refactoring provides clean separation of concerns, fixes the engine configuration bug, and establishes a maintainable foundation for WASM backend evolution.
