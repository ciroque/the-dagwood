// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use super::executors::{CStyleNodeExecutor, ComponentNodeExecutor, WasiNodeExecutor};
use crate::backends::wasm::{
    processing_node::{ProcessingNodeError, ProcessingNodeExecutor},
    ComponentType, LoadedModule, WasmComponentDetector,
};
use std::sync::Arc;

pub struct ProcessingNodeFactory;

impl ProcessingNodeFactory {
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
