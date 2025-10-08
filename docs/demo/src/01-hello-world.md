# Hello World: Single Processor

Welcome to our first demonstration! This example shows the simplest possible DAG execution with a single processor and no dependencies.

## What You'll Learn

- **Basic Rust ownership patterns** in processor execution
- **Simple async/await usage** with tokio runtime
- **ProcessorRequest and ProcessorResponse** structures from protobuf
- **Entry point detection** in DAG execution algorithms

## Configuration Overview

Let's examine our first configuration file:

```yaml
# Demo 1: Hello World - Single Processor
# This demonstrates the simplest possible DAG: one processor with no dependencies

strategy: work_queue
failure_strategy: fail_fast

executor_options:
  max_concurrency: 1

processors:
  - id: hello_processor
    backend: local
    impl: change_text_case_upper
    depends_on: []
    options: {}
```

### Key Configuration Elements

- **`strategy: work_queue`**: Uses the Work Queue executor with dependency counting
- **`failure_strategy: fail_fast`**: Stops execution immediately on any processor failure
- **`max_concurrency: 1`**: Limits to single-threaded execution for simplicity
- **`depends_on: []`**: Empty dependencies make this processor an entry point

## Rust Concepts in Action

### 1. Ownership and Borrowing

When the DAG executor processes this configuration, it demonstrates several Rust ownership patterns:

```rust
// Entry point detection (simplified)
for processor_config in &config.processors {
    if processor_config.depends_on.is_empty() {
        entry_points_vec.push(processor_config.id.clone()); // Clone needed for ownership
    }
}
```

**Why clone?** The `processor_config.id` is borrowed from the config, but `entry_points_vec` needs owned `String` values.

### 2. Async/Await with Tokio

The processor execution uses Rust's async/await pattern:

```rust
// Simplified processor execution
let processor_response = processor.process(input).await?;
```

**Key insight**: The `await` point allows other tasks to run, but since we have `max_concurrency: 1`, this example runs sequentially.

### 3. Result<T, E> Error Handling

Every operation returns a `Result` for graceful error handling:

```rust
let config = load_and_validate_config(config_file)?; // ? operator propagates errors
let (results, metadata) = executor.execute_with_strategy(/* ... */).await?;
```

## Expected Output

When you run this demo, you should see:

```
📋 Configuration: docs/demo/configs/01-hello-world.yaml
🔧 Strategy: WorkQueue
⚙️  Max Concurrency: 1
🛡️  Failure Strategy: FailFast

📊 Execution Results:
⏱️  Execution Time: ~1ms
🔢 Processors Executed: 1

🔄 Processor Chain:
  1. hello_processor → "HELLO WORLD"
     📝 Metadata: 1 entries

🎯 Final Transformation:
   Input:  "hello world"
   Output: "HELLO WORLD"
```

## Architecture Insights

### Entry Point Detection

The DAG executor identifies entry points by finding processors with empty `depends_on` arrays:

```rust
// From src/engine/work_queue.rs (simplified)
let mut entry_points = Vec::new();
for (processor_id, dependencies) in &dependency_graph.0 {
    if dependencies.is_empty() {
        entry_points.push(processor_id.clone());
    }
}
```

### Processor Factory Pattern

The `change_text_case_upper` implementation is resolved through the factory pattern:

```rust
// From src/backends/local/factory.rs
match impl_name {
    "change_text_case_upper" => Ok(Box::new(ChangeTextCaseProcessor::new(TextCase::Upper))),
    // ... other processors
}
```

This demonstrates Rust's **trait objects** (`Box<dyn Processor>`) for runtime polymorphism.

## Try It Yourself

Run this demo with:

```bash
cargo run --release -- --demo-mode
```

Or run just this configuration:

```bash
cargo run --release -- docs/demo/configs/01-hello-world.yaml "hello world"
```

## What's Next?

In the next demo, we'll explore **linear chains** where processors depend on each other, introducing:
- Data flow between processors
- Dependency resolution algorithms
- More complex ownership patterns with `Arc<T>` and `Mutex<T>`

---

> 💡 **Rust Learning Tip**: Notice how Rust's ownership system prevents data races and memory issues that are common in other languages. The compiler ensures that our DAG execution is memory-safe without runtime overhead!
