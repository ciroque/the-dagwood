// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Centralized message types for structured logging.
//!
//! This module contains all message types used throughout The DAGwood project
//! for diagnostic and operational logging. Each message type implements the
//! `Display` trait to provide consistent, human-readable output while enabling
//! future internationalization.
//!
//! # Organization
//!
//! Messages are organized by subsystem to maintain Single Responsibility Principle:
//!
//! * `engine` - DAG executor lifecycle and execution events
//! * `processor` - Processor execution and lifecycle events  
//! * `validation` - Configuration validation warnings and errors
//! * `wasm` - WASM backend loading and execution events
//!
//! # Usage Pattern
//!
//! ```rust
//! use the_dagwood::observability::messages::engine::ExecutionStarted;
//!
//! let msg = ExecutionStarted {
//!     strategy: "WorkQueue",
//!     processor_count: 5,
//!     max_concurrency: 4,
//! };
//!
//! tracing::info!("{}", msg);
//! ```

pub mod engine;
pub mod processor;
pub mod validation;
pub mod wasm;
