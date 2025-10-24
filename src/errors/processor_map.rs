// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Errors for processor map creation and processor instantiation.

use crate::config::BackendType;
use std::error::Error;
use std::fmt;

/// Errors that can occur during processor map creation
#[derive(Debug)]
pub enum ProcessorMapError {
    /// A backend type is not yet implemented
    BackendNotImplemented {
        processor_id: String,
        backend: BackendType,
    },

    /// Failed to create a processor from configuration
    ProcessorCreationFailed {
        processor_id: String,
        backend: BackendType,
        reason: String,
    },
}

impl fmt::Display for ProcessorMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessorMapError::BackendNotImplemented {
                processor_id,
                backend,
            } => {
                let description = match backend {
                    BackendType::Local => "native Rust",
                    BackendType::Loadable => "dynamic library loading",
                    BackendType::Grpc => "gRPC client",
                    BackendType::Http => "HTTP client",
                    BackendType::Wasm => "WebAssembly",
                };
                write!(
                    f,
                    "Backend type '{:?}' is not implemented for processor '{}'. {} is not yet supported.",
                    backend,
                    processor_id,
                    description
                )
            }
            ProcessorMapError::ProcessorCreationFailed {
                processor_id,
                backend,
                reason,
            } => {
                write!(
                    f,
                    "Failed to create {:?} processor '{}': {}",
                    backend, processor_id, reason
                )
            }
        }
    }
}

impl Error for ProcessorMapError {}
