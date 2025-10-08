# Complex Workflow: Multi-Backend Pipeline

The final demonstration showcases a sophisticated workflow that combines everything learned: multiple execution strategies, mixed backends, advanced error handling, and production-ready patterns.

## What You'll Learn

- **Level-by-Level vs Work Queue** execution strategies
- **Mixed local and WASM** processor coordination
- **Advanced error handling** with failure strategies
- **Production-ready workflow** orchestration patterns

## Configuration Overview

```yaml
# Demo 5: Complex Workflow - Multi-Backend Pipeline
# This demonstrates advanced DAG with multiple backends and execution strategies

strategy: level  # Use level-by-level execution for comparison
failure_strategy: best_effort

executor_options:
  max_concurrency: 6

processors:
  # Entry points: multiple input processors
  - id: input_a
    backend: local
    impl: change_text_case_upper
    depends_on: []
    options: {}

  - id: input_b
    backend: local
    impl: change_text_case_lower
    depends_on: []
    options: {}

  # Processing layer: mix of local and WASM
  - id: process_a
    backend: local
    impl: reverse_text
    depends_on: [input_a]
    options: {}

  - id: process_b_wasm
    backend: wasm
    module: wasm_modules/hello_world.wasm
    depends_on: [input_b]
    options:
      intent: transform

  # Analysis layer: parallel analysis of both paths
  - id: analyze_a
    backend: local
    impl: token_counter
    depends_on: [process_a]
    options:
      count_type: "characters"

  - id: analyze_b
    backend: local
    impl: word_frequency_analyzer
    depends_on: [process_b_wasm]
    options: {}

  # Convergence: combine all results
  - id: final_merge
    backend: local
    impl: prefix_suffix_adder
    depends_on: [analyze_a, analyze_b]
    options:
      prefix: "🔄 Multi-Backend Result: "
      suffix: " [COMPLETE]"
```

### Complex DAG Analysis

This creates a sophisticated multi-path DAG:

```
    input_a ──► process_a ──► analyze_a ──┐
                                          ├──► final_merge
    input_b ──► process_b_wasm ──► analyze_b ──┘
```

- **Multiple entry points**: Two independent starting processors
- **Mixed backends**: Local and WASM processors intermixed
- **Parallel analysis**: Two analysis paths that converge
- **Level-by-Level execution**: Different strategy for comparison

## Rust Concepts in Action

### 1. Level-by-Level Execution Strategy

Unlike Work Queue's dependency counting, Level-by-Level uses topological levels:

```rust
// From src/engine/level_by_level.rs
fn compute_topological_levels(graph: &DependencyGraph) -> Result<Vec<Vec<String>>, ExecutionError> {
    let mut levels = Vec::new();
    let mut processed = HashSet::new();
    let mut current_level = Vec::new();
    
    // Level 0: Entry points (no dependencies)
    for (processor_id, dependencies) in &graph.0 {
        if dependencies.is_empty() {
            current_level.push(processor_id.clone());
        }
    }
    
    while !current_level.is_empty() {
        levels.push(current_level.clone());
        
        // Mark current level as processed
        for processor_id in &current_level {
            processed.insert(processor_id.clone());
        }
        
        // Find next level: processors whose dependencies are all processed
        let mut next_level = Vec::new();
        for (processor_id, dependencies) in &graph.0 {
            if !processed.contains(processor_id) {
                let all_deps_processed = dependencies.iter()
                    .all(|dep| processed.contains(dep));
                    
                if all_deps_processed {
                    next_level.push(processor_id.clone());
                }
            }
        }
        
        current_level = next_level;
    }
    
    Ok(levels)
}
```

**Algorithm characteristics**:
- **Batch processing**: Execute entire levels at once
- **Clear boundaries**: Explicit level separation
- **Predictable ordering**: Deterministic level assignment
- **Memory efficiency**: O(V + E) space complexity

### 2. Best Effort Failure Strategy

The `best_effort` failure strategy demonstrates resilient execution:

```rust
// Simplified error handling in level execution
match processor.process(input).await {
    Ok(response) => {
        // Success: store result and continue
        results_guard.insert(processor_id.clone(), response);
    },
    Err(e) => match failure_strategy {
        FailureStrategy::FailFast => {
            return Err(e); // Stop immediately
        },
        FailureStrategy::BestEffort => {
            // Continue with other processors
            // Failed processor won't contribute to dependents
            eprintln!("Processor {} failed but continuing: {}", processor_id, e);
        },
        FailureStrategy::ContinueOnError => {
            // Similar to BestEffort but with different semantics
        }
    }
}
```

**Resilience patterns**:
- **Graceful degradation**: System continues despite failures
- **Partial results**: Successful processors still contribute
- **Error isolation**: Failures don't cascade unnecessarily

### 3. Multi-Backend Coordination

The processor factory seamlessly handles different backends:

```rust
// From src/config/processor_map.rs
pub fn resolve_processor(config: &ProcessorConfig) -> Result<Box<dyn Processor>, ProcessorError> {
    match config.backend {
        BackendType::Local => {
            LocalProcessorFactory::create_processor(config)
        },
        BackendType::Wasm => {
            WasmProcessorFactory::create_processor(config)
        },
        // Future backends: RPC, SharedLibrary, etc.
    }
}
```

**Architectural benefits**:
- **Backend abstraction**: Uniform processor interface
- **Easy extension**: New backends integrate seamlessly
- **Type safety**: Rust's type system prevents backend confusion

### 4. Advanced Concurrency Patterns

With `max_concurrency: 6`, this workflow demonstrates sophisticated parallelism:

```rust
// Level-by-level parallel execution within levels
async fn execute_level(
    level_processors: &[String],
    // ... other parameters
) -> Result<(), ExecutionError> {
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut task_handles = Vec::new();
    
    // Spawn tasks for all processors in this level
    for processor_id in level_processors {
        let permit = semaphore.clone().acquire_owned().await?;
        let task_handle = tokio::spawn(async move {
            let _permit = permit; // RAII: auto-release on completion
            
            // Execute processor (may be local or WASM)
            execute_single_processor(processor_id, input).await
        });
        
        task_handles.push(task_handle);
    }
    
    // Wait for all processors in this level to complete
    for handle in task_handles {
        handle.await??; // Double ? for JoinError and ExecutionError
    }
    
    Ok(())
}
```

## Expected Output

```
📋 Configuration: docs/demo/configs/05-complex-workflow.yaml
🔧 Strategy: Level
⚙️  Max Concurrency: 6
🛡️  Failure Strategy: BestEffort

📊 Execution Results:
⏱️  Execution Time: ~8ms
🔢 Processors Executed: 7

🔄 Processor Chain:
Level 0 (Entry Points):
  1. input_a → "HELLO WORLD"
  2. input_b → "hello world"

Level 1 (Processing):
  3. process_a → "DLROW OLLEH"
  4. process_b_wasm → "hello world-wasm"

Level 2 (Analysis):
  5. analyze_a → "DLROW OLLEH" (+ metadata: char_count: 11)
  6. analyze_b → "hello world-wasm" (+ metadata: word_analysis: {...})

Level 3 (Convergence):
  7. final_merge → "🔄 Multi-Backend Result: DLROW OLLEH [COMPLETE]"

🎯 Final Transformation:
   Input:  "hello world"
   Output: "🔄 Multi-Backend Result: DLROW OLLEH [COMPLETE]"
   
   Pipeline Metadata:
   analyze_a:
      • character_count: 11
      • processing_time_ms: 0.1
   analyze_b:
      • unique_words: 2
      • most_frequent: hello
   process_b_wasm:
      • module_path: wasm_modules/hello_world.wasm
      • execution_time_ms: 2.3
```

## Architecture Comparison

### Level-by-Level vs Work Queue

| Aspect | Level-by-Level | Work Queue |
|--------|----------------|------------|
| **Algorithm** | Topological levels | Dependency counting |
| **Execution** | Batch by level | Individual readiness |
| **Memory** | Level arrays | Priority queue + counters |
| **Parallelism** | Within levels only | Across entire DAG |
| **Predictability** | High (clear phases) | Medium (dynamic ordering) |
| **Efficiency** | Good for regular DAGs | Better for irregular DAGs |

### Performance Characteristics

```
Level-by-Level Timeline:
0ms: Level 0 starts (input_a, input_b) - parallel
1ms: Level 0 completes
1ms: Level 1 starts (process_a, process_b_wasm) - parallel
3ms: Level 1 completes (WASM takes longer)
3ms: Level 2 starts (analyze_a, analyze_b) - parallel
4ms: Level 2 completes
4ms: Level 3 starts (final_merge)
5ms: Level 3 completes

Work Queue Timeline (hypothetical):
0ms: input_a, input_b start - parallel
1ms: input_a completes, process_a starts
1ms: input_b completes, process_b_wasm starts
2ms: process_a completes, analyze_a starts
3ms: process_b_wasm completes, analyze_b starts
3ms: analyze_a completes
4ms: analyze_b completes, final_merge starts
5ms: final_merge completes
```

**Key insight**: Level-by-Level can be more efficient for regular DAGs due to better batching, while Work Queue excels with irregular dependency patterns.

## Production Patterns

### 1. Error Recovery Strategies

```rust
// Production-ready error handling
match execute_workflow(config).await {
    Ok(results) => {
        log::info!("Workflow completed successfully: {} processors", results.len());
        Ok(results)
    },
    Err(ExecutionError::ProcessorError { processor_id, source }) => {
        log::error!("Processor {} failed: {}", processor_id, source);
        // Could implement retry logic here
        Err(e)
    },
    Err(ExecutionError::ValidationError { message }) => {
        log::error!("Configuration invalid: {}", message);
        // Could implement config auto-correction
        Err(e)
    },
    Err(e) => {
        log::error!("Unexpected error: {}", e);
        Err(e)
    }
}
```

### 2. Observability Integration

```rust
// Future observability patterns
struct WorkflowMetrics {
    total_execution_time: Duration,
    processor_execution_times: HashMap<String, Duration>,
    memory_usage_peak: usize,
    concurrency_utilization: f64,
}

// Tracing integration
#[tracing::instrument(skip(processors, executor))]
async fn execute_workflow(
    processors: ProcessorRegistry,
    executor: Box<dyn DagExecutor>,
    // ...
) -> Result<WorkflowResults, ExecutionError> {
    let span = tracing::info_span!("workflow_execution");
    // ... execution with detailed tracing
}
```

### 3. Resource Management

```rust
// Production resource limits
struct ExecutorConfig {
    max_concurrency: usize,
    max_memory_mb: usize,
    execution_timeout: Duration,
    processor_timeout: Duration,
}

// Graceful shutdown
impl DagExecutor {
    async fn shutdown_gracefully(&self, timeout: Duration) -> Result<(), ShutdownError> {
        // Cancel running tasks
        // Wait for cleanup
        // Release resources
    }
}
```

## Try It Yourself

### Experiment with Strategies

1. **Change to Work Queue**: Modify `strategy: work_queue` and compare execution
2. **Add failure scenarios**: Create a processor that always fails
3. **Scale up**: Add more processors and observe concurrency patterns
4. **Mix more backends**: When RPC backend is available, create 3-way mixing

### Performance Testing

```bash
# Benchmark different strategies
time cargo run --release -- docs/demo/configs/05-complex-workflow.yaml "test input"

# Profile memory usage
valgrind --tool=massif cargo run --release -- docs/demo/configs/05-complex-workflow.yaml "test"
```

## What's Next?

This completes the progressive demo journey! The demonstrations have shown:

✅ **Single processor basics** (Hello World)  
✅ **Linear dependency chains** (Text Pipeline)  
✅ **Parallel diamond patterns** (Diamond Analysis)  
✅ **WASM integration** (Sandboxed Processing)  
✅ **Complex multi-backend workflows** (This demo)

### Future Exploration

- **Reactive Executor**: Event-driven execution for real-time workflows
- **Hybrid Strategies**: Combining multiple execution approaches
- **Advanced WASM**: WASI integration and component model
- **Distributed Execution**: Multi-node DAG orchestration
- **Machine Learning Integration**: AI-powered workflow optimization

---

> 🚀 **Production Insight**: This complex workflow demonstrates that The DAGwood Project is ready for real-world usage. The combination of Rust's safety, multiple execution strategies, WASM sandboxing, and robust error handling provides a solid foundation for production workflow orchestration systems!
