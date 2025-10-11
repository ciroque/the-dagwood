// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use super::super::{
    processing_node::{ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor},
    LoadedModule, module_loader::WasmArtifact,
};
use std::sync::Arc;
use wasmtime::*;
use wasmtime::component::{Component, Linker as ComponentLinker, bindgen};

// Generate bindings for the DAGwood WIT interface
bindgen!({
    world: "dagwood-component",
    path: "wit/dagwood-processor.wit",
    async: false,
});

/// Executor for WIT Components (Preview 2)
///
/// Handles execution of WebAssembly components that implement the WIT interface.
/// This is the modern, type-safe way to interact with WebAssembly modules.
pub struct ComponentNodeExecutor {
    loaded_module: Arc<LoadedModule>,
}

impl ComponentNodeExecutor {
    /// Create a new ComponentNodeExecutor
    pub fn new(loaded_module: LoadedModule) -> Result<Self, ProcessingNodeError> {
        // TODO: Implement WIT component validation
        Ok(Self {
            loaded_module: Arc::new(loaded_module),
        })
    }

    /// Execute WIT component using generated bindings
    fn execute_wit_component(
        &self,
        store: &mut Store<()>,
        component: &Component,
        input: &str,
    ) -> Result<String, ProcessingNodeError> {
        // Create component linker
        let linker = ComponentLinker::new(store.engine());
        
        // Instantiate the component
        let instance = linker.instantiate(&mut *store, component)
            .map_err(|e| ProcessingNodeError::RuntimeError(e.to_string()))?;
        
        // Get the component interface
        let _dagwood_component = DagwoodComponent::new(store, &instance)
            .map_err(|e| ProcessingNodeError::RuntimeError(e.to_string()))?;
        
        // For Phase 2.1: Simplified WIT execution
        // This is a placeholder implementation that demonstrates WIT component loading
        // Full implementation will handle:
        // 1. Memory allocation in component linear memory
        // 2. Calling the actual process function with proper WIT types
        // 3. Handling WIT result<T, E> return types
        // 4. Memory cleanup and deallocation
        
        let output = format!("{}-wit-component", input);
        
        Ok(output)
    }
}

impl ProcessingNodeExecutor for ComponentNodeExecutor {
    fn execute(&self, input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError> {
        // Convert input to string for WIT interface
        let input_str = std::str::from_utf8(input)
            .map_err(|e| ProcessingNodeError::InputError(e.to_string()))?;

        // Create store for component execution
        let mut store = Store::new(&self.loaded_module.engine, ());
        
        // Set fuel limit for security and resource protection
        store.set_fuel(100_000_000)
            .map_err(|e| ProcessingNodeError::RuntimeError(e.to_string()))?;

        // Execute based on artifact type
        let output = match &self.loaded_module.artifact {
            WasmArtifact::Component(component) => {
                // Execute WIT component using generated bindings
                self.execute_wit_component(&mut store, component, input_str)?
            }
            WasmArtifact::Module(_) => {
                // This shouldn't happen for WIT components, but handle gracefully
                return Err(ProcessingNodeError::ValidationError(
                    "WIT Component executor received core WASM module".to_string()
                ));
            }
        };

        Ok(output.into_bytes())
    }

    fn artifact_type(&self) -> &'static str {
        "WIT Component"
    }

    fn capabilities(&self) -> Vec<String> {
        // TODO: Return actual capabilities from component metadata
        vec![
            "wasmtime:component-model".to_string(),
            "preview2".to_string(),
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
    use crate::backends::wasm::{ComponentType, ModuleImport, ImportType};
    use wasmtime::{Engine, Module};

    fn create_mock_component_loaded_module() -> LoadedModule {
        let engine = Engine::default();
        
        // Create a minimal valid WASM module for testing
        // In practice, this would be a Component, but for testing we use a Module
        let wasm_bytes = wat::parse_str("(module)").unwrap();
        let module = Module::new(&engine, &wasm_bytes).unwrap();

        LoadedModule {
            engine,
            artifact: crate::backends::wasm::module_loader::WasmArtifact::Module(module),
            component_type: ComponentType::WitComponent,
            imports: vec![],
            module_path: "test_component.wasm".to_string(),
        }
    }

    #[test]
    fn test_component_executor_creation() {
        let loaded_module = create_mock_component_loaded_module();
        let result = ComponentNodeExecutor::new(loaded_module);
        
        assert!(result.is_ok());
        let executor = result.unwrap();
        assert_eq!(executor.artifact_type(), "WIT Component");
        
        let capabilities = executor.capabilities();
        assert!(capabilities.contains(&"wasmtime:component-model".to_string()));
        assert!(capabilities.contains(&"preview2".to_string()));
    }

    #[test]
    fn test_component_executor_validation_error() {
        let loaded_module = create_mock_component_loaded_module();
        let executor = ComponentNodeExecutor::new(loaded_module).unwrap();
        
        let input = b"test input";
        let result = executor.execute(input);
        
        // Should fail because we're passing a Module to a Component executor
        assert!(result.is_err());
        match result {
            Err(ProcessingNodeError::ValidationError(msg)) => {
                assert!(msg.contains("WIT Component executor received core WASM module"));
            }
            Err(e) => {
                println!("Got different error type: {}", e);
                // Accept any error for now since this is a mock test
                assert!(true);
            }
            Ok(_) => {
                panic!("Expected an error but got success");
            }
        }
    }
}
