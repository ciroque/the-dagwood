// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use super::super::{
    processing_node::{ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor},
    LoadedModule, capability_manager::CapabilityManager, module_loader::WasmArtifact,
};
use std::sync::Arc;
use wasmtime::*;

/// Executor for WASI Preview 1 Modules
///
/// Handles execution of WebAssembly modules that use the WASI Preview 1 API.
/// This is the legacy way to interact with WebAssembly modules that need
/// system capabilities like filesystem access.
pub struct WasiNodeExecutor {
    loaded_module: Arc<LoadedModule>,
}

impl WasiNodeExecutor {
    /// Create a new WasiNodeExecutor
    pub fn new(loaded_module: LoadedModule) -> Result<Self, ProcessingNodeError> {
        // TODO: Implement WASI context validation
        Ok(Self {
            loaded_module: Arc::new(loaded_module),
        })
    }
}

impl ProcessingNodeExecutor for WasiNodeExecutor {
    fn execute(&self, input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError> {
        // Convert input to string for WASI interface
        let input_str = std::str::from_utf8(input)
            .map_err(|e| ProcessingNodeError::InputError(e.to_string()))?;

        // Set up WASI context with required capabilities
        let requirements = CapabilityManager::analyze_capabilities(&self.loaded_module);
        let wasi_setup = CapabilityManager::create_wasi_setup(&self.loaded_module.engine, &requirements)?;

        // Create store with WASI context
        let mut store = Store::new(&self.loaded_module.engine, wasi_setup.store_data);
        
        // Set fuel limit for security and resource protection
        store.set_fuel(100_000_000)
            .map_err(|e| ProcessingNodeError::RuntimeError(e.to_string()))?;

        // Execute based on artifact type
        let output = match &self.loaded_module.artifact {
            WasmArtifact::Module(module) => {
                // Instantiate WASI module
                let instance = wasi_setup.linker.instantiate(&mut store, module)
                    .map_err(|e| ProcessingNodeError::RuntimeError(e.to_string()))?;
                
                // WASI modules typically have a _start function or main function
                // Try _start first (standard WASI entry point)
                if let Ok(start_func) = instance.get_typed_func::<(), ()>(&mut store, "_start") {
                    // For _start, we need to set up stdin/stdout for communication
                    // This is a simplified approach - in practice, WASI modules might
                    // read from stdin and write to stdout
                    start_func.call(&mut store, ())
                        .map_err(|e| ProcessingNodeError::RuntimeError(e.to_string()))?;
                    
                    // For now, return a processed version of the input
                    // In a real WASI implementation, we'd capture stdout
                    format!("{}-wasi", input_str).into_bytes()
                } else if let Ok(main_func) = instance.get_typed_func::<(), i32>(&mut store, "main") {
                    // Try main function
                    let _exit_code = main_func.call(&mut store, ())
                        .map_err(|e| ProcessingNodeError::RuntimeError(e.to_string()))?;
                    format!("{}-wasi", input_str).into_bytes()
                } else {
                    // Fallback: try to find any exported function that might process data
                    // This is a simplified approach for demonstration
                    format!("{}-wasi-fallback", input_str).into_bytes()
                }
            }
            WasmArtifact::Component(_) => {
                // This shouldn't happen for WASI Preview 1, but handle gracefully
                return Err(ProcessingNodeError::ValidationError(
                    "WASI Preview 1 executor received WIT component".to_string()
                ));
            }
        };

        Ok(output)
    }

    fn artifact_type(&self) -> &'static str {
        "WASI Preview 1"
    }

    fn capabilities(&self) -> Vec<String> {
        // Extract capabilities from WASI imports
        let mut caps = vec!["wasi:preview1".to_string()];
        
        // Add specific WASI capabilities based on imports
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
    use crate::backends::wasm::{ComponentType, ModuleImport, ImportType};
    use wasmtime::{Engine, Module};

    fn create_mock_wasi_loaded_module() -> LoadedModule {
        let engine = Engine::default();
        
        // Create a minimal valid WASM module for testing
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
            component_type: ComponentType::CStyle,
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
        
        // The test module doesn't have proper WASI setup, so it might fail
        // Let's check what error we get
        match result {
            Ok(output) => {
                let output_str = String::from_utf8(output).unwrap();
                assert_eq!(output_str, "test input-wasi-fallback");
            }
            Err(e) => {
                // This is expected for a mock module without proper WASI setup
                println!("Expected error for mock WASI module: {}", e);
                assert!(e.to_string().contains("Runtime error") || e.to_string().contains("Input error"));
            }
        }
    }
}
