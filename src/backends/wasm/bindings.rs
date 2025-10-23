// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WIT Component Model bindings for DAGwood processors
//!
//! This module contains generated bindings from the WIT interface definition.
//! The bindings are generated at compile time using wasmtime::component::bindgen!

use wasmtime::component::*;

bindgen!({
    world: "dagwood-component",
    path: "wit/versions/v1.0.0",
});
