# ADR 11: Parallel Execution Result Collection Strategy

## Context

The current Work Queue executor implementation has a critical issue with non-deterministic behavior when processors run in parallel. When multiple processors depend on the same upstream processor and then feed into a downstream processor, the system exhibits race conditions because it arbitrarily selects the first available dependency's output (lines 172-187 in `work_queue.rs`).

**Problem Scenarios:**
1. **Diamond Dependencies**: Processor A → [B, C] → D, where D receives non-deterministic input from either B or C
2. **Fan-out Analysis**: Text processing → [token_counter, word_frequency] → analysis_merger, where the merger gets inconsistent inputs
3. **Read-only Processors**: Multiple analysis processors that don't modify input but produce metadata should run safely in parallel

**Current Behavior:**
```rust
// Multiple dependencies: for now, use the first dependency's output
// TODO: Implement proper input combination strategy for multiple dependencies
let dep_id = &dependencies[0];
```

This causes the downstream processor to receive input from whichever dependency completes first, leading to non-deterministic pipeline results.

**Research into Industry Solutions:**
- **Apache Airflow**: Uses XCom for explicit data retrieval from specific upstream tasks
- **Prefect**: Employs functional programming with automatic result collection via task mapping
- **Kubeflow Pipelines**: Uses `dsl.Collected` to explicitly gather outputs from parallel tasks
- **Argo Workflows**: Relies on synchronization primitives and artifact passing
- **Temporal**: Uses async/await patterns with explicit coordination

## Decision

We will implement a **Kubeflow-inspired explicit result collection strategy** using a dedicated `ResultCollector` processor type with configurable collection strategies.

**Key Components:**

1. **Collection Strategies Enum:**
   ```rust
   pub enum CollectionStrategy {
       FirstAvailable,                    // Current behavior (fallback)
       MergeMetadata {                   // Primary + metadata pattern
           primary_source: String,
           metadata_sources: Vec<String>,
       },
       Concatenate { separator: Option<String> },  // Combine all outputs
       JsonMerge {                       // Smart JSON combination
           merge_arrays: bool,
           conflict_resolution: ConflictResolution,
       },
       Custom { combiner_impl: String }, // Extensible custom logic
   }
   ```

2. **ResultCollector Processor:**
   - New processor type specifically for collecting parallel results
   - Implements different collection strategies based on configuration
   - Uses existing protobuf metadata field for secondary data

3. **Configuration Format:**
   ```yaml
   - id: analysis_collector
     type: local
     impl: ResultCollector
     collection_strategy:
       type: merge_metadata
       primary_source: token_counter
       metadata_sources: [word_frequency]
     depends_on: [token_counter, word_frequency]
   ```

4. **Work Queue Integration:**
   - Collector processors receive all dependency results as input
   - Regular processors maintain existing behavior
   - Deterministic execution order through explicit dependency resolution

**Selected Pattern: Explicit Collection with Metadata Merge**
- Primary processor output becomes the main payload
- Secondary processor outputs are added as metadata
- Downstream processors receive deterministic, structured data
- Leverages existing protobuf metadata field (no schema changes)

## Consequences

**Positive:**
- **Deterministic Execution**: Eliminates race conditions in parallel processor scenarios
- **Explicit Control**: Developers explicitly define how parallel results are combined
- **Backward Compatible**: Existing processors and configurations work unchanged
- **Flexible**: Multiple collection strategies support different use cases
- **No Protobuf Changes**: Uses existing metadata field for secondary data
- **Industry Alignment**: Similar to Kubeflow's `dsl.Collected` pattern

**Negative:**
- **Additional Complexity**: Introduces new processor type and configuration concepts
- **Verbose Configuration**: Requires explicit collection processor definitions
- **Learning Curve**: Developers must understand collection strategies
- **Potential Over-Engineering**: Simple cases now require more configuration

**Implementation Requirements:**
- New `ResultCollector` processor in local backend
- Collection strategy configuration parsing
- Work queue executor updates for collector processor handling
- Comprehensive testing for all collection scenarios
- Documentation and examples for common patterns

**Migration Path:**
- Existing configurations continue to work with current (non-deterministic) behavior
- New configurations can opt into explicit collection strategies
- Gradual migration as users encounter non-determinism issues

This decision provides a foundation for reliable parallel execution while maintaining flexibility for future enhancements and complex data flow patterns.
