// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use super::super::{
    processing_node::{ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor},
    LoadedModule,
};
use async_trait::async_trait;
use std::sync::Arc;

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

#[async_trait]
impl ProcessingNodeExecutor for WasiNodeExecutor {
    async fn execute(&self, _input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError> {
        // TODO: Implement WASI module execution with proper context
        // This is a placeholder implementation
        Ok(b"WASI Preview 1 execution not yet implemented".to_vec())
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
