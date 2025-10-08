# Diamond Analysis: Parallel Execution

Our third demonstration showcases the classic diamond dependency pattern, where multiple processors run in parallel and their results converge. This is where DAG execution becomes truly powerful!

## What You'll Learn

- **Parallel execution** with tokio async tasks
- **Canonical payload architecture** (Transform vs Analyze)
- **Metadata collection and merging** strategies
- **Race condition prevention** in concurrent execution

## Configuration Overview

```yaml
# Demo 3: Diamond Analysis - Parallel Execution
# This demonstrates the classic diamond dependency pattern with parallel analysis

strategy: work_queue
failure_strategy: fail_fast

executor_options:
  max_concurrency: 4

processors:
  # Entry point: prepare the text
  - id: prepare_text
    backend: local
    impl: change_text_case_lower
    depends_on: []
    options: {}

  # Parallel analysis processors (both depend on prepare_text)
  - id: count_tokens
    backend: local
    impl: token_counter
    depends_on: [prepare_text]
    options:
      count_type: "words"

  - id: analyze_frequency
    backend: local
    impl: word_frequency_analyzer
    depends_on: [prepare_text]
    options: {}

  # Convergence point: combine results (depends on both analysis processors)
  - id: final_summary
    backend: local
    impl: prefix_suffix_adder
    depends_on: [count_tokens, analyze_frequency]
    options:
      prefix: "Analysis Complete: "
      suffix: " [END]"
```

### Diamond Pattern Analysis

This creates the diamond pattern: `prepare_text → [count_tokens, analyze_frequency] → final_summary`

- **Divergence**: `prepare_text` feeds two parallel processors
- **Parallel execution**: `count_tokens` and `analyze_frequency` run concurrently
- **Convergence**: `final_summary` waits for both analysis results

## Rust Concepts in Action

### 1. Concurrent Task Execution

The Work Queue executor spawns multiple async tasks that run in parallel:

```rust
// Simplified parallel execution
let semaphore = Arc::new(Semaphore::new(max_concurrency)); // Limit concurrent tasks

for ready_processor in ready_processors {
    let permit = semaphore.clone().acquire_owned().await?;
    let task_handle = tokio::spawn(async move {
        let _permit = permit; // Hold permit for duration of task
        
        // Execute processor
        let result = processor.process(input).await?;
        
        // Update shared state
        {
            let mut results_guard = results.lock().await;
            results_guard.insert(processor_id, result);
        }
        
        Ok(())
    });
    
    task_handles.push(task_handle);
}
```

**Key Rust features**:
- **`Arc<Semaphore>`**: Shared concurrency control
- **`acquire_owned()`**: Move permit into async task
- **`tokio::spawn`**: True parallel execution
- **RAII**: Permit automatically released when task completes

### 2. Canonical Payload Architecture

The breakthrough insight that eliminates race conditions:

```rust
// Revolutionary approach: canonical payload tracking
let canonical_payload_mutex = Arc::new(Mutex::new(original_input.payload.clone()));

// For processors with dependencies
let input_for_processor = {
    let canonical_payload = canonical_payload_mutex.lock().await;
    ProcessorRequest {
        payload: canonical_payload.clone(), // Same payload for all parallel processors
        metadata: merged_dependency_metadata, // Different metadata per processor
    }
};

// After processor execution
match processor.declared_intent() {
    ProcessorIntent::Transform => {
        // Only Transform processors update canonical payload
        let mut canonical_guard = canonical_payload_mutex.lock().await;
        *canonical_guard = processor_response.payload;
    },
    ProcessorIntent::Analyze => {
        // Analyze processors only contribute metadata
        // Payload remains unchanged
    }
}
```

**Architectural benefits**:
- **Deterministic execution**: No race conditions regardless of completion order
- **Clear separation**: Transform vs Analyze processor roles
- **Simplified logic**: No complex payload merging strategies needed

### 3. Metadata Merging Strategy

Dependency metadata is collected and merged using collision-resistant namespacing:

```rust
// From src/utils/metadata.rs
fn merge_metadata_from_responses(
    base_metadata: HashMap<String, String>,
    dependency_responses: &HashMap<String, ProcessorResponse>
) -> HashMap<String, ProcessorMetadata> {
    let mut result = HashMap::new();
    
    // Add base metadata
    result.insert("input".to_string(), ProcessorMetadata { 
        metadata: base_metadata 
    });
    
    // Add dependency metadata with processor-based namespacing
    for (processor_id, response) in dependency_responses {
        if let Some(metadata) = &response.metadata {
            result.insert(processor_id.clone(), metadata.clone());
        }
    }
    
    result
}
```

## Expected Output

```
📋 Configuration: docs/demo/configs/03-diamond-analysis.yaml
🔧 Strategy: WorkQueue
⚙️  Max Concurrency: 4
🛡️  Failure Strategy: FailFast

📊 Execution Results:
⏱️  Execution Time: ~3ms
🔢 Processors Executed: 4

🔄 Processor Chain:
  1. prepare_text → "hello world"
  2. count_tokens → "hello world" (+ metadata: word_count: 2)
  3. analyze_frequency → "hello world" (+ metadata: frequency_map: {...})
  4. final_summary → "Analysis Complete: hello world [END]"

🎯 Final Transformation:
   Input:  "hello world"
   Output: "Analysis Complete: hello world [END]"
   
   Pipeline Metadata:
   count_tokens:
      • word_count: 2
      • character_count: 11
   analyze_frequency:
      • most_frequent_word: hello
      • unique_words: 2
```

## Architecture Deep Dive

### Race Condition Prevention

Before the canonical payload architecture, this pattern had non-deterministic behavior:

```rust
// OLD (problematic): First dependency to complete wins
let input_payload = dependency_results.values().next().unwrap().payload;

// NEW (deterministic): Canonical payload for all
let input_payload = canonical_payload_mutex.lock().await.clone();
```

### Processor Intent Classification

The `ProcessorIntent` trait enables architectural enforcement:

```rust
pub trait Processor: Send + Sync {
    fn declared_intent(&self) -> ProcessorIntent;
    // ...
}

pub enum ProcessorIntent {
    Transform, // Can modify payload
    Analyze,   // Only contributes metadata
}
```

**Real implementations**:
- `ChangeTextCaseProcessor`: `Transform` (modifies text)
- `TokenCounterProcessor`: `Analyze` (counts without modifying)
- `WordFrequencyAnalyzer`: `Analyze` (analyzes without modifying)

### Dependency Isolation

Metadata collection ensures processors only receive data from their actual dependencies:

```rust
// Collect metadata only from actual dependencies
let mut dependency_results = HashMap::new();
for dep_id in &processor_dependencies {
    if let Some(dep_response) = results_guard.get(dep_id) {
        dependency_results.insert(dep_id.clone(), dep_response.clone());
    }
}
// No contamination from unrelated processors!
```

## Performance Analysis

### Parallel Speedup

With `max_concurrency: 4`, the analysis processors run truly in parallel:

```
Timeline:
0ms: prepare_text starts
1ms: prepare_text completes
1ms: count_tokens AND analyze_frequency start (parallel!)
2ms: Both analysis processors complete
2ms: final_summary starts
3ms: final_summary completes
```

Compare to sequential execution: 1ms + 1ms + 1ms + 1ms = 4ms total
Parallel execution: 1ms + max(1ms, 1ms) + 1ms = 3ms total

### Memory Efficiency

The `Arc<ProcessorRequest>` pattern minimizes memory usage:

```rust
// Shared input across parallel processors
let input_arc = Arc::new(processor_input);
let count_task_input = input_arc.clone(); // Cheap reference count increment
let freq_task_input = input_arc.clone();  // Another cheap increment
```

## Try It Yourself

Experiment with processor intents:

1. **Change `token_counter` to Transform**: What happens to the final output?
2. **Add more analysis processors**: How does concurrency scale?
3. **Create nested diamonds**: Can you build `A → [B, C] → [D, E] → F`?

## What's Next?

In our next demo, we'll explore **WASM integration** where processors run in secure sandboxes, introducing:
- Cross-language processor execution
- Memory management across WASM boundaries
- Security isolation patterns
- Multi-backend coordination

---

> ⚡ **Performance Insight**: The diamond pattern is where DAG execution shines! By running analysis processors in parallel while maintaining deterministic results, we achieve both performance and correctness - a classic challenge in distributed systems that Rust's ownership model helps us solve elegantly.
