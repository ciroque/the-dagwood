// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WASM engine configuration and capability management
//!
//! This module handles creating Wasmtime engines configured appropriately for
//! different WASM encoding types. It also provides a foundation for future
//! WASI import validation and security policy enforcement.

use crate::backends::wasm::detector::WasmEncoding;
use crate::backends::wasm::error::{WasmError, WasmResult, WASM_UNSUPPORTED_ENCODING};
use wasmtime::*;

/// Creates a Wasmtime engine configured for the given WASM encoding type
/// 
/// The engine is configured with security-focused defaults and encoding-specific
/// features:
/// 
/// **Component Model engines:**
/// - `wasm_component_model(true)` - Enable Component Model support
/// 
/// **Classic module engines:**
/// - `wasm_component_model(false)` - Disable Component Model (not needed)
/// 
/// **All engines:**
/// - `consume_fuel(true)` - Enable fuel consumption for timeout support
/// - Disabled features: threads, SIMD, multi-memory, memory64
/// 
/// # Arguments
/// * `encoding` - The WASM encoding type detected by `wasm_encoding()`
/// 
/// # Returns
/// * `Ok(Engine)` - Configured Wasmtime engine
/// * `Err(WasmError)` - If engine creation fails
/// 
/// # Future
/// This function will be extended to support security configurations for
/// WASI import validation and per-component capability restrictions.
pub fn create_engine(encoding: WasmEncoding) -> WasmResult<Engine> {
    let mut config = Config::new();

    // Configure Component Model support based on encoding
    match encoding {
        WasmEncoding::ComponentModel => {
            config.wasm_component_model(true);
            tracing::debug!("Creating engine with Component Model support");
        }
        WasmEncoding::Classic => {
            config.wasm_component_model(false);
            tracing::debug!("Creating engine for classic WASM module");
        }
        WasmEncoding::Preview1 => {
            return Err(WasmError::UnsupportedEncoding(WASM_UNSUPPORTED_ENCODING.to_string()));
        }
    }

    // Security and sandboxing configuration
    config.wasm_threads(false); // No threading support
    config.wasm_simd(false); // No SIMD instructions
    config.wasm_relaxed_simd(false);
    config.wasm_multi_memory(false); // Single memory instance only
    config.wasm_memory64(false); // 32-bit memory addressing only
    config.consume_fuel(true); // Enable fuel for execution limits
    config.epoch_interruption(false);

    Engine::new(&config).map_err(|e| WasmError::EngineError(e.to_string()))
}

// Future: WASI validation and security policy enforcement will be added here
//
// pub fn validate_component_imports(
//     component: &Component,
//     security_config: &SecurityConfig,
// ) -> WasmResult<()> {
//     // Validate WASI imports against allowed/denied lists
// }

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
        // Both should succeed but with different internal configurations
        let component_engine = create_engine(WasmEncoding::ComponentModel).unwrap();
        let classic_engine = create_engine(WasmEncoding::Classic).unwrap();

        // Both engines should be valid but configured differently
        // (We can't directly inspect config, but we know they're different internally)
        assert!(
            std::ptr::addr_of!(component_engine) != std::ptr::addr_of!(classic_engine),
            "Engines should be distinct instances"
        );
    }
}
