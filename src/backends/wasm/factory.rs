// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WASM executor factory
//!
//! This module orchestrates the creation of appropriate executor implementations
//! based on the WASM encoding type. It handles engine creation, module/component
//! instantiation, and executor wrapping.

use crate::backends::wasm::capability_manager::create_engine;
use crate::backends::wasm::detector::ComponentType;
use crate::backends::wasm::error::{WasmError, WasmResult};
use crate::backends::wasm::executors::{CStyleNodeExecutor, WitNodeExecutor};
use crate::backends::wasm::processing_node::ProcessingNodeExecutor;
use wasmtime::component::Component;
use wasmtime::Module;

/// Creates the appropriate executor based on WASM component type
///
/// This function orchestrates the complete executor creation flow:
/// 1. Creates an appropriately configured engine via `create_engine()`
/// 2. Instantiates the WASM binary as either a Module or Component
/// 3. Wraps it in the appropriate executor implementation
///
/// # Arguments
/// * `bytes` - The WASM binary bytes (from `load_wasm_bytes()`)
/// * `component_type` - The detected component type (from `detect_component_type()`)
/// * `fuel_level` - Maximum fuel (instruction count) for execution
///
/// # Returns
/// * `Ok(Box<dyn ProcessingNodeExecutor>)` - Executor ready for use
/// * `Err(WasmError)` - If engine creation, parsing, or executor creation fails
///
/// # Executor Types
/// - **Wit** → `WitNodeExecutor` (modern Component Model with WIT interface)
/// - **CStyle** → `CStyleNodeExecutor` (classic core WASM modules with C-style interface)
pub fn create_executor(
    bytes: &[u8],
    component_type: ComponentType,
    fuel_level: u64,
) -> WasmResult<Box<dyn ProcessingNodeExecutor>> {
    use crate::observability::messages::wasm::ExecutorCreated;

    let engine = create_engine(component_type)?;
    match component_type {
        ComponentType::Wit => {
            let component = Component::new(&engine, bytes).map_err(|e| {
                WasmError::ModuleError(format!("Failed to parse Component Model component: {}", e))
            })?;

            let executor = WitNodeExecutor::new(component, engine, fuel_level)?;
            tracing::info!(
                "{}",
                ExecutorCreated {
                    executor_type: "WitNodeExecutor",
                    fuel_level,
                }
            );

            Ok(Box::new(executor))
        }
        ComponentType::CStyle => {
            let module = Module::new(&engine, bytes).map_err(|e| {
                WasmError::ModuleError(format!("Failed to parse classic WASM module: {}", e))
            })?;

            let executor = CStyleNodeExecutor::new(module, engine, fuel_level)?;
            tracing::info!(
                "{}",
                ExecutorCreated {
                    executor_type: "CStyleNodeExecutor",
                    fuel_level,
                }
            );
            Ok(Box::new(executor))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_minimal_wasm_module() -> Vec<u8> {
        wat::parse_str(
            r#"
            (module
                (func (export "process") (param i32 i32) (result i32)
                    i32.const 0
                )
            )
            "#,
        )
        .unwrap()
    }

    #[test]
    fn test_create_classic_executor() {
        let wasm_bytes = create_minimal_wasm_module();
        let result = create_executor(&wasm_bytes, ComponentType::CStyle, 100_000_000);

        assert!(
            result.is_ok(),
            "Should create CStyleNodeExecutor for classic module"
        );
    }

    #[test]
    fn test_invalid_wasm_bytes() {
        let invalid_bytes = b"not a valid wasm module";
        let result = create_executor(invalid_bytes, ComponentType::CStyle, 100_000_000);

        assert!(result.is_err(), "Should fail with invalid WASM bytes");

        if let Err(WasmError::ModuleError(msg)) = result {
            assert!(
                msg.contains("Failed to parse"),
                "Error should mention parsing failure"
            );
        } else {
            panic!("Expected ModuleError");
        }
    }
}
