// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WASM executor strategy implementations for different artifact types.
//!
//! This module provides concrete implementations of the `ProcessingNodeExecutor` trait,
//! each optimized for a specific WASM artifact type. The strategy pattern enables clean
//! separation between different execution approaches while maintaining a unified interface.
//!
//! # Available Strategies
//!
//! ## WitNodeExecutor (Modern Component Model)
//! The preferred executor for modern WebAssembly Component Model components:
//! - **Memory Management**: Automatic via canonical ABI
//! - **Interface**: Type-safe WIT bindings generated at compile time
//! - **WASI**: Full WASI Preview 2 support with capability control
//! - **Performance**: Minimal overhead with optimized memory handling
//! - **Use Case**: New processors, polyglot workflows, secure sandboxing
//!
//! ### Key Features
//! - Automatic `cabi_realloc` handling
//! - Type-safe function calls via wit-bindgen
//! - Comprehensive error handling with context
//! - Zero manual memory management
//!
//! ## CStyleNodeExecutor (Legacy Core Modules)
//! Executor for classic WASM modules with C-style interfaces:
//! - **Memory Management**: Manual via `alloc`/`dealloc` exports
//! - **Interface**: Direct function exports (`process`, `alloc`, `dealloc`)
//! - **WASI**: Optional, limited support
//! - **Performance**: Low overhead but requires careful memory handling
//! - **Use Case**: Legacy modules, maximum control, minimal dependencies
//!
//! ### Key Features
//! - Direct memory control
//! - Simple function call convention
//! - Minimal runtime overhead
//! - Compatible with C/C++/Rust compiled to WASM
//!
//! # Strategy Selection
//!
//! The factory (`factory.rs`) automatically selects the appropriate executor based on
//! WASM binary encoding detected by `detector.rs`:
//! ```text
//! Component Model binary → WitNodeExecutor
//! Classic WASM module    → CStyleNodeExecutor
//! ```
//!
//! # Architecture
//!
//! All executors implement the `ProcessingNodeExecutor` trait:
//! ```rust,ignore
//! pub trait ProcessingNodeExecutor: Send + Sync {
//!     fn execute(&self, input: &[u8]) -> Result<Vec<u8>, ProcessingNodeError>;
//!     fn artifact_type(&self) -> &'static str;
//!     fn capabilities(&self) -> Vec<String>;
//!     fn execution_metadata(&self) -> ExecutionMetadata;
//! }
//! ```
//!
//! This enables:
//! - Uniform interface regardless of strategy
//! - Type-erased storage via `Arc<dyn ProcessingNodeExecutor>`
//! - Easy addition of new strategies
//!
//! # Examples
//!
//! ## Using WitNodeExecutor
//! ```rust,no_run
//! use the_dagwood::backends::wasm::executors::WitNodeExecutor;
//! use the_dagwood::backends::wasm::ProcessingNodeExecutor;
//! use wasmtime::Engine;
//! use wasmtime::component::Component;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let engine = Engine::default();
//! let bytes = std::fs::read("component.wasm")?;
//! let component = Component::new(&engine, &bytes)?;
//!
//! let executor = WitNodeExecutor::new(component, engine, 100_000_000)?;
//! let output = executor.execute(b"input data")?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Using CStyleNodeExecutor
//! ```rust,no_run
//! use the_dagwood::backends::wasm::executors::CStyleNodeExecutor;
//! use the_dagwood::backends::wasm::ProcessingNodeExecutor;
//! use wasmtime::{Engine, Module};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let engine = Engine::default();
//! let bytes = std::fs::read("module.wasm")?;
//! let module = Module::new(&engine, &bytes)?;
//!
//! let executor = CStyleNodeExecutor::new(module, engine, 100_000_000)?;
//! let output = executor.execute(b"input data")?;
//! # Ok(())
//! # }
//! ```

mod cstyle_executor;
mod wit_executor;

pub use cstyle_executor::CStyleNodeExecutor;
pub use wit_executor::WitNodeExecutor;
