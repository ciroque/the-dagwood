# ADR 9: Error Handling Strategy

## Context

Processors in the DAG may fail due to logic errors, resource exhaustion, network issues, or sandbox violations. The system requires a consistent error handling strategy across all execution backends (local, RPC, WASM) to ensure predictable behavior and developer ergonomics.

Key considerations:

* Whether to **short-circuit** the pipeline on error or allow unaffected subgraphs to continue.
* How to represent errors uniformly across backends.
* Retry and timeout strategies for transient errors.
* Distinguishing between **recoverable** vs. **fatal** errors.
* Propagation of error metadata for observability and debugging.

## Decision

We will adopt the following principles for error handling:

1. **Uniform representation:** All processors return a `ProcessorResponse` with either `next_payload` or an `error` (see ADR 5). RPC transport errors are mapped to the same structure where possible.
2. **Short-circuit default:** By default, any processor error terminates dependent nodes. Independent branches of the DAG may continue if not affected.
3. **Retries and timeouts:** Configurable per processor in the pipeline configuration. Defaults will be conservative (e.g., one retry, fixed timeout).
4. **Classification:** Errors will include a code and message. Codes will be categorized (e.g., `UNAVAILABLE`, `INVALID_INPUT`, `SANDBOX_VIOLATION`).
5. **Observability integration:** Error metadata is captured and exported via OpenTelemetry (see ADR 8).

## Consequences

* **Consistency:** All backends adhere to the same error model, simplifying executor logic.
* **Determinism:** Dependent nodes will never run if an upstream node fails.
* **Flexibility:** Independent subgraphs may still complete successfully, improving resilience.
* **Operational clarity:** Configurable retries and timeouts allow tuning per workload.
* **Complexity:** Requires schema and registry support for retry/timeout configuration.
* **Future extension:** Alternative policies (e.g., best-effort continuation, compensation steps) can be added later as configurable options.
