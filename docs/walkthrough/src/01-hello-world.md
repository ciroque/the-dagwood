# Hello World: Single Processor

## What You'll See

This demonstration shows the simplest possible DAG execution: a single processor with no dependencies. You'll see how DAGwood handles the most basic workflow scenario and learn fundamental Rust concepts used throughout the system.

**Key Learning Points:**
- Basic Rust ownership patterns in processor execution
- Simple async/await usage with tokio runtime
- ProcessorRequest and ProcessorResponse structures
- Entry point detection in DAG execution

## The Demo

### Command Line

```bash
cargo run --release -- docs/walkthrough/configs/01-hello-world.yaml "hello world"
```

### Configuration

```yaml
# Demo 1: Hello World - Single Processor
# This demonstrates the simplest possible DAG: one processor with no dependencies

strategy: work_queue
failure_strategy: fail_fast

executor_options:
  max_concurrency: 1

processors:
  - id: hello_processor
    type: local
    processor: change_text_case_upper
    depends_on: []
    options:
    # No additional options needed for this simple example
```

**Configuration Elements:**
- **Strategy**: `work_queue` (dependency counting algorithm)
- **Failure Strategy**: `fail_fast` (stop on first error)
- **Concurrency**: Limited to 1 for simplicity
- **Single Processor**: `hello_processor` with no dependencies

### Expected Output

When you run this demo, you'll see:

```
🚀 DAGwood Execution Strategy Demo
═══════════════════════════════════
Input: "hello world"
Config files: ["docs/walkthrough/configs/01-hello-world.yaml"]

📋 Configuration: docs/walkthrough/configs/01-hello-world.yaml
🔧 Strategy: WorkQueue
⚙️  Max Concurrency: 1
🛡️  Failure Strategy: FailFast

📊 Execution Results:
⏱️  Execution Time: ~2ms
🔢 Processors Executed: 1

🔄 Processor Chain:
  1. hello_processor → "HELLO WORLD"

🎯 Final Transformation:
   Input:  "hello world"
   Output: "HELLO WORLD"
```

## What You Just Saw

This demo demonstrated:

**DAG Execution Basics:**
- Single processor workflow with no dependencies
- Entry point detection (processors with empty `depends_on`)
- Work Queue strategy handling the simplest case

**Rust Fundamentals:**
- Ownership patterns in data processing
- Async/await for non-blocking execution
- Result-based error handling throughout
- Protobuf integration for structured data

**System Architecture:**
- Configuration-driven processor selection
- Factory pattern for processor creation
- Clean separation between configuration and execution

## What's Next?

In the next demo, the exploration moves to **linear chains** where processors depend on each other, introducing:
- Data flow between processors
- Dependency resolution algorithms
- More complex ownership patterns with `Arc<T>` and `Mutex<T>`

---

> 💡 **Rust Learning Tip**: Notice how Rust's ownership system prevents data races and memory issues that are common in other languages. The compiler ensures that DAG execution is memory-safe without runtime overhead!
