# RUSTME.md - Reactive Executor (`src/engine/reactive.rs`)

This document explores the Rust language features and patterns used in the Reactive Executor implementation. The reactive executor demonstrates advanced async programming, event-driven architecture, and concurrent data structures.

**Related Documentation:**
- [`RUSTME.md`](./RUSTME.md) - Core async/await patterns and concurrency fundamentals
- [`RUSTME-WorkQueue.md`](./RUSTME-WorkQueue.md) - Dependency-counting execution strategy
- [`RUSTME-LevelByLevel.md`](./RUSTME-LevelByLevel.md) - Level-based execution approach
- [`../traits/RUSTME.md`](../traits/RUSTME.md) - DagExecutor trait definition and async traits
- [`../config/RUSTME.md`](../config/RUSTME.md) - Configuration system and executor factory

## Beginner Level Concepts

### 1. **Enum Pattern Matching with Data**
The reactive executor uses enums to represent different types of events in the system:

```rust
#[derive(Debug, Clone)]
enum ProcessorEvent {
    Execute { metadata: HashMap<String, String> },
    DependencyCompleted { dependency_id: String, metadata: HashMap<String, String> },
}
```

**Why this pattern?** Enums with data allow us to create a type-safe event system where each event carries exactly the data it needs. Pattern matching ensures we handle all cases:

```rust
match event {
    ProcessorEvent::Execute { metadata } => {
        // Handle execution with metadata
    }
    ProcessorEvent::DependencyCompleted { dependency_id, metadata } => {
        // Handle dependency completion
    }
}
```

### 2. **Struct with Named Fields**
The `ProcessorNode` struct organizes all the state needed for each processor in the reactive system:

```rust
struct ProcessorNode {
    receiver: mpsc::UnboundedReceiver<ProcessorEvent>,
    dependents: Vec<String>,
    pending_dependencies: usize,
    dependency_results: HashMap<String, ProcessorResponse>,
}
```

**Why named fields?** They make the code self-documenting and prevent field ordering mistakes that can happen with tuple structs.

### 3. **HashMap for Key-Value Storage**
HashMaps are used extensively for processor lookups and result storage:

```rust
let mut senders = HashMap::new();
let mut nodes = HashMap::new();
```

**Why HashMap?** O(1) average lookup time makes processor and result retrieval very fast, which is crucial for the event-driven architecture.

## Intermediate Level Concepts

### 1. **Multi-Producer, Single-Consumer Channels (MPSC)**
The reactive executor uses unbounded channels for event communication:

```rust
let (sender, receiver) = mpsc::unbounded_channel();
```

**Why unbounded channels?** In event-driven systems, we don't want senders to block when notifying dependents. The unbounded nature prevents deadlocks but requires careful memory management.

**Channel ownership pattern:**
```rust
// Sender is cloned and shared among multiple notifiers
senders.insert(processor_id.clone(), sender);

// Receiver is moved into the processor node
nodes.insert(processor_id.clone(), ProcessorNode {
    receiver,
    // ...
});
```

### 2. **Arc<Mutex<T>> for Shared Mutable State**
Critical shared state uses Arc<Mutex<T>> for thread-safe access:

```rust
let canonical_payload_mutex = Arc::new(Mutex::new(input.payload.clone()));
let results_mutex = Arc::new(Mutex<HashMap<String, ProcessorResponse>>>::new(HashMap::new()));
```

**Why Arc<Mutex<T>>?** 
- `Arc` (Atomically Reference Counted) allows multiple tasks to own the same data
- `Mutex` provides exclusive access for mutations
- The combination enables safe concurrent access to shared state

**Lock acquisition pattern:**
```rust
let canonical_payload = {
    let guard = canonical_payload_mutex.lock().await;
    guard.clone() // Clone the data, then release the lock
};
```

### 3. **Async Task Spawning with Move Closures**
Each processor runs in its own async task:

```rust
let task = tokio::spawn(Self::spawn_processor_task(
    processor_id.clone(),
    node,
    processors_arc.clone(),
    canonical_payload_mutex.clone(),
    results_mutex.clone(),
    senders_arc.clone(),
    failure_strategy,
    semaphore.clone(),
    cancellation_token.clone(),
));
```

**Why spawn separate tasks?** This creates true parallelism where processors can run concurrently, but each task owns its required data through cloning Arc references.

### 4. **Tokio Select for Concurrent Operations**
The reactive executor uses `tokio::select!` to handle multiple async operations:

```rust
tokio::select! {
    _ = cancellation_token.cancelled() => {
        return Err(ExecutionError::InternalError {
            message: format!("Processor '{}' cancelled due to failure in another processor", processor_id),
        });
    }
    event_result = node.receiver.recv() => {
        // Handle incoming events
    }
}
```

**Why select?** It allows a task to wait on multiple async operations simultaneously and react to whichever completes first - essential for cancellation and event handling.

## Advanced Level Concepts

### 1. **Cancellation Token Pattern**
The reactive executor implements graceful shutdown using cancellation tokens:

```rust
let cancellation_token = CancellationToken::new();

// In failure scenarios:
cancellation_token.cancel();

// In each task:
tokio::select! {
    _ = cancellation_token.cancelled() => {
        // Graceful shutdown
    }
    // ... other operations
}
```

**Why this pattern?** It provides a clean way to coordinate shutdown across multiple async tasks without using channels or shared flags. The cancellation propagates immediately to all waiting tasks.

### 2. **Semaphore for Concurrency Control**
Concurrency is limited using a semaphore:

```rust
let semaphore = Arc::new(tokio::sync::Semaphore::new(self.max_concurrency));

// In each processor task:
let _permit = semaphore.acquire().await
    .map_err(|e| ExecutionError::InternalError {
        message: format!("Failed to acquire semaphore permit for processor '{}': {}", processor_id, e),
    })?;
```

**Why semaphores over thread pools?** Semaphores provide fine-grained control over resource usage while maintaining the async nature of the system. The permit is automatically released when `_permit` goes out of scope.

### 3. **Event-Driven State Machine**
Each processor acts as a state machine that transitions based on events:

```rust
// State: Waiting for dependencies
while node.pending_dependencies > 0 {
    // Wait for DependencyCompleted events
    // Decrement pending_dependencies
}

// State: Ready to execute
// Execute processor

// State: Notify dependents
for dependent_id in &node.dependents {
    // Send DependencyCompleted events
}
```

**Why state machines?** They make complex async coordination explicit and debuggable. Each processor's state is clear from its `pending_dependencies` count.

### 4. **Error Propagation with Context**
The reactive executor uses Result types with contextual error information:

```rust
let processor = processors.get(&processor_id)
    .ok_or_else(|| ExecutionError::ProcessorNotFound(processor_id.clone()))?;

let _permit = semaphore.acquire().await
    .map_err(|e| ExecutionError::InternalError {
        message: format!("Failed to acquire semaphore permit for processor '{}': {}", processor_id, e),
    })?;
```

**Why contextual errors?** In distributed async systems, knowing exactly which processor and operation failed is crucial for debugging. The `?` operator provides clean error propagation while maintaining context.

### 5. **Arc::try_unwrap for Ownership Recovery**
At the end of execution, the reactive executor recovers ownership of results:

```rust
let final_results = Arc::try_unwrap(results_mutex)
    .map_err(|_| ExecutionError::InternalError {
        message: "Failed to unwrap results Arc - multiple references still exist".into(),
    })?
    .into_inner();
```

**Why try_unwrap?** It's an optimization that avoids cloning the entire results HashMap. If all tasks have finished, we should be able to recover unique ownership. If not, it indicates a logic error (tasks still running).

## Key Architectural Insights

### **Event-Driven vs. Polling**
The reactive executor uses events (channels) instead of polling for dependency completion. This is more efficient because:
- No CPU cycles wasted on checking status
- Immediate response to dependency completion
- Natural backpressure through channel buffering

### **Canonical Payload Architecture**
Only Transform processors update the canonical payload, while Analyze processors only contribute metadata:

```rust
if processor.declared_intent() == ProcessorIntent::Transform {
    if let Some(Outcome::NextPayload(new_payload)) = &processor_response.outcome {
        let mut canonical_guard = canonical_payload_mutex.lock().await;
        *canonical_guard = new_payload.clone();
    }
}
```

**Why this separation?** It eliminates race conditions in diamond dependency patterns while maintaining clear architectural boundaries between data transformation and analysis.

### **Failure Strategy Implementation**
Different failure strategies are implemented through pattern matching:

```rust
match failure_strategy {
    FailureStrategy::FailFast => {
        cancellation_token.cancel(); // Stop all other tasks
        return Err(ExecutionError::ProcessorFailed { /* ... */ });
    }
    FailureStrategy::ContinueOnError | FailureStrategy::BestEffort => {
        // Store error but don't propagate to dependents
    }
}
```

**Why pattern matching for strategies?** It makes the different behaviors explicit and ensures all strategies are handled. The compiler enforces completeness.

This reactive executor demonstrates how Rust's ownership system, async capabilities, and type system combine to create safe, efficient, and maintainable concurrent systems.

## Summary

The Reactive Executor showcases Rust's advanced async and concurrency features for building event-driven DAG execution systems:

- **Event-Driven Architecture**: MPSC channels and `tokio::select!` enable responsive, non-blocking processor coordination
- **Safe Concurrency**: `Arc<Mutex<T>>` and cancellation tokens provide thread-safe shared state management
- **Resource Control**: Semaphores and configurable concurrency prevent system overload
- **Graceful Failure**: Comprehensive error handling with contextual information and failure strategy support
- **Performance Optimization**: Canonical payload architecture eliminates race conditions while maintaining efficiency
- **Type Safety**: Rust's ownership system prevents data races and ensures memory safety in complex async scenarios

The key innovation is the event-driven state machine approach where each processor reacts to dependency completion events rather than polling, combined with sophisticated cancellation and error propagation mechanisms. This creates a highly responsive and scalable execution model that maintains deterministic behavior while maximizing concurrent throughput.
