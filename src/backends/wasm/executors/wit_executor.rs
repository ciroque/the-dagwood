// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use super::super::{
    bindings::DagwoodComponent,
    processing_node::{ComponentExecutionError, ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor},
};
use std::sync::Arc;
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

pub struct WitNodeExecutor {
    component: Arc<Component>,
    engine: Arc<Engine>,
}

impl WitNodeExecutor {
    pub fn new(component: Component, engine: Engine) -> Result<Self, ProcessingNodeError> {
        Ok(Self {
            component: Arc::new(component),
            engine: Arc::new(engine),
        })
    }
}

struct Ctx {
    wasi: WasiCtx,
    table: ResourceTable,
    http: WasiHttpCtx,
}

impl WasiView for Ctx  {
    fn ctx(&mut self) -> WasiCtxView<'_>  {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table
        }
    }
}

impl WasiHttpView for Ctx {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl ProcessingNodeExecutor for WitNodeExecutor {
    fn execute(&self, input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError> {
        // Create WASI context
        let wasi_ctx = WasiCtxBuilder::new()
            .inherit_stdio()
            .args(&["dagwood-component"])
            .build();

        let store_data = Ctx { 
            wasi: wasi_ctx, 
            table: ResourceTable::new(),
            http: WasiHttpCtx::new(),
        };
        let mut store = Store::new(&self.engine, store_data);

        // Add all WASI interfaces for JavaScript components built with jco
        let mut linker = Linker::new(&self.engine);
        
        // // Add complete WASI support (CLI, filesystem, etc.)
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
            .map_err(|e| ProcessingNodeError::ComponentError(
                ComponentExecutionError::InstantiationFailed(format!(
                    "Failed to add WASI to linker: {}",
                    e
                ))
            ))?;
        //
        // // Add HTTP support (only HTTP-specific interfaces, no overlap)
        // wasmtime_wasi_http::add_only_http_to_linker_sync(&mut linker)
        //     .map_err(|e| ProcessingNodeError::ComponentError(
        //         ComponentExecutionError::InstantiationFailed(format!(
        //             "Failed to add WASI HTTP to linker: {}",
        //             e
        //         ))
        //     ))?;








        linker
            .root()
            .func_wrap(
                "memory-allocator:cabi-realloc",
                |_: wasmtime::StoreContextMut<Ctx>, (_old_ptr, _old_size, _new_size): (u32, u32, u32)| {
                    // No-op: return () for testing
                    Ok(())
                },
            )
            .map_err(|e| format!("Failed to add cabi-realloc: {}", e))?;






        // Instantiate using wit-bindgen generated bindings
        // This handles all the WIT interface setup automatically!
        let bindings = DagwoodComponent::instantiate(&mut store, &self.component, &linker)
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
            module_path: "".to_string(), // Path no longer stored in executor
            artifact_type: self.artifact_type().to_string(),
            import_count: 0, // Import tracking removed
            capabilities: self.capabilities(),
        }
    }
}

/// Test helper function to load a WIT component from a filepath and execute it
/// Uses the new ADR-17 flow
#[cfg(test)]
pub fn test_with_file<P: AsRef<std::path::Path>>(
    filepath: P,
    input: &[u8],
) -> Result<Vec<u8>, ProcessingNodeError> {
    use super::super::{load_wasm_bytes, wasm_encoding};
    use wasmtime::component::Component;
    use wasmtime::Engine;
    
    // ADR-17 flow
    let bytes = load_wasm_bytes(filepath)
        .map_err(|e| ProcessingNodeError::RuntimeError(e.to_string()))?;
    
    let encoding = wasm_encoding(&bytes)
        .map_err(|e| ProcessingNodeError::ValidationError(e.to_string()))?;
    
    if !encoding.is_component_model() {
        return Err(ProcessingNodeError::ValidationError(
            "Expected Component Model component".to_string()
        ));
    }
    
    // Create engine with component model support
    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)
        .map_err(|e| ProcessingNodeError::RuntimeError(e.to_string()))?;
    
    // Parse component
    let component = Component::new(&engine, &bytes)
        .map_err(|e| ProcessingNodeError::RuntimeError(e.to_string()))?;
    
    // Create executor
    let executor = WitNodeExecutor::new(component, engine)?;
    
    // Execute
    executor.execute(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "This is intended to be run manually for now"]
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
