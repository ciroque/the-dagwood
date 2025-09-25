# Rust Language Features: Async Execution and Concurrency Patterns

This directory showcases Rust's powerful async/await system and safe concurrency primitives for building high-performance DAG execution engines.

## Beginner: Async/Await Fundamentals

### The `async fn` Declaration
```rust
async fn execute_processor(processor: Arc<dyn Processor>, input: ProcessorRequest) -> ProcessorResponse {
    processor.process(input).await
}
```

**Key concepts:**
- `async fn` returns a `Future` that must be `.await`ed
- `await` yields control back to the async runtime
- Enables concurrent execution without blocking threads

### The `#[async_trait]` Macro
```rust
#[async_trait]
pub trait DagExecutor: Send + Sync {
    async fn execute(&self, processors: ProcessorMap, ...) -> Result<...>;
}
```

**Why this is needed:**
- Rust doesn't natively support async functions in traits (yet)
- `async_trait` macro transforms async trait methods into regular methods returning `Pin<Box<dyn Future>>`
- `Send + Sync` bounds ensure the trait can be used across threads

## Intermediate: Concurrency with Arc and Mutex

### Shared Ownership with `Arc<T>`
```rust
let processors = Arc::new(ProcessorMap::from(processor_map));
let processor_clone = processors.clone();  // Cheap reference count increment

tokio::spawn(async move {
    let processor = processor_clone.get(&processor_id)?;
    // Use processor...
});
```

**Why `Arc` (Atomically Reference Counted):**
- Enables multiple owners of the same data
- Thread-safe reference counting
- Data is dropped when the last `Arc` is dropped

### Safe Mutation with `Mutex<T>`
```rust
let results = Arc::new(Mutex::new(HashMap::<String, ProcessorResponse>::new()));
let results_clone = results.clone();

tokio::spawn(async move {
    let mut results_guard = results_clone.lock().await;
    results_guard.insert(processor_id, response);
    // Lock automatically released when guard goes out of scope
});
```

**Mutex benefits:**
- Prevents data races at compile time
- Async-aware locking with `tokio::sync::Mutex`
- RAII (Resource Acquisition Is Initialization) - lock released automatically

### The `tokio::spawn` Pattern
```rust
let task_handle = tokio::spawn(async move {
    // This closure takes ownership of moved variables
    let response = processor.process(input).await;
    // Return value becomes the task result
    response
});

let result = task_handle.await?;  // Wait for task completion
```

## Advanced: Complex Concurrency Orchestration

### Fine-Grained Locking Strategy
Our WorkQueue executor uses multiple mutexes to minimize contention:

```rust
let active_tasks = Arc::new(Mutex::new(0));
let results_mutex = Arc::new(Mutex::new(HashMap::new()));
let work_queue_mutex = Arc::new(Mutex::new(VecDeque::new()));
let dependency_counts_mutex = Arc::new(Mutex::new(dependency_counts));
```

**Why separate mutexes:**
- Reduces lock contention - different data structures can be accessed concurrently
- Prevents deadlocks through consistent lock ordering
- Enables fine-grained parallelism

### Concurrency Control Pattern
```rust
loop {
    let next_processor_id = {
        let mut queue = work_queue_mutex.lock().await;
        let active_count = *active_tasks.lock().await;
        
        if active_count >= self.max_concurrency {
            break;  // Respect concurrency limits
        }
        
        queue.pop_front()  // Lock released here
    };
    
    if let Some(processor_id) = next_processor_id {
        // Spawn task outside of lock
        tokio::spawn(async move { /* ... */ });
    }
}
```

**Advanced patterns demonstrated:**
- **Lock scoping**: Use blocks to limit lock lifetime
- **Backpressure**: Respect concurrency limits to prevent resource exhaustion
- **Lock-free task spawning**: Spawn tasks outside of critical sections

### Dependency Coordination
```rust
// Update dependency counts atomically
{
    let mut counts = dependency_counts_mutex.lock().await;
    for dependent_id in dependents {
        if let Some(count) = counts.get_mut(dependent_id) {
            *count -= 1;
            if *count == 0 {
                // Processor is ready - add to work queue
                let mut queue = work_queue_mutex.lock().await;
                queue.push_back(dependent_id.clone());
            }
        }
    }
}
```

## Key Rust Concurrency Concepts

### 1. **Send and Sync Traits**
```rust
pub trait DagExecutor: Send + Sync {
    // Send: Can be transferred between threads
    // Sync: Can be shared between threads (via &T)
}
```

### 2. **Move Closures**
```rust
tokio::spawn(async move {
    // `move` transfers ownership of captured variables into the closure
    // Essential for async tasks that outlive their creating scope
});
```

### 3. **Future Combinators**
```rust
// Wait for all tasks to complete
let results = futures::future::join_all(task_handles).await;

// Race multiple futures
let first_result = futures::future::select_all(futures).await;
```

### 4. **Async Drop and Resource Management**
```rust
// Async mutexes automatically release locks when guards are dropped
{
    let _guard = mutex.lock().await;
    // Critical section
}  // Lock automatically released here
```

## Performance Considerations

### 1. **Avoid Blocking in Async Context**
```rust
// ❌ Bad: Blocks the async runtime
std::thread::sleep(Duration::from_secs(1));

// ✅ Good: Yields to other tasks
tokio::time::sleep(Duration::from_secs(1)).await;
```

### 2. **Minimize Lock Contention**
```rust
// ❌ Bad: Hold lock during async operation
let guard = mutex.lock().await;
expensive_async_operation().await;
drop(guard);

// ✅ Good: Release lock before async operation
let data = {
    let guard = mutex.lock().await;
    guard.clone()  // Copy data while holding lock
};
expensive_async_operation_with_data(data).await;
```

### 3. **Batch Operations When Possible**
```rust
// Process multiple items in one lock acquisition
let mut guard = results_mutex.lock().await;
for (id, response) in batch_results {
    guard.insert(id, response);
}
```

## Design Patterns Applied

1. **Actor Model**: Each processor acts independently, communicating through channels
2. **Work Stealing**: Dynamic work distribution across available tasks
3. **Backpressure**: Limit concurrent operations to prevent resource exhaustion
4. **Graceful Degradation**: Continue processing when individual processors fail

This async architecture enables The DAGwood to efficiently execute complex dependency graphs while maintaining safety and preventing data races through Rust's ownership system.
