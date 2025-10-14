# WASI Capability Implementation Plan

## Overview
Transition DAGwood from "zero WASI imports" to a capability-based WASI security model, enabling modern WASM languages while maintaining security through explicit capability grants.

## Implementation Phases

### **Phase 0: Interface Clarification** 🏗️
**Goal**: Clean up terminology to avoid confusion between DAGwood processors and WASM components

**Tasks**:
- Rename WIT `interface processor` → `interface component`
- Update documentation and comments referencing the interface
- Update any code comments that reference "processor interface"
- This is a **pure naming cleanup** with no functional changes

**Benefits**:
- Clear terminology foundation for WASI work
- Less confusion for developers implementing WASM components
- Documentation alignment with WASM Component Model terminology

**Risk**: Low - pure naming change, no functional impact

---

### **Phase 1: Allow Basic WASI** 🚪
**Goal**: Remove blanket WASI rejection and allow essential WASI functions

**Tasks**:
- Modify `WasmProcessor` to allow basic WASI imports:
    - `wasi_snapshot_preview1.proc_exit`
    - `wasi_snapshot_preview1.random_get`
    - `wasi_snapshot_preview1.clock_time_get`
    - Memory management functions
- Remove the current WASI import rejection logic
- No config changes yet - just "open the door"
- Test with existing hello_world WASM module
- Test with Grain RLE component

**Benefits**:
- Unlocks Grain and other modern WASM languages
- Enables basic functionality without complex configuration
- Provides foundation for capability system

**Risk**: Medium - opens security surface, but limited to essential functions

---

### **Phase 2: Add Capability Declarations** 📋
**Goal**: Add explicit capability declarations to processor configurations

**Config Changes**:
```yaml
processors:
  - id: grain_rle_processor
    type: wasm
    module: wasm_components/rle_grain.wasm
    capabilities:
      - memory:manage     # malloc/free operations
      - random:secure     # cryptographic randomness
      - time:monotonic    # performance timing only
```

**Tasks**:
- Add `capabilities: []` field to WASM processor config schema
- Implement capability parsing and validation
- Create capability taxonomy:
    - `memory:manage` - malloc/free/realloc
    - `random:secure` - cryptographic random
    - `random:pseudo` - fast PRNG
    - `time:wall` - wall clock time
    - `time:monotonic` - performance timing
- Default to basic capabilities if not specified (backward compatibility)
- Add configuration validation and helpful error messages

**Benefits**:
- Explicit security model - clear what each processor needs
- Least privilege principle
- Auditable security configuration
- Extensible for future capabilities

**Risk**: Low - additive change with sensible defaults

---

### **Phase 3: Fine-grained Capability Enforcement** 🔒
**Goal**: Implement runtime capability checking and advanced capabilities

**Advanced Capabilities**:
```yaml
capabilities:
  - fs:read:/tmp/*                    # scoped filesystem read
  - fs:write:/workspace/output/*      # scoped filesystem write  
  - net:http:outbound                 # HTTP client requests
  - net:tcp:connect:api.example.com:443  # specific endpoint access
  - env:read:CONFIG_*                 # environment variable patterns
  - dagwood:metadata:read             # access dependency metadata
  - dagwood:config:read               # read processor configuration
  - dagwood:logging:info              # structured logging
```

**Tasks**:
- Implement WASI capability broker/interceptor
- Runtime capability checking for each WASI call
- Per-processor capability isolation
- Advanced capabilities (filesystem, network, environment)
- DAGwood-specific capabilities (metadata access, logging)
- Comprehensive testing and security audit

**Benefits**:
- Complete capability-based security model
- Support for complex WASM components
- DAGwood-native capabilities for rich integration
- Production-ready security

**Risk**: High - complex security implementation, requires thorough testing

---

## Success Criteria

### **Phase 0**:
- [ ] WIT interface renamed without breaking existing code
- [ ] Documentation updated and consistent
- [ ] All tests pass

### **Phase 1**:
- [ ] Grain RLE component loads and executes successfully
- [ ] Basic WASI functions work (memory, random, time)
- [ ] No security regressions in non-WASI components
- [ ] Performance impact < 5%

### **Phase 2**:
- [ ] Capability configuration parsing works
- [ ] Validation provides helpful error messages
- [ ] Backward compatibility maintained
- [ ] Documentation includes capability examples

### **Phase 3**:
- [ ] All capability types enforced at runtime
- [ ] Security audit passes
- [ ] Performance impact < 10%
- [ ] Complex WASM components work (file I/O, network)

---

## Testing Strategy

### **Unit Tests**:
- Capability parsing and validation
- WASI function interception
- Security boundary enforcement

### **Integration Tests**:
- Grain RLE component end-to-end
- Multi-language WASM components
- Capability violation scenarios

### **Security Tests**:
- Attempt to access denied capabilities
- Capability escalation attempts
- Resource exhaustion protection

---

## Rollback Plan

Each phase can be independently rolled back:
- **Phase 0**: Revert WIT naming changes
- **Phase 1**: Re-enable WASI import rejection
- **Phase 2**: Remove capability config parsing
- **Phase 3**: Disable capability enforcement

---

## Timeline Estimate

- **Phase 0**: 1-2 days (naming cleanup)
- **Phase 1**: 3-5 days (basic WASI support)
- **Phase 2**: 5-7 days (capability configuration)
- **Phase 3**: 10-14 days (full capability enforcement)

**Total**: 3-4 weeks for complete implementation
