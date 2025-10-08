# Text Pipeline: Linear Chain

Our second demonstration shows data flowing through a sequence of processors, introducing dependency resolution and the fundamental concepts of DAG execution.

## What You'll Learn

- **Data flow chaining** between processors
- **Dependency resolution** and topological ordering
- **Rust Result<T, E>** error handling patterns
- **Arc and Mutex** for shared state management

## Configuration Overview

```yaml
# Demo 2: Text Pipeline - Linear Chain
# This demonstrates data flow through a sequence of processors

strategy: work_queue
failure_strategy: fail_fast

executor_options:
  max_concurrency: 2

processors:
  - id: uppercase
    backend: local
    impl: change_text_case_upper
    depends_on: []
    options: {}

  - id: reverse
    backend: local
    impl: reverse_text
    depends_on: [uppercase]
    options: {}

  - id: add_prefix
    backend: local
    impl: prefix_suffix_adder
    depends_on: [reverse]
    options:
      prefix: ">>> "
      suffix: " <<<"
```

### Dependency Chain Analysis

This creates a linear dependency chain: `uppercase → reverse → add_prefix`

- **`uppercase`**: Entry point (no dependencies)
- **`reverse`**: Depends on `uppercase` output
- **`add_prefix`**: Depends on `reverse` output

## Rust Concepts in Action

### 1. Dependency Counting Algorithm

The Work Queue executor uses a dependency counting algorithm implemented in Rust:

```rust
// From src/engine/work_queue.rs (simplified)
let mut dependency_counts = HashMap::new();
for (processor_id, dependencies) in &dependency_graph.0 {
    dependency_counts.insert(processor_id.clone(), dependencies.len());
}

// Processors with 0 dependencies are ready to execute
let mut ready_queue = PriorityWorkQueue::new();
for (processor_id, count) in &dependency_counts {
    if *count == 0 {
        ready_queue.push(PrioritizedTask::new(processor_id.clone(), /* ... */));
    }
}
```

**Key Rust features**:
- **HashMap<String, usize>**: Efficient dependency tracking
- **Clone semantics**: Necessary for moving data into async tasks
- **Pattern matching**: `if *count == 0` dereferences the count

### 2. Async Task Coordination

Each processor runs in its own async task, coordinated through shared state:

```rust
// Simplified task spawning
let task_handle = tokio::spawn(async move {
    let processor_response = processor.process(input).await?;
    
    // Update shared results
    {
        let mut results_guard = results.lock().await;
        results_guard.insert(processor_id, processor_response);
    }
    
    // Notify dependent processors
    // ... dependency counting logic
});
```

**Rust async patterns**:
- **`tokio::spawn`**: Creates independent async tasks
- **`Arc<Mutex<T>>`**: Thread-safe shared state
- **Move closures**: Transfer ownership into async tasks

### 3. Data Flow Chaining

The critical insight: processors receive outputs from their dependencies, not the original input:

```rust
// From canonical payload architecture
let input_for_processor = if dependencies.is_empty() {
    // Entry point: use original input
    original_input.clone()
} else {
    // Dependent processor: use canonical payload + dependency metadata
    ProcessorRequest {
        payload: canonical_payload.lock().await.clone(),
        metadata: merged_dependency_metadata,
    }
};
```

## Expected Output

```
📋 Configuration: docs/demo/configs/02-text-pipeline.yaml
🔧 Strategy: WorkQueue
⚙️  Max Concurrency: 2
🛡️  Failure Strategy: FailFast

📊 Execution Results:
⏱️  Execution Time: ~2ms
🔢 Processors Executed: 3

🔄 Processor Chain:
  1. uppercase → "HELLO WORLD"
  2. reverse → "DLROW OLLEH"
  3. add_prefix → ">>> DLROW OLLEH <<<"

🎯 Final Transformation:
   Input:  "hello world"
   Output: ">>> DLROW OLLEH <<<"
```

## Architecture Deep Dive

### Topological Ordering

The Work Queue executor ensures processors execute in dependency order:

```rust
// Kahn's algorithm implementation (simplified)
while let Some(current_processor) = ready_queue.pop() {
    // Execute processor
    execute_processor(current_processor).await?;
    
    // Update dependency counts for dependents
    for dependent_id in dependents_of(&current_processor) {
        dependency_counts[dependent_id] -= 1;
        if dependency_counts[dependent_id] == 0 {
            ready_queue.push(dependent_id);
        }
    }
}
```

### Canonical Payload Architecture

A revolutionary insight from our development: only Transform processors update the payload, while Analyze processors contribute metadata:

```rust
// After processor execution
if processor.declared_intent() == ProcessorIntent::Transform {
    // Update canonical payload
    let mut canonical_guard = canonical_payload_mutex.lock().await;
    *canonical_guard = processor_response.payload;
}
// Analyze processors only contribute metadata
```

**Benefits**:
- **Eliminates race conditions** in diamond patterns
- **Enforces architectural separation** between Transform and Analyze
- **Simplifies dependency resolution** logic

## Performance Characteristics

### Concurrency Analysis

With `max_concurrency: 2`, this linear chain still executes sequentially because each processor depends on the previous one. However, the infrastructure is ready for parallel execution when we introduce diamond patterns.

### Memory Efficiency

The `Arc<ProcessorRequest>` optimization reduces memory usage:

```rust
// Cheap Arc cloning instead of expensive data cloning
let input_arc = input.clone(); // Only increments reference count
let processor_task = tokio::spawn(async move {
    let owned_input = (*input_arc).clone(); // Clone only when needed
    processor.process(owned_input).await
});
```

## Try It Yourself

Experiment with different processor orders:

```yaml
# Try reversing the order - what happens?
processors:
  - id: add_prefix
    depends_on: [reverse]  # This will fail validation!
  - id: reverse
    depends_on: [uppercase]
  - id: uppercase
    depends_on: []
```

The dependency validation will catch this cycle during config loading!

## What's Next?

In our next demo, we'll explore the **diamond dependency pattern** where multiple processors can run in parallel, introducing:
- True concurrent execution
- Metadata merging strategies  
- Race condition prevention
- The canonical payload architecture in action

---

> 🔍 **Architecture Insight**: The linear chain might seem simple, but it demonstrates the foundation for complex DAG execution. Every workflow orchestration system must solve dependency resolution - Rust's ownership system makes this both memory-safe and performant!
