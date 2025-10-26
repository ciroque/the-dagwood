// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Observability module for structured logging, tracing, and metrics.
//!
//! This module provides centralized message types for all diagnostic and operational
//! logging throughout The DAGwood project. Message types follow a struct-based pattern
//! with `Display` trait implementation to:
//!
//! * Eliminate magic strings scattered throughout the codebase
//! * Enable future internationalization without code changes
//! * Maintain Single Responsibility Principle (SRP)
//! * Provide consistent, structured logging output
//!
//! # Architecture
//!
//! Messages are organized by subsystem:
//! * `messages::engine` - DAG executor lifecycle and execution events
//! * `messages::processor` - Processor execution and lifecycle events
//! * `messages::validation` - Configuration validation warnings and errors
//! * `messages::wasm` - WASM backend loading and execution events
//!
//! # Usage
//!
//! ```rust
//! use the_dagwood::observability::messages::processor::ProcessorExecutionFailed;
//!
//! let error = std::io::Error::new(std::io::ErrorKind::Other, "test error");
//! let msg = ProcessorExecutionFailed {
//!     processor_id: "my_processor",
//!     error: &error,
//! };
//!
//! tracing::error!("{}", msg);
//! ```
//!
//! # Design Decisions
//!
//! See ADR 18 for detailed rationale on observability implementation choices.

pub mod messages;
