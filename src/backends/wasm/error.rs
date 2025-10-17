// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use std::time::Duration;
use thiserror::Error;

pub const WASM_UNSUPPORTED_ENCODING: &str = "Unsupported WASM binary: Legacy Preview 1 Component Model detected. \
Please upgrade to modern Component Model (binary version 2+) or use classic WASM.";


#[derive(Error, Debug)]
pub enum WasmError {
    #[error("Invalid WASM binary: {0}")]
    InvalidWasmBinary(String),

    #[error("Unknown WASM encoding: {0}")]
    UnknownEncoding(String),

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

    #[error("Engine creation error: {0}")]
    EngineError(String),

    #[error("String conversion error: {0}")]
    StringError(String),

    #[error("Processor error: {0}")]
    ProcessorError(String),

    #[error("Unsupported encoding: {0}")]
    UnsupportedEncoding(String),

    #[error("WASM parser error: {0}")]
    ParserError(#[from] wasmparser::BinaryReaderError),
}

pub type WasmResult<T> = Result<T, WasmError>;
