# ADR 1: Language Choice (Rust)

## Context

We need a systems programming language to implement a configurable pipeline engine that executes directed acyclic graphs (DAGs) of processors. These processors may run in-process, across RPC (gRPC/HTTP/sockets), or inside WebAssembly (WASM) sandboxes.

The language must provide:

* Strong performance for both CPU-bound and I/O-bound workloads.
* Safe concurrency to support parallel execution of independent DAG nodes.
* Mature async ecosystem for networked backends (gRPC, HTTP).
* Good integration with WASM runtimes.
* Support for Protobuf and gRPC for defining a public plugin API.
* A foundation for long-term maintainability and third-party extensibility.

Alternatives considered:

* **Go**: great RPC story, easy plugin loading, but weaker in WASM and lower-level control.
* **C++**: excellent performance, but unsafe by default and slower developer velocity.
* **Elixir/Erlang**: strong for fault-tolerance and supervision, but not as suitable for embedding WASM or providing a performant, type-safe SDK for third parties.
* **.NET**: good Protobuf and async support, but less portable for embedding in diverse environments.

## Decision

We will implement the pipeline engine in **Rust**. Rust provides strong memory safety without garbage collection, high performance, an expressive type system, and a thriving async ecosystem (`tokio`, `hyper`, `tonic`). Rust also has first-class support for Protobuf via `prost` and excellent WASM integration through `wasmtime` and `wasmer`.

Rust’s ecosystem maturity, combined with its focus on safety and performance, makes it the most appropriate choice for building a pluggable, extensible execution engine.

## Consequences

* We gain strong guarantees on safety, performance, and concurrency.
* Protobuf and gRPC integration is straightforward using `prost` and `tonic`.
* WASM sandboxing can leverage `wasmtime` or `extism` with minimal friction.
* Third-party SDKs in other languages can still target the Protobuf/gRPC contract, even though the core is implemented in Rust.
* Rust’s learning curve and compile times may slow initial iteration speed, but the long-term benefits in safety and performance outweigh this cost.
