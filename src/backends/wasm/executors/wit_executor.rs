// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use super::super::{
    bindings::DagwoodComponent,
    processing_node::{ComponentExecutionError, ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor},
};
use std::sync::Arc;
use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

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
    table: wasmtime::component::ResourceTable,
}

impl WasiView for Ctx {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl ProcessingNodeExecutor for WitNodeExecutor {
    fn execute(&self, input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError> {
        let wasi_ctx = WasiCtxBuilder::new()
            .inherit_stdio()
            .args(&["dagwood-component"])
            .build();

        let store_data = Ctx {
            wasi: wasi_ctx,
            table: wasmtime::component::ResourceTable::new(),
        };
        let mut store = Store::new(&self.engine, store_data);

        let mut linker = Linker::<Ctx>::new(&self.engine);
        
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
            .map_err(|e| ProcessingNodeError::ComponentError(
                ComponentExecutionError::InstantiationFailed(format!(
                    "Failed to add WASI to linker: {}",
                    e
                ))
            ))?;

        let bindings = DagwoodComponent::instantiate(&mut store, &self.component, &linker)
            .map_err(|e| {
                ProcessingNodeError::ComponentError(
                    ComponentExecutionError::InstantiationFailed(format!(
                        "Failed to instantiate component: {}",
                        e
                    )),
                )
            })?;

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

        let output = result.map_err(|processing_error| {
            ProcessingNodeError::ComponentError(
                ComponentExecutionError::FunctionCallFailed(format!(
                    "Component process() returned error: {:?}",
                    processing_error
                )),
            )
        })?;

        Ok(output)
    }

    fn artifact_type(&self) -> &'static str {
        "WIT Component"
    }

    fn capabilities(&self) -> Vec<String> {
        vec![]
    }

    fn execution_metadata(&self) -> ExecutionMetadata {
        ExecutionMetadata {
            module_path: "".to_string(),
            artifact_type: self.artifact_type().to_string(),
            import_count: 0,
            capabilities: self.capabilities(),
        }
    }
}
