# ADR 14: Progressive WIT Integration Strategy

**Date**: 2025-10-09  
**Status**: Accepted  
**Context**: WASM Component Interface Evolution  
**Decision Makers**: DAGwood Core Team  

## Context

The DAGwood project has successfully implemented WASM processor support using C-style exports (`process`, `allocate`, `deallocate`). With the emergence of WebAssembly Interface Types (WIT) and the Component Model, we need to decide how to evolve our WASM component interface while maintaining production stability and security requirements.

## Problem Statement

We face a strategic decision about WASM component interface evolution:

1. **Current State**: C-style exports with manual memory management and null pointer error handling
2. **Industry Direction**: WIT and Component Model promise better type safety and developer experience
3. **Security Requirements**: Complete sandboxing with no WASI imports
4. **Production Needs**: Backward compatibility and stability for existing components

## Decision

**We will implement a Progressive WIT Integration Strategy** consisting of three phases:

### Phase 1: Enhanced C-Style Interface ✅ (Completed)
- Maintain existing C-style exports for backward compatibility
- Enhance error handling with structured error parsing
- Implement component type detection (C-style vs WIT)
- Add rich metadata collection and performance metrics

### Phase 2: WIT Specification & Infrastructure 🚧 (Current)
- Create formal WIT specification (`dagwood-processor.wit`)
- Implement semantic versioning system (`wit/versions/`)
- Build automated tooling (`setup-wit-deps.sh`, build scripts)
- Support parallel C-style and WIT component development

### Phase 3: Full Component Model Migration 🔮 (Future)
- Migrate to wasmtime's Component Model APIs when mature
- Implement rich type system (records, variants, resources)
- Enable advanced features (async processing, streaming)
- Deprecate C-style interface with clear migration timeline

## Rationale

### Why Not Full WIT Component Model Now?

#### 1. **Ecosystem Maturity Concerns**
- **wasmtime Component Model**: Functional but still evolving rapidly
- **wit-bindgen tooling**: Generates bindings but with limitations and frequent changes
- **Debugging/profiling tools**: Limited ecosystem support for Component Model debugging

#### 2. **Security Model Conflicts**
- **DAGwood requirement**: Complete sandboxing with no WASI imports
- **wit-bindgen defaults**: Often includes WASI imports for memory management
- **Component Model design**: Assumes WASI availability for richer capabilities

#### 3. **Production Risk Management**
- **Current interface**: Proven, stable, cross-language compatible
- **Migration complexity**: Would require runtime changes, tooling updates, ecosystem coordination
- **Backward compatibility**: Risk of breaking existing component authors

#### 4. **Technical Challenges**
```rust
// Current: Direct wasmtime core APIs
let process_func = instance.get_typed_func::<(i32, i32, i32), i32>(&mut store, "process")?;

// Component Model: Different instantiation and calling patterns
let component = Component::new(&engine, &component_bytes)?;
let instance = linker.instantiate(&mut store, &component)?;
// More complex binding and calling mechanisms
```

### Why Progressive Integration?

#### 1. **Risk Mitigation**
- **Incremental adoption**: Test WIT concepts without breaking production
- **Fallback capability**: C-style components continue working
- **Learning opportunity**: Gain experience with WIT before full commitment

#### 2. **Future Readiness**
- **WIT specification**: Provides formal interface documentation
- **Versioning system**: Enables ecosystem coordination
- **Migration foundation**: Clear upgrade path when Component Model matures

#### 3. **Enhanced Developer Experience**
- **Better error messages**: Structured error parsing improves debugging
- **Automated tooling**: Component setup and build automation
- **Type safety preparation**: WIT specification enables binding generation

## Implementation Details

### Current Architecture
```
DAGwood Runtime
      ↓
WasmProcessor (Enhanced)
      ↓
┌─────────────────┬─────────────────┐
│   C-Style       │   WIT-Ready     │
│   Components    │   Components    │
│   (Current)     │   (Future)      │
└─────────────────┴─────────────────┘
```

### Component Type Detection
```rust
enum WasmComponentType {
    CStyle,        // process, allocate, deallocate exports
    WitComponent,  // WIT-generated exports with # separators
}
```

### Error Handling Enhancement
```rust
// Before: Generic errors
"WASM module returned null pointer" → HTTP 500

// After: Structured error mapping
"Invalid input (400): Non-ASCII characters" → HTTP 400
"Input too large (413): 15MB exceeds limit" → HTTP 413
"Processing failed (500): Algorithm error" → HTTP 500
```

## Alternatives Considered

### Alternative 1: Immediate Full Component Model Adoption
**Rejected because:**
- High risk of breaking existing components
- Ecosystem not mature enough for production use
- Security model conflicts with WASI requirements
- Would require significant runtime architecture changes

### Alternative 2: Maintain C-Style Interface Only
**Rejected because:**
- Misses opportunity for improved developer experience
- No preparation for industry evolution toward Component Model
- Limited error handling and debugging capabilities
- Reduces long-term competitiveness

### Alternative 3: Dual Implementation (Separate Runtimes)
**Rejected because:**
- Increased maintenance burden
- Code duplication and complexity
- Confusing for component authors
- Resource overhead of maintaining two systems

## Success Criteria

### Phase 1 Success Metrics ✅
- [x] All existing C-style components continue working
- [x] Enhanced error messages with appropriate HTTP status codes
- [x] Component type detection working correctly
- [x] Rich metadata collection implemented
- [x] 10+ comprehensive tests covering all scenarios

### Phase 2 Success Metrics 🚧
- [x] WIT specification v1.0.0 released and documented
- [x] Automated component setup tooling (`setup-wit-deps.sh`)
- [x] Semantic versioning system with compatibility tracking
- [x] Build system supporting both component types
- [ ] 5+ community-contributed WIT-based components

### Phase 3 Success Metrics 🔮
- [ ] Full Component Model runtime integration
- [ ] Rich type system support (records, variants, resources)
- [ ] Advanced features (async, streaming, multi-input/output)
- [ ] Complete migration of existing components
- [ ] Deprecation of C-style interface

## Risks and Mitigations

### Risk 1: Component Model Evolution
**Risk**: WIT/Component Model standards change significantly
**Mitigation**: 
- Maintain C-style interface as stable fallback
- Version WIT specifications for backward compatibility
- Monitor WebAssembly standards development closely

### Risk 2: Security Model Conflicts
**Risk**: Component Model requires WASI imports we cannot allow
**Mitigation**:
- Continue research into WASI-free Component Model usage
- Maintain strict security validation in WasmProcessor
- Consider custom Component Model runtime if needed

### Risk 3: Ecosystem Fragmentation
**Risk**: Community splits between C-style and WIT approaches
**Mitigation**:
- Clear documentation of migration timeline
- Automated migration tools when Phase 3 begins
- Strong backward compatibility guarantees

## Monitoring and Review

### Review Schedule
- **Quarterly reviews**: Assess Component Model ecosystem maturity
- **Annual decision point**: Evaluate Phase 3 readiness
- **Community feedback**: Regular surveys of component authors

### Key Indicators for Phase 3 Transition
1. **wasmtime stability**: Component Model APIs reach 1.0 stability
2. **Security compatibility**: WASI-free Component Model usage proven
3. **Tooling maturity**: Debugging, profiling, and development tools available
4. **Community readiness**: 80%+ of component authors ready to migrate

## Conclusion

The Progressive WIT Integration Strategy balances innovation with stability, allowing DAGwood to:

1. **Maintain production reliability** with proven C-style interfaces
2. **Prepare for the future** with WIT specifications and tooling
3. **Enhance developer experience** with better error handling and automation
4. **Minimize migration risk** through incremental adoption

This approach positions DAGwood as both a stable production platform and an early adopter of emerging WebAssembly standards, ensuring long-term competitiveness while protecting existing investments.

## References

- [WebAssembly Component Model Specification](https://github.com/WebAssembly/component-model)
- [WIT (WebAssembly Interface Types)](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md)
- [wasmtime Component Model Support](https://docs.wasmtime.dev/api/wasmtime/component/index.html)
- [DAGwood WIT Specification v1.0.0](../wit/versions/v1.0.0/dagwood-processor.wit)
- [ADR 11: Parallel Execution Result Collection Strategy](ADR%2011%20-%20Parallel%20Execution%20Result%20Collection%20Strategy.md)
