# ADR 10: Security & Sandboxing (Deferred)

## Context

The engine executes third-party processors via multiple backends (local/loadable libraries, RPC, WASM). These introduce risks: arbitrary code execution, supply-chain tampering, remote endpoint impersonation, data exfiltration, resource exhaustion, and privilege escalation. A consistent security model is required but will be finalized after the core POC is validated.

## Decision

Security & Sandboxing policy is **deferred**. The POC will run in a trusted environment with minimal restrictions. Future implementation will standardize controls across backends:

* **WASM:** Use a hardened runtime (e.g., wasmtime) with instance pooling, memory caps, fuel/epoch deadlines, wall-clock timeouts, deny-by-default host calls, explicit capability exports (logging, clock, random, http-kv), and deterministic mode where feasible.
* **RPC:** Enforce mTLS between engine and processors, per-processor credentials, allowlists for endpoints, per-call deadlines, bounded payload sizes, and circuit breakers. Integrate auth (API keys/OIDC/JWT) as needed.
* **Loadable libraries:** Signed artifacts, plugin directory allowlist, restricted exported symbols, stable ABI boundary, and process isolation option (helper daemon) for untrusted plugins.
* **Local processors:** Reserved for trusted code compiled with the engine.
* **Data handling:** Explicit classification of payloads; optional encryption at rest/in transit; redaction in logs/metrics.
* **Supply chain:** SBOMs and signature verification (e.g., Cosign) for plugins, WASM modules, and containers.
* **Configuration trust:** Signed config files; checksum/attestation on load; environment variable allowlist.
* **Least privilege:** Per-processor resource policies (CPU/mem), filesystem/network sandboxing where applicable.

## Consequences

* **POC velocity:** Minimal barriers for initial development.
* **Future work:** Subsequent ADRs will fix concrete mechanisms (cert distribution, signing workflow, capability catalog, isolation strategy for loadable libs), with staged rollout.
* **Operational clarity:** Centralized policy will let operators choose trust levels per processor type and environment.
