// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Message types for configuration validation warnings and errors.
//!
//! This module contains message types for logging events related to:
//! * Dependency graph validation
//! * Cyclic dependency detection
//! * Unresolved dependency detection
//! * Duplicate processor ID detection
//! * Diamond pattern warnings

use crate::observability::messages::StructuredLog;
use std::fmt::{Display, Formatter};
use tracing::Span;

/// Cyclic dependency detected in configuration.
///
/// # Log Level
/// `error!` - Failure requiring attention
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::validation::CyclicDependencyDetected;
///
/// let cycle = vec!["proc1", "proc2", "proc3", "proc1"];
/// let msg = CyclicDependencyDetected {
///     cycle: &cycle,
/// };
///
/// tracing::error!("{}", msg);
/// ```
pub struct CyclicDependencyDetected<'a> {
    pub cycle: &'a [&'a str],
}

impl Display for CyclicDependencyDetected<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Cyclic dependency detected: {}", self.cycle.join(" -> "))
    }
}

impl StructuredLog for CyclicDependencyDetected<'_> {
    fn log(&self) {
        tracing::error!(
            cycle = self.cycle.join(" -> "),
            cycle_length = self.cycle.len(),
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::span!(
            tracing::Level::ERROR,
            "span_name",
            name = name,
            cycle = self.cycle.join(" -> "),
            cycle_length = self.cycle.len(),
        )
    }
}

/// Unresolved dependency detected in configuration.
///
/// # Log Level
/// `error!` - Failure requiring attention
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::validation::UnresolvedDependency;
///
/// let msg = UnresolvedDependency {
///     processor_id: "proc1",
///     missing_dependency: "missing_proc",
/// };
///
/// tracing::error!("{}", msg);
/// ```
pub struct UnresolvedDependency<'a> {
    pub processor_id: &'a str,
    pub missing_dependency: &'a str,
}

impl Display for UnresolvedDependency<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Processor '{}' depends on missing processor '{}'",
            self.processor_id, self.missing_dependency
        )
    }
}

impl StructuredLog for UnresolvedDependency<'_> {
    fn log(&self) {
        tracing::error!(
            processor_id = self.processor_id,
            missing_dependency = self.missing_dependency,
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::span!(
            tracing::Level::ERROR,
            "span_name",
            name = name,
            processor_id = self.processor_id,
            missing_dependency = self.missing_dependency,
        )
    }
}

/// Duplicate processor ID detected in configuration.
///
/// # Log Level
/// `error!` - Failure requiring attention
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::validation::DuplicateProcessorId;
///
/// let msg = DuplicateProcessorId {
///     processor_id: "duplicate_proc",
/// };
///
/// tracing::error!("{}", msg);
/// ```
pub struct DuplicateProcessorId<'a> {
    pub processor_id: &'a str,
}

impl Display for DuplicateProcessorId<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Duplicate processor ID: '{}'", self.processor_id)
    }
}

impl StructuredLog for DuplicateProcessorId<'_> {
    fn log(&self) {
        tracing::error!(
            processor_id = self.processor_id,
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::span!(
            tracing::Level::ERROR,
            "span_name",
            name = name,
            processor_id = self.processor_id,
        )
    }
}

/// Diamond pattern detected in configuration (potential non-determinism).
///
/// # Log Level
/// `warn!` - Potential issue or degraded behavior
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::validation::DiamondPatternDetected;
///
/// let paths = vec![
///     vec!["entry", "proc1", "convergence"],
///     vec!["entry", "proc2", "convergence"],
/// ];
/// let msg = DiamondPatternDetected {
///     convergence_processor: "convergence",
///     parallel_path_count: 2,
/// };
///
/// tracing::warn!("{}", msg);
/// ```
pub struct DiamondPatternDetected<'a> {
    pub convergence_processor: &'a str,
    pub parallel_path_count: usize,
}

impl Display for DiamondPatternDetected<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Diamond pattern detected at '{}' with {} parallel paths - may cause non-deterministic behavior",
            self.convergence_processor, self.parallel_path_count
        )
    }
}

impl StructuredLog for DiamondPatternDetected<'_> {
    fn log(&self) {
        tracing::warn!(
            convergence_processor = self.convergence_processor,
            parallel_path_count = self.parallel_path_count,
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::span!(
            tracing::Level::WARN,
            "span_name",
            name = name,
            convergence_processor = self.convergence_processor,
            parallel_path_count = self.parallel_path_count,
        )
    }
}

/// Configuration validation started.
///
/// # Log Level
/// `info!` - Important operational event
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::validation::ValidationStarted;
///
/// let msg = ValidationStarted {
///     processor_count: 5,
/// };
///
/// tracing::info!("{}", msg);
/// ```
pub struct ValidationStarted {
    pub processor_count: usize,
}

impl Display for ValidationStarted {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Starting configuration validation for {} processors",
            self.processor_count
        )
    }
}

impl StructuredLog for ValidationStarted {
    fn log(&self) {
        tracing::info!(
            processor_count = self.processor_count,
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::span!(
            tracing::Level::INFO,
            "span_name",
            name = name,
            processor_count = self.processor_count,
        )
    }
}

/// Configuration validation completed successfully.
///
/// # Log Level
/// `info!` - Important operational event
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::validation::ValidationCompleted;
///
/// let msg = ValidationCompleted {
///     processor_count: 5,
///     warning_count: 1,
/// };
///
/// tracing::info!("{}", msg);
/// ```
pub struct ValidationCompleted {
    pub processor_count: usize,
    pub warning_count: usize,
}

impl Display for ValidationCompleted {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        if self.warning_count > 0 {
            write!(
                f,
                "Configuration validation completed for {} processors with {} warnings",
                self.processor_count, self.warning_count
            )
        } else {
            write!(
                f,
                "Configuration validation completed successfully for {} processors",
                self.processor_count
            )
        }
    }
}

impl StructuredLog for ValidationCompleted {
    fn log(&self) {
        tracing::info!(
            processor_count = self.processor_count,
            warning_count = self.warning_count,
            has_warnings = self.warning_count > 0,
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::span!(
            tracing::Level::INFO,
            "span_name",
            name = name,
            processor_count = self.processor_count,
            warning_count = self.warning_count,
            has_warnings = self.warning_count > 0,
        )
    }
}

/// Configuration validation failed.
///
/// # Log Level
/// `error!` - Failure requiring attention
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::validation::ValidationFailed;
///
/// let msg = ValidationFailed {
///     error_count: 3,
/// };
///
/// tracing::error!("{}", msg);
/// ```
pub struct ValidationFailed {
    pub error_count: usize,
}

impl Display for ValidationFailed {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Configuration validation failed with {} errors",
            self.error_count
        )
    }
}

impl StructuredLog for ValidationFailed {
    fn log(&self) {
        tracing::error!(
            error_count = self.error_count,
            "{}", self
        );
    }

    fn span(&self, name: &str) -> Span {
        tracing::span!(
            tracing::Level::ERROR,
            "span_name",
            name = name,
            error_count = self.error_count,
        )
    }
}
