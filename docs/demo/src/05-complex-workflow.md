# Complex Workflow: Multi-Backend Pipeline

## What You'll See

This final demonstration showcases a sophisticated workflow that combines everything learned. You'll see multiple execution strategies, mixed backends, advanced error handling, and production-ready patterns working together in a complex DAG.

**Key Learning Points:**
- Level-by-Level vs Work Queue execution strategies
- Mixed local and WASM processor coordination
- Advanced error handling with failure strategies
- Production-ready workflow orchestration patterns

## The Demo

### Command Line

```bash
cargo run --release -- docs/demo/configs/05-complex-workflow.yaml "hello world"
```

### Configuration

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

### Expected Output

When you run this demo, you'll see:

```
🚀 DAGwood Execution Strategy Demo
═══════════════════════════════════
Input: "hello world"
Config files: ["docs/demo/configs/05-complex-workflow.yaml"]

📋 Configuration: docs/demo/configs/05-complex-workflow.yaml
🔧 Strategy: LevelByLevel
⚙️  Max Concurrency: 6
🛡️  Failure Strategy: BestEffort

📊 Execution Results:
⏱️  Execution Time: ~8ms
🔢 Processors Executed: 6

🔄 Processor Chain:
  Level 0: input_a, input_b (parallel)
  Level 1: process_a, process_b_wasm (parallel)
  Level 2: analyze_a, analyze_b (parallel)
  Level 3: final_merge

🎯 Final Transformation:
   Input:  "hello world"
   Output: "🔄 Multi-Backend Result: hello world [COMPLETE]"
```

## What You Just Saw

This demo demonstrated:

**Complex DAG Orchestration:**
- Multiple entry points executing in parallel
- Mixed Local and WASM backends in a single workflow
- Level-by-Level execution strategy with clear synchronization points
- BestEffort failure strategy allowing graceful degradation

**Advanced Rust Patterns:**
- Topological level computation with HashSet and Vec operations
- Semaphore-based concurrency control across execution levels
- Factory pattern enabling seamless backend switching
- RAII-based resource management with automatic cleanup

**Production-Ready Features:**
- Resilient error handling that continues despite individual failures
- Rich metadata collection across multiple processor types
- Configurable concurrency limits for resource management
- Clean separation between execution strategy and processor implementation

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

**Key insight**: Level-by-Level provides predictable execution phases with clear synchronization points, while Work Queue offers maximum parallelism with dynamic scheduling.

## What's Next?

This completes the progressive demo journey! The demonstrations have shown:

✅ **Single processor basics** (Hello World)  
✅ **Linear dependency chains** (Text Pipeline)  
✅ **Parallel diamond patterns** (Diamond Analysis)  
✅ **WASM integration** (Sandboxed Processing)  
✅ **Complex multi-backend workflows** (This demo)

### Exploration Opportunities

- **Try different strategies**: Compare `work_queue` vs `level` execution
- **Experiment with failure modes**: Test `fail_fast` vs `best_effort` strategies
- **Add custom processors**: Extend the Local backend with new implementations
- **Build complex DAGs**: Create workflows with multiple diamond patterns
- **Performance analysis**: Measure execution times with different concurrency settings

---

> 🏆 **Congratulations!** You've completed the full DAGwood demo journey, from simple single processors to complex multi-backend workflows. You've seen Rust's power in building safe, concurrent, and extensible workflow orchestration systems.

---

> 🚀 **Production Insight**: This complex workflow demonstrates that The DAGwood Project is ready for real-world usage. The combination of Rust's safety, multiple execution strategies, WASM sandboxing, and robust error handling provides a solid foundation for production workflow orchestration systems!
