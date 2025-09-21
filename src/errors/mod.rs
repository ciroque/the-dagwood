mod config;

pub use config::ValidationError;

/// Errors that can occur during DAG execution
#[derive(Debug, Clone)]
pub enum ExecutionError {
    /// A processor referenced in the DAG was not found in the processor registry
    ProcessorNotFound(String),
    
    /// A processor failed during execution
    ProcessorFailed {
        processor_id: String,
        error: String,
    },
    
    /// A processor could not execute because one of its dependencies failed
    DependencyFailed {
        processor_id: String,
        failed_dependency: String,
    },
    
    /// A processor execution timed out
    Timeout {
        processor_id: String,
        timeout_duration: std::time::Duration,
    },
    
    /// Multiple processors failed during execution
    MultipleFailed {
        failures: Vec<ExecutionError>,
    },
    
    /// Invalid processor response (e.g., missing outcome)
    InvalidResponse {
        processor_id: String,
        reason: String,
    },
    
    /// Executor internal error (e.g., concurrency issues, resource exhaustion)
    InternalError {
        message: String,
    },
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::ProcessorNotFound(id) => {
                write!(f, "Processor '{}' not found in registry", id)
            }
            ExecutionError::ProcessorFailed { processor_id, error } => {
                write!(f, "Processor '{}' failed: {}", processor_id, error)
            }
            ExecutionError::DependencyFailed { processor_id, failed_dependency } => {
                write!(f, "Processor '{}' blocked due to failed dependency '{}'", processor_id, failed_dependency)
            }
            ExecutionError::Timeout { processor_id, timeout_duration } => {
                write!(f, "Processor '{}' timed out after {:?}", processor_id, timeout_duration)
            }
            ExecutionError::MultipleFailed { failures } => {
                write!(f, "Multiple processors failed: {} failures", failures.len())
            }
            ExecutionError::InvalidResponse { processor_id, reason } => {
                write!(f, "Processor '{}' returned invalid response: {}", processor_id, reason)
            }
            ExecutionError::InternalError { message } => {
                write!(f, "Executor internal error: {}", message)
            }
        }
    }
}

impl std::error::Error for ExecutionError {}

/// Strategy for handling processor failures during DAG execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureStrategy {
    /// Stop entire DAG execution on first processor failure (default)
    FailFast,
    
    /// Continue executing independent branches, but block dependent processors
    ContinueOnError,
    
    /// Attempt to complete as much of the DAG as possible, collecting all failures
    BestEffort,
}

impl Default for FailureStrategy {
    fn default() -> Self {
        FailureStrategy::FailFast
    }
}