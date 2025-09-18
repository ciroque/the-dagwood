# ADR 4: Unified Abstraction Layer

## Context

Processors may be executed in different environments: in-process (native Rust or loadable libraries), remote via RPC (gRPC/HTTP), or sandboxed in WASM. To ensure configurability and composability, the system requires a **unified abstraction** that normalizes the interaction with these processors.

This abstraction must:

* Represent inputs and outputs consistently, regardless of backend.
* Support asynchronous execution.
* Allow error propagation in a uniform format.
* Be extensible for additional execution backends in the future.
* Integrate cleanly with DAG execution patterns (pluggable strategies).

## Decision

We will define a **Processor trait** as the unified abstraction layer:

```rust
#[async_trait::async_trait]
pub trait Processor: Send + Sync {
    async fn process(&self, req: ProcessorRequest) -> ProcessorResponse;
    fn name(&self) -> &'static str;
}
```

* **`ProcessorRequest`** and **`ProcessorResponse`** are generated from Protobuf definitions, ensuring cross-language compatibility.
* Each backend (local, RPC, WASM) will provide its own adapter that implements this trait.
* DAG executors will only depend on this trait, not backend-specific details.

## Consequences

* All processors become interchangeable, regardless of execution backend.
* Enables configuration-driven pipelines where processors can be swapped or reordered without code changes.
* Simplifies DAG executor implementations by enforcing a single contract.
* Adds an indirection layer: adapters must translate between `ProcessorRequest`/`ProcessorResponse` and backend-native representations.
* Third-party developers targeting the Protobuf API can create processors in any language, and the system can wrap them in a conforming adapter.
* Future backends (e.g., GPU, TPU, cloud functions) can be added by implementing the `Processor` trait.
