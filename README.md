# The DAGwood

**The DAGwood** – A pluggable, configurable pipeline engine that executes Directed Acyclic Graphs (DAGs) of processors. Supports in-process (native or loadable), RPC (gRPC/HTTP), and WASM-sandboxed plugins, all under a unified abstraction layer.

---

## Features

* **Pluggable Execution**: Run processors in-process, as loadable libraries, over RPC, or inside WASM sandboxes.
* **Unified Abstraction Layer**: One consistent trait/API across all backends.
* **Config-Driven Pipelines**: Define processors and DAGs declaratively via configuration.
* **Multiple DAG Execution Strategies**: Swap execution engines (work queue, level-by-level, reactive, hybrid).
* **Cross-Language Plugin API**: Protobuf + gRPC contract for third-party processors.
* **Future-Ready**: Hooks for observability, error handling strategies, and security sandboxing.

---

## Getting Started

### Prerequisites

* Rust (latest stable)
* Protobuf compiler (`protoc`)

### Build

```bash
cargo build
```

### Run a Sample Pipeline

```bash
cargo run --example basic_pipeline
```

### Configuration Example (YAML)

```yaml
# DAG execution strategy: choose how the pipeline runs (work_queue, level, reactive, hybrid)
strategy: work_queue

# Processor definitions: each processor declares its type, config, and dependencies
processors:
  - id: logger
    type: local
    impl: Logger

  - id: auth
    type: grpc
    endpoint: https://auth-service:50051
    dependsOn: [logger]

  - id: metrics
    type: local
    impl: MetricsCollector
    dependsOn: [logger]

  - id: audit
    type: grpc
    endpoint: https://audit-service:50052
    dependsOn: [auth, metrics]

  - id: sanitizer
    type: wasm
    module: ./plugins/sanitize.wasm
    dependsOn: [auth]
```

---

## Repository Structure

```
the-dagwood/
├── Cargo.toml               # Workspace root manifest
├── build.rs                 # Protobuf compilation entrypoint
├── proto/                   # Protobuf definitions (public plugin API)
│   └── processor.proto
├── examples/                # Small runnable demos
│   ├── basic_pipeline.yaml
│   └── basic_pipeline.rs
├── configs/                 # Sample configurations (realistic flows)
│   └── demo.yaml
├── src/
│   ├── lib.rs               # Crate entrypoint (re-exports modules)
│   │
│   ├── engine/              # Core DAG execution engine
│   │   ├── mod.rs
│   │   ├── executor.rs      # DAG executor trait + pluggable strategies
│   │   ├── work_queue.rs    # Work-queue + dependency-counted impl
│   │   ├── level.rs         # Level-by-level executor
│   │   ├── reactive.rs      # Event-driven executor
│   │   └── hybrid.rs        # Scheduler/DAG split executor
│   │
│   ├── backends/            # Execution backends (local, RPC, WASM)
│   │   ├── mod.rs
│   │   ├── local.rs         # In-process processors & loadable libs
│   │   ├── rpc.rs           # gRPC/HTTP clients
│   │   └── wasm.rs          # Wasmtime/Extism adapter
│   │
│   ├── config/              # Config & registry
│   │   ├── mod.rs
│   │   ├── loader.rs        # YAML/TOML parsing, env interpolation
│   │   ├── schema.rs        # Validation (JSON Schema / Schemars)
│   │   └── registry.rs      # ProcessorResolver (id → Processor impl)
│   │
│   ├── proto/               # Generated Rust code from Protobuf
│   │   └── processor.v1.rs
│   │
│   ├── traits/              # Unified abstractions
│   │   └── processor.rs     # Processor trait (unified interface)
│   │
│   ├── errors.rs            # Error model & classification
│   ├── observability.rs     # Logging/tracing/metrics hooks (stubbed for POC)
│   └── main.rs              # CLI entrypoint (optional for running flows)
│
├── plugins/                 # Directory for loadable libraries (dynamic .so/.dll)
├── tests/                   # Integration tests
│   ├── pipeline_end_to_end.rs
│   └── wasm_integration.rs
└── docs/
    ├── ADRs/                # Architecture Decision Records
    └── overview.md          # High-level system overview
```

---

## Roadmap

* [x] ADRs for architecture decisions
* [ ] Pluggable DAG executors
* [ ] WASM instance pool + sandboxing
* [ ] Public SDK for third-party plugins
* [ ] Observability hooks (OpenTelemetry)
* [ ] Security and sandboxing policies

---

## License

MIT
