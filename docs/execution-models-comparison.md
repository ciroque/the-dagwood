# DAG Execution Models: Comparison and Analysis

This document compares the four execution models chosen for The DAGwood project, as outlined in ADR 2. Each model represents a different approach to coordinating processor execution in directed acyclic graphs (DAGs).

## Overview of Models

| Model | Primary Use Case | Coordination Strategy | Parallelism Level |
|-------|------------------|----------------------|-------------------|
| Level-by-Level | Simple parallel stages | Topological layers | Stage-based |
| Work Queue + Dependency Counting | Dynamic, irregular DAGs | Dependency tracking | Maximum |
| Reactive/Event-Driven | Streaming, real-time | Event propagation | Event-based |
| Hybrid Scheduler + DAG | Multi-backend orchestration | Separate scheduling layer | Backend-dependent |

## 1. Level-by-Level Execution (Kahn's Algorithm)

### Concept
Organizes processors into topological levels where all processors in a level can execute in parallel, but levels execute sequentially.

### Algorithm
1. Calculate topological levels for all processors
2. Execute all processors in level 0 (no dependencies)
3. Wait for level completion before starting next level
4. Repeat until all levels complete

### Advantages
- **Simple coordination**: Clear stage boundaries
- **Predictable resource usage**: Known maximum concurrency per stage
- **Easy debugging**: Clear execution phases
- **Deterministic ordering**: Same level ordering every time

### Disadvantages
- **Suboptimal parallelism**: May wait unnecessarily between levels
- **Resource underutilization**: Can't start ready processors early
- **Rigid scheduling**: No adaptation to varying processor execution times

### Best For
- Simple pipelines with clear stages
- Resource-constrained environments
- Debugging and development
- Predictable workloads

### Example DAG Execution
```
Level 0: [A, B] (parallel)
Level 1: [C] (waits for A, B)
Level 2: [D, E] (parallel, wait for C)
```

## 2. Work Queue + Dependency Counting

### Concept
Maintains a dynamic queue of ready-to-execute processors, using dependency counting to track when processors become available.

### Algorithm
1. Initialize dependency counts for all processors
2. Queue processors with zero dependencies
3. Execute available processors concurrently (up to limit)
4. When processor completes, decrement dependent counts
5. Queue newly ready processors
6. Repeat until queue empty

### Advantages
- **Maximum parallelism**: Processors start as soon as dependencies complete
- **Dynamic adaptation**: Responds to varying execution times
- **Scalable**: Handles irregular DAG shapes efficiently
- **Optimal resource usage**: No unnecessary waiting

### Disadvantages
- **Complex coordination**: Requires careful dependency tracking
- **Non-deterministic ordering**: Execution order varies with timing
- **Memory overhead**: Maintains dependency state
- **Debugging complexity**: Harder to predict execution flow

### Best For
- Complex, irregular DAGs
- Performance-critical applications
- Variable processor execution times
- Large-scale parallel processing

### Example DAG Execution
```
Time 0: Start A, B (both ready)
Time 1: A completes, start C (now ready)
Time 2: B completes, C already running
Time 3: C completes, start D, E (both ready)
```

## 3. Reactive/Event-Driven Execution

### Concept
Processors react to completion events from their dependencies, creating a push-based execution model.

### Algorithm
1. Set up event listeners for each processor
2. Trigger entry point processors with initial input
3. When processor completes, emit completion event
4. Dependent processors react to events when all dependencies satisfied
5. Continue until no more events

### Advantages
- **Low latency**: Immediate response to completion events
- **Streaming friendly**: Natural fit for continuous data flows
- **Decoupled**: Processors don't need global coordination
- **Extensible**: Easy to add external event sources

### Disadvantages
- **Complex state management**: Event ordering and processor state
- **Potential race conditions**: Multiple events arriving simultaneously
- **Memory overhead**: Event queues and handler state
- **Debugging difficulty**: Asynchronous event flows

### Best For
- Real-time streaming pipelines
- Event-driven architectures
- Systems with external triggers
- Low-latency requirements

### Example DAG Execution
```
Event: Input arrives → trigger A, B
Event: A completes → check C dependencies
Event: B completes → trigger C (all deps satisfied)
Event: C completes → trigger D, E
```

## 4. Hybrid Scheduler + DAG

### Concept
Separates DAG dependency management from execution backend scheduling, allowing different execution strategies per backend type.

### Algorithm
1. Dependency resolver determines execution readiness
2. Scheduler routes ready processors to appropriate backends
3. Local backend uses work queue, RPC backend uses connection pooling
4. Results flow back through dependency resolver
5. Process continues until DAG completion

### Advantages
- **Backend optimization**: Each backend uses optimal execution strategy
- **Separation of concerns**: Dependency logic separate from execution
- **Flexibility**: Can mix execution strategies within single DAG
- **Scalability**: Backends can scale independently

### Disadvantages
- **Implementation complexity**: Multiple coordination layers
- **Overhead**: Additional abstraction layers
- **Debugging complexity**: Multiple execution contexts
- **Configuration complexity**: Backend-specific tuning

### Best For
- Multi-backend DAGs (local + RPC + WASM)
- Large-scale distributed processing
- Systems requiring backend-specific optimizations
- Complex orchestration scenarios

### Example DAG Execution
```
Scheduler: Route A (local) → Work Queue
Scheduler: Route B (RPC) → Connection Pool
Scheduler: Route C (WASM) → Sandbox Manager
Dependency Resolver: Coordinate results for D
```

## Performance Characteristics

### Latency Comparison
| Model | Best Case | Worst Case | Typical |
|-------|-----------|------------|---------|
| Level-by-Level | Good | Poor (unnecessary waits) | Moderate |
| Work Queue | Excellent | Good | Excellent |
| Reactive | Excellent | Moderate | Good |
| Hybrid | Variable | Variable | Backend-dependent |

### Throughput Comparison
| Model | Simple DAGs | Complex DAGs | Mixed Backends |
|-------|-------------|--------------|----------------|
| Level-by-Level | Good | Poor | Poor |
| Work Queue | Excellent | Excellent | Good |
| Reactive | Good | Good | Moderate |
| Hybrid | Good | Excellent | Excellent |

### Resource Usage
| Model | Memory | CPU | Coordination Overhead |
|-------|--------|-----|----------------------|
| Level-by-Level | Low | Moderate | Low |
| Work Queue | Moderate | High | Moderate |
| Reactive | High | Moderate | High |
| Hybrid | High | Variable | High |

## Implementation Complexity

### Development Effort
1. **Level-by-Level**: Low - straightforward topological sort
2. **Work Queue**: Moderate - dependency counting and async coordination
3. **Reactive**: High - event system and state management
4. **Hybrid**: Very High - multiple abstraction layers

### Maintenance Burden
1. **Level-by-Level**: Low - simple, predictable behavior
2. **Work Queue**: Moderate - well-understood patterns
3. **Reactive**: High - complex async debugging
4. **Hybrid**: Very High - multiple moving parts

## Decision Matrix

Choose execution model based on:

| Requirement | Recommended Model |
|-------------|-------------------|
| Simple linear pipelines | Level-by-Level |
| Maximum performance | Work Queue |
| Real-time streaming | Reactive |
| Multi-backend orchestration | Hybrid |
| Development speed | Level-by-Level |
| Debugging ease | Level-by-Level |
| Resource efficiency | Work Queue |
| Extensibility | Reactive or Hybrid |

## Conclusion

Each execution model serves different use cases:

- **Level-by-Level** provides simplicity and predictability
- **Work Queue** offers optimal performance for most scenarios
- **Reactive** enables real-time and streaming use cases
- **Hybrid** supports complex multi-backend orchestration

The pluggable architecture allows switching between models based on workload characteristics, providing flexibility to optimize for specific requirements while maintaining a consistent DAG definition interface.
