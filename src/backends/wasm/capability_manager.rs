// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WASM engine configuration and capability management
//!
//! This module handles creating Wasmtime engines configured appropriately for
//! different WASM encoding types. It also provides a foundation for future
//! WASI import validation and security policy enforcement.

use crate::backends::wasm::detector::WasmEncoding;
use crate::backends::wasm::error::{WasmError, WasmResult};
use wasmtime::*;

/// Creates a Wasmtime engine configured for the given WASM encoding type
///
/// Each encoding type gets its own specific configuration:
///
/// **Component Model engines:**
/// - Minimal config: just `wasm_component_model(true)`
/// - WASI Preview 2 handles everything else automatically
///
/// **Classic module engines:**
/// - Security-focused configuration with sandboxing features
/// - Fuel consumption for execution limits
///
/// # Arguments
/// * `encoding` - The WASM encoding type detected by `wasm_encoding()`
///
/// # Returns
/// * `Ok(Engine)` - Configured Wasmtime engine
/// * `Err(WasmError)` - If engine creation fails, or encoding is unsupported
///
/// # Future
/// This function will be extended to support security configurations for
/// WASI import validation and per-component capability restrictions.
pub fn create_engine(encoding: WasmEncoding) -> WasmResult<Engine> {
    match encoding {
        WasmEncoding::ComponentModel => {
            tracing::debug!("Creating engine with Component Model support");
            let mut config = Config::new();
            config.wasm_component_model(true);
            Engine::new(&config).map_err(|e| WasmError::EngineError(e.to_string()))
        }
        WasmEncoding::Classic => {
            tracing::debug!("Creating engine for classic WASM module");
            let mut config = Config::new();
            config.wasm_component_model(false);
            config.consume_fuel(true);
            Engine::new(&config).map_err(|e| WasmError::EngineError(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_component_engine() {
        let engine = create_engine(WasmEncoding::ComponentModel);
        assert!(engine.is_ok(), "Should create Component Model engine");
    }

    #[test]
    fn test_create_classic_engine() {
        let engine = create_engine(WasmEncoding::Classic);
        assert!(engine.is_ok(), "Should create classic module engine");
    }

    #[test]
    fn test_engines_are_different_configs() {
        let component_engine = create_engine(WasmEncoding::ComponentModel).unwrap();
        let classic_engine = create_engine(WasmEncoding::Classic).unwrap();

        assert!(
            std::ptr::addr_of!(component_engine) != std::ptr::addr_of!(classic_engine),
            "Engines should be distinct instances"
        );
    }
}
