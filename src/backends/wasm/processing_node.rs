// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Synchronous WASM execution trait and error types for CPU-bound operations.
//!
//! This module defines the core abstraction for WASM processor execution. It provides a
//! synchronous trait (`ProcessingNodeExecutor`) designed specifically for CPU-bound WASM
//! operations, along with comprehensive error types for different execution strategies.
//!
//! # Architecture
//!
//! The processing node abstraction separates concerns between:
//! - **Synchronous Execution**: CPU-bound WASM operations (this module)
//! - **Async Processor**: I/O-bound workflow operations (`Processor` trait)
//!
//! This separation enables:
//! - Direct synchronous WASM execution without async overhead
//! - Clean error handling specific to WASM execution
//! - Strategy pattern for different WASM artifact types
//!
//! # Trait: ProcessingNodeExecutor
//!
//! The core trait for WASM execution strategies:
//! ```rust
//! use the_dagwood::backends::wasm::ProcessingNodeExecutor;
//!
//! # struct MyExecutor;
//! # impl the_dagwood::backends::wasm::ProcessingNodeExecutor for MyExecutor {
//! fn execute(&self, input: &[u8]) -> Result<Vec<u8>, the_dagwood::backends::wasm::ProcessingNodeError> {
//!     // Synchronous WASM execution
//!     Ok(input.to_vec())
//! }
//! #     fn artifact_type(&self) -> &'static str { "test" }
//! #     fn capabilities(&self) -> Vec<String> { vec![] }
//! #     fn execution_metadata(&self) -> the_dagwood::backends::wasm::ExecutionMetadata {
//! #         the_dagwood::backends::wasm::ExecutionMetadata {
//! #             module_path: String::new(),
//! #             artifact_type: String::new(),
//! #             import_count: 0,
//! #             capabilities: vec![],
//! #         }
//! #     }
//! # }
//! ```
//!
//! ## Implementations
//! - **WitNodeExecutor**: Component Model with automatic memory management
//! - **CStyleNodeExecutor**: Classic WASM with manual memory management
//!
//! # Error Handling
//!
//! The module provides strategy-specific error types:
//! - **ComponentExecutionError**: Component Model failures
//! - **WasiExecutionError**: WASI-related failures
//! - **CStyleExecutionError**: Classic WASM failures
//!
//! All errors are wrapped in `ProcessingNodeError` for unified handling.
//!
//! # Design Rationale
//!
//! ## Why Synchronous?
//! WASM execution is CPU-bound and benefits from:
//! - **No async overhead**: Direct function calls
//! - **Simpler error handling**: No async error propagation
//! - **Better performance**: Eliminates async runtime overhead
//! - **Clear separation**: Async at processor level, sync at execution level
//!
//! ## Thread Safety
//! The `Send + Sync` bounds enable:
//! - Sharing executors across async tasks
//! - Concurrent execution in thread pools
//! - Safe use with `Arc<dyn ProcessingNodeExecutor>`

use std::fmt;

/// Synchronous WASM executor trait for CPU-bound processor operations.
///
/// This trait defines the interface for executing WASM processors synchronously.
/// Implementations handle strategy-specific details like memory management,
/// function calling conventions, and WASI integration.
///
/// ## Thread Safety
/// - **Send**: Can be transferred between threads
/// - **Sync**: Can be shared between threads (typically via `Arc`)
///
/// ## Implementations
/// - `WitNodeExecutor`: Component Model with canonical ABI
/// - `CStyleNodeExecutor`: Classic WASM with manual memory
///
/// ## Design
/// Synchronous by design for CPU-bound operations:
/// - No async overhead for compute-intensive work
/// - Direct function calls without async machinery
/// - Simpler error handling and stack traces
pub trait ProcessingNodeExecutor: Send + Sync {
    fn execute(&self, input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError>;
    fn artifact_type(&self) -> &'static str;
    fn capabilities(&self) -> Vec<String>;
    fn execution_metadata(&self) -> ExecutionMetadata;
}

/// Metadata about a WASM processor execution environment.
///
/// Provides introspection into the loaded WASM artifact including its type,
/// capabilities, and import requirements.
///
/// # Fields
/// - **module_path**: Original file path of the WASM artifact
/// - **artifact_type**: Type of artifact ("WIT Component", "C-Style", etc.)
/// - **import_count**: Number of imports required by the module
/// - **capabilities**: List of supported capabilities and features
#[derive(Debug, Clone)]
pub struct ExecutionMetadata {
    pub module_path: String,
    pub artifact_type: String,
    pub import_count: usize,
    pub capabilities: Vec<String>,
}

use crate::backends::wasm::error::WasmError;

/// Top-level error type for WASM processing node execution.
///
/// This enum wraps strategy-specific errors and provides common error categories
/// for validation, input handling, and runtime failures.
///
/// # Variants
/// - **ComponentError**: Component Model execution failures
/// - **WasiError**: WASI-related failures
/// - **CStyleError**: C-Style module execution failures
/// - **InputError**: Invalid input data
/// - **ValidationError**: Module validation failures
/// - **RuntimeError**: Generic runtime errors
#[derive(Debug)]
pub enum ProcessingNodeError {
    /// Component Model execution error.
    ComponentError(ComponentExecutionError),

    /// WASI execution error.
    WasiError(WasiExecutionError),

    /// C-Style module execution error.
    CStyleError(CStyleExecutionError),

    /// Invalid input data error.
    InputError(String),

    /// Module validation error.
    ValidationError(String),

    /// Generic runtime error.
    RuntimeError(String),
}

impl From<std::string::String> for ProcessingNodeError {
    fn from(error: String) -> Self {
        ProcessingNodeError::RuntimeError(error)
    }
}

/// Errors specific to Component Model execution.
///
/// These errors occur during Component Model component instantiation,
/// interface binding, or function execution.
#[derive(Debug)]
pub enum ComponentExecutionError {
    /// Component instantiation failed.
    InstantiationFailed(String),

    /// Required WIT interface not found in component.
    InterfaceNotFound(String),

    /// Component function call failed.
    FunctionCallFailed(String),

    /// Memory allocation failed in component.
    MemoryAllocationFailed(String),

    /// Memory access failed in component.
    MemoryAccessFailed(String),
}

/// Errors specific to WASI execution.
///
/// These errors occur during WASI context setup or WASI function calls.
#[derive(Debug)]
pub enum WasiExecutionError {
    /// WASI context creation failed.
    ContextCreationFailed(String),

    /// Required WASI function not available.
    FunctionNotAvailable(String),

    /// WASI runtime error.
    RuntimeError(String),

    /// WASI memory error.
    MemoryError(String),
}

/// Errors specific to C-Style module execution.
///
/// These errors occur during C-Style module function lookup, validation,
/// or execution with manual memory management.
#[derive(Debug)]
pub enum CStyleExecutionError {
    /// Required function export not found.
    FunctionNotFound(String),

    /// Function signature doesn't match expected type.
    SignatureMismatch(String),

    /// Memory allocation failed.
    AllocationFailed(String),

    /// Function execution failed.
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
                write!(
                    f,
                    "Required interface '{}' not found in component",
                    interface
                )
            }
            ComponentExecutionError::FunctionCallFailed(msg) => {
                write!(f, "Component function call failed: {}", msg)
            }
            ComponentExecutionError::MemoryAllocationFailed(msg) => {
                write!(f, "Component memory allocation failed: {}", msg)
            }
            ComponentExecutionError::MemoryAccessFailed(msg) => {
                write!(f, "Component memory access failed: {}", msg)
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
                write!(
                    f,
                    "Required function '{}' not found in module exports",
                    func
                )
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

impl From<ProcessingNodeError> for WasmError {
    fn from(error: ProcessingNodeError) -> Self {
        WasmError::ProcessorError(error.to_string())
    }
}
