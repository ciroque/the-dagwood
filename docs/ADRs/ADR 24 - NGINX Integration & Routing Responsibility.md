# ADR 024: NGINX Integration & Routing Responsibility

## Status
Proposed

## Context

The DAGwood daemon needs sophisticated request routing capabilities:
- **A/B testing**: Split traffic between pipeline variants (different strategies, processors)
- **Canary deployment**: Route 5% of traffic to new version, 95% to stable
- **Load balancing**: Distribute requests across multiple DAGwood instances
- **TLS termination**: Handle HTTPS encryption/decryption
- **Rate limiting**: Protect against abuse and overload
- **Authentication**: Validate JWT tokens, API keys, mTLS certificates
- **Caching**: Cache responses for idempotent operations
- **Observability**: Access logs, metrics, distributed tracing headers

We could implement these features in DAGwood, but:
- **Complexity**: Each feature is substantial engineering effort
- **Maintenance burden**: Security updates, performance tuning, bug fixes
- **Reinventing wheel**: Industry-standard tools already solve these problems
- **Operational expertise**: Teams already know how to operate NGINX, HAProxy, Envoy

**Key insight**: The project maintainer works for NGINX, bringing deep expertise in load balancing and reverse proxy patterns.

**Decision needed**: Should DAGwood implement sophisticated routing, or delegate to specialized tools?

## Decision

We will adopt a **Separation of Concerns** architecture where NGINX handles complex routing and DAGwood focuses on pipeline execution.

### Division of Responsibilities

**NGINX Handles:**
- Load balancing across DAGwood instances
- Traffic splitting for A/B testing and canary deployments
- TLS termination (HTTPS)
- Rate limiting and request throttling
- Authentication and authorization (JWT, API keys, mTLS)
- Request routing (path-based, header-based, query-param-based)
- Caching for idempotent operations
- Access logs and request metrics
- Distributed tracing header propagation

**DAGwood Handles:**
- Pipeline execution (DAG orchestration, processor execution)
- Pipeline lifecycle management (initialization, hot-reload)
- Simple name-based routing (request → pipeline name → execution)
- Pipeline-specific metrics (execution time, processor performance)
- Protocol handling (HTTP, gRPC, Unix sockets)

### Architecture Pattern

```
                    ┌─────────────────┐
                    │  NGINX          │
                    │  (Routing)      │
                    └────────┬─────────┘
                             │
          ┌──────────────────┼──────────────────┐
          │                  │                  │
    ┌─────▼─────┐      ┌─────▼─────┐     ┌─────▼─────┐
    │ DAGwood 1 │      │ DAGwood 2 │     │ DAGwood 3 │
    │ Instance  │      │ Instance  │     │ Instance  │
    └───────────┘      └───────────┘     └───────────┘
```

### A/B Testing Example

**DAGwood Config (Simple):**
```yaml
pipelines:
  - name: text_processing_workqueue
    strategy: work_queue
    processors: [uppercase, reverse]
  
  - name: text_processing_level
    strategy: level
    processors: [uppercase, reverse]
  
  - name: text_processing_reactive
    strategy: reactive
    processors: [uppercase, reverse]
```

**NGINX Config (Complex Routing):**
```nginx
upstream dagwood {
    server dagwood:8080;
}

# Split traffic by strategy using split_clients
split_clients "${remote_addr}${request_id}" $pipeline_name {
    33.3%  text_processing_workqueue;
    33.3%  text_processing_level;
    *      text_processing_reactive;
}

server {
    listen 80;
    
    location /text_processing {
        # Rewrite to specific pipeline based on split
        rewrite ^/text_processing$ /pipelines/$pipeline_name break;
        proxy_pass http://dagwood;
        
        # Add headers for observability
        proxy_set_header X-Pipeline-Name $pipeline_name;
        proxy_set_header X-Request-ID $request_id;
    }
}
```

### Canary Deployment Example

```nginx
upstream dagwood_stable {
    server dagwood-stable:8080;
}

upstream dagwood_canary {
    server dagwood-canary:8080;
}

split_clients "${remote_addr}" $upstream_name {
    5%   "canary";
    *    "stable";
}

server {
    listen 80;
    
    location /text_processing {
        if ($upstream_name = "canary") {
            proxy_pass http://dagwood_canary/pipelines/text_processing_v2;
        }
        if ($upstream_name = "stable") {
            proxy_pass http://dagwood_stable/pipelines/text_processing_v1;
        }
    }
}
```

### DAGwood API (Simple)

```
POST /pipelines/{pipeline_name}
{
  "payload": "...",
  "metadata": {...}
}

Response:
{
  "results": {...},
  "pipeline_metadata": {...},
  "execution_time_ms": 45
}
```

No routing logic in DAGwood - just execute the named pipeline.

## Alternatives Considered

### Alternative 1: DAGwood-Native Routing

**Approach**: Implement A/B testing, canary, load balancing in DAGwood

```yaml
routing:
  - path: /text_processing
    split:
      - pipeline: text_processing_workqueue
        weight: 33
      - pipeline: text_processing_level
        weight: 33
      - pipeline: text_processing_reactive
        weight: 34
```

**Pros**:
- Self-contained: No external dependencies
- Unified config: Everything in one place
- Protocol-agnostic: Works for HTTP, gRPC, Unix sockets

**Cons**:
- Reinventing wheel: NGINX already solves this
- Maintenance burden: Security updates, performance tuning
- Feature creep: Users will want more routing features (regex, conditions, etc.)
- Operational complexity: Teams must learn DAGwood-specific routing
- Limited ecosystem: No integration with existing tools (Prometheus, Grafana, etc.)

**Rejected**: Violates "do one thing well" principle. DAGwood should focus on pipeline execution, not request routing.

### Alternative 2: Service Mesh (Istio, Linkerd)

**Approach**: Deploy DAGwood in Kubernetes with service mesh

**Pros**:
- Industry standard: Well-understood patterns
- Rich features: Automatic retries, circuit breaking, observability
- Multi-protocol: Works for HTTP, gRPC
- Distributed tracing: Automatic span propagation

**Cons**:
- Kubernetes required: Not applicable for single-machine deployments
- Operational complexity: Service mesh is complex to operate
- Resource overhead: Sidecar proxies consume memory/CPU
- Overkill: Too heavy for simple deployments
- Learning curve: Steep for teams unfamiliar with service mesh

**Rejected**: Appropriate for large Kubernetes deployments, but overkill for most DAGwood use cases. Users who need service mesh can deploy DAGwood in Kubernetes.

### Alternative 3: Client-Side Routing

**Approach**: Clients choose which pipeline to call

```bash
# Client decides which strategy to test
curl -X POST http://dagwood:8080/pipelines/text_processing_workqueue
curl -X POST http://dagwood:8080/pipelines/text_processing_level
curl -X POST http://dagwood:8080/pipelines/text_processing_reactive
```

**Pros**:
- Simple: No routing logic needed
- Flexible: Clients have full control
- Transparent: Clear which pipeline is being called

**Cons**:
- Client complexity: Clients must implement routing logic
- Inconsistent: Different clients may route differently
- No centralized control: Cannot change routing without updating clients
- Poor for A/B testing: Clients must coordinate traffic split

**Rejected**: Pushes complexity to clients. Centralized routing provides better control and consistency.

### Alternative 4: API Gateway (Kong, Tyk, AWS API Gateway)

**Approach**: Use commercial API gateway for routing

**Pros**:
- Rich features: Rate limiting, auth, analytics, developer portal
- Managed service: No operational burden (for cloud gateways)
- Ecosystem: Plugins for many integrations

**Cons**:
- Cost: Commercial gateways can be expensive
- Vendor lock-in: Especially for cloud gateways
- Overkill: Too many features for simple routing
- Operational complexity: Another service to manage (for self-hosted)

**Rejected**: Appropriate for API-first companies, but overkill for most DAGwood deployments. NGINX provides sufficient routing with lower complexity.

## Consequences

### Positive

- **Leverage Expertise**: Use NGINX's battle-tested routing and load balancing
- **Operational Simplicity**: Teams already know how to operate NGINX
- **Rich Ecosystem**: Integration with Prometheus, Grafana, ELK, etc.
- **Performance**: NGINX is highly optimized for request routing
- **Security**: NGINX has mature TLS, auth, and rate limiting
- **Flexibility**: NGINX config can be updated without changing DAGwood
- **Separation of Concerns**: DAGwood focuses on pipeline execution
- **Protocol Support**: NGINX handles HTTP/HTTPS, DAGwood can focus on gRPC and Unix sockets

### Negative

- **External Dependency**: Requires NGINX for advanced routing (but optional)
- **Two Configs**: NGINX config separate from DAGwood config
- **Learning Curve**: Users must learn NGINX config (but most already know it)
- **Unix Socket Limitation**: NGINX cannot load balance Unix sockets (but that's local-only anyway)

### Neutral

- **Deployment Pattern**: NGINX + DAGwood is common pattern (not unusual)
- **Observability**: Need to combine NGINX metrics with DAGwood metrics
- **Development**: Can test DAGwood without NGINX (direct HTTP calls)

## Implementation Notes

### DAGwood Implementation

**Simple routing only:**
- `POST /pipelines/{name}` - Execute named pipeline
- No traffic splitting, no canary, no A/B testing
- Just map request to pipeline name and execute

**Multiple protocol receivers:**
- HTTP receiver on port 8080
- gRPC receiver on port 50051
- Unix socket receiver at `/var/run/dagwood.sock`

All receivers use same simple routing logic.

### NGINX Integration

**For production deployments:**
- NGINX in front of DAGwood for HTTP/gRPC
- NGINX handles routing, load balancing, TLS, auth
- DAGwood focuses on pipeline execution

**For development/testing:**
- Direct HTTP calls to DAGwood (no NGINX)
- Simple testing without routing complexity

### Protocol-Specific Considerations

**HTTP/HTTPS:**
- NGINX provides full routing, load balancing, TLS
- DAGwood provides simple HTTP receiver

**gRPC:**
- NGINX can proxy gRPC (HTTP/2 based)
- DAGwood provides gRPC receiver

**Unix Sockets:**
- Direct connection to DAGwood (no NGINX)
- Local IPC, no routing needed

## Related ADRs

- [ADR 20 - Multi-Pipeline Architecture & Registry Pattern](./ADR%2020%20-%20Multi-Pipeline%20Architecture%20&%20Registry%20Pattern.md) - Multiple pipelines enable A/B testing
- [ADR 21 - Pluggable Protocol Receiver Architecture](./ADR%2021%20-%20Pluggable%20Protocol%20Receiver%20Architecture.md) - Protocol receivers do simple routing
- [ADR 23 - Hot-Reload Strategy](./ADR%2023%20-%20Hot-Reload%20Strategy%20(Drain-and-Switch).md) - A/B testing via pipeline names + NGINX routing

## References

- [DAEMONIZATION_ROADMAP.md](../../DAEMONIZATION_ROADMAP.md) - Overall server architecture
- NGINX Documentation: https://nginx.org/en/docs/
- NGINX Load Balancing: https://docs.nginx.com/nginx/admin-guide/load-balancer/
- NGINX A/B Testing: https://www.nginx.com/blog/performing-a-b-testing-nginx-plus/
