# ADR 026: Observability Strategy (Health, Metrics, Logging)

## Status
Proposed

## Context

A production daemon requires comprehensive observability to support:
- **Health monitoring**: Is the server healthy? Are pipelines ready?
- **Performance analysis**: Which pipelines are slow? Where are bottlenecks?
- **Debugging**: What happened during a failed execution?
- **Alerting**: When should operators be notified?
- **Capacity planning**: How much load can the system handle?

The DAGwood project already has structured logging infrastructure:
- `StructuredLog` trait for machine-readable logs
- OpenTelemetry spans for distributed tracing
- Per-processor and per-executor instrumentation
- See [ADR 19 - Structured Logging and Distributed Tracing Strategy](./ADR%2019%20-%20Structured%20Logging%20and%20Distributed%20Tracing%20Strategy.md)

With the daemon architecture, we need additional observability:
- **Server-level health**: Is the daemon accepting requests?
- **Pipeline-level health**: Which pipelines are ready/initializing/failed?
- **Protocol-level metrics**: Requests per protocol, error rates
- **Lifecycle events**: Pipeline initialization, hot-reload, shutdown

**Decision needed**: How should we expose health, metrics, and logs for the daemon architecture?

## Decision

We will implement a **Three-Layer Observability Strategy** with health checks, Prometheus metrics, and structured logging.

### Layer 1: Health Checks

**Endpoint**: `GET /health`

**Purpose**: Quick health status for load balancers and monitoring systems

**Response Format**:
```json
{
  "server": "healthy",
  "version": "0.3.0",
  "uptime_seconds": 3600,
  "pipelines": {
    "text_processing": {
      "status": "ready",
      "startup": "auto",
      "initialized_at": "2025-10-25T19:00:00Z",
      "requests_processed": 1523,
      "last_request_at": "2025-10-25T19:30:00Z"
    },
    "image_processing": {
      "status": "uninitialized",
      "startup": "on-demand"
    },
    "broken_pipeline": {
      "status": "permanently_failed",
      "error": "WASM module not found: /path/to/module.wasm",
      "failed_attempts": 3,
      "last_attempt": "2025-10-25T19:25:00Z"
    }
  },
  "protocols": {
    "http_public": {
      "type": "http",
      "status": "listening",
      "address": "0.0.0.0:8080"
    },
    "grpc": {
      "type": "grpc",
      "status": "listening",
      "address": "0.0.0.0:50051"
    }
  }
}
```

**Health Semantics**:
- **Server healthy**: All `startup: auto` pipelines are `ready`
- **Server degraded**: Some `startup: auto` pipelines are `failed` or `initializing`
- **Server unhealthy**: Cannot accept requests (protocol receivers failed)

**Configuration**:
```yaml
health_check:
  strict: true  # Server unhealthy if any startup:auto pipeline not ready
  # or
  strict: false  # Server healthy as long as it's running
```

### Layer 2: Prometheus Metrics

**Endpoint**: `GET /metrics`

**Purpose**: Detailed metrics for monitoring, alerting, and analysis

**Metrics Exposed**:

**Server-level:**
```
# Server uptime
dagwood_server_uptime_seconds

# Total requests across all pipelines
dagwood_requests_total{protocol="http",pipeline="text_processing"}

# Request duration histogram
dagwood_request_duration_seconds{protocol="http",pipeline="text_processing"}

# Active requests gauge
dagwood_requests_active{protocol="http",pipeline="text_processing"}
```

**Pipeline-level:**
```
# Pipeline state (0=uninitialized, 1=initializing, 2=ready, 3=failed, 4=permanently_failed)
dagwood_pipeline_state{pipeline="text_processing"}

# Pipeline initialization duration
dagwood_pipeline_initialization_duration_seconds{pipeline="text_processing"}

# Pipeline requests processed
dagwood_pipeline_requests_total{pipeline="text_processing",status="success"}
dagwood_pipeline_requests_total{pipeline="text_processing",status="error"}

# Pipeline execution duration (from existing instrumentation)
dagwood_pipeline_execution_duration_seconds{pipeline="text_processing",strategy="work_queue"}
```

**Protocol-level:**
```
# Protocol receiver status (0=stopped, 1=listening, 2=error)
dagwood_protocol_status{protocol="http_public",type="http"}

# Requests per protocol
dagwood_protocol_requests_total{protocol="http_public",type="http"}

# Protocol errors
dagwood_protocol_errors_total{protocol="http_public",type="http",error_type="parse_error"}
```

**Hot-reload:**
```
# Hot-reload events
dagwood_hot_reload_total{pipeline="text_processing",result="success"}
dagwood_hot_reload_total{pipeline="text_processing",result="failure"}

# Drain duration during hot-reload
dagwood_hot_reload_drain_duration_seconds{pipeline="text_processing"}
```

### Layer 3: Structured Logging

**Purpose**: Detailed event logs for debugging and audit trails

**Leverage Existing Infrastructure**:
- Use existing `StructuredLog` trait and message types
- Add new message types for daemon-specific events
- Maintain OpenTelemetry span hierarchy

**New Message Types**:

**Server lifecycle:**
```rust
ServerStarted {
    version: &str,
    config_path: &str,
    pipeline_count: usize,
    protocol_count: usize,
}

ServerShutdown {
    reason: &str,
    uptime_seconds: u64,
}
```

**Pipeline lifecycle:**
```rust
PipelineInitializationStarted {
    pipeline_name: &str,
    startup_mode: &str,
}

PipelineInitializationCompleted {
    pipeline_name: &str,
    duration_ms: u64,
}

PipelineInitializationFailed {
    pipeline_name: &str,
    error: &str,
    attempt: u32,
    max_retries: u32,
}
```

**Hot-reload:**
```rust
HotReloadTriggered {
    trigger: &str,  // "file_watch", "admin_api", "signal"
    pipelines_affected: usize,
}

PipelineDrainStarted {
    pipeline_name: &str,
    in_flight_requests: usize,
}

PipelineDrainCompleted {
    pipeline_name: &str,
    drain_duration_ms: u64,
}
```

**Protocol events:**
```rust
ProtocolReceiverStarted {
    protocol_name: &str,
    protocol_type: &str,
    address: &str,
}

ProtocolReceiverFailed {
    protocol_name: &str,
    protocol_type: &str,
    error: &str,
}
```

### Integration with Existing Observability

**Spans (from ADR 19):**
- Root span: `server_request` (new, wraps everything)
  - Child span: `dag_execution` (existing)
    - Child span: `processor_execution` (existing)
      - Child span: `wasm_execution` (existing)

**Structured logs (from ADR 19):**
- All existing processor and executor logs continue to work
- New daemon-specific logs added alongside

**Trace hierarchy:**
```
Trace: request_abc123
│
├─ Span: server_request (protocol=http, pipeline=text_processing)
│  ├─ Event: "Request received" [INFO]
│  │
│  ├─ Span: dag_execution (strategy=WorkQueue)
│  │  ├─ Span: processor_execution (processor_id=uppercase)
│  │  │  └─ Span: wasm_execution (module=uppercase.wasm)
│  │  └─ Span: processor_execution (processor_id=reverse)
│  │
│  └─ Event: "Request completed: 45ms" [INFO]
```

## Alternatives Considered

### Alternative 1: Custom Metrics Format

**Approach**: Define custom JSON metrics format instead of Prometheus

**Pros**:
- Flexible: Can include any data structure
- Simple: No need to learn Prometheus format

**Cons**:
- Non-standard: Requires custom tooling
- No ecosystem: Cannot use Grafana, Prometheus, etc.
- Reinventing wheel: Prometheus is industry standard

**Rejected**: Prometheus is the de facto standard for metrics. Custom format would limit integration with existing tools.

### Alternative 2: No Health Checks (Rely on Metrics)

**Approach**: Only expose metrics, no dedicated health endpoint

**Pros**:
- Simpler: One less endpoint
- Flexible: Can derive health from metrics

**Cons**:
- Slow: Metrics endpoint may be slow for health checks
- Complex: Load balancers must parse metrics
- Non-standard: Health checks are expected pattern

**Rejected**: Health checks are expected by load balancers and orchestration systems. Dedicated endpoint is simpler and faster.

### Alternative 3: Separate Metrics Per Pipeline

**Approach**: Each pipeline has its own `/metrics` endpoint

```
GET /pipelines/text_processing/metrics
GET /pipelines/image_processing/metrics
```

**Pros**:
- Isolation: Can scrape pipelines independently
- Granular: Can control which pipelines to monitor

**Cons**:
- Complex: Prometheus must scrape multiple endpoints
- Inefficient: Multiple HTTP requests per scrape
- Non-standard: Prometheus expects single `/metrics` endpoint

**Rejected**: Single `/metrics` endpoint with labels is standard Prometheus pattern.

### Alternative 4: Push Metrics (StatsD, InfluxDB)

**Approach**: Push metrics to external system instead of pull-based Prometheus

**Pros**:
- Real-time: Metrics pushed immediately
- No scraping: Simpler for some deployments

**Cons**:
- External dependency: Requires StatsD/InfluxDB server
- Network overhead: Constant metric pushing
- Non-standard: Prometheus pull model is more common

**Rejected**: Pull-based Prometheus is industry standard. Users who need push can use Prometheus Pushgateway.

### Alternative 5: Unstructured Logging

**Approach**: Use traditional string-based logging (log crate)

**Pros**:
- Simple: No structured log infrastructure needed
- Familiar: Traditional logging pattern

**Cons**:
- Not queryable: Must parse strings to extract data
- No metrics: Cannot automatically extract metrics from logs
- Already decided: ADR 19 established structured logging

**Rejected**: Already committed to structured logging in ADR 19. Daemon observability should follow same pattern.

## Consequences

### Positive

- **Production-Ready**: Comprehensive observability for production deployments
- **Standard Formats**: Prometheus and OpenTelemetry are industry standards
- **Ecosystem Integration**: Works with Grafana, Prometheus, Jaeger, etc.
- **Consistent**: Follows patterns from ADR 19
- **Operational Visibility**: Clear view of server, pipeline, and protocol health
- **Debugging Support**: Structured logs provide detailed event history
- **Alerting**: Prometheus metrics enable alerting rules

### Negative

- **Endpoint Overhead**: Health and metrics endpoints consume resources
- **Metrics Cardinality**: High-cardinality labels (pipeline names) can cause issues
- **Storage**: Metrics and logs consume storage

### Neutral

- **Learning Curve**: Operators must learn Prometheus and OpenTelemetry
- **Configuration**: May need to tune health check semantics per deployment

## Implementation Notes

### Phase 5.1: Health Check Endpoint
- Add `GET /health` endpoint to HTTP protocol receiver
- Implement health check logic (server + pipeline + protocol status)
- Add configuration for health check semantics (strict vs permissive)

See [DAEMONIZATION_ROADMAP.md - Phase 5.1](../../DAEMONIZATION_ROADMAP.md#51-health-check-endpoint-) for detailed implementation plan.

### Phase 5.2: Metrics Endpoint
- Add `GET /metrics` endpoint to HTTP protocol receiver
- Integrate `prometheus` crate for metric collection
- Add server, pipeline, and protocol metrics
- Ensure metrics are collected during execution

See [DAEMONIZATION_ROADMAP.md - Phase 5.2](../../DAEMONIZATION_ROADMAP.md#52-metrics-endpoint-) for detailed implementation plan.

### Phase 5.3: Structured Logging
- Add new message types for daemon lifecycle events
- Integrate with existing structured logging infrastructure
- Add root `server_request` span for request tracing

See [DAEMONIZATION_ROADMAP.md - Phase 5.3](../../DAEMONIZATION_ROADMAP.md#53-structured-logging-) for detailed implementation plan.

## Related ADRs

- [ADR 19 - Structured Logging and Distributed Tracing Strategy](./ADR%2019%20-%20Structured%20Logging%20and%20Distributed%20Tracing%20Strategy.md) - Foundation for daemon observability
- [ADR 20 - Multi-Pipeline Architecture & Registry Pattern](./ADR%2020%20-%20Multi-Pipeline%20Architecture%20&%20Registry%20Pattern.md) - Health checks expose pipeline states
- [ADR 22 - Pipeline Lifecycle & Lazy Loading Strategy](./ADR%2022%20-%20Pipeline%20Lifecycle%20&%20Lazy%20Loading%20Strategy.md) - Metrics track lifecycle states

## References

- [DAEMONIZATION_ROADMAP.md - Phase 5.1: Health Check Endpoint](../../DAEMONIZATION_ROADMAP.md#51-health-check-endpoint-)
- [DAEMONIZATION_ROADMAP.md - Phase 5.2: Metrics Endpoint](../../DAEMONIZATION_ROADMAP.md#52-metrics-endpoint-)
- [DAEMONIZATION_ROADMAP.md - Phase 5.3: Structured Logging](../../DAEMONIZATION_ROADMAP.md#53-structured-logging-)
- [ADR 19 - Structured Logging](./ADR%2019%20-%20Structured%20Logging%20and%20Distributed%20Tracing%20Strategy.md)
- Prometheus Best Practices: https://prometheus.io/docs/practices/naming/
- Health Check Patterns: https://microservices.io/patterns/observability/health-check-api.html
