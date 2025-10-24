// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use crate::config::consts::{DEFAULT_FUEL_LEVEL, MAX_FUEL_LEVEL, MIN_FUEL_LEVEL};
use crate::errors::FailureStrategy;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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
/// * `wasm` - WASM-specific configuration options (optional)
/// * `processors` - Vector of processor configurations that define the DAG nodes
///
/// # Example
/// ```yaml
/// strategy: work_queue
/// failure_strategy: fail_fast
/// executor_options:
///   max_concurrency: 4
///   timeout_seconds: 30
/// wasm:
///   fuel:
///     default: 100000000
///     maximum: 500000000
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
    #[serde(default)]
    pub wasm: WasmConfig,
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

/// WASM-specific configuration options.
///
/// These options control WASM module execution behavior, including resource limits
/// and security constraints. All fields are optional and use sensible defaults.
///
/// # Fields
/// * `fuel` - Fuel consumption configuration for execution limits
///
/// # Example
/// ```yaml
/// wasm:
///   fuel:
///     default: 100000000
///     minimum: 1000000
///     maximum: 500000000
/// ```
#[derive(Debug, Deserialize)]
pub struct WasmConfig {
    #[serde(default)]
    pub fuel: FuelConfig,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            fuel: FuelConfig::default(),
        }
    }
}

/// Fuel consumption configuration for WASM execution.
///
/// Fuel limits prevent infinite loops and resource exhaustion by limiting the number
/// of instructions a WASM module can execute. All values are optional and validated
/// against security bounds.
///
/// # Fields
/// * `default` - Default fuel level for WASM processors (defaults to 100M)
/// * `minimum` - Minimum allowed fuel level (defaults to 1M)
/// * `maximum` - Maximum allowed fuel level (defaults to 500M) - security limit
///
/// # Security
/// The maximum fuel level is enforced as a hard security limit to prevent resource
/// exhaustion attacks. Individual processors cannot exceed this limit.
///
/// # Example
/// ```yaml
/// fuel:
///   default: 100000000   # 100 million instructions
///   minimum: 1000000     # 1 million instructions
///   maximum: 500000000   # 500 million instructions (hard limit)
/// ```
#[derive(Debug, Deserialize)]
pub struct FuelConfig {
    pub default: Option<u64>,
    pub minimum: Option<u64>,
    pub maximum: Option<u64>,
}

impl Default for FuelConfig {
    fn default() -> Self {
        Self {
            default: None,
            minimum: None,
            maximum: None,
        }
    }
}

impl FuelConfig {
    /// Get the default fuel level, using built-in default if not configured.
    pub fn get_default(&self) -> u64 {
        self.default.unwrap_or(DEFAULT_FUEL_LEVEL)
    }

    /// Get the minimum fuel level, using built-in default if not configured.
    pub fn get_minimum(&self) -> u64 {
        self.minimum.unwrap_or(MIN_FUEL_LEVEL)
    }

    /// Get the maximum fuel level, using built-in default if not configured.
    pub fn get_maximum(&self) -> u64 {
        self.maximum.unwrap_or(MAX_FUEL_LEVEL)
    }

    /// Validate and clamp a fuel level to configured bounds.
    ///
    /// This method ensures that a requested fuel level falls within the configured
    /// minimum and maximum bounds. If the value is out of bounds, it is clamped
    /// to the nearest valid value.
    ///
    /// # Arguments
    /// * `requested` - The requested fuel level
    ///
    /// # Returns
    /// The validated fuel level, clamped to [minimum, maximum]
    ///
    /// # Example
    /// ```
    /// use the_dagwood::config::FuelConfig;
    ///
    /// let config = FuelConfig::default();
    /// let fuel = config.validate_and_clamp(1_000_000_000); // Too high
    /// assert_eq!(fuel, 500_000_000); // Clamped to maximum
    /// ```
    pub fn validate_and_clamp(&self, requested: u64) -> u64 {
        let min = self.get_minimum();
        let max = self.get_maximum();

        // TODO(steve): warn if requested is out of bounds
        requested.clamp(min, max)
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
/// * `options` - Additional processor-specific configuration options
///
/// # Example
/// ```yaml
/// id: "data_processor"
/// type: local
/// processor: "DataProcessor"
/// depends_on: ["input_validator"]
/// ```
#[derive(Debug, Deserialize)]
pub struct ProcessorConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub backend: BackendType,
    pub processor: Option<String>, // for local
    pub endpoint: Option<String>,  // for rpc
    pub module: Option<String>,    // for wasm
    #[serde(default)]
    pub depends_on: Vec<String>, // defaults empty
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
#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BackendType {
    Local,
    Loadable,
    Grpc,
    Http,
    Wasm,
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
pub fn load_and_validate_config<P: AsRef<Path>>(
    path: P,
) -> Result<Config, Box<dyn std::error::Error>> {
    let cfg = load_config(path)?;

    // Validate the dependency graph
    if let Err(validation_errors) = crate::config::validate_dependency_graph(&cfg) {
        // Convert validation errors into a single error message
        let error_messages: Vec<String> = validation_errors.iter().map(|e| e.to_string()).collect();
        let combined_error = format!(
            "Configuration validation failed:\n{}",
            error_messages.join("\n")
        );
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

    #[test]
    fn test_wasm_config_defaults() {
        let yaml = r#"
strategy: work_queue
processors:
  - id: processor1
    type: local
    impl_: SomeProcessor
"#;

        let cfg: Config = serde_yaml::from_str(yaml).unwrap();

        // Should use built-in defaults
        assert_eq!(cfg.wasm.fuel.get_default(), 100_000_000);
        assert_eq!(cfg.wasm.fuel.get_minimum(), 1_000_000);
        assert_eq!(cfg.wasm.fuel.get_maximum(), 500_000_000);
    }

    #[test]
    fn test_wasm_config_custom_values() {
        let yaml = r#"
strategy: work_queue
wasm:
  fuel:
    default: 200000000
    minimum: 5000000
    maximum: 300000000
processors:
  - id: processor1
    type: local
    impl_: SomeProcessor
"#;

        let cfg: Config = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(cfg.wasm.fuel.get_default(), 200_000_000);
        assert_eq!(cfg.wasm.fuel.get_minimum(), 5_000_000);
        assert_eq!(cfg.wasm.fuel.get_maximum(), 300_000_000);
    }

    #[test]
    fn test_wasm_config_partial_override() {
        let yaml = r#"
strategy: work_queue
wasm:
  fuel:
    default: 150000000
processors:
  - id: processor1
    type: local
    impl_: SomeProcessor
"#;

        let cfg: Config = serde_yaml::from_str(yaml).unwrap();

        // Custom default, built-in min/max
        assert_eq!(cfg.wasm.fuel.get_default(), 150_000_000);
        assert_eq!(cfg.wasm.fuel.get_minimum(), 1_000_000);
        assert_eq!(cfg.wasm.fuel.get_maximum(), 500_000_000);
    }

    #[test]
    fn test_fuel_config_validate_and_clamp() {
        let config = FuelConfig {
            default: Some(100_000_000),
            minimum: Some(10_000_000),
            maximum: Some(200_000_000),
        };

        // Within bounds - no change
        assert_eq!(config.validate_and_clamp(50_000_000), 50_000_000);

        // Below minimum - clamped to minimum
        assert_eq!(config.validate_and_clamp(1_000_000), 10_000_000);

        // Above maximum - clamped to maximum
        assert_eq!(config.validate_and_clamp(1_000_000_000), 200_000_000);

        // Exactly at bounds
        assert_eq!(config.validate_and_clamp(10_000_000), 10_000_000);
        assert_eq!(config.validate_and_clamp(200_000_000), 200_000_000);
    }

    #[test]
    fn test_fuel_config_validate_with_defaults() {
        let config = FuelConfig::default();

        // Should use built-in constants for validation
        assert_eq!(config.validate_and_clamp(50_000_000), 50_000_000);
        assert_eq!(config.validate_and_clamp(100), 1_000_000); // Below min
        assert_eq!(config.validate_and_clamp(1_000_000_000), 500_000_000); // Above max
    }
}
