use serde::Deserialize;
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
/// * `processors` - Vector of processor configurations that define the DAG nodes
///
/// # Example
/// ```yaml
/// strategy: work_queue
/// processors:
///   - id: "processor1"
///     type: local
///     impl_: "my_processor"
/// ```
#[derive(Debug, Deserialize)]
pub struct Config {
    pub strategy: Strategy,
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
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    WorkQueue,
    Level,
    Reactive,
    Hybrid,
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
/// * `impl_` - Implementation name/path (for local backends)
/// * `endpoint` - Network endpoint (for RPC backends)
/// * `module` - WASM module path (for WASM backends)
/// * `depends_on` - List of processor IDs that this processor depends on
///
/// # Example
/// ```yaml
/// id: "data_processor"
/// type: local
/// impl_: "DataProcessor"
/// depends_on: ["input_validator"]
/// ```
#[derive(Debug, Deserialize)]
pub struct ProcessorConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub backend: BackendType,
    pub impl_: Option<String>,       // for local
    pub endpoint: Option<String>,    // for rpc
    pub module: Option<String>,      // for wasm
    #[serde(default)]
    pub depends_on: Vec<String>,     // defaults empty
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

/// Load a config from a YAML file
/// TODO(steve): use thiserror and custom enums 
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let cfg: Config = serde_yaml::from_str(&content)?;
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
}
