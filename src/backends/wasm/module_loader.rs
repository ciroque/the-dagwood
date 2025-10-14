// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use crate::backends::wasm::error::{WasmError, WasmResult};
use std::path::Path;
use wasmtime::component::Component;
use wasmtime::*;

const MAX_WASM_COMPONENT_SIZE: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub enum ComponentType {
    CStyle,
    WitComponent,
    WasiPreview1,
}

pub enum WasmArtifact {
    Module(Module),
    Component(Component),
}

pub struct LoadedModule {
    pub engine: Engine,
    pub artifact: WasmArtifact,
    pub imports: Vec<ModuleImport>,
    pub module_path: String,
}

#[derive(Debug, Clone)]
pub struct ModuleImport {
    pub module_name: String,
    pub function_name: String,
    pub import_type: ImportType,
}

#[derive(Debug, Clone)]
pub enum ImportType {
    Wasi,
    Other,
}

pub struct WasmModuleLoader;

impl WasmModuleLoader {
    pub fn load_module<P: AsRef<Path>>(module_path: P) -> WasmResult<LoadedModule> {
        let module_path = module_path.as_ref();
        let module_path_str = module_path.to_string_lossy().to_string();

        let engine = Self::create_engine()?;
        let module_bytes = std::fs::read(module_path).map_err(WasmError::IoError)?;

        if module_bytes.len() > MAX_WASM_COMPONENT_SIZE {
            return Err(WasmError::ValidationError(format!(
                "WASM module too large: {} bytes (max: {} bytes)",
                module_bytes.len(),
                MAX_WASM_COMPONENT_SIZE
            )));
        }

        let (artifact, imports) = match Component::new(&engine, &module_bytes) {
            Ok(component) => {
                tracing::debug!("Loaded as WIT component: {}", module_path_str);
                let imports = Self::parse_component_imports(&component)?;
                (WasmArtifact::Component(component), imports)
            }
            Err(_component_err) => {
                tracing::debug!(
                    "Failed as component, trying as core module: {}",
                    module_path_str
                );
                let module = Module::new(&engine, &module_bytes)
                    .map_err(|e| WasmError::ModuleError(e.to_string()))?;

                let imports = Self::parse_module_imports(&module)?;
                (WasmArtifact::Module(module), imports)
            }
        };

        Ok(LoadedModule {
            engine,
            artifact,
            imports,
            module_path: module_path_str,
        })
    }

    fn create_engine() -> WasmResult<Engine> {
        let mut config = Config::new();

        config.wasm_threads(false);
        config.wasm_simd(false);
        config.wasm_relaxed_simd(false);
        config.wasm_multi_memory(false);
        config.wasm_memory64(false);
        config.wasm_component_model(false);
        config.consume_fuel(true);
        config.epoch_interruption(false);

        Engine::new(&config).map_err(|e| WasmError::EngineError(e.to_string()))
    }

    fn parse_module_imports(module: &Module) -> WasmResult<Vec<ModuleImport>> {
        let mut imports = Vec::new();

        for import in module.imports() {
            let module_name = import.module().to_string();
            let function_name = import.name().to_string();

            let import_type = if module_name.starts_with("wasi") {
                Self::validate_wasi_import(&module_name, &function_name)?;
                ImportType::Wasi
            } else {
                ImportType::Other
            };

            imports.push(ModuleImport {
                module_name,
                function_name,
                import_type,
            });
        }

        Ok(imports)
    }

    fn parse_component_imports(_component: &Component) -> WasmResult<Vec<ModuleImport>> {
        tracing::debug!("WIT component imports parsing - currently returns empty (sandboxed)");
        Ok(Vec::new())
    }

    fn validate_wasi_import(module_name: &str, function_name: &str) -> WasmResult<()> {
        let allowed_wasi_functions = [
            "proc_exit",
            "random_get",
            "clock_time_get",
            "fd_write",
            "fd_read",
        ];

        if !allowed_wasi_functions.contains(&function_name) {
            return Err(WasmError::ValidationError(format!(
                "WASI function '{}' from module '{}' is not allowed. Allowed functions: {:?}",
                function_name, module_name, allowed_wasi_functions
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_engine_creation() {
        let engine = WasmModuleLoader::create_engine();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_module_size_validation() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let large_data = vec![0u8; MAX_WASM_COMPONENT_SIZE + 1];
        temp_file.write_all(&large_data).unwrap();

        let result = WasmModuleLoader::load_module(temp_file.path());
        assert!(result.is_err());

        if let Err(WasmError::ValidationError(msg)) = result {
            assert!(msg.contains("WASM module too large"));
        } else {
            panic!("Expected ValidationError for oversized module");
        }
    }

    #[test]
    fn test_wasi_import_validation() {
        let result = WasmModuleLoader::validate_wasi_import("wasi_snapshot_preview1", "proc_exit");
        assert!(result.is_ok());

        let result = WasmModuleLoader::validate_wasi_import("wasi_snapshot_preview1", "path_open");
        assert!(result.is_err());

        if let Err(WasmError::ValidationError(msg)) = result {
            assert!(msg.contains("is not allowed"));
            assert!(msg.contains("path_open"));
        } else {
            panic!("Expected ValidationError for disallowed WASI function");
        }
    }

    #[test]
    fn test_component_type_detection() {}
}
