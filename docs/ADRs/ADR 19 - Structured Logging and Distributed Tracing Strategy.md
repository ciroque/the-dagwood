# ADR 019: Structured Logging and Distributed Tracing Strategy

## Status
Accepted

## Context

The DAGwood project needs production-ready observability to support:
- Performance analysis and optimization
- Debugging data flow through complex DAGs
- Monitoring and alerting on operational metrics
- Distributed tracing for request flow visualization

Traditional string-based logging has limitations:
- Requires string parsing to extract metrics
- Difficult to query and aggregate
- No automatic trace context propagation
- Not i18n-friendly (messages hardcoded in English)

We've implemented a `StructuredLog` trait that provides:
- `.log()` - Emits human-readable message + machine-readable fields
- `.span()` - Creates OpenTelemetry spans with attributes

**Decision needed**: Where and how should we use structured logging vs distributed tracing across different component types?

## Decision

We will adopt a **dual-strategy approach** based on component type and operation characteristics:

### Strategy 1: Use Spans for Operations with Duration

**Components**: DAG Executors, Processor Execution, WASM Execution

**Pattern**:
```rust
let msg = ExecutionStarted { ... };
let span = msg.span("operation_name");
let _guard = span.enter();

msg.log();  // Log start event with structured fields

// Operation happens here...

CompletionMsg { ... }.log();  // Log completion with metrics
// Span auto-closes when _guard drops
```

**Rationale**:
- Spans capture duration automatically
- Enable distributed tracing across DAG execution
- Provide hierarchical request flow visualization
- Support performance analysis and bottleneck identification

### Strategy 2: Use Structured Logging for Events and Metrics

**Components**: All components (including those with spans)

**Pattern**:
```rust
EventMsg { ... }.log();  // Emits message + structured fields
```

**Rationale**:
- Queryable logs without string parsing
- Automatic metrics extraction from fields
- i18n-ready (fields are language-independent)
- Works for both one-time events and operation lifecycle

## Implementation Guidelines

### DAG Executors (WorkQueue, LevelByLevel, Reactive)

**Use Spans**: ✅ Root span for entire DAG execution
```rust
// In execute_with_strategy()
let start_msg = ExecutionStarted {
    strategy: "WorkQueue",
    processor_count: self.processors.len(),
    max_concurrency: self.max_concurrency,
};

let span = start_msg.span("dag_execution");
let _guard = span.enter();
start_msg.log();

// Processor executions happen here (as child spans)

ExecutionCompleted {
    strategy: "WorkQueue",
    processor_count: self.processors.len(),
    duration,
}.log();
```

**Benefits**:
- Root span for entire request trace
- All processor spans nest under this
- Query traces by strategy, processor_count, concurrency
- Automatic duration tracking

**Use Structured Logging**: ✅ Lifecycle events
- Start: Captures configuration (strategy, concurrency, processor count)
- Complete: Captures results (duration, processor count)
- Fail: Captures error context (strategy, error details)

---

### Built-in Processors (Local Backend)

**Use Spans**: ✅✅ Each processor execution
```rust
// In process() method
let start_msg = ProcessorExecutionStarted {
    processor_id: &self.id,
    input_size: input.payload.len(),
};

let span = start_msg.span("processor_execution");
let _guard = span.enter();
start_msg.log();

// Processing happens here...

ProcessorExecutionCompleted {
    processor_id: &self.id,
    input_size: input.payload.len(),
    output_size: response.payload.len(),
    duration,
}.log();
```

**Benefits**:
- Nested under DAG execution span
- Identify slow processors
- Trace data flow through DAG
- Per-processor performance metrics

**Use Structured Logging**: ✅ Input/output metrics
- Start: processor_id, input_size
- Complete: processor_id, input_size, output_size, duration
- Fail: processor_id, error details

---

### WASM Executors (CStyle, WASI, Wit)

**Use Spans**: ✅✅✅ WASM execution (most critical for performance)
```rust
// In execute() method
let exec_msg = wasm::ExecutionStarted {
    module_path: &self.module_path,
    executor_type: "WitNodeExecutor",
    input_size: input.len(),
};

let span = exec_msg.span("wasm_execution");
let _guard = span.enter();
exec_msg.log();

// WASM execution...

wasm::ExecutionCompleted {
    module_path: &self.module_path,
    executor_type: "WitNodeExecutor",
    input_size: input.len(),
    output_size: output.len(),
    duration,
}.log();
```

**Benefits**:
- Identify slow WASM modules
- Track fuel consumption patterns
- Nested under processor span (3-level hierarchy)
- Compare executor types (CStyle vs WASI vs Wit)

**Use Structured Logging**: ✅✅ WASM-specific metrics
- Module loading: module_path, size_bytes, component_type
- Execution: module_path, executor_type, input_size, output_size, duration
- Failures: module_path, executor_type, error details

---

### Config Validation

**Use Spans**: ❌ No spans
**Use Structured Logging**: ✅ Events only

**Rationale**: One-time startup operation, not request-scoped

```rust
ValidationStarted { processor_count }.log();
// Validation logic...
ValidationCompleted { processor_count, warning_count }.log();
```

---

### Dependency Graph Operations

**Use Spans**: ❌ No spans
**Use Structured Logging**: ✅ Events only

**Rationale**: Internal utility operations, not user-facing

```rust
TopologicalSortFailed { reason }.log();
LevelComputationCompleted { level_count, processor_count }.log();
```

---

### Processor Factories

**Use Spans**: ❌ No spans
**Use Structured Logging**: ✅ Events only

**Rationale**: One-time instantiation during initialization

```rust
ProcessorInstantiationFailed { processor_id, backend, reason }.log();
ProcessorFallbackToStub { processor_id, reason }.log();
```

## Trace Hierarchy

The span strategy creates a 3-level hierarchy:

```
Trace: request_abc123
│
└─ Span: dag_execution (strategy=WorkQueue, processor_count=5)
   ├─ Event: "Starting DAG execution..." [INFO]
   │
   ├─ Span: processor_execution (processor_id="uppercase")
   │  ├─ Event: "Processor 'uppercase' started" [INFO]
   │  │
   │  ├─ Span: wasm_execution (module="uppercase.wasm", executor="Wit")
   │  │  ├─ Event: "WASM execution started" [INFO]
   │  │  └─ Event: "WASM execution completed: 5ms" [INFO]
   │  │
   │  └─ Event: "Processor 'uppercase' completed: 10ms" [INFO]
   │
   ├─ Span: processor_execution (processor_id="reverse")
   │  ├─ Event: "Processor 'reverse' started" [INFO]
   │  └─ Event: "Processor 'reverse' completed: 3ms" [INFO]
   │
   └─ Event: "DAG execution completed: 250ms" [INFO]
```

## Decision Rules

### Use Spans When:
1. **Operation has duration** - Start and end times matter
2. **Request-scoped** - Happens per DAG execution
3. **Hierarchical** - Can nest under parent operation
4. **Performance-critical** - Need to identify bottlenecks

### Use Structured Logging When:
1. **Always** - Every operation logs start/complete/fail
2. **Metrics needed** - Sizes, counts, durations, errors
3. **Queryable data** - Need to filter/aggregate without parsing
4. **One-time events** - Startup, validation, initialization

### Never Use Spans For:
1. **One-time startup** - Validation, initialization
2. **Internal utilities** - Graph algorithms, helpers
3. **Factory operations** - Processor instantiation
4. **Non-request operations** - Configuration loading

## Implementation Pattern

Standard pattern for operations with duration:

```rust
// 1. Create start message
let start_msg = OperationStarted { ... };

// 2. Create span and enter
let span = start_msg.span("operation_name");
let _guard = span.enter();

// 3. Log start event with structured fields
start_msg.log();

// 4. Perform operation
let start_time = Instant::now();
let result = do_work().await;
let duration = start_time.elapsed();

// 5. Log completion/failure
match result {
    Ok(output) => {
        OperationCompleted { ..., duration }.log();
    }
    Err(e) => {
        OperationFailed { ..., error: &e }.log();
    }
}

// 6. Span auto-closes when _guard drops
```

## Benefits

### Queryable Logs
```bash
# Find slow processors
jq 'select(.fields.duration_ms > 100)' logs.json

# Find large payloads
jq 'select(.fields.output_size > 1000000)' logs.json
```

### Automatic Metrics
OpenTelemetry extracts metrics from span attributes:
- `dag_execution_duration{strategy="WorkQueue"}` - histogram
- `processor_execution_count{processor_id="uppercase"}` - counter
- `wasm_execution_duration{executor_type="Wit"}` - histogram

### Distributed Tracing
```bash
# Find slow DAG executions
otel query 'span.name = "dag_execution" AND duration > 1s'

# Find specific processor issues
otel query 'span.attributes.processor_id = "uppercase"'
```

### i18n Ready
Structured fields are language-independent - only messages need translation.

## Consequences

### Positive
- **Production-ready observability** - Comprehensive metrics and tracing
- **Performance analysis** - Identify bottlenecks at DAG/processor/WASM levels
- **Debugging support** - Trace data flow through complex DAGs
- **Monitoring/alerting** - Query logs and create alerts without parsing
- **Future-proof** - Ready for i18n when needed

### Negative
- **Slightly more verbose** - Span creation adds ~3 lines per operation
- **Learning curve** - Developers need to understand span hierarchy
- **Storage cost** - JSON logs are larger than plain text (but compressible)

### Neutral
- **Backward compatible** - Existing `Display` trait preserved
- **Opt-in adoption** - Can migrate incrementally
- **No runtime overhead** - Spans are zero-cost when tracing disabled

## Alternatives Considered

### Alternative 1: Spans Everywhere
**Rejected**: Creates noise for one-time operations (validation, initialization)

### Alternative 2: Structured Logging Only
**Rejected**: Loses distributed tracing benefits and hierarchical request flow

### Alternative 3: Manual Instrumentation
**Rejected**: Error-prone, inconsistent, difficult to maintain

## References
- [OpenTelemetry Tracing Specification](https://opentelemetry.io/docs/specs/otel/trace/)
- [Structured Logging Best Practices](https://www.honeycomb.io/blog/structured-logging-and-your-team)
- `docs/observability/STRUCTURED_LOGGING.md` - Implementation guide
- `src/observability/messages/mod.rs` - StructuredLog trait definition
