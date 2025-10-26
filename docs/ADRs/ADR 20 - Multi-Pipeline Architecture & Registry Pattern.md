# ADR 020: Multi-Pipeline Architecture & Registry Pattern

## Status
Proposed

## Context

The DAGwood project currently supports executing a single pipeline per process. This works well for library usage and CLI tools, but limits operational flexibility:

- **No multi-tenancy**: Cannot host multiple workflows in one process
- **Resource inefficiency**: Each pipeline requires a separate process, duplicating memory for WASM modules, processor instances, and configuration
- **Operational complexity**: Managing multiple processes increases deployment and monitoring overhead
- **Limited A/B testing**: Cannot easily compare different execution strategies or processor implementations within the same deployment

As we move toward a daemon/server architecture, we need to support:
- Multiple named pipelines in a single process
- Shared resources (WASM modules, processor pools) across pipelines
- Independent lifecycle management per pipeline
- Simple routing from requests to the appropriate pipeline

**Decision needed**: How should we architect multi-pipeline support while maintaining simplicity and resource efficiency?

## Decision

We will adopt a **Registry Pattern** where a single DAGwood process hosts multiple named pipelines through a centralized `PipelineRegistry` and `PipelineRouter`.

### Architecture Components

**PipelineRegistry**
- Manages collection of named pipelines
- Tracks pipeline state (Uninitialized, Initializing, Ready, Failed)
- Handles pipeline lifecycle (initialization, hot-reload, removal)
- Provides thread-safe access via `Arc<Mutex<>>`

**PipelineRouter**
- Routes incoming requests to pipelines by name
- Handles pipeline not found errors
- Manages request queueing during pipeline initialization
- Protocol-agnostic (works with HTTP, gRPC, Unix sockets)

**Pipeline**
- Encapsulates configuration, executor, and processor registry
- Independent lifecycle from other pipelines
- Shares underlying resources (WASM modules can be cached)

### Configuration Structure

**Legacy Format (Backward Compatible)**
```yaml
strategy: work_queue
processors:
  - id: processor1
    # ...
```

Automatically wrapped as:
```yaml
pipelines:
  - name: "default"
    strategy: work_queue
    processors:
      - id: processor1
        # ...
```

**New Server Format**
```yaml
pipelines:
  - name: text_processing_workqueue
    strategy: work_queue
    processors: [...]
  
  - name: text_processing_level
    strategy: level
    processors: [...]
  
  - name: image_processing
    strategy: reactive
    processors: [...]
```

### Request Flow

```
Request → ProtocolReceiver → PipelineRouter → PipelineRegistry → Pipeline → Executor → Processors
```

1. Protocol receiver extracts pipeline name from request
2. Router looks up pipeline in registry
3. Registry returns pipeline instance (or queues if initializing)
4. Pipeline executes using its configured strategy
5. Response flows back through same path

## Alternatives Considered

### Alternative 1: Multiple Processes (One Per Pipeline)

**Approach**: Deploy separate DAGwood process for each pipeline

**Pros**:
- Complete isolation between pipelines
- Independent scaling per pipeline
- Simpler code (no registry needed)
- Process crash only affects one pipeline

**Cons**:
- Resource duplication (WASM modules loaded N times)
- Higher memory footprint
- More complex deployment (N processes to manage)
- Inter-process communication overhead for shared resources
- Cannot share processor instances or caches

**Rejected**: Resource inefficiency and operational complexity outweigh isolation benefits. Process-level isolation can be achieved at deployment level if needed.

### Alternative 2: Dynamic Plugin System

**Approach**: Load pipelines as dynamic libraries (.so/.dylib/.dll) at runtime

**Pros**:
- True hot-reload without process restart
- Language-agnostic (any language that compiles to shared lib)
- Could unload unused pipelines to free memory

**Cons**:
- Complex: FFI boundaries, ABI stability, symbol resolution
- Platform-specific: Different behavior on Linux/macOS/Windows
- Safety concerns: Unsafe code required, potential for crashes
- Build complexity: Separate compilation for each pipeline
- Debugging difficulty: Stack traces across FFI boundaries

**Rejected**: Complexity far exceeds benefits. WASM already provides sandboxed execution for untrusted code. Hot-reload can be achieved with drain-and-switch pattern.

### Alternative 3: Microservices Architecture

**Approach**: Each pipeline is a separate microservice, orchestrated by API gateway

**Pros**:
- Independent deployment per pipeline
- Language-agnostic (each service can use different language)
- Horizontal scaling per pipeline
- Clear service boundaries

**Cons**:
- Network latency between services
- Operational complexity (service discovery, health checks, etc.)
- Resource overhead (each service needs its own runtime)
- Overkill for single-machine deployments
- Doesn't solve the "multiple pipelines in one process" use case

**Rejected**: Appropriate for distributed systems, but not for single-process daemon use case. Users can deploy multiple DAGwood instances if they need microservices architecture.

### Alternative 4: Actor Model (Tokio Actors)

**Approach**: Each pipeline is an actor, registry is actor supervisor

**Pros**:
- Message-passing isolation between pipelines
- Built-in supervision and restart logic
- Natural fit for async Rust
- Mailbox provides request queueing

**Cons**:
- Additional abstraction layer (actors on top of async)
- Learning curve for actor patterns
- Potential for mailbox overflow
- Harder to reason about shared state
- Not significantly simpler than registry pattern

**Rejected**: Adds complexity without clear benefits. Registry pattern with `Arc<Mutex<>>` provides sufficient isolation and is more idiomatic Rust.

## Consequences

### Positive

- **Resource Efficiency**: WASM modules, processor instances, and configuration loaded once and shared
- **Operational Simplicity**: Single process to deploy, monitor, and manage
- **A/B Testing**: Multiple pipeline variants (different strategies, processors) in one deployment
- **Hot-Reload**: Update individual pipelines without restarting entire server
- **Backward Compatible**: Legacy single-pipeline configs automatically wrapped
- **Simple Mental Model**: Named pipelines in a registry, straightforward routing
- **Protocol Agnostic**: Same registry/router works for HTTP, gRPC, Unix sockets

### Negative

- **Shared Fate**: Pipeline crash could affect other pipelines (mitigated by error handling)
- **Resource Contention**: Pipelines compete for CPU/memory (mitigated by concurrency limits)
- **Complexity**: More complex than single-pipeline architecture
- **State Management**: Need to track state for multiple pipelines

### Neutral

- **Scaling**: Horizontal scaling requires multiple processes (same as before)
- **Isolation**: Less isolation than separate processes, but sufficient for most use cases
- **Memory Usage**: Lower per-pipeline overhead, but total memory grows with pipeline count

## Implementation Notes

### Phase 1.1: Pipeline Registry & Router
- Create `PipelineRegistry` to manage multiple pipeline configs
- Create `PipelineRouter` to route requests by name
- Update config loader to support `pipelines: []` array
- Implement backward compatibility wrapper

See [DAEMONIZATION_ROADMAP.md - Phase 1.1](../../DAEMONIZATION_ROADMAP.md#11-pipeline-registry--router-) for detailed implementation plan.

### Phase 1.2: Pipeline Lifecycle States
- Add state tracking (Uninitialized, Initializing, Ready, Failed)
- Implement lazy initialization on first request
- Add request queueing during initialization

See [DAEMONIZATION_ROADMAP.md - Phase 1.2](../../DAEMONIZATION_ROADMAP.md#12-pipeline-lifecycle-states-) for detailed implementation plan.

## Related ADRs

- [ADR 21 - Pluggable Protocol Receiver Architecture](./ADR%2021%20-%20Pluggable%20Protocol%20Receiver%20Architecture.md) - Protocol receivers use PipelineRouter
- [ADR 22 - Pipeline Lifecycle & Lazy Loading Strategy](./ADR%2022%20-%20Pipeline%20Lifecycle%20&%20Lazy%20Loading%20Strategy.md) - Lifecycle states managed by PipelineRegistry
- [ADR 23 - Hot-Reload Strategy](./ADR%2023%20-%20Hot-Reload%20Strategy%20(Drain-and-Switch).md) - Hot-reload operates on PipelineRegistry

## References

- [DAEMONIZATION_ROADMAP.md - Phase 1.1: Pipeline Registry & Router](../../DAEMONIZATION_ROADMAP.md#11-pipeline-registry--router-)
- [DAEMONIZATION_ROADMAP.md - Phase 1.2: Pipeline Lifecycle States](../../DAEMONIZATION_ROADMAP.md#12-pipeline-lifecycle-states-)
- Registry Pattern: https://martinfowler.com/eaaCatalog/registry.html
