use super::super::{
    processing_node::{ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor},
    LoadedModule,
};
use std::sync::Arc;
use wasmtime::*;

pub struct WasiNodeExecutor {
    loaded_module: Arc<LoadedModule>,
}

impl WasiNodeExecutor {
    pub fn new(loaded_module: LoadedModule) -> Result<Self, ProcessingNodeError> {
        Ok(Self {
            loaded_module: Arc::new(loaded_module),
        })
    }
}

impl ProcessingNodeExecutor for WasiNodeExecutor {
    fn execute(&self, _input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError> {
        Ok(Vec::new())
    }

    fn artifact_type(&self) -> &'static str {
        "WASI Preview 1"
    }

    fn capabilities(&self) -> Vec<String> {
        let mut caps = vec!["wasi:preview1".to_string()];

        for import in &self.loaded_module.imports {
            if import.module_name == "wasi_snapshot_preview1" {
                if !caps.contains(&import.function_name) {
                    caps.push(format!("wasi:{}", import.function_name));
                }
            }
        }

        caps
    }

    fn execution_metadata(&self) -> ExecutionMetadata {
        ExecutionMetadata {
            module_path: self.loaded_module.module_path.clone(),
            artifact_type: self.artifact_type().to_string(),
            import_count: self.loaded_module.imports.len(),
            capabilities: self.capabilities(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::wasm::{ImportType, ModuleImport};
    use wasmtime::{Engine, Module};

    fn create_mock_wasi_loaded_module() -> LoadedModule {
        let engine = Engine::default();

        let wasm_bytes = wat::parse_str("(module)").unwrap();
        let module = Module::new(&engine, &wasm_bytes).unwrap();

        let imports = vec![ModuleImport {
            module_name: "wasi_snapshot_preview1".to_string(),
            function_name: "proc_exit".to_string(),
            import_type: ImportType::Wasi,
        }];

        LoadedModule {
            engine,
            artifact: crate::backends::wasm::module_loader::WasmArtifact::Module(module),
            imports,
            module_path: "test_wasi.wasm".to_string(),
        }
    }

    #[test]
    fn test_wasi_executor_creation() {
        let loaded_module = create_mock_wasi_loaded_module();
        let result = WasiNodeExecutor::new(loaded_module);

        assert!(result.is_ok());
        let executor = result.unwrap();
        assert_eq!(executor.artifact_type(), "WASI Preview 1");

        let capabilities = executor.capabilities();
        assert!(capabilities.contains(&"wasi:preview1".to_string()));
        assert!(capabilities.contains(&"wasi:proc_exit".to_string()));
    }

    #[test]
    fn test_wasi_executor_fallback_execution() {
        let loaded_module = create_mock_wasi_loaded_module();
        let executor = WasiNodeExecutor::new(loaded_module).unwrap();

        let input = b"test input";
        let result = executor.execute(input);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output, Vec::<u8>::new());
    }
}
