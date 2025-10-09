# Text Pipeline: Linear Chain

## What You'll See

This demonstration shows data flowing through a sequence of processors in a linear chain. You'll see how DAGwood handles dependency resolution, data flow between processors, and sequential execution patterns.

**Key Learning Points:**
- Data flow chaining between processors
- Dependency resolution and topological ordering
- Rust Result<T, E> error handling patterns
- Arc and Mutex for shared state management

## The Demo

### Command Line

```bash
cargo run --release -- docs/walkthrough/configs/02-text-pipeline.yaml "hello world"
```

### Configuration

```yaml
# Demo 2: Text Pipeline - Linear Chain
# This demonstrates data flow through a sequence of processors

strategy: work_queue
failure_strategy: fail_fast

executor_options:
  max_concurrency: 2

processors:
  - id: uppercase
    type: local
    processor: change_text_case_upper
    depends_on: []
    options: {}

  - id: reverse
    type: local
    processor: reverse_text
    depends_on: [uppercase]
    options: {}

  - id: add_prefix
    type: local
    processor: prefix_suffix_adder
    depends_on: [reverse]
    options:
      prefix: ">>> "
      suffix: " <<<"
```

**Configuration Elements:**
- **Strategy**: `work_queue` (dependency counting algorithm)
- **Failure Strategy**: `fail_fast` (stop on first error)
- **Concurrency**: Set to 2 (though this chain executes sequentially)
- **Linear Chain**: `uppercase → reverse → add_prefix`

### Expected Output

When you run this demo, you'll see:

```
🚀 DAGwood Execution Strategy Demo
═══════════════════════════════════
Input: "hello world"
Config files: ["docs/walkthrough/configs/02-text-pipeline.yaml"]

📋 Configuration: docs/walkthrough/configs/02-text-pipeline.yaml
🔧 Strategy: WorkQueue
⚙️  Max Concurrency: 2
🛡️  Failure Strategy: FailFast

📊 Execution Results:
⏱️  Execution Time: ~3ms
🔢 Processors Executed: 3

🔄 Processor Chain:
  1. uppercase → "HELLO WORLD"
  2. reverse → "DLROW OLLEH"
  3. add_prefix → ">>> DLROW OLLEH <<<"

🎯 Final Transformation:
   Input:  "hello world"
   Output: ">>> DLROW OLLEH <<<"
```

## What You Just Saw

This demo demonstrated:

**Linear DAG Execution:**
- Sequential processor execution despite concurrency=2
- Data flowing through the chain: input → uppercase → reverse → add_prefix
- Dependency counting algorithm managing execution order

**Rust Concurrency Patterns:**
- Async task spawning and coordination
- Shared state management with Arc<Mutex<T>>
- Memory-efficient Arc cloning for large payloads

**System Architecture:**
- Canonical payload architecture preventing race conditions
- Transform processor intent modifying data flow
- Dependency validation catching configuration errors

## What's Next?

In the next demo, the exploration moves to the **diamond dependency pattern** where multiple processors can run in parallel, introducing:
- True concurrent execution
- Metadata merging strategies  
- Race condition prevention
- The canonical payload architecture in action

---

> 🔍 **Architecture Insight**: The linear chain might seem simple, but it demonstrates the foundation for complex DAG execution. Every workflow orchestration system must solve dependency resolution - Rust's ownership system makes this both memory-safe and performant!
