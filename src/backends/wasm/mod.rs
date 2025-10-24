// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WebAssembly processor backend with sandboxed execution and multiple execution strategies.
//!
//! This module provides a comprehensive WASM backend that supports both classic core WASM modules
//! and modern Component Model components. It implements a clean three-step execution flow (ADR-17)
//! with automatic strategy selection based on binary encoding detection.
//!
//! # Architecture Overview
//!
//! The WASM backend implements a sophisticated multi-strategy execution system:
//! ```text
//! WASM Binary → Encoding Detection → Strategy Selection → Execution
//!      ↓                  ↓                     ↓              ↓
//!  load_wasm_bytes()  wasm_encoding()    create_executor()   execute()
//! ```
//!
//! ## Key Components
//!
//! ### Core Flow (ADR-17)
//! 1. **Loader** (`loader.rs`): Load and validate WASM binaries from disk
//! 2. **Detector** (`detector.rs`): Detect encoding (Component Model vs Classic)
//! 3. **Factory** (`factory.rs`): Create appropriate executor for encoding type
//! 4. **Executor** (`executors/`): Execute WASM with strategy-specific logic
//!
//! ### Execution Strategies
//! - **WitNodeExecutor**: Modern Component Model with WIT bindings
//!   - Automatic memory management via canonical ABI
//!   - WASI Preview 2 support
//!   - Type-safe interface generation
//! - **CStyleNodeExecutor**: Classic core WASM modules
//!   - Manual memory management
//!   - Direct function exports
//!   - Legacy compatibility
//!
//! ### Supporting Infrastructure
//! - **Capability Manager**: Engine configuration for different encoding types
//! - **Error Handling**: Comprehensive error types with context
//! - **Processing Node**: Synchronous executor trait for CPU-bound operations
//!
//! # Execution Strategies
//!
//! ## WIT Component Model (Modern)
//! The preferred approach using WebAssembly Component Model:
//! - **Memory Management**: Automatic via canonical ABI
//! - **Interface**: Type-safe WIT bindings
//! - **WASI**: Preview 2 with full capability control
//! - **Performance**: Minimal overhead with optimized memory handling
//! - **Use Case**: New processors, polyglot workflows, secure sandboxing
//!
//! ## C-Style Core Modules (Legacy)
//! Classic WASM modules with manual memory management:
//! - **Memory Management**: Manual allocation/deallocation
//! - **Interface**: Direct function exports (process, alloc, dealloc)
//! - **WASI**: Optional, limited support
//! - **Performance**: Low overhead but requires careful memory handling
//! - **Use Case**: Legacy modules, maximum control, minimal dependencies
//!
//! # ADR-17: Clean Three-Step Flow
//!
//! The WASM backend implements ADR-17's clean separation of concerns:
//!
//! ## Step 1: Load Binary
//! ```rust,no_run
//! use the_dagwood::backends::wasm::load_wasm_bytes;
//!
//! let bytes = load_wasm_bytes("processor.wasm")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Step 2: Detect Encoding
//! ```rust,no_run
//! use the_dagwood::backends::wasm::{load_wasm_bytes, wasm_encoding};
//!
//! let bytes = load_wasm_bytes("processor.wasm")?;
//! let encoding = wasm_encoding(&bytes)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Step 3: Create Executor
//! ```rust,no_run
//! use the_dagwood::backends::wasm::{load_wasm_bytes, wasm_encoding, create_executor};
//!
//! let bytes = load_wasm_bytes("processor.wasm")?;
//! let encoding = wasm_encoding(&bytes)?;
//! let executor = create_executor(&bytes, encoding)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Security & Sandboxing
//!
//! The WASM backend provides strong isolation guarantees:
//! - **Memory Isolation**: WASM linear memory separate from host
//! - **Capability-Based Security**: WASI capabilities control system access
//! - **Deterministic Execution**: No ambient authority or hidden state
//! - **Resource Limits**: Configurable fuel consumption and memory limits
//!
//! # Performance Characteristics
//!
//! - **Overhead**: ~1-10µs for simple operations (depends on strategy)
//! - **Memory**: Isolated linear memory + host overhead
//! - **Throughput**: Near-native for compute-bound operations
//! - **Latency**: Sub-millisecond for typical text processing
//!
//! # Examples
//!
//! ## Complete Processor Creation
//! ```rust,no_run
//! use the_dagwood::backends::wasm::WasmProcessor;
//! use the_dagwood::traits::Processor;
//! use the_dagwood::proto::processor_v1::ProcessorRequest;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let processor = WasmProcessor::new(
//!     "text_processor".to_string(),
//!     "processors/text_transform.wasm".to_string(),
//! )?;
//!
//! let request = ProcessorRequest {
//!     payload: b"hello world".to_vec(),
//! };
//!
//! let response = processor.process(request).await;
//! # Ok(())
//! # }
//! ```
//!
//! ## Low-Level Executor Usage
//! ```rust,no_run
//! use the_dagwood::backends::wasm::{load_wasm_bytes, wasm_encoding, create_executor};
//! use the_dagwood::backends::wasm::ProcessingNodeExecutor;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let bytes = load_wasm_bytes("processor.wasm")?;
//! let encoding = wasm_encoding(&bytes)?;
//! let executor = create_executor(&bytes, encoding)?;
//!
//! let input = b"test input";
//! let output = executor.execute(input)?;
//! # Ok(())
//! # }
//! ```

pub mod bindings;
pub mod capability_manager;
pub mod detector;
pub mod factory;
pub mod loader;
mod error;
pub mod executors;
pub mod processing_node;
pub mod processor;

pub use error::{WasmError, WasmResult};

pub use detector::{wasm_encoding, WasmEncoding};
pub use factory::create_executor;
pub use loader::load_wasm_bytes;

pub use executors::{CStyleNodeExecutor, WitNodeExecutor};

pub use processing_node::{ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor};

pub use processor::WasmProcessor;
