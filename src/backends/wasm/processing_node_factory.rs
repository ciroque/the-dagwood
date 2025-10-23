// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! DEPRECATED: This module is superseded by `factory.rs` (ADR-17)
//! 
//! Kept for backward compatibility during transition. Will be removed in future version.

use crate::backends::wasm::{
    processing_node::{ProcessingNodeError, ProcessingNodeExecutor},
    LoadedModule,
};
use std::sync::Arc;

#[deprecated(since = "0.2.0", note = "Use `factory::create_executor()` instead (ADR-17)")]
pub struct ProcessingNodeFactory;

#[allow(deprecated)]
impl ProcessingNodeFactory {
    #[deprecated(since = "0.2.0", note = "Use `factory::create_executor()` instead (ADR-17)")]
    pub fn create_executor(
        _loaded_module: LoadedModule,
    ) -> Result<Arc<dyn ProcessingNodeExecutor>, ProcessingNodeError> {
        Err(ProcessingNodeError::ValidationError(
            "ProcessingNodeFactory is deprecated. Use factory::create_executor() with the new ADR-17 flow instead.".to_string()
        ))
    }
}

#[cfg(test)]
mod tests {}
