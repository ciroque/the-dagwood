// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use super::super::{
    bindings::DagwoodComponent,
    processing_node::{ComponentExecutionError, ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor},
    LoadedModule, WasmArtifact,
};
use std::sync::Arc;
use wasmtime::component::Linker;
use wasmtime::Store;

pub struct WitNodeExecutor {
    loaded_module: Arc<LoadedModule>,
}

impl WitNodeExecutor {
    pub fn new(loaded_module: LoadedModule) -> Result<Self, ProcessingNodeError> {
        Ok(Self {
            loaded_module: Arc::new(loaded_module),
        })
    }
}

impl ProcessingNodeExecutor for WitNodeExecutor {
    fn execute(&self, input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError> {
        // Extract the Component from the WasmArtifact
        let component = match &self.loaded_module.artifact {
            WasmArtifact::Component(c) => c,
            WasmArtifact::Module(_) => {
                return Err(ProcessingNodeError::ComponentError(
                    ComponentExecutionError::InstantiationFailed(
                        "Expected WIT Component, got core WASM module".to_string(),
                    ),
                ));
            }
        };

        // Create a store
        let mut store = Store::new(&self.loaded_module.engine, ());

        // Create linker for the component
        let linker = Linker::new(&self.loaded_module.engine);

        // Instantiate using wit-bindgen generated bindings
        // This handles all the WIT interface setup automatically!
        let bindings = DagwoodComponent::instantiate(&mut store, component, &linker)
            .map_err(|e| {
                ProcessingNodeError::ComponentError(
                    ComponentExecutionError::InstantiationFailed(format!(
                        "Failed to instantiate component: {}",
                        e
                    )),
                )
            })?;

        // Call the process function using wit-bindgen's generated API
        // This automatically handles ALL memory management via canonical ABI!
        let result = bindings
            .dagwood_component_processing_node()
            .call_process(&mut store, input)
            .map_err(|e| {
                ProcessingNodeError::ComponentError(
                    ComponentExecutionError::FunctionCallFailed(format!(
                        "Component instantiation/call failed: {}",
                        e
                    )),
                )
            })?;

        // Handle the WIT-level Result<list<u8>, processing-error>
        let output = result.map_err(|processing_error| {
            ProcessingNodeError::ComponentError(
                ComponentExecutionError::FunctionCallFailed(format!(
                    "Component process() returned error: {:?}",
                    processing_error
                )),
            )
        })?;

        // That's it! wit-bindgen handled:
        // - Memory allocation
        // - Writing input bytes to component memory
        // - Calling the process function
        // - Reading output bytes from component memory
        // - Memory deallocation
        // All through the canonical ABI!

        Ok(output)
    }

    fn artifact_type(&self) -> &'static str {
        "WIT Component (JavaScript)"
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["javascript".to_string()]
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

/// Test helper function to load a WIT component from a filepath and execute it
/// This is a convenience wrapper around WasmModuleLoader and execute()
#[cfg(test)]
pub fn test_with_file<P: AsRef<std::path::Path>>(
    filepath: P,
    input: &[u8],
) -> Result<Vec<u8>, ProcessingNodeError> {
    use super::super::WasmModuleLoader;
    
    // Load the module from the filepath
    let loaded_module = WasmModuleLoader::load_module(filepath)
        .map_err(|e| ProcessingNodeError::RuntimeError(e.to_string()))?;
    
    // Create the executor
    let executor = WitNodeExecutor::new(loaded_module)?;
    
    // Execute using the real implementation
    executor.execute(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wit_executor_with_rle_js() {
        let result = test_with_file(
            "/data/development/projects/the-dagwood/wasm_components/rle_js.wasm",
            b"test input",
        );
        
        match result {
            Ok(output) => {
                println!("RLE JS output: {:?}", String::from_utf8_lossy(&output));
            }
            Err(e) => {
                println!("Error: {:?}", e);
                panic!("Test failed: {:?}", e);
            }
        }
    }
}
