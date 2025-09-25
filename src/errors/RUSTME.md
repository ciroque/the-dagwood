# Rust Language Features: Error Handling Patterns

This directory demonstrates Rust's powerful error handling system and how to build robust, type-safe error management for complex systems.

## Beginner: Result Types and Error Propagation

### The `Result<T, E>` Type
Rust uses `Result<T, E>` instead of exceptions for error handling:

```rust
// Instead of throwing exceptions, functions return Results
fn validate_processor_id(id: &str) -> Result<(), ValidationError> {
    if id.is_empty() {
        Err(ValidationError::EmptyProcessorId)
    } else {
        Ok(())
    }
}
```

**Why this is better:**
- Errors are part of the function signature
- Compiler forces you to handle errors
- No hidden control flow like exceptions

### The `?` Operator for Error Propagation
```rust
fn load_and_validate_config(path: &str) -> Result<Config, ValidationError> {
    let config = load_config(path)?;  // Propagates error if load_config fails
    validate_dependency_graph(&config)?;  // Propagates validation errors
    Ok(config)  // Only reached if both operations succeed
}
```

The `?` operator automatically converts compatible error types and returns early on failure.

## Intermediate: Custom Error Types and Enums

### Structured Error Enums
Our `ValidationError` enum demonstrates how to create expressive error types:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    CyclicDependency { cycle_path: Vec<String> },
    UnresolvedDependency { processor_id: String, missing_dependency: String },
    DuplicateProcessorId { processor_id: String },
}
```

**Key Rust features used:**
- **Enum variants with data**: Each error carries relevant context
- **Derive macros**: `Debug`, `Clone`, `PartialEq` auto-generate implementations
- **Structured data**: Errors contain actionable information, not just strings

### The `Display` Trait for User-Friendly Messages
```rust
impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::CyclicDependency { cycle_path } => {
                write!(f, "Cyclic dependency detected: {}", cycle_path.join(" -> "))
            }
            ValidationError::UnresolvedDependency { processor_id, missing_dependency } => {
                write!(f, "Processor '{}' depends on non-existent processor '{}'", 
                       processor_id, missing_dependency)
            }
            // ... other variants
        }
    }
}
```

**Why this pattern works:**
- Separates error data (enum) from presentation (Display)
- Enables both programmatic error handling and user-friendly messages
- Follows Rust's trait-based design philosophy

## Advanced: Error Composition and From Conversions

### Hierarchical Error Types
Our `ExecutionError` demonstrates how to compose errors from different subsystems:

```rust
#[derive(Debug, Clone)]
pub enum ExecutionError {
    ProcessorNotFound(String),
    ProcessorFailed { processor_id: String, error_message: String },
    ValidationFailed(ValidationError),  // Wraps validation errors
    MultipleFailed { failures: Vec<ExecutionError> },  // Recursive composition
}
```

### Automatic Error Conversion with `From`
```rust
impl From<ValidationError> for ExecutionError {
    fn from(validation_error: ValidationError) -> Self {
        ExecutionError::ValidationFailed(validation_error)
    }
}
```

This enables seamless error propagation across subsystem boundaries:
```rust
fn execute_dag(config: &Config) -> Result<DagResults, ExecutionError> {
    validate_dependency_graph(config)?;  // ValidationError auto-converts to ExecutionError
    // ... rest of execution
}
```

### Error Aggregation Patterns
For systems that can have multiple simultaneous failures:

```rust
ExecutionError::MultipleFailed { 
    failures: vec![
        ExecutionError::ProcessorFailed { /* ... */ },
        ExecutionError::ProcessorFailed { /* ... */ },
    ]
}
```

**Advanced techniques demonstrated:**
- **Recursive error types**: Errors can contain other errors
- **Zero-cost conversions**: `From` trait enables automatic error type conversion
- **Failure aggregation**: Collect multiple errors instead of failing fast

## Key Rust Concepts Demonstrated

1. **Sum Types (Enums)**: Model different error conditions as enum variants
2. **Pattern Matching**: Handle different error types with `match` expressions
3. **Trait System**: `Display`, `Error`, `From` traits for error interoperability
4. **Zero-Cost Abstractions**: Rich error types with no runtime overhead
5. **Ownership**: Error types own their data, enabling safe error propagation
6. **Type Safety**: Impossible to ignore errors or have unhandled error states

## Design Principles Applied

- **Make invalid states unrepresentable**: Use types to prevent error conditions
- **Fail fast with context**: Provide actionable error information
- **Composable error handling**: Errors from different subsystems work together
- **No silent failures**: All error conditions are explicit and typed

This error handling approach makes The DAGwood robust and debuggable while leveraging Rust's type system to prevent entire classes of runtime errors.
