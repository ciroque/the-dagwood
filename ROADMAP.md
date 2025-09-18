# The DAGwood Roadmap

This roadmap outlines a **hybrid iteration approach** for building The DAGwood: starting with top-down abstractions to maintain clarity, while implementing bottom-up demos for quick validation.

---

## Phase 1: Foundations (Top-down stubs)

* [X] Define the **`Processor` trait** (Unified Abstraction Layer).
* [X] Define the **`DagExecutor` trait** (pluggable execution strategies).
* [X] Implement config parsing into strongly typed structs.
* [X] Build the **registry** that resolves processors from config into runtime instances.
* [X] Validate dependency graphs (acyclic, all references resolved).

---

## Phase 2: First Demo (Bottom-up prototype)

* [ ] Implement the **Local backend** with hard-coded processors (see table below).
* [ ] Implement the **Work Queue executor** (dependency-counted).
* [ ] Run a trivial pipeline: `change text case -> reverse text`.
* [ ] Add basic error handling (short-circuit on failure).

The following processors will be completed for this phase:

| Processor                | Description                          | Recommended Type | Why                                                                |
| ------------------------ | ------------------------------------ | ---------------- | ------------------------------------------------------------------ |
| Change Text Case         | Upper/lower/proper/title casing      | **Local**        | Trivial string transform, great for chaining.                      |
| Token Counter            | Count chars/words                    | **Local**        | Simple, fast, no external deps.                                    |
| Word Frequency Analyzer  | Histogram of words                   | **Local**        | Easy to implement, nice “non-trivial but local” example.           |
| Reverse Text             | Reverse input string                 | **Local**        | Dead simple sanity check processor.                                |
| Prefix/Suffix Adder      | Add prefix/suffix around string      | **Local**        | Shows off config-driven behavior.                                  |


---

## Phase 3: Expand Backends

* [ ] Add **WASM adapter** (wasmtime/Extism).
* [ ] Add **RPC adapter** (gRPC client via tonic).
* [ ] Add support for loadable shared libraries in the Local backend.
* [ ] Support configurable payload size limits.

The following processors will be completed for this phase:

| Processor                | Description                          | Recommended Type | Why                                                                |
| ------------------------ | ------------------------------------ | ---------------- | ------------------------------------------------------------------ |
| Regex Replacer           | Replace text via regex pattern       | **Local**        | Demonstrates use of external crate (`regex`), still simple.        |
| JSON Path Extractor      | Extract fields from JSON             | **Local**        | Structured parsing demo, good to show config-driven selects.       |
| Sentiment Analyzer (toy) | Naive positive/negative word check   | **gRPC**         | Good to show off remote service integration (can be in Go/Python). |
| Language Detector (toy)  | Detect language by stopwords/charset | **HTTP**         | Another external API style demo, e.g., wrap a tiny HTTP service.   |
| WASM Hello World         | Append “-wasm” to string             | **WASM**         | Proves WASM sandbox execution, even with trivial logic.            |

---

## Phase 4: Execution Strategies

* [ ] Implement **Level-by-Level executor** (Kahn’s algorithm).
* [ ] Implement **Reactive/Event-Driven executor**.
* [ ] Implement **Hybrid Scheduler** (decouple DAG resolution from backend scheduling).
* [ ] Add `strategy:` option in config to select executor.

---

## Phase 5: Operational Features

* [ ] Enhance **error handling** (timeouts, retries, error classification).
* [ ] Add **observability hooks** (OpenTelemetry spans, metrics).
* [ ] Extend config with per-processor options (retries, resource limits).
* [ ] Add CI/CD pipeline with linting, formatting, and integration tests.

---

## Phase 6: Future Enhancements

* [ ] Persistence of DAG definitions (DB or service API).
* [ ] Signed configs and plugins (security hardening).
* [ ] Advanced WASM policies (capabilities, deterministic mode).
* [ ] SDKs for other languages to simplify third-party processor development.

---

This hybrid approach ensures steady progress: **clear abstractions first, quick demos second, expanding capabilities third**.
