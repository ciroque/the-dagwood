// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use std::fmt;

pub trait ProcessingNodeExecutor: Send + Sync {
    fn execute(&self, input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError>;
    fn artifact_type(&self) -> &'static str;
    fn capabilities(&self) -> Vec<String>;
    fn execution_metadata(&self) -> ExecutionMetadata;
}

#[derive(Debug, Clone)]
pub struct ExecutionMetadata {
    pub module_path: String,
    pub artifact_type: String,
    pub import_count: usize,
    pub capabilities: Vec<String>,
}

use crate::backends::wasm::error::WasmError;

#[derive(Debug)]
pub enum ProcessingNodeError {
    ComponentError(ComponentExecutionError),

    WasiError(WasiExecutionError),

    CStyleError(CStyleExecutionError),

    InputError(String),

    ValidationError(String),

    RuntimeError(String),
}

#[derive(Debug)]
pub enum ComponentExecutionError {
    InstantiationFailed(String),

    InterfaceNotFound(String),

    FunctionCallFailed(String),

    MemoryAllocationFailed(String),

    MemoryAccessFailed(String),
}

#[derive(Debug)]
pub enum WasiExecutionError {
    ContextCreationFailed(String),

    FunctionNotAvailable(String),

    RuntimeError(String),

    MemoryError(String),
}

#[derive(Debug)]
pub enum CStyleExecutionError {
    FunctionNotFound(String),

    SignatureMismatch(String),

    AllocationFailed(String),

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

// Convert ProcessingNodeError to WasmError for better error handling
impl From<ProcessingNodeError> for WasmError {
    fn from(error: ProcessingNodeError) -> Self {
        WasmError::ProcessorError(error.to_string())
    }
}
