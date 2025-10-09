// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WasmError {
    #[error("Memory error: {0}")]
    MemoryError(String),
    
    #[error("Execution timed out after {0:?}")]
    Timeout(Duration),
    
    #[error("Invalid pointer: {0}")]
    InvalidPointer(i32),
    
    #[error("Memory access out of bounds: {0}")]
    OutOfBounds(String),
    
    #[error("WASM module error: {0}")]
    ModuleError(String),
    
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    
    #[error("WASM execution error: {0}")]
    ExecutionError(#[from] wasmtime::Error),
    
    #[error("Invalid input: {0}")]
    ValidationError(String),
}


pub type WasmResult<T> = Result<T, WasmError>;
