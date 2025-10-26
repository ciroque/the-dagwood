// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Centralized message types for structured logging and distributed tracing.
//!
//! This module contains all message types used throughout The DAGwood project
//! for diagnostic and operational logging. Each message type implements:
//!
//! * `Display` - Human-readable output (supports future i18n)
//! * `StructuredLog` - Machine-readable fields + OpenTelemetry span creation
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
//! # Usage Patterns
//!
//! ## Basic Logging (Human-Readable)
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
//!
//! ## Structured Logging (Machine-Readable)
//! ```rust
//! use the_dagwood::observability::messages::{StructuredLog, engine::ExecutionStarted};
//!
//! let msg = ExecutionStarted {
//!     strategy: "WorkQueue",
//!     processor_count: 5,
//!     max_concurrency: 4,
//! };
//!
//! // Emits both human-readable message AND structured fields
//! msg.log();
//! ```
//!
//! ## Distributed Tracing (OpenTelemetry)
//! ```rust
//! use the_dagwood::observability::messages::{StructuredLog, engine::ExecutionStarted};
//!
//! let msg = ExecutionStarted {
//!     strategy: "WorkQueue",
//!     processor_count: 5,
//!     max_concurrency: 4,
//! };
//!
//! // Create span with message fields as attributes
//! let span = msg.span("dag_execution");
//! let _guard = span.enter();
//!
//! // ... work happens here with span context ...
//! ```

pub mod engine;
pub mod processor;
pub mod validation;
pub mod wasm;

use tracing::Span;

/// Trait for messages that support structured logging and distributed tracing.
///
/// This trait provides two key capabilities:
///
/// 1. **Structured Logging** - Emit log events with machine-readable fields
///    for querying, metrics extraction, and alerting
/// 2. **Distributed Tracing** - Create OpenTelemetry spans with attributes
///    for end-to-end request tracing and performance analysis
///
/// # Benefits
///
/// ## Structured Fields
/// - **Queryable**: Filter logs by field values without string parsing
/// - **Metrics**: Automatically extract metrics from log fields
/// - **Alerting**: Create alerts based on field values (e.g., `processor_count > 10`)
/// - **i18n-Ready**: Fields are language-independent, only messages change
///
/// ## Distributed Tracing
/// - **Request Flow**: See entire DAG execution as a trace with nested spans
/// - **Performance**: Automatic timing capture for each span
/// - **Context Propagation**: Spans automatically propagate trace context
/// - **Filtering**: Query traces by span attributes (strategy, processor_id, etc.)
///
/// # Example: Structured Logging
/// ```rust
/// use the_dagwood::observability::messages::{StructuredLog, engine::ExecutionStarted};
///
/// let msg = ExecutionStarted {
///     strategy: "WorkQueue",
///     processor_count: 5,
///     max_concurrency: 4,
/// };
///
/// // Emits: INFO message + fields {strategy, processor_count, max_concurrency}
/// msg.log();
/// ```
///
/// # Example: Distributed Tracing
/// ```rust
/// use the_dagwood::observability::messages::{StructuredLog, engine::ExecutionStarted};
///
/// let msg = ExecutionStarted {
///     strategy: "WorkQueue",
///     processor_count: 5,
///     max_concurrency: 4,
/// };
///
/// // Create span with attributes
/// let span = msg.span("dag_execution");
/// let _guard = span.enter();
///
/// // All logs/spans created here will be children of this span
/// // Span automatically closed when _guard is dropped
/// ```
///
/// # JSON Output Example
/// With a JSON formatter (e.g., `tracing-subscriber` with JSON layer):
/// ```json
/// {
///   "timestamp": "2025-10-25T17:28:00Z",
///   "level": "INFO",
///   "message": "Starting DAG execution with WorkQueue strategy: 5 processors, max_concurrency=4",
///   "fields": {
///     "strategy": "WorkQueue",
///     "processor_count": 5,
///     "max_concurrency": 4
///   },
///   "span": {
///     "name": "dag_execution",
///     "trace_id": "abc123..."
///   }
/// }
/// ```
pub trait StructuredLog {
    /// Emit a log event with structured fields.
    ///
    /// This logs both:
    /// - Human-readable message (via `Display` trait)
    /// - Machine-readable fields for querying and metrics
    ///
    /// The appropriate log level (info, warn, error) is determined by the
    /// message type's semantic meaning.
    ///
    /// # Example
    /// ```rust
    /// use the_dagwood::observability::messages::{StructuredLog, engine::ExecutionStarted};
    ///
    /// ExecutionStarted {
    ///     strategy: "WorkQueue",
    ///     processor_count: 5,
    ///     max_concurrency: 4,
    /// }.log();
    /// ```
    fn log(&self);

    /// Create an OpenTelemetry span with this message's fields as attributes.
    ///
    /// The span includes all message fields as attributes, enabling:
    /// - Filtering traces by attribute values
    /// - Automatic metrics extraction from span attributes
    /// - Context propagation across service boundaries
    ///
    /// # Arguments
    /// * `name` - The span name (e.g., "dag_execution", "processor_execution")
    ///
    /// # Returns
    /// A `tracing::Span` that can be entered to create trace context.
    /// The span is automatically closed when dropped.
    ///
    /// # Example
    /// ```rust
    /// use the_dagwood::observability::messages::{StructuredLog, engine::ExecutionStarted};
    ///
    /// let msg = ExecutionStarted {
    ///     strategy: "WorkQueue",
    ///     processor_count: 5,
    ///     max_concurrency: 4,
    /// };
    ///
    /// let span = msg.span("dag_execution");
    /// let _guard = span.enter();
    /// // Work happens here with span context
    /// // Span automatically closed when _guard drops
    /// ```
    fn span(&self, name: &str) -> Span;
}
