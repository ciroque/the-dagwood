# DAG Execution Strategies

The DAGwood project implements multiple execution strategies, each optimized for different types of pipelines and performance characteristics. This chapter explores the algorithms, trade-offs, and use cases for each approach.

## Strategy Overview

| Strategy | Algorithm | Best For | Complexity | Parallelism |
|----------|-----------|----------|------------|-------------|
| **Work Queue** | Dependency Counting | Irregular DAGs | O(V + E) | Maximum |
| **Level-by-Level** | Topological Levels | Regular DAGs | O(V + E) | Within Levels |
| **Reactive** | Event-Driven | Real-time | O(V + E) | Event-Based |
| **Hybrid** | Adaptive | Mixed Workloads | Variable | Adaptive |

## Work Queue Strategy

### Algorithm: Kahn's Algorithm with Priority Queue

The Work Queue executor uses a sophisticated dependency counting approach:

```rust
// Simplified Work Queue algorithm
struct WorkQueueExecutor {
    priority_queue: PriorityWorkQueue,
    dependency_counts: HashMap<String, usize>,
    results: Arc<Mutex<HashMap<String, ProcessorResponse>>>,
}

impl WorkQueueExecutor {
    async fn execute(&self) -> Result<ExecutionResults, ExecutionError> {
        // 1. Initialize dependency counts
        for (processor_id, dependencies) in &dependency_graph.0 {
            self.dependency_counts.insert(processor_id.clone(), dependencies.len());
        }
        
        // 2. Queue processors with no dependencies
        for (processor_id, count) in &self.dependency_counts {
            if *count == 0 {
                self.priority_queue.push(PrioritizedTask::new(processor_id.clone()));
            }
        }
        
        // 3. Execute until queue is empty
        while let Some(task) = self.priority_queue.pop_next_available(&blocked_processors) {
            let result = self.execute_processor(task).await?;
            
            // 4. Update dependency counts for dependents
            for dependent_id in self.get_dependents(&task.processor_id) {
                self.dependency_counts[dependent_id] -= 1;
                if self.dependency_counts[dependent_id] == 0 {
                    self.priority_queue.push(PrioritizedTask::new(dependent_id));
                }
            }
        }
        
        Ok(self.results.lock().await.clone())
    }
}
```

### Key Features

#### Priority-Based Execution
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct PrioritizedTask {
    processor_id: String,
    topological_rank: usize,    // Higher rank = higher priority
    processor_intent: ProcessorIntent, // Transform > Analyze
}

impl Ord for PrioritizedTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Primary: topological rank (critical path optimization)
        match self.topological_rank.cmp(&other.topological_rank) {
            Ordering::Equal => {
                // Secondary: Transform processors before Analyze
                match (self.processor_intent, other.processor_intent) {
                    (ProcessorIntent::Transform, ProcessorIntent::Analyze) => Ordering::Greater,
                    (ProcessorIntent::Analyze, ProcessorIntent::Transform) => Ordering::Less,
                    _ => Ordering::Equal,
                }
            },
            other => other,
        }
    }
}
```

#### Canonical Payload Architecture
```rust
// Revolutionary approach: eliminates race conditions
let canonical_payload_mutex = Arc::new(Mutex::new(original_input.payload.clone()));

// All processors with dependencies receive canonical payload
let processor_input = if dependencies.is_empty() {
    original_input.clone() // Entry point
} else {
    ProcessorRequest {
        payload: canonical_payload_mutex.lock().await.clone(),
        metadata: merged_dependency_metadata,
    }
};

// Only Transform processors update canonical payload
if processor.declared_intent() == ProcessorIntent::Transform {
    let mut canonical_guard = canonical_payload_mutex.lock().await;
    *canonical_guard = processor_response.payload;
}
```

### Performance Characteristics

**Strengths**:
- **Maximum parallelism**: Executes processors as soon as dependencies complete
- **Efficient for irregular DAGs**: Handles complex dependency patterns well
- **Dynamic scheduling**: Adapts to varying processor execution times
- **Critical path optimization**: Priority queue favors processors on critical path

**Trade-offs**:
- **Memory overhead**: Priority queue and dependency counting structures
- **Complex state management**: More intricate than level-based approaches
- **Non-deterministic ordering**: Execution order varies with timing

## Level-by-Level Strategy

### Algorithm: Topological Level Computation

The Level-by-Level executor groups processors into execution levels:

```rust
// Simplified Level-by-Level algorithm
impl LevelByLevelExecutor {
    async fn execute(&self) -> Result<ExecutionResults, ExecutionError> {
        // 1. Compute topological levels
        let levels = self.compute_topological_levels(&dependency_graph)?;
        
        // 2. Execute each level sequentially
        for (level_index, level_processors) in levels.iter().enumerate() {
            println!("Executing level {}: {:?}", level_index, level_processors);
            
            // 3. Execute all processors in this level in parallel
            self.execute_level(level_processors, &input).await?;
        }
        
        Ok(self.results.lock().await.clone())
    }
    
    fn compute_topological_levels(&self, graph: &DependencyGraph) -> Result<Vec<Vec<String>>, ExecutionError> {
        let mut levels = Vec::new();
        let mut processed = HashSet::new();
        
        loop {
            // Find processors whose dependencies are all processed
            let current_level: Vec<String> = graph.0.iter()
                .filter(|(id, deps)| {
                    !processed.contains(*id) && 
                    deps.iter().all(|dep| processed.contains(dep))
                })
                .map(|(id, _)| id.clone())
                .collect();
                
            if current_level.is_empty() {
                break; // No more processors to process
            }
            
            // Mark current level as processed
            for processor_id in &current_level {
                processed.insert(processor_id.clone());
            }
            
            levels.push(current_level);
        }
        
        Ok(levels)
    }
}
```

### Key Features

#### Batch Execution Within Levels
```rust
async fn execute_level(&self, level_processors: &[String]) -> Result<(), ExecutionError> {
    let semaphore = Arc::new(Semaphore::new(self.max_concurrency));
    let mut task_handles = Vec::new();
    
    // Spawn all processors in this level
    for processor_id in level_processors {
        let permit = semaphore.clone().acquire_owned().await?;
        let task_handle = tokio::spawn(async move {
            let _permit = permit; // RAII cleanup
            self.execute_single_processor(processor_id).await
        });
        task_handles.push(task_handle);
    }
    
    // Wait for entire level to complete
    for handle in task_handles {
        handle.await??;
    }
    
    Ok(())
}
```

#### Reverse Dependencies Optimization
```rust
// O(1) dependent lookup instead of O(n) iteration
let mut dependents_map = HashMap::new();
for (processor_id, dependencies) in &graph.0 {
    for dependency_id in dependencies {
        dependents_map.entry(dependency_id.clone())
            .or_insert_with(Vec::new)
            .push(processor_id.clone());
    }
}

// Fast dependent lookup during level computation
if let Some(dependents) = dependents_map.get(&current_id) {
    for dependent_id in dependents {
        // Process dependent...
    }
}
```

### Performance Characteristics

**Strengths**:
- **Predictable execution**: Clear level boundaries and ordering
- **Efficient for regular DAGs**: Optimal for layered architectures
- **Simple state management**: Straightforward level-by-level progression
- **Good cache locality**: Processors in same level often have similar data access patterns

**Trade-offs**:
- **Limited parallelism**: Cannot execute across level boundaries
- **Level imbalance**: Uneven levels can underutilize resources
- **Synchronization overhead**: Must wait for entire level completion

## Reactive Strategy (Future Implementation)

### Algorithm: Event-Driven Execution

The Reactive executor will use an event-driven approach:

```rust
// Planned Reactive executor design
struct ReactiveExecutor {
    event_bus: Arc<EventBus>,
    processor_nodes: HashMap<String, ProcessorNode>,
}

struct ProcessorNode {
    processor: Box<dyn Processor>,
    dependencies: Vec<String>,
    dependents: Vec<String>,
    state: ProcessorState,
}

enum ProcessorState {
    Waiting { pending_dependencies: HashSet<String> },
    Ready,
    Executing,
    Completed { result: ProcessorResponse },
    Failed { error: ProcessorError },
}

impl ReactiveExecutor {
    async fn execute(&self) -> Result<ExecutionResults, ExecutionError> {
        // 1. Initialize all processors in Waiting state
        for (processor_id, node) in &self.processor_nodes {
            if node.dependencies.is_empty() {
                self.event_bus.publish(ProcessorEvent::Ready { processor_id: processor_id.clone() });
            }
        }
        
        // 2. Event loop
        while let Some(event) = self.event_bus.next_event().await {
            match event {
                ProcessorEvent::Ready { processor_id } => {
                    self.execute_processor_async(&processor_id).await?;
                },
                ProcessorEvent::Completed { processor_id, result } => {
                    self.notify_dependents(&processor_id, &result).await?;
                },
                ProcessorEvent::Failed { processor_id, error } => {
                    self.handle_processor_failure(&processor_id, &error).await?;
                },
            }
        }
        
        Ok(self.collect_results())
    }
}
```

### Planned Features

- **Real-time responsiveness**: Immediate reaction to processor completion
- **Event sourcing**: Complete audit trail of execution events
- **Dynamic reconfiguration**: Ability to modify DAG during execution
- **Backpressure handling**: Automatic flow control under load

## Hybrid Strategy (Future Implementation)

### Algorithm: Adaptive Strategy Selection

The Hybrid executor will dynamically choose strategies based on DAG characteristics:

```rust
// Planned Hybrid executor design
struct HybridExecutor {
    work_queue: WorkQueueExecutor,
    level_by_level: LevelByLevelExecutor,
    reactive: ReactiveExecutor,
}

impl HybridExecutor {
    async fn execute(&self, dag: &DependencyGraph) -> Result<ExecutionResults, ExecutionError> {
        let strategy = self.analyze_dag_characteristics(dag);
        
        match strategy {
            OptimalStrategy::WorkQueue => self.work_queue.execute(dag).await,
            OptimalStrategy::LevelByLevel => self.level_by_level.execute(dag).await,
            OptimalStrategy::Reactive => self.reactive.execute(dag).await,
            OptimalStrategy::Mixed { regions } => self.execute_mixed_strategy(regions).await,
        }
    }
    
    fn analyze_dag_characteristics(&self, dag: &DependencyGraph) -> OptimalStrategy {
        let metrics = DagMetrics::analyze(dag);
        
        match (metrics.regularity, metrics.size, metrics.parallelism_potential) {
            (High, _, _) => OptimalStrategy::LevelByLevel,
            (_, Large, High) => OptimalStrategy::WorkQueue,
            (_, _, _) if metrics.has_real_time_requirements => OptimalStrategy::Reactive,
            _ => OptimalStrategy::Mixed { regions: self.partition_dag(dag) },
        }
    }
}
```

## Strategy Selection Guide

### When to Use Work Queue

**Ideal for**:
- Irregular DAG structures
- High parallelism requirements
- Variable processor execution times
- Critical path optimization needs

**Example use cases**:
- Data processing pipelines with conditional branches
- Machine learning pipelines with dynamic dependencies
- Build systems with complex dependency graphs

### When to Use Level-by-Level

**Ideal for**:
- Regular, layered DAG structures
- Predictable execution patterns
- Resource-constrained environments
- Debugging and observability needs

**Example use cases**:
- Neural network inference pipelines
- ETL pipelines with clear stages
- Batch processing systems
- Testing and validation pipelines

### Performance Comparison

```rust
// Benchmark results (hypothetical)
struct BenchmarkResults {
    dag_type: DagType,
    work_queue_time: Duration,
    level_by_level_time: Duration,
    memory_usage_work_queue: usize,
    memory_usage_level_by_level: usize,
}

// Example results
let results = vec![
    BenchmarkResults {
        dag_type: DagType::Linear,
        work_queue_time: Duration::from_millis(100),
        level_by_level_time: Duration::from_millis(95),  // Slightly better
        memory_usage_work_queue: 1024 * 1024,
        memory_usage_level_by_level: 512 * 1024,        // Much better
    },
    BenchmarkResults {
        dag_type: DagType::Diamond,
        work_queue_time: Duration::from_millis(80),      // Much better
        level_by_level_time: Duration::from_millis(120),
        memory_usage_work_queue: 2048 * 1024,
        memory_usage_level_by_level: 1024 * 1024,
    },
];
```

## Implementation Insights

### Shared Infrastructure

Both strategies share common infrastructure:

```rust
// Common traits and structures
trait DagExecutor: Send + Sync {
    async fn execute_with_strategy(
        &self,
        processors: ProcessorRegistry,
        dependency_graph: DependencyGraph,
        entry_points: EntryPoints,
        input: ProcessorRequest,
        pipeline_metadata: PipelineMetadata,
        failure_strategy: FailureStrategy,
    ) -> Result<(HashMap<String, ProcessorResponse>, PipelineMetadata), ExecutionError>;
}

// Shared utilities
struct ExecutorUtils;
impl ExecutorUtils {
    fn validate_dag(graph: &DependencyGraph) -> Result<(), ValidationError> { /* ... */ }
    fn compute_topological_ranks(graph: &DependencyGraph) -> HashMap<String, usize> { /* ... */ }
    fn merge_metadata(responses: &[ProcessorResponse]) -> PipelineMetadata { /* ... */ }
}
```

### Configuration-Driven Selection

```yaml
# Strategy selection in configuration
strategy: work_queue  # or: level, reactive, hybrid

executor_options:
  max_concurrency: 4
  strategy_hints:
    prefer_deterministic_ordering: true
    optimize_for_memory: false
    enable_critical_path_optimization: true
```

---

> 📊 **Strategy Philosophy**: Different DAG structures benefit from different execution strategies. The DAGwood project's pluggable architecture allows you to choose the optimal approach for your specific use case, or even mix strategies within a single pipeline for maximum efficiency.
