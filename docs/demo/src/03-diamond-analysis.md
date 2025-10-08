# Diamond Analysis: Parallel Execution

## What You'll See

This demonstration showcases the classic diamond dependency pattern where multiple processors run in parallel and their results converge. You'll see true concurrent execution, metadata merging, and how the canonical payload architecture prevents race conditions.

**Key Learning Points:**
- Parallel execution with tokio async tasks
- Canonical payload architecture (Transform vs Analyze)
- Metadata collection and merging strategies
- Race condition prevention in concurrent execution

## The Demo

### Command Line

```bash
cargo run --release -- docs/demo/configs/03-diamond-analysis.yaml "hello world"
```

### Configuration

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
    type: local
    processor: change_text_case_lower
    depends_on: []
    options: {}

  # Parallel analysis processors (both depend on prepare_text)
  - id: count_tokens
    type: local
    processor: token_counter
    depends_on: [prepare_text]
    options:
      count_type: "words"

  - id: analyze_frequency
    type: local
    processor: word_frequency_analyzer
    depends_on: [prepare_text]
    options: {}

  # Convergence point: combine results (depends on both analysis processors)
  - id: final_summary
    type: local
    processor: prefix_suffix_adder
    depends_on: [count_tokens, analyze_frequency]
    options:
      prefix: "Analysis Complete: "
      suffix: " [END]"
```

**Configuration Elements:**
- **Strategy**: `work_queue` (dependency counting algorithm)
- **Failure Strategy**: `fail_fast` (stop on first error)
- **Concurrency**: Set to 4 (enables true parallel execution)
- **Diamond Pattern**: `prepare_text → [count_tokens, analyze_frequency] → final_summary`

### Expected Output

When you run this demo, you'll see:

```
🚀 DAGwood Execution Strategy Demo
═══════════════════════════════════
Input: "hello world"
Config files: ["docs/demo/configs/03-diamond-analysis.yaml"]

📋 Configuration: docs/demo/configs/03-diamond-analysis.yaml
🔧 Strategy: WorkQueue
⚙️  Max Concurrency: 4
🛡️  Failure Strategy: FailFast

📊 Execution Results:
⏱️  Execution Time: ~4ms
🔢 Processors Executed: 4

🔄 Processor Chain:
  1. prepare_text → "hello world"
  2. count_tokens → "2" (parallel)
  3. analyze_frequency → "{\"hello\": 1, \"world\": 1}" (parallel)
  4. final_summary → "Analysis Complete: hello world [END]"

🎯 Final Transformation:
   Input:  "hello world"
   Output: "Analysis Complete: hello world [END]"
```

## What You Just Saw

This demo demonstrated:

**Diamond Pattern Execution:**
- True parallel execution of `count_tokens` and `analyze_frequency`
- Convergence at `final_summary` waiting for both analysis results
- Canonical payload preventing race conditions in parallel execution

**Rust Concurrency Mastery:**
- Semaphore-controlled parallel task spawning
- Arc<Mutex<T>> for thread-safe shared state
- RAII permit management for resource cleanup
- Async task coordination with tokio::spawn

**System Architecture:**
- Transform vs Analyze processor intent separation
- Metadata merging with collision-resistant namespacing
- Deterministic execution regardless of completion order
- Dependency isolation ensuring clean data flow

## Performance Analysis

### Parallel Speedup
With `max_concurrency: 4`, the analysis processors run truly in parallel:
- **Sequential**: prepare_text (1ms) + count_tokens (1ms) + analyze_frequency (1ms) + final_summary (1ms) = 4ms
- **Parallel**: prepare_text (1ms) + max(count_tokens, analyze_frequency) (1ms) + final_summary (1ms) = 3ms
- **25% speedup** from parallel execution

### Memory Efficiency
- **Arc<ProcessorRequest>**: Shared input across parallel processors with reference counting
- **Canonical payload**: Single source of truth eliminates payload duplication
- **Metadata isolation**: Only relevant dependency metadata passed to each processor

## What's Next?

In the next demo, the exploration moves to **WASM integration** where processors run in secure sandboxes, introducing:
- Cross-language processor execution
- Memory management across WASM boundaries
- Security isolation patterns
- Multi-backend coordination

---

> ⚡ **Performance Insight**: The diamond pattern is where DAG execution shines! By running analysis processors in parallel while maintaining deterministic results, both performance and correctness are achieved - a classic challenge in distributed systems that Rust's ownership model helps solve elegantly.
