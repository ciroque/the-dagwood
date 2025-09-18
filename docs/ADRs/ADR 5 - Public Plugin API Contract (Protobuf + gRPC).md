# ADR 5: Public Plugin API Contract (Protobuf + gRPC)

## Context

Third-party processors must integrate with the engine regardless of implementation language or runtime (local, RPC, WASM). We need a stable, language-agnostic contract for request/response exchange and a transport for remote execution. The contract must support versioning, metadata, and a uniform error model.

## Decision

Adopt **Protocol Buffers** for schema and **gRPC** for the canonical RPC transport. Define a minimal, composable envelope:

```proto
syntax = "proto3";
package processor.v1;

message ProcessorRequest {
  bytes payload = 1;                  // opaque, engine-defined content
  map<string,string> metadata = 2;    // tracing, auth hints, schema ver
}

message ErrorDetail {                 // structured error, transport-agnostic
  int32 code = 1;                     // app-specific code
  string message = 2;                 // human-readable summary
}

message ProcessorResponse {
  oneof outcome {
    bytes next_payload = 1;           // success path
    ErrorDetail error = 2;            // failure path
  }
}

service Processor {                   // canonical remote contract
  rpc Process(ProcessorRequest) returns (ProcessorResponse);
}
```

* **Payload is opaque bytes** so WASM guests can operate without bundling a Protobuf runtime; adapters serialize/deserialize as needed.
* HTTP/JSON fallback is supported via transcoding (e.g., gRPC-Gateway) using the same messages for compatibility.
* The contract lives under `proto/processor.proto` and code is generated at build time (see build system ADRs).

## Consequences

* **Interoperability:** First-class support across Rust, Go, C/C++, .NET, Java, Python, JS/TS via standard Protobuf toolchains.
* **Stability:** Backward-compatible evolution rules apply:

  * Never reuse or renumber fields; only add optional fields.
  * Prefer additive changes; deprecate but do not remove until major version.
  * Version namespace by package (`processor.v1`, `processor.v2`) when making breaking changes.
* **Uniform errors:** All processors return `ProcessorResponse` with either `next_payload` or `error`; transport errors (timeouts) are mapped to `ErrorDetail` by adapters when possible.
* **WASM friendliness:** Engine passes/receives `bytes` to WASM entrypoints; adapters perform Protobuf (de)serialization at the host boundary.
* **Observability-ready:** `metadata` carries correlation IDs, trace/span IDs, and schema versions; exact keys are reserved/documented separately.
* **Transport choice:** gRPC is the canonical remote API; HTTP/JSON can be exposed via gateway without altering the core schema.
* **Tooling:** Recommend `buf` for linting/breaking-change checks; `prost`/`tonic` for Rust, official plugins for other languages.
* **Security:** AuthN/Z, mTLS, and signing are addressed in the deferred Security & Sandboxing ADR; this ADR defines only the message contract.
