# ADR 028: Memory Management & Eviction Policy

## Status
Proposed

## Context

A long-running daemon with lazy-loaded pipelines faces memory management challenges:
- **Memory growth**: Each initialized pipeline consumes memory (WASM modules, processor state, caches)
- **Unused pipelines**: Experimental or rarely-used pipelines waste memory after initialization
- **Resource limits**: Servers have finite memory, cannot load unlimited pipelines
- **Performance**: Evicting pipelines impacts first-request latency when re-initialized

Memory consumption per pipeline:
- **WASM modules**: 1-10 MB per module (compiled code + linear memory)
- **Processor instances**: 100 KB - 1 MB per processor (state, caches)
- **Configuration**: 10-100 KB per pipeline (dependency graph, metadata)
- **Total**: 2-50 MB per pipeline (varies widely)

Example scenarios:
- **10 pipelines**: 20-500 MB (manageable)
- **100 pipelines**: 200 MB - 5 GB (concerning)
- **1000 pipelines**: 2-50 GB (problematic)

Trade-offs:
- **Keep everything**: Simple, predictable, but wastes memory
- **Evict aggressively**: Memory efficient, but impacts latency
- **Evict selectively**: Balanced, but complex

**Decision needed**: How should we manage memory for long-running daemons with many pipelines?

## Decision

We will implement a **Two-Phase Strategy**: Keep-Everything for Phase 1, LRU Eviction for Phase 2 (future).

### Phase 1: Keep Everything (Initial Implementation)

**Approach**: Once initialized, pipelines stay in memory forever

**Rationale**:
- Simple: No eviction logic needed
- Predictable: Memory usage grows to max, then stable
- Fast: No re-initialization overhead
- Sufficient: Most deployments have < 50 pipelines

**Memory characteristics**:
- Memory grows as pipelines initialize
- Memory never shrinks (until server restart)
- Predictable maximum: `num_pipelines * avg_pipeline_size`

**When to use**:
- Small number of pipelines (< 50)
- Sufficient server memory (> 4 GB)
- All pipelines used regularly
- Predictable memory usage preferred

### Phase 2: LRU Eviction (Future Enhancement)

**Approach**: Evict least-recently-used pipelines when memory pressure detected

**Trigger conditions**:
```yaml
memory_management:
  enabled: true
  max_memory_mb: 2048  # Evict when total memory > 2 GB
  # or
  max_pipelines: 50  # Evict when pipeline count > 50
  # or
  idle_timeout: 1h  # Evict pipelines unused for 1 hour
  
  eviction_policy: lru  # Least Recently Used
  protected_pipelines:  # Never evict these
    - critical_pipeline
    - health_check_pipeline
```

**Eviction algorithm**:
1. Track last-used timestamp per pipeline
2. When memory pressure detected:
   - Sort pipelines by last-used (oldest first)
   - Skip protected pipelines (`startup: auto` or explicitly protected)
   - Evict oldest until memory pressure relieved
3. Evicted pipeline transitions to `Uninitialized` state
4. Re-initialize on next request (back to lazy loading)

**Protection rules**:
- `startup: auto` pipelines never evicted (critical for availability)
- Explicitly protected pipelines never evicted
- Pipelines with in-flight requests never evicted

**Memory pressure detection**:
- **Option A**: Monitor process RSS (Resident Set Size)
- **Option B**: Track allocated memory per pipeline
- **Option C**: Count initialized pipelines

### Configuration

**Phase 1 (Keep Everything):**
```yaml
# No configuration needed - default behavior
```

**Phase 2 (LRU Eviction):**
```yaml
memory_management:
  enabled: true
  policy: lru
  
  # Trigger conditions (any one triggers eviction)
  max_memory_mb: 2048
  max_pipelines: 100
  idle_timeout: 1h
  
  # Protection
  protect_startup_auto: true  # Never evict startup:auto pipelines
  protected_pipelines:
    - critical_pipeline
  
  # Eviction behavior
  eviction_batch_size: 5  # Evict 5 pipelines at a time
  min_free_memory_mb: 512  # Stop evicting when 512 MB free
```

### State Transitions with Eviction

```
Ready (in use)
    ↓ (idle for idle_timeout OR memory pressure)
Evicting (wait for in-flight requests)
    ↓ (no in-flight requests)
Uninitialized (evicted)
    ↓ (next request)
Initializing
    ↓ (success)
Ready
```

### Metrics

**Phase 2 adds metrics:**
```
# Memory usage
dagwood_memory_usage_bytes{type="total"}
dagwood_memory_usage_bytes{type="pipelines"}

# Eviction events
dagwood_pipeline_evictions_total{pipeline="text_processing"}

# Eviction duration
dagwood_pipeline_eviction_duration_seconds{pipeline="text_processing"}

# Protected pipeline count
dagwood_protected_pipelines
```

## Alternatives Considered

### Alternative 1: Time-Based Eviction (TTL)

**Approach**: Evict pipelines after fixed time period (e.g., 1 hour)

**Pros**:
- Simple: Just check timestamp
- Predictable: Know when eviction happens

**Cons**:
- Arbitrary: Why 1 hour vs 2 hours?
- Wasteful: May evict frequently-used pipelines
- Inefficient: May not evict when memory pressure high

**Rejected**: LRU is more adaptive and efficient. Can combine with LRU as secondary trigger.

### Alternative 2: Reference Counting

**Approach**: Evict pipelines when reference count reaches zero

**Pros**:
- Precise: Know exactly when pipeline unused
- Automatic: No manual tracking needed

**Cons**:
- Complex: Requires reference counting throughout codebase
- Rust challenges: Conflicts with Rust's ownership model
- Not applicable: Pipelines are long-lived, not request-scoped

**Rejected**: Reference counting is for short-lived objects. Pipelines are long-lived resources.

### Alternative 3: Manual Eviction Only

**Approach**: Only evict via admin API, no automatic eviction

```
POST /admin/pipelines/{name}/evict
```

**Pros**:
- Simple: No automatic eviction logic
- Predictable: Operators control eviction
- Safe: No unexpected evictions

**Cons**:
- Operational burden: Requires manual intervention
- Reactive: Must wait for memory issues before evicting
- Poor UX: Operators must monitor memory constantly

**Rejected**: Automatic eviction is essential for long-running daemons. Manual eviction can be added as supplement.

### Alternative 4: Separate Process Per Pipeline

**Approach**: Run each pipeline in separate process, OS handles memory

**Pros**:
- Isolation: Pipeline crash doesn't affect others
- OS management: OS handles memory pressure
- Simple: No eviction logic needed

**Cons**:
- Resource duplication: Each process loads same libraries
- IPC overhead: Inter-process communication for routing
- Operational complexity: Many processes to manage
- Contradicts multi-pipeline architecture (ADR 20)

**Rejected**: Contradicts the multi-pipeline architecture decision. Users who need process isolation can deploy multiple DAGwood instances.

### Alternative 5: Disk-Based Caching

**Approach**: Serialize evicted pipelines to disk, load from disk on next request

**Pros**:
- Fast re-initialization: Load from disk vs full initialization
- Persistent: Survives server restart

**Cons**:
- Complex: Requires serialization of all pipeline state
- Disk I/O: May be slower than re-initialization
- Storage: Requires disk space
- Compatibility: Processors may not be serializable

**Rejected**: Over-engineered. Re-initialization is fast enough for most pipelines.

## Consequences

### Positive

- **Phase 1 Simplicity**: No eviction logic, easy to implement and understand
- **Phase 2 Efficiency**: LRU eviction optimizes memory usage
- **Predictable**: Protected pipelines never evicted
- **Adaptive**: Eviction based on actual memory pressure
- **Operational Control**: Configuration for eviction behavior
- **Metrics**: Visibility into memory usage and eviction

### Negative

- **Phase 1 Memory Growth**: Memory never shrinks until restart
- **Phase 2 Complexity**: Eviction logic adds code complexity
- **Latency Impact**: Evicted pipelines have slow first request
- **Configuration Burden**: Must tune eviction parameters

### Neutral

- **Two-Phase Approach**: Simple first, optimize later
- **Protection Rules**: Balance between memory efficiency and availability
- **Eviction Timing**: May evict during low-traffic periods (good) or high-traffic periods (bad)

## Implementation Notes

### Phase 1: Keep Everything (MVP)

**Implementation:**
- No eviction logic
- Pipelines stay in memory once initialized
- Simple and predictable

**When to implement**: Initial daemon implementation

### Phase 2: LRU Eviction (Future)

**Implementation:**
- Track last-used timestamp per pipeline
- Monitor memory usage (RSS or allocated memory)
- Implement LRU eviction algorithm
- Add protection rules
- Add configuration
- Add metrics

**When to implement**: When users report memory issues with many pipelines

**Files to Create/Modify:**
- `src/server/eviction.rs` (new)
- `src/server/pipeline_lifecycle.rs` (modify - add eviction state)
- `src/server/pipeline_registry.rs` (modify - track last-used timestamps)

See [DAEMONIZATION_ROADMAP.md - Phase 6.2](../../DAEMONIZATION_ROADMAP.md#62-memory-management-lru-eviction-) for detailed implementation plan.

## Related ADRs

- [ADR 20 - Multi-Pipeline Architecture & Registry Pattern](./ADR%2020%20-%20Multi-Pipeline%20Architecture%20&%20Registry%20Pattern.md) - Registry manages pipeline lifecycle
- [ADR 22 - Pipeline Lifecycle & Lazy Loading Strategy](./ADR%2022%20-%20Pipeline%20Lifecycle%20&%20Lazy%20Loading%20Strategy.md) - Eviction returns pipeline to Uninitialized state
- [ADR 26 - Observability Strategy](./ADR%2026%20-%20Observability%20Strategy%20(Health%2C%20Metrics%2C%20Logging).md) - Metrics track memory usage and eviction

## References

- [DAEMONIZATION_ROADMAP.md - Phase 6.2: Memory Management (LRU Eviction)](../../DAEMONIZATION_ROADMAP.md#62-memory-management-lru-eviction-)
- LRU Cache: https://en.wikipedia.org/wiki/Cache_replacement_policies#Least_recently_used_(LRU)
- Memory Management Patterns: https://www.memorymanagement.org/
