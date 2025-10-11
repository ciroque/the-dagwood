// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Processing Node Execution Strategy Pattern
//!
//! This module implements the Strategy Pattern for WASM artifact execution,
//! providing clean separation between different WASM artifact types:
//! - C-Style modules with direct function exports
//! - WASI Preview 1 modules with WASI runtime dependencies
//! - WIT Components with structured interfaces (Preview 2)
//!
//! ## Architecture
//!
//! The `ProcessingNodeExecutor` trait defines the interface for all execution
//! strategies, while concrete implementations handle the specifics of each
//! artifact type. This enables:
//! - Type-specific optimizations
//! - Rich error context
//! - Independent testing
//! - Easy extensibility

use std::fmt;

/// Processing Node execution strategy trait
///
/// This trait defines the interface for executing different types of WASM artifacts.
/// Each implementation handles the specifics of one artifact type while providing
/// a consistent interface for the `WasmProcessor`.
///
/// # Design Principles
///
/// - **Single Responsibility**: Each implementation handles one artifact type
/// - **Async Execution**: Consistent with DAGwood's async architecture
/// - **Rich Errors**: Strategy-specific error context for debugging
/// - **Extensibility**: Easy to add new artifact types
pub trait ProcessingNodeExecutor: Send + Sync {
    /// Execute the WASM artifact with the given input
    ///
    /// # Arguments
    ///
    /// * `input` - Input bytes to process
    ///
    /// # Returns
    ///
    /// Returns the processed output bytes or a strategy-specific error
    fn execute(&self, input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError>;

    /// Get a human-readable description of the artifact type
    fn artifact_type(&self) -> &'static str;

    /// Get the capabilities/features supported by this executor
    fn capabilities(&self) -> Vec<String>;

    /// Get execution metadata for observability
    fn execution_metadata(&self) -> ExecutionMetadata;
}

/// Execution metadata for observability and debugging
#[derive(Debug, Clone)]
pub struct ExecutionMetadata {
    pub module_path: String,
    pub artifact_type: String,
    pub import_count: usize,
    pub capabilities: Vec<String>,
}

use crate::backends::wasm::error::WasmError;

/// Strategy-specific processing node errors
///
/// This enum provides rich error context specific to each execution strategy,
/// enabling better debugging and user experience.
#[derive(Debug)]
pub enum ProcessingNodeError {
    /// WIT Component execution error (Preview 2)
    ComponentError(ComponentExecutionError),
    
    /// WASI Preview 1 module execution error
    WasiError(WasiExecutionError),
    
    /// C-Style module execution error
    CStyleError(CStyleExecutionError),
    
    /// Input processing error
    InputError(String),
    
    /// General validation error
    ValidationError(String),
    
    /// Runtime execution error
    RuntimeError(String),
}

/// WIT Component execution errors (Preview 2)
#[derive(Debug)]
pub enum ComponentExecutionError {
    /// Component instantiation failed
    InstantiationFailed(String),
    
    /// Required interface not found
    InterfaceNotFound(String),
    
    /// Function call failed
    FunctionCallFailed(String),
    
    /// Memory allocation failed
    MemoryAllocationFailed(String),
}

/// WASI Preview 1 execution errors
#[derive(Debug)]
pub enum WasiExecutionError {
    /// WASI context creation failed
    ContextCreationFailed(String),
    
    /// Required WASI function not available
    FunctionNotAvailable(String),
    
    /// WASI runtime error
    RuntimeError(String),
    
    /// Memory management error
    MemoryError(String),
}

/// C-Style execution errors
#[derive(Debug)]
pub enum CStyleExecutionError {
    /// Required function export not found
    FunctionNotFound(String),
    
    /// Function signature mismatch
    SignatureMismatch(String),
    
    /// Memory allocation failed
    AllocationFailed(String),
    
    /// Function execution failed
    FunctionExecutionFailed(String),
}

impl fmt::Display for ProcessingNodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessingNodeError::ComponentError(e) => write!(f, "Component execution error: {}", e),
            ProcessingNodeError::WasiError(e) => write!(f, "WASI execution error: {}", e),
            ProcessingNodeError::CStyleError(e) => write!(f, "C-Style execution error: {}", e),
            ProcessingNodeError::InputError(msg) => write!(f, "Input error: {}", msg),
            ProcessingNodeError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ProcessingNodeError::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
        }
    }
}

impl fmt::Display for ComponentExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComponentExecutionError::InstantiationFailed(msg) => {
                write!(f, "Component instantiation failed: {}", msg)
            }
            ComponentExecutionError::InterfaceNotFound(interface) => {
                write!(f, "Required interface '{}' not found in component", interface)
            }
            ComponentExecutionError::FunctionCallFailed(msg) => {
                write!(f, "Component function call failed: {}", msg)
            }
            ComponentExecutionError::MemoryAllocationFailed(msg) => {
                write!(f, "Component memory allocation failed: {}", msg)
            }
        }
    }
}

impl fmt::Display for WasiExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WasiExecutionError::ContextCreationFailed(msg) => {
                write!(f, "WASI context creation failed: {}", msg)
            }
            WasiExecutionError::FunctionNotAvailable(func) => {
                write!(f, "Required WASI function '{}' not available", func)
            }
            WasiExecutionError::RuntimeError(msg) => {
                write!(f, "WASI runtime error: {}", msg)
            }
            WasiExecutionError::MemoryError(msg) => {
                write!(f, "WASI memory error: {}", msg)
            }
        }
    }
}

impl fmt::Display for CStyleExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CStyleExecutionError::FunctionNotFound(func) => {
                write!(f, "Required function '{}' not found in module exports", func)
            }
            CStyleExecutionError::SignatureMismatch(msg) => {
                write!(f, "Function signature mismatch: {}", msg)
            }
            CStyleExecutionError::AllocationFailed(msg) => {
                write!(f, "Memory allocation failed: {}", msg)
            }
            CStyleExecutionError::FunctionExecutionFailed(msg) => {
                write!(f, "C-Style execution failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for ProcessingNodeError {}
impl std::error::Error for ComponentExecutionError {}
impl std::error::Error for WasiExecutionError {}
impl std::error::Error for CStyleExecutionError {}

impl From<WasmError> for ProcessingNodeError {
    fn from(error: WasmError) -> Self {
        ProcessingNodeError::RuntimeError(error.to_string())
    }
}

// Convert ProcessingNodeError to WasmError for better error handling
impl From<ProcessingNodeError> for WasmError {
    fn from(error: ProcessingNodeError) -> Self {
        WasmError::ProcessorError(error.to_string())
    }
}
