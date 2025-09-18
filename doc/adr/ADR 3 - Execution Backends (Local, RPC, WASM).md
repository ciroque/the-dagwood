# ADR 3: Execution Backends (Local, RPC, WASM)

## Context

The system must support processors implemented in different execution environments:

* **Local (in-process)**: Compiled directly into the engine and run as native Rust code. May also include loadable shared libraries (e.g., `.so`/`.dll`) discovered from a plugins directory to allow dynamic extension without recompilation.
* **RPC (remote services)**: Accessed via gRPC, HTTP, or other network protocols. Allows scaling, delegation of heavy workloads, or cross-team ownership.
* **WASM (sandboxed)**: WebAssembly modules run in a controlled environment, allowing third-party extensions, hot-reload, and secure isolation.

Each backend type has different performance, safety, and extensibility trade-offs. A unified abstraction must make them interchangeable at runtime so that pipeline definitions do not need to be aware of execution details.

## Decision

We will support **three execution backends** from the start:

1. **Local (Rust trait objects and loadable libraries)**: for maximum performance and tight integration, with optional dynamic discovery of shared object plugins from a designated directory.
2. **RPC (gRPC/HTTP)**: for distributed workloads and polyglot extensibility.
3. **WASM (wasmtime/waPC/Extism)**: for sandboxed execution of untrusted or third-party code.

All backends will conform to the same **Processor trait** (defined in the Unified Abstraction ADR) so they can be scheduled uniformly inside DAG execution patterns.

## Consequences

* The system requires adapter implementations for each backend.
* Local processors will be the fastest but require recompilation to extend, unless distributed as loadable libraries, which introduces ABI stability concerns.
* RPC processors enable multi-language support and horizontal scaling but add network overhead and require retries/timeouts.
* WASM processors allow safe third-party plugins but require careful sandboxing (memory/time/fuel limits).
* The engine must manage resource pools: local thread pools, RPC client pools, and WASM instance pools.
* Third parties can choose their preferred backend type depending on their needs (performance, portability, or safety).
