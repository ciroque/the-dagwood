// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WASM Processor Implementation
//!
//! This module provides a WebAssembly (WASM) processor backend for The DAGwood project.
//! It uses the wasmtime runtime to execute WASM modules in a sandboxed environment
//! with proper security isolation and resource management.
//!
//! ## Architecture Overview
//!
//! The WASM processor provides secure, isolated execution of user-defined processing logic
//! compiled to WebAssembly. This enables:
//! - **Security**: Complete sandboxing with no host system access
//! - **Performance**: Near-native execution speed with wasmtime's optimizations
//! - **Flexibility**: Support for any WASM-compiled language (Rust, C, AssemblyScript, etc.)
//! - **Deterministic Execution**: Reproducible results with controlled resource access
//!
//! ## WASM Module Interface
//!
//! WASM modules must export the following C-style functions:
//! ```c
//! // Main processing function - takes data pointer and length, returns allocated data
//! uint8_t* process(const uint8_t* input_ptr, size_t input_len, size_t* output_len);
//!
//! // Memory management functions for host-WASM communication
//! void* allocate(size_t size);
//! void deallocate(void* ptr, size_t size);
//! ```
//!
//! ## Security Model
//!
//! ### Sandboxing
//! - **Complete isolation**: WASM modules cannot access host filesystem, network, or system calls
//! - **Memory isolation**: WASM linear memory is separate from host memory
//! - **Limited WASI**: Allows essential WASI functions (proc_exit, random_get, clock_time_get) for modern WASM languages
//!
//! ### Resource Limits
//! - **Fuel consumption**: Computational budget prevents infinite loops and runaway execution
//! - **Memory limits**: WASM modules have bounded linear memory (default: 64KB pages)
//! - **Input size limits**: Maximum input size prevents memory exhaustion attacks
//! - **Module size limits**: Maximum WASM module size prevents storage attacks
//!
//! ### Timeout Protection
//! - **Fuel-based timeouts**: Execution stops when fuel budget is exhausted
//! - **Epoch interruption disabled**: Prevents false interrupts in wasmtime 25.0+

use crate::backends::wasm::error::WasmResult;
use crate::backends::wasm::module_loader::{WasmModuleLoader, LoadedModule};
use crate::backends::wasm::processing_node_factory::ProcessingNodeFactory;
use crate::backends::wasm::processing_node::ProcessingNodeExecutor;
use std::sync::Arc;
use crate::proto::processor_v1::{
    processor_response::Outcome, ErrorDetail, PipelineMetadata, ProcessorMetadata,
    ProcessorRequest, ProcessorResponse,
};
use crate::traits::processor::{Processor, ProcessorIntent};
use async_trait::async_trait;
use std::collections::HashMap;

/// WASM Processor - orchestrates WASM module execution using specialized components
/// 
/// This processor follows the Single Responsibility Principle by delegating to:
/// - WasmModuleLoader: Module loading and validation
/// - CapabilityManager: Capability analysis and WASI setup
/// - WasmExecutor: Pure execution and memory management
///
/// # Security Features
///
/// - **Memory Protection**: Strict bounds checking and memory isolation
/// - **Capability-Based Security**: Explicit WASI capability declarations
/// - **Input Validation**: Size limits and content validation
/// - **Resource Limits**: Fuel-based execution budgets
/// - **Deterministic Execution**: Controlled execution environment
pub struct WasmProcessor {
    /// Unique identifier for this processor instance
    processor_id: String,
    /// Path to the WASM module file
    module_path: String,
    /// The appropriate executor for this WASM artifact type
    executor: Arc<dyn ProcessingNodeExecutor>,
    /// Processor intent (Transform or Analyze)
    intent: ProcessorIntent,
}

impl WasmProcessor {
    /// Creates a new WasmProcessor with the specified configuration.
    ///
    /// This constructor orchestrates the loading and validation process using
    /// specialized components following the Single Responsibility Principle.
    ///
    /// # Security
    ///
    /// The processor enforces several security measures:
    /// - Module validation and capability analysis
    /// - Memory limits and fuel-based execution budgets
    /// - WASI capability restrictions
    /// - Input validation and size limits
    pub fn new(
        processor_id: String,
        module_path: String,
        intent: ProcessorIntent,
    ) -> WasmResult<Self> {
        // Use WasmModuleLoader to handle loading and validation
        let loaded_module = WasmModuleLoader::load_module(&module_path)?;

        tracing::info!(
            "Loaded WASM module: {} (type: {:?}, imports: {})",
            module_path,
            loaded_module.component_type,
            loaded_module.imports.len()
        );

        // Create the appropriate executor based on the WASM artifact type
        let executor = ProcessingNodeFactory::create_executor(loaded_module)
            .map_err(|e| WasmResult::<()>::Err(e.into()).unwrap_err())?;
        
        tracing::info!(
            "Created WasmProcessor '{}' with {} executor",
            processor_id,
            executor.artifact_type()
        );

        Ok(Self {
            processor_id,
            module_path,
            executor,
            intent,
        })
    }

    /// Creates a new WasmProcessor from an already loaded module.
    ///
    /// This constructor is used by the factory when the module has already been
    /// loaded and analyzed for artifact type detection. This avoids double-loading
    /// the same WASM bytes.
    ///
    /// # Arguments
    ///
    /// * `processor_id` - Unique identifier for this processor instance
    /// * `loaded_module` - Pre-loaded and validated WASM module
    /// * `intent` - Processor intent (Transform or Analyze)
    pub fn from_loaded_module(
        processor_id: String,
        loaded_module: LoadedModule,
        intent: ProcessorIntent,
    ) -> WasmResult<Self> {
        let module_path = loaded_module.module_path.clone();

        tracing::debug!(
            "Creating WasmProcessor from loaded module: {} (type: {:?}, imports: {})",
            module_path,
            loaded_module.component_type,
            loaded_module.imports.len()
        );

        // Create the appropriate executor based on the WASM artifact type
        let executor = ProcessingNodeFactory::create_executor(loaded_module)
            .map_err(|e| WasmResult::<()>::Err(e.into()).unwrap_err())?;
        
        tracing::info!(
            "Created WasmProcessor '{}' with {} executor",
            processor_id,
            executor.artifact_type()
        );

        Ok(Self {
            processor_id,
            module_path,
            executor,
            intent,
        })
    }

    /// Executes the WASM module with the given input bytes using the ProcessingNodeExecutor.
    ///
    /// This method uses the pre-created executor that was determined during initialization
    /// based on the detected WASM artifact type (C-Style, WASI Preview 1, or WIT Component).
    ///
    /// # Arguments
    ///
    /// * `input` - The input bytes to process
    ///
    /// # Returns
    ///
    /// Returns the processed output bytes or an error if execution fails.
    fn execute_wasm(
        &self,
        input: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!(
            "Executing WASM module '{}' using {} executor",
            self.module_path,
            self.executor.artifact_type()
        );
        
        // Execute using the ProcessingNodeExecutor
        match self.executor.execute(input) {
            Ok(output) => {
                tracing::debug!(
                    "WASM execution successful: input_size={}, output_size={}, artifact_type={}",
                    input.len(),
                    output.len(),
                    self.executor.artifact_type()
                );
                Ok(output)
            }
            Err(error) => {
                tracing::error!(
                    "WASM execution failed for {}: {}", 
                    self.executor.artifact_type(),
                    error
                );
                Err(Box::new(error))
            }
        }
    }
}

#[async_trait]
impl Processor for WasmProcessor {
    fn name(&self) -> &'static str {
        "WasmProcessor"
    }

    fn declared_intent(&self) -> ProcessorIntent {
        self.intent
    }

    async fn process(&self, request: ProcessorRequest) -> ProcessorResponse {
        let input = request.payload;
        
        // Execute WASM module using orchestrated components
        match self.execute_wasm(&input) {
            Ok(output) => {
                // Create successful response with metadata
                let mut processor_metadata_map = HashMap::new();
                processor_metadata_map.insert("processor_id".to_string(), self.processor_id.clone());
                processor_metadata_map.insert("module_path".to_string(), self.module_path.clone());
                processor_metadata_map.insert("artifact_type".to_string(), self.executor.artifact_type().to_string());
                processor_metadata_map.insert("capabilities".to_string(), format!("{:?}", self.executor.capabilities()));
                processor_metadata_map.insert("input_length".to_string(), input.len().to_string());
                processor_metadata_map.insert("output_length".to_string(), output.len().to_string());

                let processor_metadata = ProcessorMetadata {
                    metadata: processor_metadata_map,
                };

                let mut pipeline_metadata_map = HashMap::new();
                pipeline_metadata_map.insert(self.processor_id.clone(), processor_metadata);

                let pipeline_metadata = PipelineMetadata {
                    metadata: pipeline_metadata_map,
                };

                ProcessorResponse {
                    outcome: Some(Outcome::NextPayload(output)),
                    metadata: Some(pipeline_metadata),
                }
            }
            Err(error) => {
                // Create error response with error outcome
                let error_detail = ErrorDetail {
                    code: 500,
                    message: format!("WASM execution failed: {}", error),
                };

                ProcessorResponse {
                    outcome: Some(Outcome::Error(error_detail)),
                    metadata: None,
                }
            }
        }
    }
}
