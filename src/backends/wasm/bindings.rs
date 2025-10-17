// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WIT Component Model bindings for DAGwood processors
//!
//! This module contains generated bindings from the WIT interface definition.
//! The bindings are generated at compile time using wasmtime::component::bindgen!

use wasmtime::component::*;

// Generate bindings from our WIT file
// This creates a module with all the types and traits we need
//
// The generated structure (publicly available):
// - DagwoodComponent (world struct)
//   - instantiate(&mut store, component, linker) - creates instance
//   - DagwoodComponentProcessingNode - interface accessor  
//     - call_process(&mut store, input: &[u8]) -> Result<Vec<u8>, ProcessingError>
//
// Usage in WitNodeExecutor:
// 1. let (bindings, _) = DagwoodComponent::instantiate(&mut store, &component, &linker)?;
// 2. let output = bindings.dagwood_component_processing_node().call_process(&mut store, input)?;
bindgen!({
    world: "dagwood-component",
    path: "wit/versions/v1.0.0",
});
