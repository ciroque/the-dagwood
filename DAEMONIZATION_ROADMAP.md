# DAGwood Daemonization Roadmap

**Goal:** Transform DAGwood from a single-pipeline execution library into a multi-pipeline server/daemon that can host multiple pipelines, support multiple protocols, enable hot-reloading, and facilitate A/B testing of execution strategies.

**Status:** Design Phase Complete - Ready for Implementation

---

## Architecture Overview

### Core Concepts

1. **Multi-Pipeline Support:** Single DAGwood process hosts multiple named pipelines
2. **Pluggable Protocols:** HTTP, gRPC, Unix sockets via `ProtocolReceiver` trait
3. **Lazy Loading:** Pipelines initialize on-demand or at startup based on config
4. **Hot-Reload:** Drain-and-switch strategy for updating pipelines without downtime
5. **NGINX Integration:** Leverage NGINX for HTTP/gRPC routing, load balancing, and A/B testing
6. **Single Instance, Multiple Receivers:** Multiple protocol receivers in one process sharing pipeline registry

### Key Design Decisions

- **Binary Strategy:** Single binary with subcommands (`dagwood run`, `dagwood serve`)
- **Backward Compatibility:** Legacy single-pipeline configs automatically wrapped as "default" pipeline
- **Versioning Strategy:** Create new pipeline with different name (not version tracking within pipeline)
- **Routing:** DAGwood does simple name-based routing; NGINX handles complex routing/splitting
- **State Management:** Keep everything once loaded (Phase 1); LRU eviction in future

---

## Phase 1: Foundation (No Server Yet)

### 1.1: Pipeline Registry & Router ⏳

**Goal:** Multi-pipeline support without networking

**Tasks:**
- [ ] Create `PipelineRegistry` struct to manage multiple pipeline configs
- [ ] Create `PipelineRouter` to route requests to pipelines by name
- [ ] Update config loader to support `pipelines: []` array format
- [ ] Implement backward compatibility (wrap legacy config as "default" pipeline)
- [ ] Add config validation for pipeline name uniqueness

**Files to Create/Modify:**
- `src/server/pipeline_registry.rs` (new)
- `src/server/pipeline_router.rs` (new)
- `src/server/mod.rs` (new)
- `src/config/loader.rs` (modify)

**Tests:**
- Load multiple pipelines from config
- Route requests to correct pipeline by name
- Legacy config automatically wrapped as "default"
- Error handling for duplicate pipeline names

**Why First:** Core abstraction needed by everything else. Can test without networking.

---

### 1.2: Pipeline Lifecycle States ⏳

**Goal:** Track pipeline initialization state and support lazy loading

**Tasks:**
- [ ] Define `PipelineState` enum: `Uninitialized`, `Initializing`, `Ready`, `Failed`, `PermanentlyFailed`
- [ ] Add `startup: auto|on-demand` config field
- [ ] Implement lazy initialization on first request
- [ ] Implement request queueing during initialization
- [ ] Add concurrent initialization protection (single init per pipeline)

**Files to Create/Modify:**
- `src/server/pipeline_lifecycle.rs` (new)
- `src/server/pipeline_registry.rs` (modify)
- `src/config/loader.rs` (modify - add startup field)

**Tests:**
- `startup: auto` pipelines initialize at server start
- `startup: on-demand` pipelines initialize on first request
- Concurrent requests trigger single initialization
- Requests queue properly during initialization
- State transitions work correctly

**Why Second:** Builds on registry, still no networking. Critical for hot-reload later.

---

## Phase 2: Protocol Foundation

### 2.1: ProtocolReceiver Trait & Factory ⏳

**Goal:** Define the protocol abstraction

**Tasks:**
- [ ] Define `ProtocolReceiver` trait with `start()`, `shutdown()`, `protocol_name()` methods
- [ ] Create `ProtocolConfig` enum for different protocol types
- [ ] Create `ProtocolReceiverFactory` to instantiate receivers from config
- [ ] Define common request/response types for protocol-agnostic routing

**Files to Create/Modify:**
- `src/protocols/mod.rs` (new)
- `src/protocols/receiver.rs` (new)
- `src/protocols/factory.rs` (new)
- `src/protocols/types.rs` (new)

**Tests:**
- Factory can parse protocol configs
- Factory creates appropriate receiver types
- Mock receiver implementation for testing

**Why First in Phase 2:** Defines the contract before implementations.

---

### 2.2: HTTP Protocol Receiver (Basic) ⏳

**Goal:** Single HTTP endpoint, no advanced features

**Tasks:**
- [ ] Implement `HttpProtocolReceiver` using axum/hyper
- [ ] Create `POST /pipelines/{name}` endpoint
- [ ] Define JSON request/response format
- [ ] Integrate with `PipelineRouter`
- [ ] Add basic error handling (404 for unknown pipeline, 503 for initializing)

**Files to Create/Modify:**
- `src/protocols/http.rs` (new)
- `Cargo.toml` (add axum, hyper, tower dependencies)

**Tests:**
- HTTP request routes to correct pipeline
- JSON serialization/deserialization works
- Error responses for unknown pipelines
- 503 response during initialization

**Why Second:** Simplest protocol, proves the pattern works.

---

### 2.3: dagwood serve Subcommand ⏳

**Goal:** Start server with protocol receivers

**Tasks:**
- [ ] Add CLI subcommand parsing (use clap)
- [ ] Implement `dagwood serve --config <path>` command
- [ ] Load server config (protocols + pipelines)
- [ ] Start all configured protocol receivers concurrently
- [ ] Implement graceful shutdown on Ctrl+C (basic version)
- [ ] Add startup logging (which protocols, which pipelines)

**Files to Create/Modify:**
- `src/bin/dagwood.rs` or `src/main.rs` (modify)
- `src/cli/mod.rs` (new)
- `src/cli/serve.rs` (new)
- `Cargo.toml` (add clap dependency)

**Tests:**
- Server starts with valid config
- Multiple protocol receivers start concurrently
- Server responds to HTTP requests
- Ctrl+C triggers shutdown

**Why Third:** Brings it all together into runnable server.

---

## Phase 3: Advanced Lifecycle

### 3.1: Initialization Retry Logic ⏳

**Goal:** Handle transient failures gracefully

**Tasks:**
- [ ] Add `initialization.max_retries` and `initialization.retry_backoff` config fields
- [ ] Implement retry state tracking in pipeline lifecycle
- [ ] Implement exponential and fixed backoff strategies
- [ ] Add `PermanentlyFailed` state after max retries exceeded
- [ ] Add logging for retry attempts

**Files to Create/Modify:**
- `src/server/pipeline_lifecycle.rs` (modify)
- `src/config/loader.rs` (modify - add initialization config)

**Tests:**
- Transient failures trigger retries
- Exponential backoff timing is correct
- Permanent failure after max retries
- Successful retry resets state to Ready

**Why Separate:** Complex state machine, can be added after basic lifecycle works.

---

### 3.2: Hot-Reload (Config File Watch) ⏳

**Goal:** Reload pipelines without restart using file watching

**Tasks:**
- [ ] Add file watcher on config file (use notify crate)
- [ ] Implement config diff detection (added/removed/modified pipelines)
- [ ] Implement drain-and-switch for modified pipelines
- [ ] Implement add new pipelines at runtime
- [ ] Implement remove pipelines (drain then delete)
- [ ] Add drain timeout configuration
- [ ] Add logging for hot-reload events

**Files to Create/Modify:**
- `src/server/hot_reload.rs` (new)
- `src/server/pipeline_lifecycle.rs` (modify - add draining state)
- `Cargo.toml` (add notify dependency)

**Tests:**
- Modify pipeline config triggers reload
- In-flight requests complete on old version
- New requests use new version after reload
- Add new pipeline makes it available
- Remove pipeline drains and deletes

**Why Separate:** Complex feature, depends on lifecycle states. File watch is simplest trigger.

---

### 3.3: Hot-Reload (Admin API) ⏳

**Goal:** Manual reload trigger via HTTP endpoints

**Tasks:**
- [ ] Add `POST /admin/reload` endpoint (reload all pipelines)
- [ ] Add `POST /admin/pipelines/{name}/reload` endpoint (reload specific pipeline)
- [ ] Add `POST /admin/pipelines/{name}/reset` endpoint (reset failed pipeline)
- [ ] Add authentication/authorization for admin endpoints
- [ ] Add admin endpoint documentation

**Files to Create/Modify:**
- `src/protocols/http.rs` (modify - add admin routes)
- `src/server/hot_reload.rs` (modify - add manual trigger support)

**Tests:**
- Admin endpoints trigger reload correctly
- Authentication prevents unauthorized access
- Reset endpoint clears failed state

**Why Separate:** Alternative trigger mechanism, can be added after file watch works.

---

## Phase 4: Additional Protocols

### 4.1: gRPC Protocol Receiver ⏳

**Goal:** gRPC support for service-to-service communication

**Tasks:**
- [ ] Define protobuf schema for pipeline requests/responses
- [ ] Generate Rust code from protobuf (use tonic)
- [ ] Implement `GrpcProtocolReceiver`
- [ ] Integrate with `PipelineRouter`
- [ ] Add gRPC-specific error handling

**Files to Create/Modify:**
- `proto/dagwood_server.proto` (new)
- `src/protocols/grpc.rs` (new)
- `build.rs` (modify - add protobuf compilation)
- `Cargo.toml` (add tonic, prost dependencies)

**Tests:**
- gRPC request routes to correct pipeline
- Protobuf serialization works
- gRPC error responses

**Why Separate:** Independent protocol, follows same pattern as HTTP.

---

### 4.2: Unix Socket Protocol Receiver ⏳

**Goal:** Local IPC support via Unix domain sockets

**Tasks:**
- [ ] Implement `UnixSocketProtocolReceiver`
- [ ] Add socket file path configuration
- [ ] Implement permission handling for socket file
- [ ] Define wire protocol for Unix socket (JSON? Binary?)
- [ ] Add cleanup on shutdown (remove socket file)

**Files to Create/Modify:**
- `src/protocols/unix_socket.rs` (new)
- `Cargo.toml` (add tokio-uds or similar)

**Tests:**
- Unix socket accepts connections
- Request/response over socket works
- Socket file created with correct permissions
- Socket file cleaned up on shutdown

**Why Separate:** Independent protocol, different use case (local IPC).

---

## Phase 5: Observability & Operations

### 5.1: Health Check Endpoint ⏳

**Goal:** Expose server and pipeline health status

**Tasks:**
- [ ] Add `GET /health` endpoint
- [ ] Return per-pipeline status (ready, initializing, failed, etc.)
- [ ] Return server-level health status
- [ ] Add configuration for health check behavior (all pipelines must be ready?)
- [ ] Support different health check formats (simple, detailed)

**Files to Create/Modify:**
- `src/protocols/http.rs` (modify - add health route)
- `src/server/health.rs` (new)

**Tests:**
- Health endpoint returns correct status
- Server health reflects pipeline states
- Health check format options work

**Why Separate:** Operational feature, not core functionality.

---

### 5.2: Metrics Endpoint ⏳

**Goal:** Expose pipeline execution metrics

**Tasks:**
- [ ] Add `GET /metrics` endpoint (Prometheus format)
- [ ] Track per-pipeline request count, latency, error rate
- [ ] Track server-level metrics (total requests, active pipelines)
- [ ] Add metrics for pipeline lifecycle events
- [ ] Consider using prometheus crate for metric collection

**Files to Create/Modify:**
- `src/protocols/http.rs` (modify - add metrics route)
- `src/server/metrics.rs` (new)
- `Cargo.toml` (add prometheus dependency)

**Tests:**
- Metrics endpoint returns Prometheus format
- Metrics reflect actual execution
- Per-pipeline metrics are isolated

**Why Separate:** Observability feature, builds on existing execution.

---

### 5.3: Structured Logging ⏳

**Goal:** Replace println!/eprintln! with proper logging framework

**Tasks:**
- [ ] Integrate tracing crate for structured logging
- [ ] Add log events for pipeline lifecycle (initialization, ready, failed)
- [ ] Add log events for request handling (start, complete, error)
- [ ] Add log events for hot-reload operations
- [ ] Configure log levels and filtering
- [ ] Add context (pipeline name, request ID) to logs

**Files to Create/Modify:**
- Multiple files across codebase
- `Cargo.toml` (add tracing, tracing-subscriber dependencies)

**Tests:**
- Logs are structured and parseable
- Log levels work correctly
- Context is included in log events

**Why Separate:** Cross-cutting concern, can be added incrementally.

---

## Phase 6: Production Hardening

### 6.1: Graceful Shutdown ⏳

**Goal:** Clean shutdown without killing in-flight requests

**Tasks:**
- [ ] Implement signal handling (SIGTERM, SIGINT)
- [ ] Stop accepting new requests on shutdown signal
- [ ] Wait for in-flight requests to complete
- [ ] Add configurable shutdown timeout
- [ ] Force-kill after timeout expires
- [ ] Add shutdown logging

**Files to Create/Modify:**
- `src/server/shutdown.rs` (new)
- `src/cli/serve.rs` (modify - integrate shutdown handler)

**Tests:**
- Shutdown during execution waits for completion
- Timeout forces shutdown after configured duration
- New requests rejected during shutdown

**Why Separate:** Production requirement, complex coordination.

---

### 6.2: Memory Management (LRU Eviction) ⏳

**Goal:** Evict unused pipelines under memory pressure

**Tasks:**
- [ ] Track last-used timestamp per pipeline
- [ ] Implement LRU eviction policy
- [ ] Add memory pressure detection
- [ ] Implement re-initialization on next request after eviction
- [ ] Add protection for `startup: auto` pipelines (never evict)
- [ ] Add configuration for eviction policy

**Files to Create/Modify:**
- `src/server/pipeline_lifecycle.rs` (modify)
- `src/server/eviction.rs` (new)

**Tests:**
- LRU eviction works correctly
- Evicted pipelines re-initialize on next request
- Protected pipelines never evicted
- Memory pressure triggers eviction

**Why Separate:** Future optimization, not needed for initial release.

---

## Suggested Implementation Order

### Minimum Viable Server (MVP)
1. ✅ Phase 1.1: Pipeline Registry & Router
2. ✅ Phase 1.2: Pipeline Lifecycle States
3. ✅ Phase 2.1: ProtocolReceiver Trait
4. ✅ Phase 2.2: HTTP Protocol Receiver
5. ✅ Phase 2.3: dagwood serve Subcommand

**Milestone:** Multi-pipeline HTTP server with lazy loading

### Hot-Reload Support
6. ✅ Phase 3.2: Hot-Reload (File Watch)

**Milestone:** Server can reload pipelines without restart

### Additional Protocols
7. ✅ Phase 4.1: gRPC Protocol Receiver
8. ✅ Phase 4.2: Unix Socket Protocol Receiver

**Milestone:** Multi-protocol support (HTTP, gRPC, Unix sockets)

### Operations & Observability
9. ✅ Phase 5.1: Health Check Endpoint
10. ✅ Phase 5.2: Metrics Endpoint

**Milestone:** Production-ready observability

### Hardening
11. ✅ Phase 3.1: Initialization Retry Logic
12. ✅ Phase 3.3: Hot-Reload (Admin API)
13. ✅ Phase 6.1: Graceful Shutdown

**Milestone:** Production-hardened server

### Future Enhancements
14. ⏳ Phase 5.3: Structured Logging (ongoing)
15. ⏳ Phase 6.2: Memory Management (optimization)

---

## Parallel Work Opportunities

**Can work in parallel:**
- Phase 2.2 (HTTP) + Phase 4.1 (gRPC) + Phase 4.2 (Unix) - all follow same pattern
- Phase 5.1 (Health) + Phase 5.2 (Metrics) - both observability endpoints
- Phase 3.2 (File Watch) + Phase 3.3 (Admin API) - different reload triggers

**Must be sequential:**
- Phase 1.1 → 1.2 → 2.1 → 2.2 → 2.3 (foundation dependencies)
- Phase 3.1 depends on 1.2 (lifecycle states)
- Phase 3.2/3.3 depend on 1.2 (lifecycle states)
- Phase 6.1 depends on 2.3 (server running)

---

## Configuration Examples

### Legacy Format (Backward Compatible)
```yaml
strategy: work_queue
failure_strategy: fail_fast
executor_options:
  max_concurrency: 4
processors:
  - id: uppercase
    backend: local
    impl: change_text_case_upper
```

### New Server Format
```yaml
protocols:
  - type: http
    name: public
    options:
      host: "0.0.0.0"
      port: 8080
  
  - type: grpc
    options:
      host: "0.0.0.0"
      port: 50051
  
  - type: unix_socket
    options:
      path: "/var/run/dagwood.sock"

pipelines:
  - name: text_processing_workqueue
    startup: auto
    strategy: work_queue
    failure_strategy: fail_fast
    initialization:
      max_retries: 3
      retry_backoff: exponential
    executor_options:
      max_concurrency: 4
    processors:
      - id: uppercase
        backend: local
        impl: change_text_case_upper
  
  - name: text_processing_level
    startup: on-demand
    strategy: level
    processors:
      - id: uppercase
        backend: local
        impl: change_text_case_upper
```

---

## NGINX Integration Example

```nginx
upstream dagwood {
    server dagwood:8080;
}

# A/B testing different execution strategies
split_clients "${remote_addr}${request_id}" $pipeline_name {
    33.3%  text_processing_workqueue;
    33.3%  text_processing_level;
    *      text_processing_reactive;
}

server {
    listen 80;
    
    location /text_processing {
        rewrite ^/text_processing$ /pipelines/$pipeline_name break;
        proxy_pass http://dagwood;
        proxy_set_header X-Pipeline-Name $pipeline_name;
        proxy_set_header X-Request-ID $request_id;
    }
}
```

---

## Open Questions / Decisions Needed

1. **Queued request timeouts** - How long do requests wait during initialization?
2. **Queue depth limits** - Prevent memory exhaustion if initialization hangs?
3. **Protocol-specific queueing behavior** - HTTP 503 vs gRPC streaming vs Unix socket blocking?
4. **Retry reset logic** - Reset retry count after successful initialization?
5. **Manual pipeline reset** - Admin API to reset permanently failed pipelines?
6. **Retry timing** - Immediate on next request or background with backoff?
7. **Drain timeout** - Force-kill in-flight requests after timeout?
8. **Hot-reload failure handling** - What if new version fails to initialize during drain?
9. **Concurrent hot-reload** - What if another reload triggered while draining?
10. **Memory limits** - Fail to initialize new pipeline if would exceed limit?
11. **Default startup mode** - `auto` or `on-demand`?
12. **Health check semantics** - Server healthy if all `startup: auto` pipelines ready?
13. **Config reload trigger** - File watch, admin API, signal, or all three?
14. **Pipeline access control** - Can protocols restrict which pipelines they expose?
15. **Graceful shutdown timeout** - Default value? Configurable per pipeline?

---

## Related Documentation

- See `/tmp/dagwood-server-architecture-discussion.md` for detailed design discussion
- See `ROADMAP.md` for overall project roadmap
- See `docs/ADRs/` for architectural decision records

---

## Legend

- ⏳ Not Started
- 🚧 In Progress
- ✅ Complete
- ❌ Blocked
- 🔄 Under Review
