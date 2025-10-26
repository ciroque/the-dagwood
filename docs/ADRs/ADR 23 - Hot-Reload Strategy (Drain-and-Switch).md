# ADR 023: Hot-Reload Strategy (Drain-and-Switch)

## Status
Proposed

## Context

Production systems require the ability to update pipelines without downtime:
- **Bug fixes**: Deploy processor fixes without restarting server
- **Configuration changes**: Modify pipeline strategy, concurrency, failure handling
- **WASM module updates**: Swap in new WASM module versions
- **A/B testing**: Dynamically add/remove pipeline variants
- **Experimentation**: Test new processors without full deployment

Traditional approaches have limitations:
- **Full restart**: Drops in-flight requests, causes downtime
- **Blue-green deployment**: Requires duplicate infrastructure
- **Rolling restart**: Complex orchestration, still has brief unavailability
- **Immediate switch**: May corrupt in-flight requests with old pipeline version

Challenges with hot-reload:
- **In-flight requests**: What happens to requests being processed during reload?
- **Version management**: How many pipeline versions can coexist?
- **State consistency**: How to handle processor state during transition?
- **Failure handling**: What if new version fails to initialize?
- **Concurrent reloads**: What if another reload triggered during transition?

**Decision needed**: How should we implement hot-reload to enable zero-downtime updates while maintaining data integrity?

## Decision

We will implement a **Drain-and-Switch** strategy where the old pipeline version drains in-flight requests before the new version activates.

### Core Principles

1. **Pipeline Name is Identity**: Hot-reload replaces pipeline with same name
2. **Versioning via Names**: Want multiple versions? Create pipelines with different names
3. **Drain Before Switch**: Old version completes in-flight requests before destruction
4. **Queue New Requests**: New requests queue for new version during drain
5. **Timeout Protection**: Force-kill old version after drain timeout

### Hot-Reload Flow

```
1. Hot-reload triggered for "text_processing"
2. Initialize new version (v2)
3. Mark v1 as "Draining" (no new requests)
4. Queue new requests for v2
5. Wait for v1 in-flight requests to complete (or timeout)
6. Destroy v1
7. Activate v2 as the active version
8. Process queued requests on v2
```

### State Transitions

```
Ready (v1)
    ↓ (hot-reload triggered)
Draining (v1) + Initializing (v2)
    ↓ (v1 drained AND v2 ready)
Ready (v2)
```

### Configuration

```yaml
pipelines:
  - name: text_processing
    hot_reload:
      drain_timeout: 30s  # Force-kill after 30s
      # or
      drain_timeout: infinite  # Wait forever
```

### Versioning Strategy

**For hot-reload (replace existing pipeline):**
```yaml
# v1 config
pipelines:
  - name: text_processing
    processors: [A, B, C]

# v2 config (hot-reload)
pipelines:
  - name: text_processing  # SAME name
    processors: [A, B, D]   # Different implementation
```

**For A/B testing (multiple versions simultaneously):**
```yaml
pipelines:
  - name: text_processing_v1
    processors: [A, B, C]
  
  - name: text_processing_v2  # NEW pipeline
    processors: [A, B, D]
```

Both versions run simultaneously. Route traffic via NGINX or client choice. Eventually remove v1 from config.

### Trigger Mechanisms

**File Watch (Automatic)**
```bash
dagwood serve --config server.yaml --watch
# Server watches config file, auto-reloads on change
```

**Admin API (Manual)**
```bash
# Reload all pipelines
curl -X POST http://localhost:8080/admin/reload

# Reload specific pipeline
curl -X POST http://localhost:8080/admin/pipelines/text_processing/reload
```

**Signal (Unix-style)**
```bash
# Send SIGHUP to reload
kill -HUP $(cat /var/run/dagwood.pid)
```

### Hot-Load (Add New Pipeline)

```yaml
# Initial config
pipelines:
  - name: text_processing

# Updated config (hot-load)
pipelines:
  - name: text_processing
  - name: image_processing  # NEW!
```

**Behavior:**
- Detect new pipeline in config
- Add to registry
- Initialize immediately if `startup: auto`
- Initialize on first request if `startup: on-demand`

### Hot-Unload (Remove Pipeline)

```yaml
# Before
pipelines:
  - name: text_processing
  - name: old_pipeline  # Remove this

# After
pipelines:
  - name: text_processing
```

**Behavior:**
- Mark `old_pipeline` as "Draining"
- Reject new requests (404 Not Found)
- Wait for in-flight requests to complete
- Remove from registry

## Alternatives Considered

### Alternative 1: Immediate Switch

**Approach**: Immediately switch to new version, abort in-flight requests

**Pros**:
- Simple: No drain logic needed
- Fast: Instant switchover
- No version coexistence: Only one version at a time

**Cons**:
- Data loss: In-flight requests aborted
- Poor UX: Clients receive errors
- Dangerous: May corrupt data mid-processing
- Unacceptable for production

**Rejected**: Data integrity is non-negotiable. Cannot abort in-flight requests.

### Alternative 2: Blue-Green Deployment

**Approach**: Run two complete environments (blue and green), switch traffic between them

**Pros**:
- Zero downtime: Switch traffic instantly
- Rollback: Easy to switch back to old version
- Testing: Can test green before switching traffic
- Industry standard: Well-understood pattern

**Cons**:
- Resource duplication: Need 2x infrastructure
- Complexity: Requires load balancer coordination
- Overkill: Can achieve same with pipeline names
- External dependency: Requires infrastructure beyond DAGwood

**Rejected**: Can achieve same result by creating two pipelines with different names (e.g., `text_processing_blue`, `text_processing_green`) and routing via NGINX. No need for built-in blue-green support.

### Alternative 3: Version Tracking Within Pipeline

**Approach**: Track multiple versions within single pipeline, route requests by version

```rust
struct Pipeline {
    name: String,
    versions: HashMap<u32, PipelineVersion>,
    active_version: u32,
}
```

**Pros**:
- Explicit versioning: Clear which version is active
- Gradual migration: Can route % of traffic to new version
- Rollback: Easy to switch back to previous version

**Cons**:
- Complex: Need version routing logic
- Memory overhead: Multiple versions in memory
- Configuration complexity: How to specify versions?
- Unclear semantics: When to garbage collect old versions?
- Overkill: Can achieve same with separate pipeline names

**Rejected**: Complexity outweighs benefits. Creating separate pipelines with different names (e.g., `text_processing_v1`, `text_processing_v2`) is simpler and more flexible.

### Alternative 4: Copy-on-Write

**Approach**: Clone pipeline state, switch pointer atomically

**Pros**:
- Fast: Atomic pointer swap
- No drain needed: Old version continues processing
- Memory efficient: Shared immutable state

**Cons**:
- Complex: Requires careful state management
- Rust challenges: Ownership and borrowing make CoW difficult
- Mutable state: Processors may have mutable state (caches, counters)
- Not applicable: Pipelines have significant mutable state

**Rejected**: Doesn't fit Rust ownership model well. Pipelines have too much mutable state for CoW to be practical.

### Alternative 5: Rolling Restart (Multiple Instances)

**Approach**: Run multiple DAGwood instances, restart one at a time

**Pros**:
- Zero downtime: Load balancer routes around restarting instance
- Simple: Just restart process
- Industry standard: Common pattern

**Cons**:
- Requires multiple instances: Not applicable for single-instance deployment
- External dependency: Needs load balancer
- Slow: Must wait for each instance to restart
- Not hot-reload: Full process restart, not in-place update

**Rejected**: Appropriate for multi-instance deployments, but doesn't solve single-instance hot-reload use case. Users can deploy multiple instances if they need rolling restart.

## Consequences

### Positive

- **Zero Downtime**: In-flight requests complete successfully
- **Data Integrity**: No aborted or corrupted requests
- **Simple Mental Model**: Drain old, activate new
- **Flexible Versioning**: Use pipeline names for A/B testing
- **Timeout Protection**: Won't hang forever on stuck requests
- **Multiple Triggers**: File watch, API, signal - choose what fits workflow
- **Operational Visibility**: State transitions visible in logs and health checks

### Negative

- **Drain Delay**: Switchover takes time (up to drain_timeout)
- **Memory Overhead**: Two versions in memory during drain
- **Complexity**: State machine for drain/switch transitions
- **Queue Management**: Need to queue requests during transition

### Neutral

- **No Built-in Versioning**: Use pipeline names for versions (simpler but less explicit)
- **No Automatic Rollback**: Must trigger another hot-reload to rollback
- **Drain Timeout Tuning**: May need to adjust per pipeline

## Implementation Notes

### Phase 3.2: Hot-Reload (Config File Watch)
- Add file watcher on config file (notify crate)
- Implement config diff detection (added/removed/modified pipelines)
- Implement drain-and-switch for modified pipelines
- Add new pipelines at runtime
- Remove pipelines (drain then delete)

See [DAEMONIZATION_ROADMAP.md - Phase 3.2](../../DAEMONIZATION_ROADMAP.md#32-hot-reload-config-file-watch-) for detailed implementation plan.

### Phase 3.3: Hot-Reload (Admin API)
- Add `POST /admin/reload` endpoint
- Add `POST /admin/pipelines/{name}/reload` endpoint
- Add `POST /admin/pipelines/{name}/reset` endpoint (reset failed pipeline)
- Add authentication for admin endpoints

See [DAEMONIZATION_ROADMAP.md - Phase 3.3](../../DAEMONIZATION_ROADMAP.md#33-hot-reload-admin-api-) for detailed implementation plan.

## Related ADRs

- [ADR 20 - Multi-Pipeline Architecture & Registry Pattern](./ADR%2020%20-%20Multi-Pipeline%20Architecture%20&%20Registry%20Pattern.md) - Hot-reload operates on PipelineRegistry
- [ADR 22 - Pipeline Lifecycle & Lazy Loading Strategy](./ADR%2022%20-%20Pipeline%20Lifecycle%20&%20Lazy%20Loading%20Strategy.md) - Drain is a lifecycle state
- [ADR 24 - NGINX Integration & Routing Responsibility](./ADR%2024%20-%20NGINX%20Integration%20&%20Routing%20Responsibility.md) - A/B testing via NGINX routing to different pipeline names

## References

- [DAEMONIZATION_ROADMAP.md - Phase 3.2: Hot-Reload (Config File Watch)](../../DAEMONIZATION_ROADMAP.md#32-hot-reload-config-file-watch-)
- [DAEMONIZATION_ROADMAP.md - Phase 3.3: Hot-Reload (Admin API)](../../DAEMONIZATION_ROADMAP.md#33-hot-reload-admin-api-)
- Hot Reload Patterns: https://martinfowler.com/bliki/BlueGreenDeployment.html
- Graceful Shutdown: https://cloud.google.com/blog/products/containers-kubernetes/kubernetes-best-practices-terminating-with-grace
