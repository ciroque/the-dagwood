// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Message types for processor execution and lifecycle events.
//!
//! This module contains message types for logging events related to:
//! * Processor instantiation and configuration
//! * Processor execution lifecycle (start, completion, failure)
//! * Processor input/output handling
//! * Processor metadata collection

use std::fmt::{Display, Formatter};

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
///     processor_id: "uppercase_processor",
///     error: &error,
/// };
///
/// tracing::error!("{}", msg);
/// ```
pub struct ProcessorExecutionFailed<'a> {
    pub processor_id: &'a str,
    pub error: &'a dyn std::error::Error,
}

impl Display for ProcessorExecutionFailed<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Processor '{}' execution failed: {}",
            self.processor_id, self.error
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
/// let msg = ProcessorInstantiationFailed {
///     processor_id: "unknown_processor",
///     backend: "local",
///     reason: "Unknown processor implementation",
/// };
///
/// tracing::error!("{}", msg);
/// ```
pub struct ProcessorInstantiationFailed<'a> {
    pub processor_id: &'a str,
    pub backend: &'a str,
    pub reason: &'a str,
}

impl Display for ProcessorInstantiationFailed<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Failed to instantiate processor '{}' with backend '{}': {}",
            self.processor_id, self.backend, self.reason
        )
    }
}

/// Processor fallback to stub implementation.
///
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
