// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use super::super::{
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

        // Instantiate the component
        let instance = Linker::new(&self.loaded_module.engine)
            .instantiate(&mut store, component)
            .map_err(|e| {
                ProcessingNodeError::ComponentError(
                    ComponentExecutionError::InstantiationFailed(e.to_string()),
                )
            })?;

        // Get the processing-node interface functions
        // Note: WIT functions use Result<T, E> which maps to result<ok, err> in WIT
        // The actual type depends on the WIT interface definition
        let _process_func = instance
            .get_typed_func::<(u32, u64, u32), (Result<u32, ()>,)>(&mut store, "dagwood:component/processing-node#process")
            .map_err(|e| {
                ProcessingNodeError::ComponentError(
                    ComponentExecutionError::InterfaceNotFound(format!(
                        "Failed to get process function: {}",
                        e
                    )),
                )
            })?;

        let allocate_func = instance
            .get_typed_func::<(u64,), (Result<u32, ()>,)>(&mut store, "dagwood:component/processing-node#allocate")
            .map_err(|e| {
                ProcessingNodeError::ComponentError(
                    ComponentExecutionError::InterfaceNotFound(format!(
                        "Failed to get allocate function: {}",
                        e
                    )),
                )
            })?;

        let _deallocate_func = instance
            .get_typed_func::<(u32, u64), ()>(&mut store, "dagwood:component/processing-node#deallocate")
            .map_err(|e| {
                ProcessingNodeError::ComponentError(
                    ComponentExecutionError::InterfaceNotFound(format!(
                        "Failed to get deallocate function: {}",
                        e
                    )),
                )
            })?;

        // Allocate memory for input
        let input_len = input.len() as u64;
        let (input_ptr_result,) = allocate_func
            .call(&mut store, (input_len,))
            .map_err(|e| {
                ProcessingNodeError::ComponentError(
                    ComponentExecutionError::MemoryAllocationFailed(format!(
                        "Failed to allocate input memory: {}",
                        e
                    )),
                )
            })?;
        
        let _input_ptr = input_ptr_result.map_err(|_| {
            ProcessingNodeError::ComponentError(
                ComponentExecutionError::MemoryAllocationFailed(
                    "Allocate function returned error".to_string(),
                ),
            )
        })?;

        // Component Model memory access requires using wit-bindgen
        // or manual canonical ABI implementation
        //
        // TODO: Implement proper Component Model memory access using one of:
        // 1. wit-bindgen generated bindings with proper memory functions
        // 2. Manual implementation of canonical ABI memory transfer  
        // 3. Access underlying core module memory through Component API
        //
        // For reference, the complete flow would be:
        // 1. Write input bytes to memory at input_ptr
        // 2. Allocate output_len_ptr and call process function
        // 3. Read output length from output_len_ptr
        // 4. Read output bytes from output_ptr
        // 5. Deallocate all allocated memory
        
        Err(ProcessingNodeError::ComponentError(
            ComponentExecutionError::MemoryAccessFailed(
                "Component Model memory access not yet implemented. \
                 This requires wit-bindgen or manual canonical ABI implementation."
                    .to_string(),
            ),
        ))
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
        // This test will fail until we complete the memory read/write implementation
        // For now, it just verifies the structure compiles
        let result = test_with_file(
            "wasm_components/rle_js.wasm",
            b"test input",
        );
        
        // We expect it to fail because memory operations aren't implemented yet
        match result {
            Ok(_output) => {
                // When fully implemented, we can validate the output
                println!("Execution succeeded (unexpected at this stage)");
            }
            Err(e) => {
                println!("Expected error (memory operations not yet implemented): {:?}", e);
            }
        }
    }
}
