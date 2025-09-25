use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use crate::errors::FailureStrategy;

/// Main configuration structure for the DAG execution engine.
///
/// This struct represents the complete configuration for a DAG workflow,
/// including the execution strategy and all processor definitions.
/// It is typically loaded from a YAML configuration file.
///
/// # Fields
/// * `strategy` - The execution strategy to use for the DAG
/// * `failure_strategy` - How to handle processor failures (optional, defaults to FailFast)
/// * `executor_options` - Executor-specific configuration options (optional)
/// * `processors` - Vector of processor configurations that define the DAG nodes
///
/// # Example
/// ```yaml
/// strategy: work_queue
/// failure_strategy: fail_fast
/// executor_options:
///   max_concurrency: 4
///   timeout_seconds: 30
/// processors:
///   - id: "processor1"
///     type: local
///     processor: "my_processor"
/// ```
#[derive(Debug, Deserialize)]
pub struct Config {
    pub strategy: Strategy,
    #[serde(default)]
    pub failure_strategy: FailureStrategy,
    #[serde(default)]
    pub executor_options: ExecutorOptions,
    pub processors: Vec<ProcessorConfig>,
}

/// Execution strategy for DAG processing.
///
/// Defines how the DAG execution engine should schedule and execute processors.
/// Each strategy has different characteristics in terms of parallelism, ordering,
/// and resource utilization.
///
/// # Variants
/// * `WorkQueue` - Uses a work queue pattern for task distribution
/// * `Level` - Executes processors level by level based on dependency depth
/// * `Reactive` - Event-driven execution based on data availability
/// * `Hybrid` - Combines multiple strategies for optimal performance
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    WorkQueue,
    Level,
    Reactive,
    Hybrid,
}

/// Executor-specific configuration options.
///
/// These options control how the DAG executor behaves during execution.
/// Different executors may use different subsets of these options.
///
/// # Fields
/// * `max_concurrency` - Maximum number of concurrent processor executions (optional)
/// * `timeout_seconds` - Timeout for individual processor execution in seconds (optional)
/// * `retry_attempts` - Number of retry attempts for failed processors (optional)
/// * `batch_size` - Batch size for batch processing executors (optional)
#[derive(Debug, Deserialize)]
pub struct ExecutorOptions {
    pub max_concurrency: Option<usize>,
    pub timeout_seconds: Option<u64>,
    pub retry_attempts: Option<u32>,
    pub batch_size: Option<usize>,
}

impl Default for ExecutorOptions {
    fn default() -> Self {
        Self {
            max_concurrency: None,
            timeout_seconds: None,
            retry_attempts: None,
            batch_size: None,
        }
    }
}

/// Configuration for a single processor in the DAG.
///
/// Each processor represents a node in the DAG and can be implemented using
/// different backend types (local, RPC, WASM, etc.). The configuration
/// specifies how to instantiate and connect the processor.
///
/// # Fields
/// * `id` - Unique identifier for this processor
/// * `backend` - The backend type that implements this processor
/// * `processor` - Implementation name/path (for local backends)
/// * `endpoint` - Network endpoint (for RPC backends)
/// * `module` - WASM module path (for WASM backends)
/// * `depends_on` - List of processor IDs that this processor depends on
/// * `collection_strategy` - Strategy for combining multiple dependency outputs
/// * `options` - Additional processor-specific configuration options
///
/// # Example
/// ```yaml
/// id: "data_processor"
/// type: local
/// processor: "DataProcessor"
/// depends_on: ["input_validator"]
/// collection_strategy:
///   type: merge_metadata
///   primary_source: "validator"
///   metadata_sources: ["analyzer"]
/// ```
#[derive(Debug, Deserialize)]
pub struct ProcessorConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub backend: BackendType,
    pub processor: Option<String>,   // for local
    pub endpoint: Option<String>,    // for rpc
    pub module: Option<String>,      // for wasm
    #[serde(default)]
    pub depends_on: Vec<String>,     // defaults empty
    #[serde(default)]
    pub collection_strategy: Option<CollectionStrategy>, // for result collection
    #[serde(default)]
    pub options: HashMap<String, serde_yaml::Value>, // processor-specific options
}

/// Backend implementation type for processors.
///
/// Defines how a processor is implemented and executed. Different backend
/// types provide different capabilities, performance characteristics, and
/// deployment options.
///
/// # Variants
/// * `Local` - In-process implementation using native Rust code
/// * `Loadable` - Dynamically loaded library/plugin
/// * `Grpc` - Remote procedure call over gRPC protocol
/// * `Http` - HTTP-based remote service
/// * `Wasm` - WebAssembly module for sandboxed execution
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendType {
    Local,
    Loadable,
    Grpc,
    Http,
    Wasm,
}

/// Collection strategy for handling multiple dependency outputs.
///
/// Defines how a processor should combine inputs from multiple dependencies
/// when they run in parallel. This addresses non-deterministic behavior
/// in parallel execution scenarios.
///
/// # Variants
/// * `FirstAvailable` - Use the first dependency output that becomes available (current behavior)
/// * `MergeMetadata` - Use one dependency as primary payload, others as metadata
/// * `Concatenate` - Combine all dependency outputs into a single payload
/// * `JsonMerge` - Intelligently merge JSON outputs from dependencies
/// * `Custom` - Use a custom combiner implementation. Requires a `combiner_impl` field specifying the implementation to use.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CollectionStrategy {
    FirstAvailable,
    MergeMetadata {
        primary_source: String,
        metadata_sources: Vec<String>,
    },
    Concatenate {
        separator: Option<String>,
    },
    JsonMerge {
        merge_arrays: bool,
        conflict_resolution: ConflictResolution,
    },
    Custom {
        combiner_impl: String,
    },
}

/// Strategy for resolving conflicts when merging JSON data.
///
/// Used by the JsonMerge collection strategy to determine how to handle
/// conflicting keys when combining JSON outputs from multiple dependencies.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    /// Take the value from the first dependency
    TakeFirst,
    /// Take the value from the last dependency
    TakeLast,
    /// Attempt to merge values (arrays concatenated, objects merged recursively)
    Merge,
    /// Return an error if conflicts are detected
    Error,
}

/// Load a config from a YAML file
/// TODO(steve): use thiserror and custom enums 
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let cfg: Config = serde_yaml::from_str(&content)?;
    Ok(cfg)
}

/// Load and validate a config from a YAML file
/// 
/// This function loads the configuration and validates the dependency graph
/// to ensure it's acyclic and all references are resolved.
pub fn load_and_validate_config<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn std::error::Error>> {
    let cfg = load_config(path)?;
    
    // Validate the dependency graph
    if let Err(validation_errors) = crate::config::validate_dependency_graph(&cfg) {
        // Convert validation errors into a single error message
        let error_messages: Vec<String> = validation_errors
            .iter()
            .map(|e| e.to_string())
            .collect();
        let combined_error = format!("Configuration validation failed:\n{}", error_messages.join("\n"));
        return Err(combined_error.into());
    }
    
    Ok(cfg)
}



#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml;

    #[test]
    fn parse_basic_config() {
        let yaml = r#"
strategy: work_queue
processors:
  - id: logger
    type: local
    impl_: Logger
  - id: auth
    type: grpc
    endpoint: https://auth-service:50051
    depends_on: [logger]
"#;

        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(matches!(cfg.strategy, Strategy::WorkQueue), true);
        assert_eq!(cfg.processors.len(), 2);
        assert_eq!(cfg.processors[1].depends_on, vec!["logger"]);
    }

    #[test]
    fn test_load_and_validate_valid_config() {
        let yaml = r#"
strategy: work_queue
processors:
  - id: logger
    type: local
    impl_: Logger
  - id: auth
    type: grpc
    endpoint: https://auth-service:50051
    depends_on: [logger]
  - id: processor
    type: wasm
    module: processor.wasm
    depends_on: [auth]
"#;
        
        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_config.yaml");
        std::fs::write(&temp_file, yaml).unwrap();
        
        // Test that validation passes
        let result = load_and_validate_config(&temp_file);
        assert!(result.is_ok());
        
        // Clean up
        std::fs::remove_file(&temp_file).unwrap();
    }

    #[test]
    fn test_load_and_validate_cyclic_config() {
        let yaml = r#"
strategy: work_queue
processors:
  - id: a
    type: local
    impl_: ProcessorA
    depends_on: [b]
  - id: b
    type: local
    impl_: ProcessorB
    depends_on: [a]
"#;
        
        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_cyclic_config.yaml");
        std::fs::write(&temp_file, yaml).unwrap();
        
        // Test that validation fails
        let result = load_and_validate_config(&temp_file);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Cyclic dependency detected"));
        
        // Clean up
        std::fs::remove_file(&temp_file).unwrap();
    }

    #[test]
    fn test_load_and_validate_unresolved_dependency() {
        let yaml = r#"
strategy: work_queue
processors:
  - id: processor
    type: local
    impl_: Processor
    depends_on: [nonexistent]
"#;
        
        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_unresolved_config.yaml");
        std::fs::write(&temp_file, yaml).unwrap();
        
        // Test that validation fails
        let result = load_and_validate_config(&temp_file);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("depends on 'nonexistent' which does not exist"));
        
        // Clean up
        std::fs::remove_file(&temp_file).unwrap();
    }

    #[test]
    fn test_parse_collection_strategy_merge_metadata() {
        let yaml = r#"
strategy: work_queue
processors:
  - id: collector
    type: local
    impl_: ResultCollector
    depends_on: [token_counter, word_frequency]
    collection_strategy:
      type: merge_metadata
      primary_source: token_counter
      metadata_sources: [word_frequency]
"#;

        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.processors.len(), 1);
        
        let processor = &cfg.processors[0];
        assert_eq!(processor.id, "collector");
        assert!(processor.collection_strategy.is_some());
        
        if let Some(CollectionStrategy::MergeMetadata { primary_source, metadata_sources }) = &processor.collection_strategy {
            assert_eq!(primary_source, "token_counter");
            assert_eq!(metadata_sources, &vec!["word_frequency"]);
        } else {
            panic!("Expected MergeMetadata collection strategy");
        }
    }

    #[test]
    fn test_parse_collection_strategy_concatenate() {
        let yaml = r#"
strategy: work_queue
processors:
  - id: concatenator
    type: local
    impl_: ResultCollector
    collection_strategy:
      type: concatenate
      separator: "\n---\n"
"#;

        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        let processor = &cfg.processors[0];
        
        if let Some(CollectionStrategy::Concatenate { separator }) = &processor.collection_strategy {
            assert_eq!(separator.as_ref().unwrap(), "\n---\n");
        } else {
            panic!("Expected Concatenate collection strategy");
        }
    }

    #[test]
    fn test_parse_collection_strategy_json_merge() {
        let yaml = r#"
strategy: work_queue
processors:
  - id: json_merger
    type: local
    impl_: ResultCollector
    collection_strategy:
      type: json_merge
      merge_arrays: true
      conflict_resolution: merge
"#;

        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        let processor = &cfg.processors[0];
        
        if let Some(CollectionStrategy::JsonMerge { merge_arrays, conflict_resolution }) = &processor.collection_strategy {
            assert_eq!(*merge_arrays, true);
            assert!(matches!(conflict_resolution, ConflictResolution::Merge));
        } else {
            panic!("Expected JsonMerge collection strategy");
        }
    }

    #[test]
    fn test_parse_processor_with_options() {
        let yaml = r#"
strategy: work_queue
processors:
  - id: processor_with_options
    type: local
    impl_: SomeProcessor
    options:
      mode: upper
      timeout: 30
      enabled: true
"#;

        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        let processor = &cfg.processors[0];
        
        assert_eq!(processor.options.len(), 3);
        assert!(processor.options.contains_key("mode"));
        assert!(processor.options.contains_key("timeout"));
        assert!(processor.options.contains_key("enabled"));
    }
}
