# Structured Logging and Distributed Tracing

This document explains how to use The DAGwood's structured logging and distributed tracing capabilities.

## Overview

All message types in `src/observability/messages/` implement two traits:
- **`Display`** - Human-readable output (supports future i18n)
- **`StructuredLog`** - Machine-readable fields + OpenTelemetry span creation

## Quick Start

### Basic Logging (Human-Readable)

```rust
use the_dagwood::observability::messages::engine::ExecutionStarted;

let msg = ExecutionStarted {
    strategy: "WorkQueue",
    processor_count: 5,
    max_concurrency: 4,
};

// Traditional logging - human-readable only
tracing::info!("{}", msg);
// Output: "Starting DAG execution with WorkQueue strategy: 5 processors, max_concurrency=4"
```

### Structured Logging (Machine-Readable)

```rust
use the_dagwood::observability::messages::{StructuredLog, engine::ExecutionStarted};

let msg = ExecutionStarted {
    strategy: "WorkQueue",
    processor_count: 5,
    max_concurrency: 4,
};

// Structured logging - human-readable + structured fields
msg.log();
```

**JSON Output** (with JSON formatter):
```json
{
  "timestamp": "2025-10-25T17:28:00Z",
  "level": "INFO",
  "message": "Starting DAG execution with WorkQueue strategy: 5 processors, max_concurrency=4",
  "fields": {
    "strategy": "WorkQueue",
    "processor_count": 5,
    "max_concurrency": 4
  }
}
```

### Distributed Tracing (OpenTelemetry)

```rust
use the_dagwood::observability::messages::{StructuredLog, engine::ExecutionStarted};

let msg = ExecutionStarted {
    strategy: "WorkQueue",
    processor_count: 5,
    max_concurrency: 4,
};

// Create span with message fields as attributes
let span = msg.span("dag_execution");
let _guard = span.enter();

// All logs/spans created here will be children of this span
// Work happens here...

// Span automatically closed when _guard drops
```

## Complete Executor Example

Here's how to use structured logging and spans in a DAG executor:

```rust
use crate::observability::messages::{StructuredLog, engine::*, processor::*};
use std::time::Instant;

pub async fn execute_with_strategy(
    &self,
    input: ProcessorRequest,
) -> Result<HashMap<String, ProcessorResponse>, ExecutionError> {
    // Create execution start message
    let start_msg = ExecutionStarted {
        strategy: "WorkQueue",
        processor_count: self.processors.len(),
        max_concurrency: self.max_concurrency,
    };
    
    // Create root span for entire DAG execution
    let execution_span = start_msg.span("dag_execution");
    let _execution_guard = execution_span.enter();
    
    // Log structured start event
    start_msg.log();
    
    let start_time = Instant::now();
    
    // Execute processors with nested spans
    for (processor_id, processor) in &self.processors {
        let proc_start_msg = ProcessorExecutionStarted {
            processor_id,
            input_size: input.payload.len(),
        };
        
        // Create nested span for this processor
        let proc_span = proc_start_msg.span("processor_execution");
        let _proc_guard = proc_span.enter();
        
        proc_start_msg.log();
        
        let proc_start = Instant::now();
        let result = processor.process(input.clone()).await;
        let proc_duration = proc_start.elapsed();
        
        match result {
            Ok(response) => {
                ProcessorExecutionCompleted {
                    processor_id,
                    input_size: input.payload.len(),
                    output_size: response.payload.len(),
                    duration: proc_duration,
                }.log();
            }
            Err(e) => {
                ProcessorExecutionFailed {
                    processor_id,
                    error: &e,
                }.log();
                
                return Err(ExecutionError::ProcessorFailed {
                    processor_id: processor_id.clone(),
                    error: e.to_string(),
                });
            }
        }
    }
    
    let duration = start_time.elapsed();
    
    // Log completion
    ExecutionCompleted {
        strategy: "WorkQueue",
        processor_count: self.processors.len(),
        duration,
    }.log();
    
    Ok(results)
}
```

## Trace Visualization

The above code produces a trace like this:

```
Trace: dag_execution_abc123
├─ Span: dag_execution (strategy=WorkQueue, processor_count=5, max_concurrency=4)
│  ├─ Event: "Starting DAG execution..." [INFO]
│  │
│  ├─ Span: processor_execution (processor_id="uppercase", input_size=1024)
│  │  ├─ Event: "Processor 'uppercase' execution started" [INFO]
│  │  └─ Event: "Processor 'uppercase' completed: 10ms" [INFO]
│  │
│  ├─ Span: processor_execution (processor_id="reverse", input_size=1024)
│  │  ├─ Event: "Processor 'reverse' execution started" [INFO]
│  │  └─ Event: "Processor 'reverse' completed: 5ms" [INFO]
│  │
│  ├─ Span: processor_execution (processor_id="token_counter", input_size=1024)
│  │  ├─ Event: "Processor 'token_counter' execution started" [INFO]
│  │  └─ Event: "Processor 'token_counter' completed: 2ms" [INFO]
│  │
│  └─ Event: "DAG execution completed: 250ms" [INFO]
```

## Benefits

### 1. Queryable Logs

With structured fields, you can query logs without string parsing:

```bash
# Find all executions with >10 processors
jq 'select(.fields.processor_count > 10)' logs.json

# Find all WorkQueue executions
jq 'select(.fields.strategy == "WorkQueue")' logs.json

# Find slow processor executions
jq 'select(.fields.duration_ms > 100)' logs.json
```

### 2. Automatic Metrics

OpenTelemetry can extract metrics from span attributes:

- `dag_execution_duration{strategy="WorkQueue"}` - histogram
- `processor_execution_count{processor_id="uppercase"}` - counter
- `processor_execution_duration{processor_id="uppercase"}` - histogram

### 3. Distributed Tracing

Query traces by attributes:

```bash
# Find slow executions
otel query 'span.name = "dag_execution" AND duration > 1s'

# Find executions with many processors
otel query 'span.attributes.processor_count > 10'

# Find all WASM executions
otel query 'span.name = "wasm_execution"'
```

### 4. i18n Ready

Structured fields are language-independent. Only the human-readable message needs translation:

```rust
// Fields remain the same across languages
{
  "strategy": "WorkQueue",
  "processor_count": 5
}

// Only message changes
// EN: "Starting DAG execution with WorkQueue strategy: 5 processors"
// ES: "Iniciando ejecución DAG con estrategia WorkQueue: 5 procesadores"
// FR: "Démarrage de l'exécution DAG avec stratégie WorkQueue: 5 processeurs"
```

## Available Message Types

### Engine Messages
- `ExecutionStarted` - DAG execution started
- `ExecutionCompleted` - DAG execution completed
- `ExecutionFailed` - DAG execution failed
- `LevelComputationCompleted` - Level-by-level computation done
- `TopologicalSortFailed` - Cyclic dependency detected

### Processor Messages
- `ProcessorExecutionStarted` - Processor execution started
- `ProcessorExecutionCompleted` - Processor execution completed
- `ProcessorExecutionFailed` - Processor execution failed
- `ProcessorInstantiationFailed` - Processor creation failed
- `ProcessorFallbackToStub` - Falling back to stub processor

### WASM Messages
- `ModuleLoaded` - WASM module loaded
- `ModuleLoadFailed` - WASM module load failed
- `ComponentTypeDetected` - WASM component type detected
- `ExecutorCreated` - WASM executor created
- `ExecutionStarted` - WASM execution started
- `ExecutionCompleted` - WASM execution completed
- `ExecutionFailed` - WASM execution failed
- `EngineCreationStarted` - WASM engine creation started

### Validation Messages
- `CyclicDependencyDetected` - Cyclic dependency found
- `UnresolvedDependency` - Missing dependency found
- `DuplicateProcessorId` - Duplicate processor ID found
- `DiamondPatternDetected` - Diamond pattern warning
- `ValidationStarted` - Validation started
- `ValidationCompleted` - Validation completed
- `ValidationFailed` - Validation failed

## Configuration

### JSON Formatter

To enable JSON output with structured fields:

```rust
use tracing_subscriber::{fmt, EnvFilter};

tracing_subscriber::fmt()
    .json()
    .with_env_filter(EnvFilter::from_default_env())
    .init();
```

### OpenTelemetry Integration

To enable distributed tracing with OpenTelemetry:

```toml
[dependencies]
opentelemetry = "0.21"
opentelemetry-jaeger = "0.20"
tracing-opentelemetry = "0.22"
```

```rust
use opentelemetry::global;
use opentelemetry_jaeger::new_agent_pipeline;
use tracing_subscriber::{layer::SubscriberExt, Registry};
use tracing_opentelemetry::OpenTelemetryLayer;

let tracer = new_agent_pipeline()
    .with_service_name("the-dagwood")
    .install_simple()
    .expect("Failed to install OpenTelemetry tracer");

let telemetry = OpenTelemetryLayer::new(tracer);

let subscriber = Registry::default()
    .with(telemetry)
    .with(tracing_subscriber::fmt::layer());

tracing::subscriber::set_global_default(subscriber)
    .expect("Failed to set subscriber");
```

## Best Practices

1. **Use `.log()` for events** - One-time occurrences (started, completed, failed)
2. **Use `.span()` for operations** - Things with duration (execution, processing)
3. **Nest spans for hierarchy** - Parent execution span, child processor spans
4. **Include relevant fields** - Add fields that help with debugging and metrics
5. **Use appropriate log levels** - info for normal, warn for issues, error for failures

## Migration Path

Existing code using traditional logging:
```rust
tracing::info!("Starting execution with {} processors", count);
```

Can be gradually migrated to:
```rust
ExecutionStarted {
    strategy: "WorkQueue",
    processor_count: count,
    max_concurrency: 4,
}.log();
```

Both approaches work - structured logging is opt-in and additive!
