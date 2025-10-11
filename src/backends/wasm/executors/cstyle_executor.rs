// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use super::super::{
    processing_node::{ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor},
    LoadedModule,
};
use async_trait::async_trait;
use std::sync::Arc;

/// Executor for C-Style WASM Modules
///
/// Handles execution of WebAssembly modules that export C-style functions:
/// - `process`: Main processing function
/// - `allocate`: Memory allocation function
/// - `deallocate`: Memory deallocation function
///
/// This is the simplest form of WebAssembly module, typically produced by
/// compiling C/C++/Rust code with minimal runtime.
pub struct CStyleNodeExecutor {
    loaded_module: Arc<LoadedModule>,
}

impl CStyleNodeExecutor {
    /// Create a new CStyleNodeExecutor
    pub fn new(loaded_module: LoadedModule) -> Result<Self, ProcessingNodeError> {
        // TODO: Validate required exports (process, allocate, deallocate)
        Ok(Self {
            loaded_module: Arc::new(loaded_module),
        })
    }
}

#[async_trait]
impl ProcessingNodeExecutor for CStyleNodeExecutor {
    async fn execute(&self, input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError> {
        // TODO: Implement C-style module execution with proper memory management
        // This is a placeholder implementation
        Ok(input.to_vec())
    }

    fn artifact_type(&self) -> &'static str {
        "C-Style"
    }

    fn capabilities(&self) -> Vec<String> {
        // C-Style modules are completely sandboxed
        vec!["c-style".to_string()]
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
