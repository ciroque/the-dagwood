# ADR 2: DAG Execution Patterns (Pluggable Implementations)

## Context

The system must execute configurable directed acyclic graphs (DAGs) of processors. Each processor may be local (in-process), remote (RPC), or sandboxed (WASM). Different workloads will benefit from different DAG execution strategies. To support flexibility and experimentation, the execution pattern should be **pluggable** so that multiple strategies can be implemented and swapped as needed.

Candidate execution approaches:

* **Topological Sort (single order)**: too limited; essentially linear.
* **Functional Composition**: clean for pure functions, less practical with side effects and RPC.
* **Level-by-Level (Kahn’s Algorithm)**: good for exposing parallelism in broad DAGs.
* **Work Queue + Dependency Counting**: robust, scalable, supports irregular DAGs and streaming.
* **Reactive/Event-Driven**: useful if we extend toward real-time streaming pipelines.
* **Hybrid Scheduler + DAG**: separates DAG structure from execution backend, aligns with our multi-backend model.

## Decision

We will implement a **pluggable DAG execution framework** with multiple interchangeable strategies:

1. **Level-by-Level** for simple parallel execution of DAG stages.
2. **Work Queue + Dependency Counting** for dynamic, scalable execution.
3. **Reactive/Event-Driven** for future real-time streaming pipelines.
4. **Hybrid Scheduler + DAG** to decouple dependency management from execution backends.

This approach ensures the engine can handle both simple linear pipelines and complex DAGs with parallelism, dynamic workloads, and diverse execution environments.

## Consequences

* The execution engine must define a common interface for DAG runners (e.g., `DagExecutor` trait) so multiple implementations can coexist.
* Initial prototypes will likely use the **Work Queue** model for robustness.
* Swapping execution patterns will allow benchmarking and optimization per workload.
* Additional complexity: we must maintain multiple implementations and validate that they behave consistently with respect to ordering, error handling, and short-circuit logic.
* Provides long-term flexibility for advanced features (e.g., streaming, distributed scheduling).
