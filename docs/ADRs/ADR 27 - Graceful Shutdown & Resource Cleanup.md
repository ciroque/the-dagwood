# ADR 027: Graceful Shutdown & Resource Cleanup

## Status
Proposed

## Context

A production daemon must handle shutdown gracefully to prevent:
- **Data loss**: In-flight requests aborted mid-processing
- **Corrupted state**: Processors left in inconsistent state
- **Resource leaks**: File handles, sockets, memory not cleaned up
- **Poor user experience**: Clients receive errors for requests that were almost complete

Shutdown can be triggered by:
- **Signals**: SIGTERM (orchestrator shutdown), SIGINT (Ctrl+C)
- **Admin API**: `POST /admin/shutdown`
- **Fatal errors**: Unrecoverable errors (port already in use, etc.)
- **Process manager**: systemd, Docker, Kubernetes

Challenges with graceful shutdown:
- **In-flight requests**: May take seconds or minutes to complete
- **Long-running pipelines**: Some pipelines process large files or complex workflows
- **Hung requests**: Processors may hang indefinitely
- **Multiple protocols**: Each protocol receiver must shut down cleanly
- **Resource cleanup**: WASM modules, file handles, sockets, threads

**Decision needed**: How should we implement graceful shutdown to balance data integrity with timely termination?

## Decision

We will implement a **Phased Shutdown with Timeout** strategy that drains in-flight requests before terminating.

### Shutdown Phases

**Phase 1: Stop Accepting New Requests (Immediate)**
- Protocol receivers stop accepting new connections
- Existing connections remain open
- Return 503 Service Unavailable for new requests
- Duration: Immediate

**Phase 2: Drain In-Flight Requests (Configurable Timeout)**
- Allow in-flight requests to complete
- Track active request count
- Wait until all requests complete OR timeout expires
- Duration: 0 to `shutdown_timeout` seconds

**Phase 3: Force Termination (Immediate)**
- Abort remaining in-flight requests
- Clean up resources (close sockets, free memory)
- Exit process
- Duration: Immediate

### Configuration

```yaml
server:
  shutdown:
    timeout: 30s  # Wait up to 30s for in-flight requests
    # or
    timeout: infinite  # Wait forever (not recommended)
    # or
    timeout: 0s  # Immediate shutdown (no drain)
```

### Signal Handling

**SIGTERM (Graceful Shutdown)**
- Trigger phased shutdown with timeout
- Use case: Orchestrator shutdown (Kubernetes, systemd)

**SIGINT (Graceful Shutdown)**
- Trigger phased shutdown with timeout
- Use case: Ctrl+C during development

**SIGKILL (Immediate Termination)**
- Cannot be caught, process killed immediately
- Use case: Force kill by orchestrator after SIGTERM timeout

### Shutdown Flow

```rust
async fn shutdown_handler(
    protocol_receivers: Vec<Box<dyn ProtocolReceiver>>,
    active_requests: Arc<AtomicUsize>,
    shutdown_timeout: Duration,
) -> Result<(), ShutdownError> {
    // Phase 1: Stop accepting new requests
    for receiver in protocol_receivers {
        receiver.shutdown().await?;
    }
    
    // Phase 2: Drain in-flight requests
    let start = Instant::now();
    while active_requests.load(Ordering::SeqCst) > 0 {
        if start.elapsed() > shutdown_timeout {
            eprintln!("Shutdown timeout exceeded, {} requests aborted", 
                     active_requests.load(Ordering::SeqCst));
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // Phase 3: Force termination (cleanup happens via Drop impls)
    Ok(())
}
```

### Request Tracking

```rust
struct RequestGuard {
    active_requests: Arc<AtomicUsize>,
}

impl RequestGuard {
    fn new(active_requests: Arc<AtomicUsize>) -> Self {
        active_requests.fetch_add(1, Ordering::SeqCst);
        Self { active_requests }
    }
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        self.active_requests.fetch_sub(1, Ordering::SeqCst);
    }
}
```

### Resource Cleanup

**Automatic (via Drop)**:
- Socket file deletion (Unix sockets)
- File handle closure
- Memory deallocation
- Thread termination (tokio runtime)

**Manual (in shutdown handler)**:
- Flush logs and metrics
- Close database connections (if any)
- Save state to disk (if needed)

### Admin API Shutdown

```
POST /admin/shutdown
{
  "timeout_seconds": 30  // Optional override
}

Response:
{
  "status": "shutting_down",
  "in_flight_requests": 5,
  "timeout_seconds": 30
}
```

## Alternatives Considered

### Alternative 1: Immediate Shutdown (No Drain)

**Approach**: Abort all in-flight requests immediately

**Pros**:
- Simple: No drain logic needed
- Fast: Shutdown completes immediately
- Predictable: Always takes same amount of time

**Cons**:
- Data loss: In-flight requests aborted
- Poor UX: Clients receive errors
- Dangerous: May corrupt data mid-processing
- Unacceptable for production

**Rejected**: Data integrity is non-negotiable. Must complete in-flight requests.

### Alternative 2: Infinite Drain (No Timeout)

**Approach**: Wait forever for in-flight requests to complete

**Pros**:
- No data loss: All requests complete
- Simple: No timeout logic needed

**Cons**:
- Hung requests: May never complete if processor hangs
- Operational issues: Cannot force shutdown for deployment
- Poor UX: Operators cannot control shutdown time
- Dangerous: Prevents emergency shutdown

**Rejected**: Must have timeout to handle hung requests and enable forced shutdown.

### Alternative 3: Per-Pipeline Timeout

**Approach**: Different timeout for each pipeline

```yaml
pipelines:
  - name: quick_pipeline
    shutdown_timeout: 5s
  
  - name: slow_pipeline
    shutdown_timeout: 60s
```

**Pros**:
- Granular: Can tune per pipeline
- Flexible: Different pipelines have different needs

**Cons**:
- Complex: Must track timeout per pipeline
- Confusing: Shutdown time depends on which pipelines have in-flight requests
- Operational burden: Must tune timeout for each pipeline

**Rejected**: Single global timeout is simpler. Users can set it high enough for slowest pipeline.

### Alternative 4: Request Cancellation (Tokio Cancellation)

**Approach**: Cancel in-flight requests using tokio task cancellation

**Pros**:
- Clean: Tasks can handle cancellation gracefully
- Flexible: Tasks can save state before terminating

**Cons**:
- Complex: Requires cancellation-aware code throughout
- Rust challenges: Cancellation is not automatic, must be explicit
- Processor compatibility: Processors may not support cancellation
- WASM: Cannot cancel WASM execution mid-function

**Rejected**: Too complex and not compatible with all processor types. Simple drain-and-timeout is more reliable.

### Alternative 5: Checkpoint and Resume

**Approach**: Save request state to disk, resume after restart

**Pros**:
- No data loss: Requests resume after restart
- Flexible: Can restart without aborting requests

**Cons**:
- Complex: Requires serializable request state
- Processor compatibility: Processors must support checkpointing
- Storage: Requires persistent storage
- Overkill: Most requests complete quickly

**Rejected**: Over-engineered for most use cases. Users who need this can implement at application level.

## Consequences

### Positive

- **Data Integrity**: In-flight requests complete successfully
- **Predictable Behavior**: Timeout ensures shutdown completes in bounded time
- **Operational Control**: Operators can force shutdown if needed
- **Resource Cleanup**: Automatic cleanup via Drop trait
- **Signal Handling**: Standard Unix signal handling
- **Admin API**: Programmatic shutdown for orchestration

### Negative

- **Timeout Tuning**: Must set timeout high enough for slowest pipeline
- **Aborted Requests**: Requests exceeding timeout are aborted
- **Complexity**: Shutdown logic adds code complexity

### Neutral

- **Default Timeout**: Need to choose reasonable default (30s?)
- **Hung Requests**: Timeout protects against hung requests, but they're still aborted
- **Orchestrator Coordination**: Must configure orchestrator timeout > DAGwood timeout

## Implementation Notes

### Phase 6.1: Graceful Shutdown

**Tasks:**
- Implement signal handling (SIGTERM, SIGINT)
- Add request tracking (atomic counter)
- Implement phased shutdown with timeout
- Add `POST /admin/shutdown` endpoint
- Add shutdown configuration
- Add shutdown logging

**Files to Create/Modify:**
- `src/server/shutdown.rs` (new)
- `src/cli/serve.rs` (modify - integrate shutdown handler)
- `src/protocols/http.rs` (modify - add admin shutdown endpoint)

**Testing:**
- Shutdown during execution waits for completion
- Timeout forces shutdown after configured duration
- New requests rejected during shutdown
- Resource cleanup verified (no leaks)

See [DAEMONIZATION_ROADMAP.md - Phase 6.1](../../DAEMONIZATION_ROADMAP.md#61-graceful-shutdown-) for detailed implementation plan.

## Related ADRs

- [ADR 21 - Pluggable Protocol Receiver Architecture](./ADR%2021%20-%20Pluggable%20Protocol%20Receiver%20Architecture.md) - Protocol receivers implement `shutdown()` method
- [ADR 23 - Hot-Reload Strategy](./ADR%2023%20-%20Hot-Reload%20Strategy%20(Drain-and-Switch).md) - Similar drain pattern for hot-reload
- [ADR 26 - Observability Strategy](./ADR%2026%20-%20Observability%20Strategy%20(Health%2C%20Metrics%2C%20Logging).md) - Shutdown events logged

## References

- [DAEMONIZATION_ROADMAP.md - Phase 6.1: Graceful Shutdown](../../DAEMONIZATION_ROADMAP.md#61-graceful-shutdown-)
- Graceful Shutdown Patterns: https://cloud.google.com/blog/products/containers-kubernetes/kubernetes-best-practices-terminating-with-grace
- Unix Signal Handling: https://www.gnu.org/software/libc/manual/html_node/Signal-Handling.html
- Tokio Shutdown: https://tokio.rs/tokio/topics/shutdown
