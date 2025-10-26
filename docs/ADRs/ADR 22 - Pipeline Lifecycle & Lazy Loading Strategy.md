# ADR 022: Pipeline Lifecycle & Lazy Loading Strategy

## Status
Proposed

## Context

In a multi-pipeline daemon, pipeline initialization has significant resource implications:
- **WASM modules**: Loading and compiling can take 100ms-1s per module
- **Processor instantiation**: Creating processor instances requires I/O and validation
- **Memory allocation**: Each pipeline allocates memory for state and caches
- **Startup time**: Initializing all pipelines at startup delays server availability

Different pipelines have different usage patterns:
- **Critical pipelines**: High-traffic, must be ready immediately (health checks, core workflows)
- **Experimental pipelines**: Low-traffic, rarely used, can initialize on-demand
- **Development pipelines**: Only used during testing, shouldn't slow down startup

Concurrent access patterns create challenges:
- **Thundering herd**: Multiple requests arrive before pipeline initialized
- **Race conditions**: Multiple threads try to initialize same pipeline
- **Deadlocks**: Circular dependencies between initializing pipelines
- **Resource exhaustion**: Too many concurrent initializations

**Decision needed**: How should we manage pipeline lifecycle to balance resource efficiency, availability, and operational simplicity?

## Decision

We will implement a **State Machine with Lazy Loading** strategy that supports both eager (`startup: auto`) and lazy (`startup: on-demand`) initialization.

### Pipeline State Machine

```
Uninitialized
    ↓ (first request OR startup:auto)
Initializing (requests queue here)
    ↓ (success)
Ready (process requests)
    ↓ (failure)
Failed (attempt N of max_retries)
    ↓ (retry)
Retrying
    ↓ (max retries exceeded)
PermanentlyFailed (reject all requests)
```

### Configuration

```yaml
pipelines:
  - name: critical_pipeline
    startup: auto  # Initialize at server startup
    initialization:
      max_retries: 3
      retry_backoff: exponential
    processors: [...]
  
  - name: experimental_pipeline
    startup: on-demand  # Initialize on first request
    initialization:
      max_retries: 1
      retry_backoff: fixed
    processors: [...]
```

### Initialization Behavior

**startup: auto**
- Pipeline initializes during server startup
- Server startup fails if initialization fails (after retries)
- Pipeline is Ready before accepting any requests
- Use for: Critical pipelines, health check endpoints, high-traffic workflows

**startup: on-demand**
- Pipeline remains Uninitialized until first request
- First request triggers initialization
- Subsequent requests queue behind initialization
- Use for: Experimental pipelines, low-traffic workflows, development/testing

### Concurrent Request Handling

**Problem**: 10 requests arrive simultaneously for uninitialized pipeline

**Solution**: Queue requests in arrival order
1. First request triggers initialization (state → Initializing)
2. Subsequent requests see "Initializing" state and queue
3. Initialization lock prevents duplicate work
4. When Ready, process queued requests in FIFO order
5. New requests go directly to Ready pipeline

**Implementation**:
```rust
struct PipelineState {
    state: State,
    initialization_lock: Arc<Mutex<()>>,
    request_queue: VecDeque<PendingRequest>,
}
```

### Retry Strategy

**Transient failures** (network timeout, temporary resource unavailability):
- Retry with backoff
- Exponential: 1s, 2s, 4s, 8s...
- Fixed: 5s between each retry

**Permanent failures** (invalid config, missing WASM module):
- Mark as PermanentlyFailed after max_retries
- Reject all requests with descriptive error
- Require manual reset (via hot-reload or admin API)

### Request Timeout

**Problem**: What if initialization takes 30 seconds?

**Solution**: Configurable timeout per pipeline
```yaml
pipelines:
  - name: slow_pipeline
    startup: on-demand
    initialization:
      timeout: 30s  # Fail requests waiting longer than 30s
      max_retries: 3
```

Queued requests that exceed timeout receive error response.

## Alternatives Considered

### Alternative 1: Eager Loading Only (startup: auto for all)

**Approach**: Initialize all pipelines at server startup

**Pros**:
- Simple: No state machine needed
- Predictable: All pipelines ready before accepting requests
- Fast request handling: No initialization delay on first request

**Cons**:
- Slow startup: Must initialize all pipelines before server ready
- Resource waste: Unused pipelines consume memory
- Deployment issues: One bad pipeline prevents server startup
- Poor development experience: Long startup for testing single pipeline

**Rejected**: Inflexible and resource-inefficient. Doesn't support experimental or rarely-used pipelines.

### Alternative 2: Lazy Loading Only (startup: on-demand for all)

**Approach**: All pipelines initialize on first request

**Pros**:
- Fast startup: Server ready immediately
- Resource efficient: Only load what's used
- Simple config: No startup mode needed

**Cons**:
- Unpredictable latency: First request to each pipeline is slow
- Health check issues: Critical pipelines not ready at startup
- Production risk: Initialization failures discovered at request time
- Cold start problem: Every pipeline has first-request penalty

**Rejected**: Unacceptable for production critical pipelines. Health checks would fail until first request.

### Alternative 3: No State Machine (Fail Fast)

**Approach**: If initialization fails, reject all future requests permanently

**Pros**:
- Simple: No retry logic needed
- Fast: Fail immediately, no waiting
- Clear: Pipeline either works or doesn't

**Cons**:
- Brittle: Transient failures become permanent
- Poor UX: Network blip causes permanent failure
- Operational burden: Requires manual intervention for transient issues
- No self-healing: Cannot recover from temporary problems

**Rejected**: Too brittle for production. Transient failures (network timeouts, temporary resource unavailability) should be retried.

### Alternative 4: Background Initialization (No Queueing)

**Approach**: Return 503 Service Unavailable during initialization, no queueing

**Pros**:
- Simple: No queue management
- Client control: Client decides whether to retry
- No memory pressure: No queued requests

**Cons**:
- Poor UX: Clients must implement retry logic
- Thundering herd: All clients retry simultaneously
- Wasted work: Clients may give up before initialization completes
- Inconsistent: Different protocols handle retries differently

**Rejected**: Pushes complexity to clients. Queueing provides better UX and consistent behavior.

### Alternative 5: Actor Model with Supervision

**Approach**: Each pipeline is an actor, supervisor restarts failed pipelines

**Pros**:
- Automatic restart: Supervisor handles failures
- Isolation: Pipeline failures don't affect others
- Proven pattern: Erlang/OTP supervision trees

**Cons**:
- Complex: Requires actor framework (Actix, Bastion)
- Learning curve: Actor patterns unfamiliar to many Rust developers
- Overkill: State machine provides sufficient restart logic
- Inconsistent: Other DAGwood components don't use actors

**Rejected**: Adds complexity without clear benefits. State machine with retry logic provides sufficient failure handling.

## Consequences

### Positive

- **Resource Efficiency**: Only initialize pipelines that are actually used
- **Fast Startup**: Server ready immediately (for on-demand pipelines)
- **Flexibility**: Critical pipelines eager-loaded, experimental pipelines lazy-loaded
- **Self-Healing**: Automatic retry for transient failures
- **Fair Queueing**: FIFO order for concurrent requests during initialization
- **Predictable Behavior**: State machine makes lifecycle explicit
- **Operational Visibility**: State exposed via health check endpoint

### Negative

- **Complexity**: State machine adds code complexity
- **First Request Latency**: On-demand pipelines have slow first request
- **Memory Growth**: Memory usage grows as pipelines initialize
- **Queue Management**: Need to handle queue depth limits and timeouts

### Neutral

- **Configuration Burden**: Users must choose startup mode per pipeline
- **State Tracking**: Need to expose state via health checks and metrics
- **Retry Tuning**: May need to adjust retry parameters per pipeline

## Implementation Notes

### Phase 1.2: Pipeline Lifecycle States
- Define `PipelineState` enum
- Add `startup: auto|on-demand` config field
- Implement lazy initialization on first request
- Add request queueing during initialization
- Add concurrent initialization protection

See [DAEMONIZATION_ROADMAP.md - Phase 1.2](../../DAEMONIZATION_ROADMAP.md#12-pipeline-lifecycle-states-) for detailed implementation plan.

### Phase 3.1: Initialization Retry Logic
- Add `max_retries` and `retry_backoff` config
- Implement retry state tracking
- Add exponential and fixed backoff strategies
- Add PermanentlyFailed state

See [DAEMONIZATION_ROADMAP.md - Phase 3.1](../../DAEMONIZATION_ROADMAP.md#31-initialization-retry-logic-) for detailed implementation plan.

## Related ADRs

- [ADR 20 - Multi-Pipeline Architecture & Registry Pattern](./ADR%2020%20-%20Multi-Pipeline%20Architecture%20&%20Registry%20Pattern.md) - PipelineRegistry manages lifecycle states
- [ADR 23 - Hot-Reload Strategy](./ADR%2023%20-%20Hot-Reload%20Strategy%20(Drain-and-Switch).md) - Hot-reload transitions through lifecycle states
- [ADR 26 - Observability Strategy](./ADR%2026%20-%20Observability%20Strategy%20(Health%2C%20Metrics%2C%20Logging).md) - Health checks expose pipeline states

## References

- [DAEMONIZATION_ROADMAP.md - Phase 1.2: Pipeline Lifecycle States](../../DAEMONIZATION_ROADMAP.md#12-pipeline-lifecycle-states-)
- [DAEMONIZATION_ROADMAP.md - Phase 3.1: Initialization Retry Logic](../../DAEMONIZATION_ROADMAP.md#31-initialization-retry-logic-)
- State Pattern: https://refactoring.guru/design-patterns/state
- Lazy Initialization: https://en.wikipedia.org/wiki/Lazy_initialization
