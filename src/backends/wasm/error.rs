// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Error types for WASM backend operations.
//!
//! This module defines comprehensive error types for all WASM-related operations
//! including loading, parsing, validation, and execution. All errors implement
//! `std::error::Error` via the `thiserror` crate for consistent error handling.

use std::time::Duration;
use thiserror::Error;

/// Error message for unsupported legacy Preview 1 Component Model binaries.
///
/// Legacy Preview 1 components (version 1 with "component" custom section) are
/// not supported. Users should upgrade to modern Component Model (version 2+)
/// or use classic core WASM modules.
pub const WASM_UNSUPPORTED_ENCODING: &str = "Unsupported WASM binary: Legacy Preview 1 Component Model detected. \
Please upgrade to modern Component Model (binary version 2+) or use classic WASM.";

/// Comprehensive error type for all WASM backend operations.
///
/// This enum covers errors from all stages of WASM processing:
/// - Binary loading and validation
/// - Encoding detection and parsing
/// - Module/component compilation
/// - Runtime execution
/// - Memory management
///
/// All variants include descriptive messages for debugging and user feedback.
#[derive(Error, Debug)]
pub enum WasmError {
    /// Invalid or malformed WASM binary format.
    #[error("Invalid WASM binary: {0}")]
    InvalidWasmBinary(String),

    /// Unknown or unrecognized WASM encoding type.
    #[error("Unknown WASM encoding: {0}")]
    UnknownEncoding(String),

    /// Memory allocation or access error in WASM linear memory.
    #[error("Memory error: {0}")]
    MemoryError(String),

    /// Execution exceeded configured timeout duration.
    #[error("Execution timed out after {0:?}")]
    Timeout(Duration),

    /// Invalid pointer value (null or out of bounds).
    #[error("Invalid pointer: {0}")]
    InvalidPointer(i32),

    /// Memory access outside valid bounds.
    #[error("Memory access out of bounds: {0}")]
    OutOfBounds(String),

    /// Module compilation or instantiation error.
    #[error("WASM module error: {0}")]
    ModuleError(String),

    /// File I/O error during module loading.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// UTF-8 decoding error for string data.
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    /// Wasmtime runtime execution error.
    #[error("WASM execution error: {0}")]
    ExecutionError(#[from] wasmtime::Error),

    /// Input validation error (size limits, format, etc.).
    #[error("Invalid input: {0}")]
    ValidationError(String),

    /// Wasmtime engine creation or configuration error.
    #[error("Engine creation error: {0}")]
    EngineError(String),

    /// String conversion or encoding error.
    #[error("String conversion error: {0}")]
    StringError(String),

    /// High-level processor operation error.
    #[error("Processor error: {0}")]
    ProcessorError(String),

    /// Unsupported WASM encoding (e.g., legacy Preview 1).
    #[error("Unsupported encoding: {0}")]
    UnsupportedEncoding(String),

    /// WASM binary parsing error from wasmparser.
    #[error("WASM parser error: {0}")]
    ParserError(#[from] wasmparser::BinaryReaderError),
}

/// Result type alias for WASM operations.
///
/// Convenience type for functions returning `Result<T, WasmError>`.
pub type WasmResult<T> = Result<T, WasmError>;
