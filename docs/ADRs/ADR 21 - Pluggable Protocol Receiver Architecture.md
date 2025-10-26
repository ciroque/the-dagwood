# ADR 021: Pluggable Protocol Receiver Architecture

## Status
Proposed

## Context

The DAGwood daemon needs to accept requests via multiple protocols:
- **HTTP/HTTPS**: REST API for web services and external clients
- **gRPC**: Efficient binary protocol for service-to-service communication
- **Unix Domain Sockets**: Low-latency local IPC for same-machine communication
- **Future protocols**: WebSocket, MQTT, custom binary protocols

Current challenges:
- **Hard-coded protocols**: Adding new protocols requires core code changes
- **Inconsistent patterns**: Each protocol would have different implementation approach
- **Testing difficulty**: Hard to test protocol handling in isolation
- **Configuration inflexibility**: Cannot enable/disable protocols or run multiple instances

The DAGwood project already uses pluggable patterns successfully:
- **Processors**: `Processor` trait with Local, WASM, RPC backends
- **WASM Executors**: `ProcessingNodeExecutor` trait with CStyle, WASI, Component implementations
- **DAG Executors**: `DagExecutor` trait with WorkQueue, LevelByLevel, Reactive strategies

**Decision needed**: How should we architect protocol support to maintain consistency with existing patterns while enabling extensibility?

## Decision

We will adopt a **Pluggable Protocol Receiver Architecture** using the same trait-based pattern as processors and executors.

### Core Abstraction

```rust
#[async_trait]
pub trait ProtocolReceiver: Send + Sync {
    /// Start listening for incoming requests
    async fn start(&self, router: Arc<PipelineRouter>) -> Result<(), ProtocolError>;
    
    /// Graceful shutdown
    async fn shutdown(&self) -> Result<(), ProtocolError>;
    
    /// Protocol identifier for logging/config
    fn protocol_name(&self) -> &str;
}
```

### Factory Pattern

```rust
pub struct ProtocolReceiverFactory;

impl ProtocolReceiverFactory {
    pub fn create(config: &ProtocolConfig) -> Result<Box<dyn ProtocolReceiver>, ProtocolError> {
        match config.protocol_type {
            ProtocolType::Http => Ok(Box::new(HttpProtocolReceiver::new(config)?)),
            ProtocolType::Grpc => Ok(Box::new(GrpcProtocolReceiver::new(config)?)),
            ProtocolType::UnixSocket => Ok(Box::new(UnixSocketProtocolReceiver::new(config)?)),
        }
    }
}
```

### Configuration Structure

```yaml
protocols:
  # Multiple HTTP receivers on different ports
  - type: http
    name: public
    options:
      host: "0.0.0.0"
      port: 8080
  
  - type: http
    name: admin
    options:
      host: "127.0.0.1"
      port: 8081
  
  # gRPC endpoint
  - type: grpc
    options:
      host: "0.0.0.0"
      port: 50051
      reflection: true
  
  # Unix socket for local IPC
  - type: unix_socket
    options:
      path: "/var/run/dagwood.sock"
      permissions: "0660"
```

### Server Orchestration

```rust
pub struct DagwoodServer {
    pipeline_registry: Arc<PipelineRegistry>,
    protocol_receivers: Vec<Box<dyn ProtocolReceiver>>,
}

impl DagwoodServer {
    pub async fn start(&mut self) -> Result<(), ServerError> {
        let router = Arc::new(PipelineRouter::new(self.pipeline_registry.clone()));
        
        // Start all configured protocols concurrently
        for receiver in &self.protocol_receivers {
            let router = router.clone();
            tokio::spawn(async move {
                receiver.start(router).await
            });
        }
        
        Ok(())
    }
}
```

### Key Design Principles

1. **Dependency Injection**: Protocol receivers receive `PipelineRouter` via `start()` method
2. **Single Responsibility**: Each receiver only handles protocol-specific concerns (parsing, serialization)
3. **Shared Routing**: All protocols use same `PipelineRouter` for consistent behavior
4. **Multiple Instances**: Can run multiple receivers of same type (e.g., HTTP on ports 8080 and 8081)
5. **Graceful Shutdown**: Each receiver implements `shutdown()` for clean termination

## Alternatives Considered

### Alternative 1: Enum-Based Protocol Handling

**Approach**: Single `ProtocolHandler` with enum for protocol types

```rust
enum Protocol {
    Http(HttpConfig),
    Grpc(GrpcConfig),
    UnixSocket(UnixSocketConfig),
}

struct ProtocolHandler {
    protocol: Protocol,
}
```

**Pros**:
- Simpler: No trait objects or dynamic dispatch
- Exhaustive matching: Compiler ensures all protocols handled

**Cons**:
- Not extensible: Adding protocol requires modifying core enum
- Tight coupling: All protocol code in one place
- Testing difficulty: Cannot mock individual protocols
- Violates Open/Closed Principle

**Rejected**: Lacks extensibility and violates established DAGwood patterns.

### Alternative 2: Middleware Pattern (Tower-style)

**Approach**: Composable middleware layers for protocol handling

```rust
pub trait ProtocolLayer {
    fn wrap(&self, inner: Box<dyn ProtocolLayer>) -> Box<dyn ProtocolLayer>;
}
```

**Pros**:
- Composable: Stack middleware for auth, logging, metrics
- Flexible: Can modify request/response at each layer
- Industry pattern: Used by Tower, Axum

**Cons**:
- Over-engineered: Don't need middleware composition for protocol handling
- Complex: Harder to understand and debug
- Not needed: Protocol receivers are leaf nodes, not middleware
- Inconsistent: Other DAGwood components don't use middleware pattern

**Rejected**: Adds complexity without clear benefits. Middleware is appropriate for request processing, not protocol abstraction.

### Alternative 3: Separate Binaries Per Protocol

**Approach**: `dagwood-http`, `dagwood-grpc`, `dagwood-unix` binaries

**Pros**:
- Smallest binaries: Only include dependencies for one protocol
- Independent deployment: Update HTTP without touching gRPC
- Clear separation: Each binary has single purpose

**Cons**:
- Deployment complexity: Multiple processes to manage
- Resource duplication: Each loads same pipelines, WASM modules
- Code duplication: Shared logic (routing, lifecycle) repeated
- Operational overhead: Multiple health checks, logs, configs
- User confusion: Which binary to use?

**Rejected**: Operational complexity outweighs binary size benefits. Users who need separation can use cargo features to disable protocols.

### Alternative 4: Plugin System (Dynamic Loading)

**Approach**: Load protocol handlers as dynamic libraries at runtime

**Pros**:
- Ultimate extensibility: Add protocols without recompiling
- Language-agnostic: Could write protocols in other languages

**Cons**:
- Complex: FFI, ABI stability, symbol resolution
- Unsafe: Requires unsafe Rust
- Platform-specific: Different behavior per OS
- Overkill: Can achieve extensibility with traits
- Debugging difficulty: Stack traces across FFI boundaries

**Rejected**: Complexity far exceeds benefits. Trait-based approach provides sufficient extensibility.

## Consequences

### Positive

- **Consistency**: Same pattern as Processors, WASM Executors, DAG Executors
- **Extensibility**: Add new protocols by implementing trait
- **Testability**: Mock protocol receivers for testing
- **Multiple Instances**: Run multiple receivers of same type (e.g., HTTP on different ports)
- **Single Process**: All protocols in one process, shared resources
- **Configuration-Driven**: Enable/disable protocols via config
- **Graceful Shutdown**: Each receiver can clean up independently
- **Dependency Injection**: Clear separation between protocol handling and routing

### Negative

- **Dynamic Dispatch**: Small runtime overhead from trait objects (negligible for I/O-bound operations)
- **Learning Curve**: Developers need to understand trait pattern
- **Boilerplate**: Each protocol needs factory integration

### Neutral

- **Binary Size**: All protocol dependencies included (can use cargo features to opt-out)
- **Compilation Time**: Slightly longer due to multiple protocol implementations

## Implementation Notes

### Phase 2.1: ProtocolReceiver Trait & Factory
- Define `ProtocolReceiver` trait
- Create `ProtocolReceiverFactory`
- Define `ProtocolConfig` enum and parsing

See [DAEMONIZATION_ROADMAP.md - Phase 2.1](../../DAEMONIZATION_ROADMAP.md#21-protocolreceiver-trait--factory-) for detailed implementation plan.

### Phase 2.2: HTTP Protocol Receiver
- Implement `HttpProtocolReceiver` using axum
- Basic `POST /pipelines/{name}` endpoint
- JSON request/response format

See [DAEMONIZATION_ROADMAP.md - Phase 2.2](../../DAEMONIZATION_ROADMAP.md#22-http-protocol-receiver-basic-) for detailed implementation plan.

### Phase 4.1: gRPC Protocol Receiver
- Define protobuf schema
- Implement `GrpcProtocolReceiver` using tonic

See [DAEMONIZATION_ROADMAP.md - Phase 4.1](../../DAEMONIZATION_ROADMAP.md#41-grpc-protocol-receiver-) for detailed implementation plan.

### Phase 4.2: Unix Socket Protocol Receiver
- Implement `UnixSocketProtocolReceiver`
- Handle socket file permissions and cleanup

See [DAEMONIZATION_ROADMAP.md - Phase 4.2](../../DAEMONIZATION_ROADMAP.md#42-unix-socket-protocol-receiver-) for detailed implementation plan.

## Related ADRs

- [ADR 20 - Multi-Pipeline Architecture & Registry Pattern](./ADR%2020%20-%20Multi-Pipeline%20Architecture%20&%20Registry%20Pattern.md) - Protocol receivers use PipelineRouter
- [ADR 24 - NGINX Integration & Routing Responsibility](./ADR%2024%20-%20NGINX%20Integration%20&%20Routing%20Responsibility.md) - NGINX handles complex routing, protocols handle simple parsing
- [ADR 25 - Single Binary with Subcommands](./ADR%2025%20-%20Single%20Binary%20with%20Subcommands.md) - `dagwood serve` starts protocol receivers

## References

- [DAEMONIZATION_ROADMAP.md - Phase 2.1: ProtocolReceiver Trait & Factory](../../DAEMONIZATION_ROADMAP.md#21-protocolreceiver-trait--factory-)
- [DAEMONIZATION_ROADMAP.md - Phase 2.2: HTTP Protocol Receiver](../../DAEMONIZATION_ROADMAP.md#22-http-protocol-receiver-basic-)
- [DAEMONIZATION_ROADMAP.md - Phase 4.1: gRPC Protocol Receiver](../../DAEMONIZATION_ROADMAP.md#41-grpc-protocol-receiver-)
- [DAEMONIZATION_ROADMAP.md - Phase 4.2: Unix Socket Protocol Receiver](../../DAEMONIZATION_ROADMAP.md#42-unix-socket-protocol-receiver-)
- Strategy Pattern: https://refactoring.guru/design-patterns/strategy
- Dependency Injection: https://en.wikipedia.org/wiki/Dependency_injection
