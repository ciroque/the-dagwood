# ADR 16 - WASM Processing Node Strategy Pattern

**Status:** Accepted  
**Date:** 2025-01-10  
**Deciders:** Development Team  

## Context

The DAGwood WASM backend currently has a monolithic approach where the `WasmProcessor` handles all types of WASM artifacts (C-style modules, WASI Preview 1 modules, and WIT components) in a single execution path. As we implement support for multiple WASM artifact types with different execution requirements, we need a clean architectural pattern that:

1. **Separates concerns** between artifact detection, strategy selection, and execution
2. **Enables extensibility** for future WASM artifact types (Preview 2, custom runtimes)
3. **Provides type-specific error handling** and debugging capabilities
4. **Maintains performance** while supporting diverse execution models

## Problem

The current implementation mixes detection logic with execution logic, making it difficult to:
- Add new WASM artifact types without modifying core processor logic
- Provide artifact-specific error messages and debugging information
- Test execution strategies in isolation
- Optimize execution paths for different artifact types

## Decision

We will implement a **Strategy Pattern** for WASM artifact execution using the following architecture:

### Core Components

#### 1. Processing Node Executor Trait
```rust
#[async_trait]
trait ProcessingNodeExecutor {
    async fn execute(&self, input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError>;
    fn artifact_type(&self) -> &'static str;
    fn capabilities(&self) -> Vec<String>;
}
```

#### 2. Strategy Implementations
- **`ComponentNodeExecutor`**: Preview 2 WIT Components (The New Hotness)
- **`WasiNodeExecutor`**: Preview 1 WASI Modules (Legacy but Common)
- **`CStyleNodeExecutor`**: C-Style Modules (Old Reliable)

#### 3. Processing Node Factory
```rust
struct ProcessingNodeFactory;

impl ProcessingNodeFactory {
    fn create_executor(loaded_module: LoadedModule) -> Box<dyn ProcessingNodeExecutor>;
}
```

#### 4. Strategy-Specific Error Types
```rust
enum ProcessingNodeError {
    ComponentError(ComponentExecutionError),
    WasiError(WasiExecutionError),
    CStyleError(CStyleExecutionError),
    ValidationError(String),
    RuntimeError(String),
}
```

### Architecture Flow

1. **Factory Detection**: `WasmProcessorFactory` loads module and detects artifact type
2. **Strategy Creation**: Factory creates appropriate `ProcessingNodeExecutor` implementation
3. **Processor Orchestration**: `WasmProcessor` uses strategy for execution and handles metadata
4. **Error Propagation**: Strategy-specific errors provide detailed debugging context

## Rationale

### Strategy Pattern Benefits
- **Single Responsibility**: Each executor handles one artifact type
- **Open/Closed Principle**: Easy to add new executors without modifying existing code
- **Testability**: Each strategy can be unit tested in isolation
- **Performance**: Type-specific optimizations possible

### Async Execution
- **Consistency**: Matches `Processor::process()` async signature
- **Future-proof**: Enables timeout support and WASI I/O operations
- **Integration**: Works seamlessly with tokio-based DAG executors

### Strategy-Specific Errors
- **Debugging**: Clear error context ("WIT component validation failed" vs "WASI function missing")
- **Error Recovery**: Different strategies can implement different recovery approaches
- **Telemetry**: Strategy-specific error metrics for monitoring
- **User Experience**: Actionable error messages with artifact-specific guidance

## Implementation Plan

### Phase 1: Core Infrastructure
1. Define `ProcessingNodeExecutor` trait
2. Create `ProcessingNodeError` enum with strategy variants
3. Implement `ProcessingNodeFactory` with detection logic

### Phase 2: Strategy Implementations
1. `CStyleNodeExecutor` - migrate existing C-style logic
2. `WasiNodeExecutor` - implement WASI Preview 1 support
3. `ComponentNodeExecutor` - placeholder for future WIT component support

### Phase 3: Integration
1. Update `WasmProcessor` to use strategy pattern
2. Update `WasmProcessorFactory` to use `ProcessingNodeFactory`
3. Comprehensive testing of all execution paths

## Consequences

### Positive
- **Extensibility**: Easy to add new WASM artifact types
- **Maintainability**: Clear separation of concerns
- **Testability**: Each strategy can be tested independently
- **Performance**: Type-specific optimizations possible
- **Error Handling**: Rich, context-specific error information
- **Future-Ready**: Foundation for Preview 2 and custom runtimes

### Negative
- **Complexity**: Additional abstraction layer
- **Initial Development**: More upfront implementation work
- **Memory**: Slight overhead from trait objects

### Risks
- **Over-engineering**: Strategy pattern might be overkill for current needs
- **Performance**: Async trait calls have minimal overhead

### Mitigations
- Start with simple implementations and evolve as needed
- Comprehensive benchmarking to validate performance assumptions
- Clear documentation and examples for each strategy

## Alternatives Considered

### Alternative 1: Monolithic Processor
**Rejected**: Current approach doesn't scale with multiple artifact types

### Alternative 2: Separate Processor Types
**Rejected**: Would require factory complexity and duplicate metadata handling

### Alternative 3: Enum-Based Dispatch
**Rejected**: Less extensible and harder to test than strategy pattern

## References

- [Strategy Pattern - Gang of Four](https://en.wikipedia.org/wiki/Strategy_pattern)
- [WebAssembly Component Model](https://github.com/WebAssembly/component-model)
- [WASI Preview 1 vs Preview 2](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/README.md)
- [DAGwood ADR 3 - Processor Backend Architecture](./ADR%203%20-%20Processor%20Backend%20Architecture.md)

## Status

**Accepted** - This ADR provides a clean, extensible architecture for supporting multiple WASM artifact types while maintaining performance and providing excellent debugging capabilities.
