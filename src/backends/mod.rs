// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Processor backend implementations for The DAGwood workflow orchestration system.
//!
//! This module provides pluggable processor backends that enable different execution
//! strategies and runtime environments. Each backend implements the `Processor` trait
//! and can be instantiated through configuration-driven factories.
//!
//! # Available Backends
//!
//! ## Local Backend
//! In-process Rust processors for text manipulation and analysis:
//! - **Text Transformation**: Case conversion, reversal, prefix/suffix addition
//! - **Text Analysis**: Token counting, word frequency analysis
//! - **Performance**: Zero-overhead native execution
//! - **Use Case**: Built-in processors, testing, rapid prototyping
//!
//! ## WASM Backend
//! Sandboxed WebAssembly execution with multiple strategies:
//! - **C-Style Modules**: Classic WASM with manual memory management
//! - **WIT Components**: Modern Component Model with automatic memory handling
//! - **Security**: Full sandboxing with configurable WASI capabilities
//! - **Use Case**: Untrusted code, polyglot processors, plugin systems
//!
//! ## Stub Backend (Test-Only)
//! Testing utilities for executor development (only available in test builds):
//! - **StubProcessor**: No-op processor for DAG structure testing
//! - **FailingProcessor**: Simulates failures for error handling tests
//! - **NoOutcomeProcessor**: Tests invalid response scenarios
//! - **Use Case**: Unit testing, integration testing, benchmarking
//! - **Note**: NOT available in production builds
//!
//! # Architecture
//!
//! All backends follow a consistent factory pattern:
//! ```text
//! Configuration → Factory → Processor Instance → Executor
//! ```
//!
//! Each backend provides:
//! - **Factory**: Creates processor instances from configuration
//! - **Processors**: Implement the `Processor` trait
//! - **Error Handling**: Backend-specific error types and conversions
//!
//! # Examples
//!
//! ## Using Local Backend
//! ```rust
//! use the_dagwood::backends::local::LocalProcessorFactory;
//! use the_dagwood::config::{ProcessorConfig, BackendType};
//! use std::collections::HashMap;
//!
//! let config = ProcessorConfig {
//!     id: "uppercase".to_string(),
//!     backend: BackendType::Local,
//!     processor: Some("change_text_case_upper".to_string()),
//!     endpoint: None,
//!     module: None,
//!     depends_on: vec![],
//!     options: HashMap::new(),
//! };
//!
//! let processor = LocalProcessorFactory::create_processor(&config)?;
//! # Ok::<(), String>(())
//! ```
//!
//! ## Using WASM Backend
//! ```rust,no_run
//! use the_dagwood::backends::wasm::WasmProcessor;
//!
//! let processor = WasmProcessor::new(
//!     "wasm_processor".to_string(),
//!     "path/to/module.wasm".to_string(),
//! )?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod local;
#[cfg(test)]
pub mod stub;
pub mod wasm;
