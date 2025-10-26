// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Message types for processor execution and lifecycle events.
//!
//! This module contains message types for logging events related to:
//! * Processor instantiation and configuration
//! * Processor execution lifecycle (start, completion, failure)
//! * Processor input/output handling
//! * Processor metadata collection

use crate::observability::messages::StructuredLog;
use std::fmt::{Display, Formatter};
use tracing::Span;

/// Processor execution started.
///
/// # Log Level
/// `info!` - Important operational event
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::processor::ProcessorExecutionStarted;
///
/// let msg = ProcessorExecutionStarted {
///     processor_id: "uppercase_processor",
///     input_size: 1024,
/// };
///
/// tracing::info!("{}", msg);
/// ```
pub struct ProcessorExecutionStarted<'a> {
    pub processor_id: &'a str,
    pub input_size: usize,
}

impl Display for ProcessorExecutionStarted<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Processor '{}' execution started: input_size={} bytes",
            self.processor_id, self.input_size
        )
    }
}

impl StructuredLog for ProcessorExecutionStarted<'_> {
    fn log(&self) {
        tracing::info!(
            processor_id = self.processor_id,
            input_size = self.input_size,
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::info_span!(
            "processor_execution_started",
            span_name = name,
            processor_id = self.processor_id,
            input_size = self.input_size,
        )
    }
}

/// Processor execution completed successfully.
///
/// # Log Level
/// `info!` - Important operational event
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::processor::ProcessorExecutionCompleted;
/// use std::time::Duration;
///
/// let msg = ProcessorExecutionCompleted {
///     processor_id: "uppercase_processor",
///     input_size: 1024,
///     output_size: 1024,
///     duration: Duration::from_millis(10),
/// };
///
/// tracing::info!("{}", msg);
/// ```
pub struct ProcessorExecutionCompleted<'a> {
    pub processor_id: &'a str,
    pub input_size: usize,
    pub output_size: usize,
    pub duration: std::time::Duration,
}

impl Display for ProcessorExecutionCompleted<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Processor '{}' completed: input={} bytes, output={} bytes, duration={:?}",
            self.processor_id, self.input_size, self.output_size, self.duration
        )
    }
}

impl StructuredLog for ProcessorExecutionCompleted<'_> {
    fn log(&self) {
        tracing::info!(
            processor_id = self.processor_id,
            input_size = self.input_size,
            output_size = self.output_size,
            duration_ms = self.duration.as_millis() as u64,
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::info_span!(
            "processor_execution_completed",
            span_name = name,
            processor_id = self.processor_id,
            input_size = self.input_size,
            output_size = self.output_size,
            duration = ?self.duration,
        )
    }
}

/// Processor execution failed.
///
/// # Log Level
/// `error!` - Failure requiring attention
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::processor::ProcessorExecutionFailed;
///
/// let error = std::io::Error::new(std::io::ErrorKind::Other, "test error");
/// let msg = ProcessorExecutionFailed {
///     processor_id: "reverse_text",
///     error: &error,
/// };
/// ```
pub struct ProcessorExecutionFailed<'a> {
    pub processor_id: &'a str,
    pub error: &'a dyn std::error::Error,
}

impl Display for ProcessorExecutionFailed<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Processor '{}' execution failed: {}",
            self.processor_id, self.error
        )
    }
}

impl StructuredLog for ProcessorExecutionFailed<'_> {
    fn log(&self) {
        tracing::error!(
            processor_id = self.processor_id,
            error = %self.error,
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::error_span!(
            "processor_execution_failed",
            span_name = name,
            processor_id = self.processor_id,
            error = %self.error,
        )
    }
}

/// Processor instantiation failed.
///
/// # Log Level
/// `error!` - Failure requiring attention
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::processor::ProcessorInstantiationFailed;
///
/// let error = std::io::Error::new(std::io::ErrorKind::Other, "test error");
/// let msg = ProcessorInstantiationFailed {
///     processor_id: "wasm_processor",
///     backend: "wasm",
///     error: &error,
/// };
/// ```
pub struct ProcessorInstantiationFailed<'a> {
    pub processor_id: &'a str,
    pub backend: &'a str,
    pub error: &'a dyn std::error::Error,
}

impl Display for ProcessorInstantiationFailed<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed to instantiate processor '{}' (backend: {}): {}",
            self.processor_id, self.backend, self.error
        )
    }
}

impl StructuredLog for ProcessorInstantiationFailed<'_> {
    fn log(&self) {
        tracing::error!(
            processor_id = self.processor_id,
            backend = self.backend,
            error = %self.error,
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::error_span!(
            "processor_instantiation_failed",
            span_name = name,
            processor_id = self.processor_id,
            backend = self.backend,
            error = %self.error,
        )
    }
}

/// Processor fallback to stub implementation.
/// # Log Level
/// `warn!` - Potential issue or degraded behavior
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::processor::ProcessorFallbackToStub;
///
/// let msg = ProcessorFallbackToStub {
///     processor_id: "missing_processor",
///     reason: "Implementation not found",
/// };
///
/// tracing::warn!("{}", msg);
/// ```
pub struct ProcessorFallbackToStub<'a> {
    pub processor_id: &'a str,
    pub reason: &'a str,
}

impl Display for ProcessorFallbackToStub<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Processor '{}' falling back to stub implementation: {}",
            self.processor_id, self.reason
        )
    }
}

impl StructuredLog for ProcessorFallbackToStub<'_> {
    fn log(&self) {
        tracing::warn!(
            processor_id = self.processor_id,
            reason = self.reason,
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::warn_span!(
            "processor_fallback",
            span_name = name,
            processor_id = self.processor_id,
            reason = self.reason,
        )
    }
}
