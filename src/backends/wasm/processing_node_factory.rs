// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Processing Node Factory
//!
//! This module provides the factory for creating appropriate ProcessingNodeExecutor
//! implementations based on detected WASM artifact types. The factory handles the
//! three-way detection strategy:
//!
//! 1. **Preview 2 WIT Component** (The New Hotness) - Proper WIT components
//! 2. **Preview 1 WASI Module** (Legacy but Common) - Modules with WASI imports
//! 3. **C-Style Module** (Old Reliable) - Modules with C-style exports

use super::executors::{CStyleNodeExecutor, ComponentNodeExecutor, WasiNodeExecutor};
use crate::backends::wasm::{
    processing_node::{ProcessingNodeError, ProcessingNodeExecutor},
    ComponentType, LoadedModule, WasmComponentDetector,
};
use std::sync::Arc;

/// Factory for creating ProcessingNodeExecutor implementations
///
/// This factory implements the three-way detection strategy documented in ADR-16,
/// creating the appropriate executor based on the detected WASM artifact type.
pub struct ProcessingNodeFactory;

impl ProcessingNodeFactory {
    /// Create an appropriate ProcessingNodeExecutor for the given loaded module
    ///
    /// # Detection Strategy
    ///
    /// 1. **WIT Component**: If loaded as Component, create ComponentNodeExecutor
    /// 2. **WASI Module**: If Module with WASI imports, create WasiNodeExecutor  
    /// 3. **C-Style Module**: If Module with C-style exports, create CStyleNodeExecutor
    ///
    /// # Arguments
    ///
    /// * `loaded_module` - Pre-loaded and validated WASM module
    ///
    /// # Returns
    ///
    /// Returns an Arc-wrapped ProcessingNodeExecutor or an error if no suitable
    /// executor can be created.
    pub fn create_executor(
        loaded_module: LoadedModule,
    ) -> Result<Arc<dyn ProcessingNodeExecutor>, ProcessingNodeError> {
        let component_type = WasmComponentDetector::determine_type(&loaded_module);
        
        match component_type {
            ComponentType::WitComponent => {
                tracing::info!(
                    "Creating ComponentNodeExecutor for WIT component: {}",
                    loaded_module.module_path
                );

                let executor = ComponentNodeExecutor::new(loaded_module)?;
                Ok(Arc::new(executor))
            }
            ComponentType::WasiPreview1 => {
                let executor = WasiNodeExecutor::new(loaded_module)?;
                Ok(Arc::new(executor))
            }

            ComponentType::CStyle => {
                let executor = CStyleNodeExecutor::new(loaded_module)?;
                Ok(Arc::new(executor))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::wasm::module_loader::WasmArtifact;
    use crate::backends::wasm::{ImportType, ModuleImport};
    use wasmtime::{Engine, Module};

    fn create_mock_loaded_module(has_wasi: bool) -> LoadedModule {
        let engine = Engine::default();

        let wasm_bytes = wat::parse_str("(module)").unwrap();
        let module = Module::new(&engine, &wasm_bytes).unwrap();

        let imports = if has_wasi {
            vec![ModuleImport {
                module_name: "wasi_snapshot_preview1".to_string(),
                function_name: "proc_exit".to_string(),
                import_type: ImportType::Wasi,
            }]
        } else {
            vec![]
        };

        LoadedModule {
            engine,
            artifact: WasmArtifact::Module(module),
            imports,
            module_path: "test.wasm".to_string(),
        }
    }

    #[test]
    #[ignore = "This is for the future"]
    fn test_create_wasi_executor() {
        let loaded_module = create_mock_loaded_module(true);
        let result = ProcessingNodeFactory::create_executor(loaded_module);

        assert!(result.is_ok());
        let executor = result.unwrap();
        assert_eq!(executor.artifact_type(), "WASI Preview 1");
    }

    #[test]
    fn test_create_cstyle_executor() {
        let loaded_module = create_mock_loaded_module(false);
        let result = ProcessingNodeFactory::create_executor(loaded_module);

        assert!(result.is_ok());
        let executor = result.unwrap();
        assert_eq!(executor.artifact_type(), "C-Style");
    }
}
