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
