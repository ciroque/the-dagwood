// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Message types for DAG executor lifecycle and execution events.
//!
//! This module contains message types for logging events related to:
//! * Executor initialization and configuration
//! * DAG execution lifecycle (start, completion, failure)
//! * Execution strategy selection
//! * Concurrency and resource management

use crate::observability::messages::StructuredLog;
use std::fmt::{Display, Formatter};
use tracing::Span;

/// Execution started with specified strategy and configuration.
///
/// # Log Level
/// `info!` - Important operational event
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::engine::ExecutionStarted;
///
/// let msg = ExecutionStarted {
///     strategy: "WorkQueue",
///     processor_count: 5,
///     max_concurrency: 4,
/// };
///
/// tracing::info!("{}", msg);
/// ```
pub struct ExecutionStarted<'a> {
    pub strategy: &'a str,
    pub processor_count: usize,
    pub max_concurrency: usize,
}

impl Display for ExecutionStarted<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Starting DAG execution with {} strategy: {} processors, max_concurrency={}",
            self.strategy, self.processor_count, self.max_concurrency
        )
    }
}

impl StructuredLog for ExecutionStarted<'_> {
    fn log(&self) {
        tracing::info!(
            strategy = self.strategy,
            processor_count = self.processor_count,
            max_concurrency = self.max_concurrency,
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::info_span!(
            "execution",
            span_name = name,
            strategy = self.strategy,
            processor_count = self.processor_count,
            max_concurrency = self.max_concurrency,
        )
    }
}

/// Execution completed successfully.
///
/// # Log Level
/// `info!` - Important operational event
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::engine::ExecutionCompleted;
/// use std::time::Duration;
///
/// let msg = ExecutionCompleted {
///     strategy: "WorkQueue",
///     processor_count: 5,
///     duration: Duration::from_millis(250),
/// };
///
/// tracing::info!("{}", msg);
/// ```
pub struct ExecutionCompleted<'a> {
    pub strategy: &'a str,
    pub processor_count: usize,
    pub duration: std::time::Duration,
}

impl Display for ExecutionCompleted<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "DAG execution completed with {} strategy: {} processors in {:?}",
            self.strategy, self.processor_count, self.duration
        )
    }
}

impl StructuredLog for ExecutionCompleted<'_> {
    fn log(&self) {
        tracing::info!(
            strategy = self.strategy,
            processor_count = self.processor_count,
            duration_ms = self.duration.as_millis() as u64,
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::info_span!(
            "execution_completed",
            span_name = name,
            strategy = self.strategy,
            processor_count = self.processor_count,
            duration = ?self.duration,
        )
    }
}

/// Execution failed with error.
/// # Log Level
/// `error!` - Failure requiring attention
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::engine::ExecutionFailed;
///
/// let error = std::io::Error::new(std::io::ErrorKind::Other, "test error");
/// let msg = ExecutionFailed {
///     strategy: "WorkQueue",
///     error: &error,
/// };
///
/// tracing::error!("{}", msg);
/// ```
pub struct ExecutionFailed<'a> {
    pub strategy: &'a str,
    pub error: &'a dyn std::error::Error,
}

impl Display for ExecutionFailed<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "DAG execution failed with {} strategy: {}",
            self.strategy, self.error
        )
    }
}

impl StructuredLog for ExecutionFailed<'_> {
    fn log(&self) {
        tracing::error!(
            strategy = self.strategy,
            error = %self.error,
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::error_span!(
            "execution_failed",
            span_name = name,
            strategy = self.strategy,
            error = %self.error,
        )
    }
}

/// Level computation completed for level-by-level executor.
///
/// # Log Level
/// `info!` - Important operational event
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::engine::LevelComputationCompleted;
///
/// let msg = LevelComputationCompleted {
///     level_count: 3,
///     processor_count: 7,
/// };
///
/// tracing::info!("{}", msg);
/// ```
pub struct LevelComputationCompleted {
    pub level_count: usize,
    pub processor_count: usize,
}

impl Display for LevelComputationCompleted {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Computed {} levels for {} processors",
            self.level_count, self.processor_count
        )
    }
}

impl StructuredLog for LevelComputationCompleted {
    fn log(&self) {
        tracing::info!(
            level_count = self.level_count,
            processor_count = self.processor_count,
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::info_span!(
            "level_computation",
            span_name = name,
            level_count = self.level_count,
            processor_count = self.processor_count,
        )
    }
}

/// Topological sort failed (cyclic dependency detected).
///
/// # Log Level
/// `error!` - Failure requiring attention
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::engine::TopologicalSortFailed;
///
/// let msg = TopologicalSortFailed {
///     reason: "Dependency graph contains cycles",
/// };
///
/// tracing::error!("{}", msg);
/// ```
pub struct TopologicalSortFailed<'a> {
    pub reason: &'a str,
}

impl Display for TopologicalSortFailed<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Topological sort failed: {}", self.reason)
    }
}

impl StructuredLog for TopologicalSortFailed<'_> {
    fn log(&self) {
        tracing::error!(
            reason = self.reason,
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::error_span!(
            "topological_sort_failed",
            span_name = name,
            reason = self.reason,
        )
    }
}
