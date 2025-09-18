# ADR 6: Configuration and Registry Mechanism

## Context

Pipelines must be assembled from a configuration that declares processors, their execution backends (local/loadable, RPC, WASM), and their dependencies with deterministic ordering. We also need a runtime registry that resolves configured processors to concrete implementations (constructors, remote endpoints, or WASM modules). The solution must be simple enough for the POC but support future growth (validation, discovery, hot-reload).

## Decision

* **Config format:** YAML (human-friendly), parsed via Serde. A JSON equivalent is supported implicitly.
* **Schema:** Provide a JSON Schema for static validation; treat config as versioned (`configVersion`).
* **Model:**

  * `processors[]`: `{ id, type: [local|loadable|grpc|http|wasm], impl|path|endpoint, options{}, dependsOn[] }`
* **Dependencies:** Each processor may declare `dependsOn: [id, id...]` listing upstream processors. The engine validates acyclicity and determinism. Execution order is resolved via topological sort before submission to the chosen DAG executor.
* **Registry:** A pluggable `ProcessorResolver` resolves entries to `Arc<dyn Processor>` using backend-specific adapters:

  * **local:** constructor registry (name → factory fn)
  * **loadable:** shared library loader (plugins dir)
  * **grpc/http:** client factory with connection pooling
  * **wasm:** module loader with precompiled cache + instance pool
* **Validation:** On load, validate schema, uniqueness of `id`, existence of targets, and absence of cycles.
* **Environment:** Support `${ENV_VAR}` interpolation in config for endpoints/paths.

## Consequences

* **Determinism:** Explicit `dependsOn` arrays ensure stable execution order.
* **Extensibility:** New backends add a resolver; config remains stable.
* **Safety:** Invalid configs fail fast with actionable errors (schema + graph checks).
* **Operational:** Enables per-node options (timeouts, retries, concurrency limits) without changing code.
* **Clarity:** Maintainers can easily see per-node dependencies without parsing complex edge lists.
* **Future work (out of scope for POC):** hot-reload with diffing, remote config sources, signed configs, multi-file includes/overlays.

