// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use std::fmt;

/// Errors that can occur during dependency graph validation
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// A circular dependency was detected in the processor graph
    CyclicDependency {
        /// The cycle path showing the circular dependency
        cycle: Vec<String>,
    },
    /// A processor references a dependency that doesn't exist
    UnresolvedDependency {
        /// The processor that has the unresolved dependency
        processor_id: String,
        /// The dependency that couldn't be resolved
        missing_dependency: String,
    },
    /// A processor has a duplicate ID
    DuplicateProcessorId {
        /// The duplicate processor ID
        processor_id: String,
    },
    /// A diamond dependency pattern was detected that may cause non-deterministic behavior
    DiamondPatternWarning {
        /// The convergence point of the diamond pattern
        convergence_processor: String,
        /// The processors that form the parallel paths
        parallel_paths: Vec<Vec<String>>,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::CyclicDependency { cycle } => {
                write!(f, "Cyclic dependency detected: {}", cycle.join(" -> "))
            }
            ValidationError::UnresolvedDependency {
                processor_id,
                missing_dependency,
            } => {
                write!(
                    f,
                    "Processor '{}' depends on '{}' which does not exist",
                    processor_id, missing_dependency
                )
            }
            ValidationError::DuplicateProcessorId { processor_id } => {
                write!(f, "Duplicate processor ID: '{}'", processor_id)
            }
            ValidationError::DiamondPatternWarning {
                convergence_processor,
                parallel_paths,
            } => {
                write!(
                    f,
                    "Diamond dependency pattern detected at '{}': ",
                    convergence_processor
                )?;
                for (i, path) in parallel_paths.iter().enumerate() {
                    if i > 0 {
                        write!(f, " and ")?;
                    }
                    write!(f, "[{}]", path.join(" -> "))?;
                }
                write!(f, " -> {}. ", convergence_processor)?;
                write!(f, "If any processors in parallel paths are Transform type, this may cause non-deterministic behavior in the reactive executor due to race conditions in canonical payload updates.")
            }
        }
    }
}

impl std::error::Error for ValidationError {}
