# ADR 18: Observability Implementation

## Status

Accepted

## Context

ADR 8 proposed observability as a future capability with OpenTelemetry as the standard. We are now implementing structured logging and tracing to provide production-ready observability while maintaining simplicity and following idiomatic Rust patterns.

Key requirements identified:
* **Structured logging** with correlation IDs and contextual fields
* **Multiple sink support** for logs, OpenTelemetry collectors, Grafana, etc.
* **Centralized message management** to avoid magic strings and enable future internationalization
* **Minimal code overhead** - focus on error, warn, info levels
* **Clean separation** between user-facing output and diagnostic logging
* **Test-friendly** - silent by default, debuggable on demand

## Decision

### 1. Tracing Crate Ecosystem

We will use the `tracing` crate ecosystem throughout the codebase:
* **`tracing`** - core instrumentation macros (`error!`, `warn!`, `info!`, `debug!`, `trace!`)
* **`tracing-subscriber`** - subscriber implementation with layer support
* **Rationale**: Native Rust solution with excellent OpenTelemetry integration, supports multiple simultaneous sinks through layering, zero-cost when disabled

### 2. Initialization Pattern

**Binary-level initialization** - each binary initializes its own tracing subscriber:
* `main.rs` - structured console output for demos
* Future daemon - JSON logs + OpenTelemetry export
* Future CLI tools - minimal or user-friendly output
* Library code - instrumentation only, no initialization

**Rationale**: Idiomatic Rust pattern - libraries provide instrumentation, binaries decide how to consume events

### 3. Output Separation

**`println!`/`eprintln!` confined to `main.rs`** for user-facing demo output:
* User-facing messages with emojis and formatting stay as `println!`
* All diagnostic/operational logging uses `tracing` macros
* Clear separation between "output for humans" vs. "logs for systems"

**Rationale**: Maintains a clean user experience while providing structured observability for operations

### 4. Log Levels

**Three primary levels** with minimal debug/trace:
* `error!` - failures requiring attention
* `warn!` - potential issues or degraded behavior
* `info!` - important operational events
* `debug!`/`trace!` - used sparingly, only for detailed diagnostics

**Rationale**: Prevents code from becoming 90% tracing statements while providing essential operational visibility

### 5. Structured Fields

**Moderate structured logging** - include key context:
```rust
tracing::error!(
    processor_id = %id,
    error = %e,
    "Processor execution failed"
);
```

* Include processor IDs, error details, relevant metrics
* Avoid excessive fields that clutter logs
* Use `%` for Display formatting, `?` for Debug formatting

**Rationale**: Balances structured data benefits with code readability

### 6. Centralized Messages

**Message types with Display trait** for internationalization-readiness:
```rust
pub struct ProcessorExecutionFailed<'a> {
    pub processor_id: &'a str,
    pub error: &'a dyn std::error::Error,
}

impl Display for ProcessorExecutionFailed<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Processor '{}' execution failed: {}", 
               self.processor_id, self.error)
    }
}

// Usage:
tracing::error!("{}", ProcessorExecutionFailed { 
    processor_id: &id, 
    error: &e 
});
```

**Rationale**: Eliminates magic strings, centralizes message logic (SRP), enables future i18n without code changes

### 7. Module Organization

**`src/observability/` top-level module** organized by subsystem:
```
src/observability/
├── mod.rs
└── messages/
    ├── mod.rs
    ├── engine.rs      # Executor messages
    ├── processor.rs   # Processor messages
    ├── validation.rs  # Validation messages
    └── wasm.rs        # WASM backend messages
```

**Rationale**: Aligns with ADR 8 terminology, provides room for future metrics/spans/traces, maintains SRP as codebase grows

### 8. Main.rs Configuration

**Structured defaults with environment override**:
```rust
tracing_subscriber::fmt()
    .with_target(true)        // Show module path
    .with_level(true)          // Show log level
    .with_env_filter(
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"))
    )
    .init();
```

* Default to INFO level
* Respect `RUST_LOG` environment variable
* Show module path and level for context
* Future: move to a configuration file

**Rationale**: Good defaults for demos, easily overridable, prepares for config-driven observability

### 9. Test Behavior

**Silent tests by default**:
* No tracing subscriber initialization in test code
* Instrumentation present but produces no output
* Developers enable via `RUST_LOG=debug cargo test` when debugging

**Rationale**: Fast clean test output, CI-friendly, on-demand debugging, idiomatic Rust pattern

### 10. Migration Strategy

**Incremental migration**:
1. Create observability module structure
2. Implement message types for existing log points
3. Initialize tracing in `main.rs`
4. Replace `eprintln!` in validation with `tracing::warn!`/`tracing::error!`
5. Add tracing to engine executors
6. Add tracing to processor implementations
7. Add tracing to WASM backend (already partially done)

**Rationale**: Allows testing at each step, maintains working system throughout migration

## Consequences

### Positive

* **Production-ready observability** - structured logs with multiple sink support
* **OpenTelemetry-ready** - tracing crate integrates seamlessly with OTel
* **Maintainable** - centralized messages, clear separation of concerns
* **Future i18n support** - message types enable localization without code changes
* **Developer-friendly** - clean test output, easy debugging with `RUST_LOG`
* **Idiomatic Rust** - follows ecosystem best practices

### Negative

* **Initial migration effort** - need to replace existing `eprintln!` calls
* **Learning curve** - team needs to understand tracing macros and structured logging
* **Message type boilerplate** - each log point needs a corresponding message struct

### Neutral

* **Dependency addition** - `tracing` and `tracing-subscriber` already in Cargo.toml
* **Configuration complexity** - future config file support will add complexity
* **Performance** - tracing has minimal overhead, but not zero-cost when enabled

## Future Considerations

* **Configuration file support** - move tracing configuration from code to YAML
* **Metrics integration** - add `tracing-prometheus` or similar for metrics
* **Distributed tracing** - add `tracing-opentelemetry` for span propagation
* **Log aggregation** - integrate with Grafana Loki, Elasticsearch, etc.
* **Sampling strategies** - implement trace sampling for high-volume scenarios
* **Correlation IDs** - propagate request IDs through processor chains
* **Performance monitoring** - add span timing for latency analysis

## References

* ADR 8: Observability (Proposed) - high-level goals
* [tracing crate documentation](https://docs.rs/tracing/)
* [tracing-subscriber documentation](https://docs.rs/tracing-subscriber/)
* [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust)
