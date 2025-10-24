// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WIT Component Model executor with automatic memory management.
//!
//! This executor implements the `ProcessingNodeExecutor` trait for modern WebAssembly
//! Component Model components using WIT (WebAssembly Interface Types) bindings.
//!
//! # Component Model Benefits
//!
//! - **Automatic Memory Management**: Canonical ABI handles all memory allocation/deallocation
//! - **Type Safety**: Compile-time generated bindings from WIT definitions
//! - **WASI Preview 2**: Full WASI support with capability-based security
//! - **Composability**: Components can be linked and composed
//!
//! # WIT Interface
//!
//! Components must implement the `dagwood-component` world defined in `wit/versions/v1.0.0`:
//! ```wit
//! interface processing-node {
//!     process: func(input: list<u8>) -> result<list<u8>, string>
//! }
//! ```
//!
//! # Execution Flow
//!
//! 1. Create WASI context with inherited stdio
//! 2. Create component linker with WASI Preview 2 support
//! 3. Instantiate component with bindings
//! 4. Call `process()` function through type-safe interface
//! 5. Handle result (success or error)
//!
//! # WASI Integration
//!
//! The executor provides:
//! - **Stdio Inheritance**: Component can write to stdout/stderr
//! - **Sandboxed Execution**: No filesystem or network access by default
//! - **Resource Management**: Automatic cleanup via ResourceTable
//!
//! # Use Cases
//!
//! - Modern WASM processors with WIT interfaces
//! - Polyglot workflows (any language compiling to Component Model)
//! - Secure sandboxed execution with WASI capabilities
//! - Composable processor pipelines

use super::super::{
    bindings::DagwoodComponent,
    processing_node::{ComponentExecutionError, ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor},
};
use std::sync::Arc;
use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

/// Executor for WIT Component Model components with automatic memory management.
///
/// This executor handles modern Component Model components that implement the
/// `dagwood-component` WIT interface with automatic canonical ABI memory handling.
///
/// # Thread Safety
/// - Uses `Arc` for shared ownership across threads
/// - Stateless execution (creates new Store per call)
/// - Safe for concurrent use
pub struct WitNodeExecutor {
    component: Arc<Component>,
    engine: Arc<Engine>,
}

impl WitNodeExecutor {
    /// Create a new WIT executor from a compiled component and engine.
    ///
    /// # Arguments
    /// * `component` - Compiled Component Model component
    /// * `engine` - Configured Wasmtime engine with component model support
    ///
    /// # Returns
    /// * `Ok(WitNodeExecutor)` - Ready-to-use executor
    /// * `Err(ProcessingNodeError)` - If initialization fails
    pub fn new(component: Component, engine: Engine) -> Result<Self, ProcessingNodeError> {
        Ok(Self {
            component: Arc::new(component),
            engine: Arc::new(engine),
        })
    }
}

/// WASI context wrapper for Component Model execution.
///
/// Combines WASI context with resource table for proper WASI Preview 2 support.
struct Ctx {
    wasi: WasiCtx,
    table: wasmtime::component::ResourceTable,
}

/// Implement WasiView to provide WASI access to components.
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
