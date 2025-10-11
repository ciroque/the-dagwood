// Copyright (c) 2025 Steve Wagner (ciroque@live.com)

use std::sync::Arc;

use super::super::{
    processing_node::{ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor},
    LoadedModule,
};
use crate::backends::wasm::capability_manager::CapabilityManager;
use crate::backends::wasm::executor::WasmExecutor;

/// Executor for C-Style WASM Modules
///
/// Handles execution of WebAssembly modules that export C-style functions:
/// C-Style WASM executor that handles WebAssembly modules
/// with C-style function exports.
pub struct CStyleNodeExecutor {
    loaded_module: Arc<LoadedModule>,
}

impl CStyleNodeExecutor {
    /// Create a new CStyleNodeExecutor
    pub fn new(loaded_module: LoadedModule) -> Result<Self, ProcessingNodeError> {
        // TODO: Add validation specific to C-style modules
        Ok(Self {
            loaded_module: Arc::new(loaded_module),
        })
    }
}

impl ProcessingNodeExecutor for CStyleNodeExecutor {
    fn execute(&self, input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError> {
        // Convert input to string for C-style interface
        let input_str = std::str::from_utf8(input)
            .map_err(|e| ProcessingNodeError::InputError(e.to_string()))?;

        // Set up WASI context with required capabilities
        let requirements = CapabilityManager::analyze_capabilities(&self.loaded_module);
        let wasi_setup = CapabilityManager::create_wasi_setup(&self.loaded_module.engine, &requirements)?;

        // Execute the WASM module
        let result = WasmExecutor::execute(&self.loaded_module, wasi_setup, input_str)?;
        Ok(result.output.into_bytes())
    }

    fn artifact_type(&self) -> &'static str {
        "C-Style"
    }

    fn capabilities(&self) -> Vec<String> {
        // C-Style modules are completely sandboxed with no system access
        vec![
            "c-style".to_string(),
            "sandboxed".to_string(),
            "no-system-access".to_string(),
        ]
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
    use crate::backends::wasm::module_loader::WasmModuleLoader;
    use std::path::Path;

    #[test]
    fn test_cstyle_executor_with_wasm_appender() {
        // Path to the built wasm_appender component
        let wasm_path = Path::new("wasm_components/wasm_appender.wasm");
        
        // Skip test if the WASM file doesn't exist
        if !wasm_path.exists() {
            println!("Skipping test: wasm_appender.wasm not found. Run 'cd wasm_components/wasm_appender && ./build.sh' to build it.");
            return;
        }

        // Load the WASM module
        let loaded_module = WasmModuleLoader::load_module(wasm_path)
            .expect("Failed to load wasm_appender module");

        // Create the executor
        let executor = CStyleNodeExecutor::new(loaded_module)
            .expect("Failed to create CStyleNodeExecutor");

        // Test input
        let input = b"hello";
        
        // Execute the module (synchronously)
        let result = executor.execute(input)
            .expect("Failed to execute WASM module");

        // Convert result back to string for verification
        let output = String::from_utf8(result)
            .expect("Output is not valid UTF-8");

        // The wasm_appender should append "-wasm" to the input
        assert_eq!(output, "hello-wasm");

        // Verify metadata
        let metadata = executor.execution_metadata();
        assert_eq!(metadata.artifact_type, "C-Style");
        assert!(metadata.capabilities.contains(&"c-style".to_string()));
        assert!(metadata.capabilities.contains(&"sandboxed".to_string()));
    }
}
