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
        let process_func = instance
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

        let deallocate_func = instance
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
        
        let input_ptr = input_ptr_result.map_err(|_| {
            ProcessingNodeError::ComponentError(
                ComponentExecutionError::MemoryAllocationFailed(
                    "Allocate function returned error".to_string(),
                ),
            )
        })?;

        // TODO: Write input data to WASM memory at input_ptr
        // This requires accessing the component's memory and writing the bytes
        // For now, we'll skip this step

        // Allocate memory for output length pointer
        let (output_len_ptr_result,) = allocate_func
            .call(&mut store, (8,)) // u64 size
            .map_err(|e| {
                ProcessingNodeError::ComponentError(
                    ComponentExecutionError::MemoryAllocationFailed(format!(
                        "Failed to allocate output length pointer: {}",
                        e
                    )),
                )
            })?;
        
        let output_len_ptr = output_len_ptr_result.map_err(|_| {
            ProcessingNodeError::ComponentError(
                ComponentExecutionError::MemoryAllocationFailed(
                    "Allocate function returned error for output length".to_string(),
                ),
            )
        })?;

        // Call the process function
        let (output_ptr_result,) = process_func
            .call(&mut store, (input_ptr, input_len, output_len_ptr))
            .map_err(|e| {
                ProcessingNodeError::ComponentError(
                    ComponentExecutionError::FunctionCallFailed(format!(
                        "Process function failed: {}",
                        e
                    )),
                )
            })?;
        
        let _output_ptr = output_ptr_result.map_err(|_| {
            ProcessingNodeError::ComponentError(
                ComponentExecutionError::FunctionCallFailed(
                    "Process function returned error".to_string(),
                ),
            )
        })?;

        // TODO: Read output length from memory at output_len_ptr
        // TODO: Read output data from memory at _output_ptr
        // For now, return empty vector as placeholder
        let output = Vec::new();

        // Clean up allocated memory
        deallocate_func
            .call(&mut store, (input_ptr, input_len))
            .map_err(|e| {
                ProcessingNodeError::ComponentError(
                    ComponentExecutionError::FunctionCallFailed(format!(
                        "Failed to deallocate input memory: {}",
                        e
                    )),
                )
            })?;

        deallocate_func
            .call(&mut store, (output_len_ptr, 8))
            .map_err(|e| {
                ProcessingNodeError::ComponentError(
                    ComponentExecutionError::FunctionCallFailed(format!(
                        "Failed to deallocate output length pointer: {}",
                        e
                    )),
                )
            })?;

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
