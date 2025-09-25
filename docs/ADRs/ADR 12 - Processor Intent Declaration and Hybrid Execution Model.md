# ADR 12: Processor Intent Declaration and Hybrid Execution Model

## Context

The DAGwood execution engine currently lacks a fundamental distinction between processors that modify data (transformers) and those that only analyze or enrich metadata (analyzers). This limitation creates several critical issues:

**Current Problems:**
1. **Unsafe Parallelism**: All processors are treated equally, preventing safe parallel execution of metadata-only processors
2. **Race Conditions**: Multiple processors modifying the same payload in parallel can cause data corruption
3. **Limited Metadata Propagation**: The current protobuf `ProcessorResponse` lacks a metadata field, preventing processors from enriching context without modifying payload
4. **Server/Daemon Incompatibility**: Multi-leaf-node pipelines don't align with single-result request-response patterns expected in production servers

**Architectural Challenges:**
- Need to distinguish between payload-modifying and metadata-only processors
- Require explicit processor intent declarations for safe parallelism
- Must support both sequential transformation chains and parallel metadata enrichment
- Need deterministic result collection from parallel processors
- Require single canonical output for server/daemon scenarios

**Industry Patterns:**
- **Middleware Pattern**: Sequential processing with pass-through capability (Express.js, ASP.NET Core)
- **Map-Reduce**: Parallel processing with explicit collection phases (Hadoop, Spark)
- **Stream Processing**: Event-driven with metadata enrichment (Kafka Streams, Apache Flink)
- **Microservices**: Request enrichment through parallel service calls

## Decision

We will implement a **Hybrid Execution Model** with **Explicit Processor Intent Declarations** that supports both sequential payload transformation and parallel metadata enrichment.

### Core Design Principles

1. **Processor Intent Declaration**: Every processor must explicitly declare its intent (Transform vs Analyze)
2. **Safe Parallelism**: Only metadata-only processors can run in parallel with siblings
3. **Metadata Propagation**: Enhanced protobuf schema to support metadata alongside payload
4. **Deterministic Collection**: Configurable strategies for combining parallel processor outputs
5. **Single Canonical Output**: Support for extracting a single result for server/daemon scenarios

### Processor Intent Types

```yaml
processors:
  - id: text_transformer
    type: local
    processor: change_text_case_upper
    intent: transform  # Modifies payload, must run sequentially
    depends_on: []

  - id: metadata_analyzer
    type: local
    processor: token_counter
    intent: analyze    # Metadata-only, can run in parallel
    depends_on: [text_transformer]
```

**Intent Types:**
- **`transform`**: Processor modifies the payload and must run sequentially with siblings
- **`analyze`**: Processor only adds metadata and acts as pass-through, can run in parallel

### Enhanced Protobuf Schema

**Critical Issue Identified**: The current system loses metadata when chaining processors because `ProcessorResponse` lacks a metadata field, but `ProcessorRequest` requires both payload and metadata.

**Current Broken Flow:**
```
Processor A: ProcessorRequest{payload, metadata} → ProcessorResponse{next_payload}
Processor B: ProcessorRequest{next_payload, ???} ← Metadata is lost!
```

**Fixed Schema:**
```protobuf
message ProcessorRequest {
  bytes payload = 1;
  map<string, string> metadata = 2;
}

message ProcessorResponse {
  oneof outcome {
    bytes next_payload = 1;
    ProcessorError error = 2;
  }
  map<string, string> metadata = 3;     // NEW: Metadata for chaining and enrichment
  ProcessorIntent declared_intent = 4;  // NEW: Declared intent for validation
}

enum ProcessorIntent {
  TRANSFORM = 0;  // Modifies payload, may modify metadata
  ANALYZE = 1;    // Payload pass-through, may add metadata
}
```

**Corrected Data Flow:**
```
Processor A: ProcessorRequest{payload, metadata} → ProcessorResponse{next_payload, enriched_metadata}
Processor B: ProcessorRequest{next_payload, enriched_metadata} → ProcessorResponse{...}
```

### Execution Rules

1. **Sequential Execution**: Processors with `transform` intent must run sequentially when they share dependencies
2. **Parallel Execution**: Processors with `analyze` intent can run in parallel if they share the same dependencies
3. **Intent Validation**: Runtime validation ensures processors behave according to their declared intent
4. **Metadata Propagation**: All processors must return metadata in ProcessorResponse for proper chaining
5. **Metadata Merging**: Parallel analyzer outputs are merged using configurable collection strategies

### Work Queue Executor Updates

**Current Broken Implementation:**
```rust
// work_queue.rs line 309-312 - LOSES METADATA!
ProcessorRequest {
    payload: payload.clone(),  // Only payload from ProcessorResponse
    ..input_clone              // Uses ORIGINAL input metadata, not accumulated!
}
```

**Required Fix:**
```rust
// Proper metadata accumulation between processors
ProcessorRequest {
    payload: dep_response.next_payload,
    metadata: dep_response.metadata,  // Propagate accumulated metadata
}
```

**Metadata Accumulation Rules:**
- **Single Dependency**: Use dependency's metadata directly
- **Multiple Dependencies**: Merge metadata using collection strategy
- **Transform Intent**: May modify both payload and metadata
- **Analyze Intent**: Must preserve payload, may enrich metadata

### Pipeline Patterns

#### Pattern 1: Linear Transformation Chain
```yaml
# Sequential payload transformation
input → transform_1 → transform_2 → output
```

#### Pattern 2: Parallel Analysis with Collection
```yaml
# Parallel metadata enrichment
input → transform_base → [analyze_1, analyze_2] → result_collector → output
```

#### Pattern 3: Hybrid Pipeline
```yaml
# Mixed sequential and parallel processing
input → transform_1 → [analyze_1, analyze_2] → transform_2 → output
```

### Collection Strategies

For processors with multiple dependencies, use existing `ResultCollector` with enhanced strategies:

- **`FirstAvailable`**: Use first completing dependency (current behavior)
- **`MergeMetadata`**: Primary payload + secondary metadata
- **`Concatenate`**: Combine all outputs with separator
- **`JsonMerge`**: Smart JSON merging with conflict resolution
- **`Custom`**: Processor-specific combination logic

### Single Canonical Output

For server/daemon scenarios, support output extraction patterns:

1. **Leaf Node Selection**: Automatically identify final processors
2. **Explicit Output Designation**: Mark specific processor as canonical output
3. **Result Aggregation**: Combine multiple outputs into single response

## Implementation Plan

### Phase 1: Core Infrastructure
1. **Enhance Protobuf Schema**: Add metadata field and intent enum to `ProcessorResponse`
2. **Update Processor Trait**: Extend trait to support metadata propagation and intent declaration
3. **Configuration Schema**: Add `intent` field to processor configuration

### Phase 2: Intent System
1. **Intent Validation**: Runtime validation of processor behavior against declared intent
2. **Execution Rules**: Implement parallelism rules based on processor intent
3. **Metadata Merging**: Automatic metadata propagation through the pipeline

### Phase 3: Enhanced Execution
1. **Work Queue Extensions**: Support configurable collection strategies per processor
2. **Parallel Scheduling**: Safe parallel execution for analyze-intent processors
3. **Output Extraction**: Single canonical output for server/daemon scenarios

### Phase 4: Validation and Testing
1. **Intent Enforcement**: Comprehensive testing of intent validation
2. **Performance Testing**: Parallel execution performance benchmarks
3. **Integration Testing**: End-to-end pipeline validation

## Consequences

### Positive
- **Safe Parallelism**: Explicit intent declarations prevent race conditions
- **Enhanced Metadata**: Rich context propagation without payload modification
- **Flexible Patterns**: Support for both transformation chains and analysis pipelines
- **Server Compatibility**: Single output extraction for production scenarios
- **Performance**: Parallel execution of metadata-only processors
- **Clarity**: Explicit processor behavior declarations improve maintainability

### Negative
- **Configuration Complexity**: Processors must declare intent, adding configuration overhead
- **Breaking Changes**: Existing processors need intent declarations and potential refactoring
- **Validation Overhead**: Runtime intent validation adds execution cost
- **Learning Curve**: Developers must understand intent system and execution rules

### Risks
- **Intent Mismatch**: Processors declaring wrong intent could cause runtime failures
- **Performance Impact**: Metadata propagation and validation may affect performance
- **Migration Complexity**: Existing pipelines need careful migration to new intent system

## Alternatives Considered

### Alternative 1: Automatic Intent Detection
**Approach**: Analyze processor behavior at runtime to determine intent
**Rejected**: Too complex, unreliable, and adds significant runtime overhead

### Alternative 2: Separate Processor Types
**Approach**: Create distinct `Transformer` and `Analyzer` processor types
**Rejected**: Requires major refactoring and breaks existing processor implementations

### Alternative 3: Configuration-Only Parallelism
**Approach**: Use configuration flags to enable/disable parallelism per processor
**Rejected**: Doesn't address fundamental safety issues or provide clear semantics

## References

- **ADR 11**: Parallel Execution Result Collection Strategy (foundation for collection strategies)
- **ADR 2**: DAG Execution Patterns (pluggable execution framework)
- **Middleware Pattern**: Express.js, ASP.NET Core pipeline architectures
- **Stream Processing**: Kafka Streams, Apache Flink metadata enrichment patterns
- **Microservices**: Request enrichment and parallel service orchestration patterns

## Status

**Proposed** - Awaiting implementation and validation

## Decision Date

2025-09-25

## Stakeholders

- **Architecture Team**: System design and implementation strategy
- **Backend Team**: Processor implementations and intent declarations
- **DevOps Team**: Production deployment and server/daemon integration
- **QA Team**: Testing strategy for parallel execution and intent validation
