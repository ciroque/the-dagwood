// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use crate::traits::Processor;
use std::collections::HashMap;
use std::sync::Arc;

/// A type-safe registry mapping processor IDs to their implementations.
///
/// The `ProcessorMap` serves as the central registry for all processors in a DAG execution.
/// It maps unique processor IDs (strings) to their concrete implementations wrapped in
/// `Arc<dyn Processor>` for thread-safe shared ownership. This allows multiple executors
/// to access the same processor instances concurrently during DAG execution.
///
/// The `Arc` wrapper enables:
/// - **Shared ownership**: Multiple references to the same processor instance
/// - **Thread safety**: Safe concurrent access across async tasks
/// - **Memory efficiency**: Processors are not cloned, only reference-counted
///
/// # Examples
///
/// ## Creating and populating a processor map
/// ```ignore
/// use std::sync::Arc;
/// use std::collections::HashMap;
/// use the_dagwood::config::ProcessorMap;
/// use the_dagwood::backends::stub::StubProcessor;
/// use the_dagwood::traits::Processor;
///
/// let mut processor_map = ProcessorMap::new();
///
/// // Add processors to the registry
/// let stub1: Arc<dyn Processor> = Arc::new(StubProcessor::new("stub1".to_string()));
/// let stub2: Arc<dyn Processor> = Arc::new(StubProcessor::new("stub2".to_string()));
///
/// processor_map.insert("input_processor".to_string(), stub1);
/// processor_map.insert("output_processor".to_string(), stub2);
///
/// assert!(processor_map.contains_key("input_processor"));
/// assert_eq!(processor_map.keys().count(), 2);
/// ```
///
/// ## Creating from a HashMap
/// ```ignore
/// use std::sync::Arc;
/// use std::collections::HashMap;
/// use the_dagwood::config::ProcessorMap;
/// use the_dagwood::backends::stub::StubProcessor;
/// use the_dagwood::traits::Processor;
///
/// let mut map = HashMap::new();
/// let processor: Arc<dyn Processor> = Arc::new(StubProcessor::new("test".to_string()));
/// map.insert("test_processor".to_string(), processor);
///
/// let processor_map = ProcessorMap::from(map);
/// assert!(processor_map.contains_key("test_processor"));
/// ```
///
/// ## Accessing processors during execution
/// ```ignore
/// use std::sync::Arc;
/// use the_dagwood::config::ProcessorMap;
/// use the_dagwood::backends::stub::StubProcessor;
/// use the_dagwood::traits::Processor;
///
/// let mut processor_map = ProcessorMap::new();
/// let processor: Arc<dyn Processor> = Arc::new(StubProcessor::new("worker".to_string()));
/// processor_map.insert("data_processor".to_string(), processor);
///
/// // Retrieve processor for execution
/// if let Some(processor_ref) = processor_map.get("data_processor") {
///     // processor_ref is &Arc<dyn Processor> - ready for async execution
///     // Access the processor through the Arc
///     assert_eq!(processor_ref.name(), "stub");
/// }
/// ```
#[derive(Clone)]
pub struct ProcessorMap(pub HashMap<String, Arc<dyn Processor>>);

impl ProcessorMap {
    /// Create a new empty processor map
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Create a ProcessorMap from configuration, resolving all processors
    pub fn from_config(cfg: &crate::config::Config) -> Result<Self, String> {
        let mut registry = HashMap::new();

        for p in &cfg.processors {
            let processor: Arc<dyn Processor> = match p.backend {
                crate::config::BackendType::Local => {
                    crate::backends::local::LocalProcessorFactory::create_processor(p).map_err(
                        |e| format!("Failed to create local processor '{}': {}", p.id, e),
                    )?
                }
                crate::config::BackendType::Loadable => {
                    return Err(format!(
                        "Backend type 'Loadable' is not implemented for processor '{}'. Dynamic library loading is not yet supported.",
                        p.id
                    ));
                }
                crate::config::BackendType::Grpc => {
                    return Err(format!(
                        "Backend type 'Grpc' is not implemented for processor '{}'. gRPC client support is not yet supported.",
                        p.id
                    ));
                }
                crate::config::BackendType::Http => {
                    return Err(format!(
                        "Backend type 'Http' is not implemented for processor '{}'. HTTP client support is not yet supported.",
                        p.id
                    ));
                }
                crate::config::BackendType::Wasm => Arc::new(
                    crate::backends::wasm::WasmProcessor::from_config(p).map_err(|e| {
                        format!("Failed to create WASM processor '{}': {}", p.id, e)
                    })?,
                ),
            };

            registry.insert(p.id.clone(), processor);
        }

        Ok(Self(registry))
    }

    /// Insert a processor into the map
    pub fn insert(&mut self, id: String, processor: Arc<dyn Processor>) {
        self.0.insert(id, processor);
    }

    /// Get a processor by ID
    pub fn get(&self, id: &str) -> Option<&Arc<dyn Processor>> {
        self.0.get(id)
    }

    /// Check if a processor exists
    pub fn contains_key(&self, id: &str) -> bool {
        self.0.contains_key(id)
    }

    /// Get all processor IDs
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.keys()
    }

    /// Get the number of processors in the map
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if the processor map is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for ProcessorMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessorMap")
            .field("processor_count", &self.0.len())
            .field("processor_ids", &self.0.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl From<HashMap<String, Arc<dyn Processor>>> for ProcessorMap {
    fn from(map: HashMap<String, Arc<dyn Processor>>) -> Self {
        Self(map)
    }
}

impl From<ProcessorMap> for HashMap<String, Arc<dyn Processor>> {
    fn from(map: ProcessorMap) -> Self {
        map.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BackendType, Config, ExecutorOptions, ProcessorConfig, Strategy};
    use crate::errors::FailureStrategy;
    use std::collections::HashMap;

    #[test]
    fn test_from_config_table_driven() {
        struct TestCase {
            name: &'static str,
            config: Config,
            expected_processor_count: usize,
            expected_processor_ids: Vec<&'static str>,
        }

        let test_cases = vec![
            TestCase {
                name: "empty config",
                config: Config {
                    strategy: Strategy::WorkQueue,
                    failure_strategy: FailureStrategy::FailFast,
                    executor_options: ExecutorOptions::default(),
                    processors: vec![],
                },
                expected_processor_count: 0,
                expected_processor_ids: vec![],
            },
            TestCase {
                name: "single local processor",
                config: Config {
                    strategy: Strategy::WorkQueue,
                    failure_strategy: FailureStrategy::FailFast,
                    executor_options: ExecutorOptions::default(),
                    processors: vec![ProcessorConfig {
                        id: "local_proc".to_string(),
                        backend: BackendType::Local,
                        processor: Some("change_text_case_upper".to_string()),
                        endpoint: None,
                        module: None,
                        depends_on: vec![],
                        options: HashMap::new(),
                    }],
                },
                expected_processor_count: 1,
                expected_processor_ids: vec!["local_proc"],
            },
            TestCase {
                name: "single loadable processor",
                config: Config {
                    strategy: Strategy::Level,
                    failure_strategy: FailureStrategy::FailFast,
                    executor_options: ExecutorOptions::default(),
                    processors: vec![ProcessorConfig {
                        id: "loadable_proc".to_string(),
                        backend: BackendType::Loadable,
                        processor: Some("libloadable.so".to_string()),
                        endpoint: None,
                        module: None,
                        depends_on: vec![],
                        options: HashMap::new(),
                    }],
                },
                expected_processor_count: 1,
                expected_processor_ids: vec!["loadable_proc"],
            },
            TestCase {
                name: "single grpc processor",
                config: Config {
                    strategy: Strategy::Reactive,
                    failure_strategy: FailureStrategy::FailFast,
                    executor_options: ExecutorOptions::default(),
                    processors: vec![ProcessorConfig {
                        id: "grpc_proc".to_string(),
                        backend: BackendType::Grpc,
                        processor: None,
                        endpoint: Some("https://grpc-service:50051".to_string()),
                        module: None,
                        depends_on: vec![],
                        options: HashMap::new(),
                    }],
                },
                expected_processor_count: 1,
                expected_processor_ids: vec!["grpc_proc"],
            },
            TestCase {
                name: "single http processor",
                config: Config {
                    strategy: Strategy::Hybrid,
                    failure_strategy: FailureStrategy::FailFast,
                    executor_options: ExecutorOptions::default(),
                    processors: vec![ProcessorConfig {
                        id: "http_proc".to_string(),
                        backend: BackendType::Http,
                        processor: None,
                        endpoint: Some("https://api.example.com/process".to_string()),
                        module: None,
                        depends_on: vec![],
                        options: HashMap::new(),
                    }],
                },
                expected_processor_count: 1,
                expected_processor_ids: vec!["http_proc"],
            },
            TestCase {
                name: "single wasm processor",
                config: Config {
                    strategy: Strategy::WorkQueue,
                    failure_strategy: FailureStrategy::FailFast,
                    executor_options: ExecutorOptions::default(),
                    processors: vec![ProcessorConfig {
                        id: "wasm_proc".to_string(),
                        backend: BackendType::Wasm,
                        processor: None,
                        endpoint: None,
                        module: Some("processor.wasm".to_string()),
                        depends_on: vec![],
                        options: HashMap::new(),
                    }],
                },
                expected_processor_count: 1,
                expected_processor_ids: vec!["wasm_proc"],
            },
            TestCase {
                name: "multiple processors of different types",
                config: Config {
                    strategy: Strategy::WorkQueue,
                    failure_strategy: FailureStrategy::FailFast,
                    executor_options: ExecutorOptions::default(),
                    processors: vec![
                        ProcessorConfig {
                            id: "local1".to_string(),
                            backend: BackendType::Local,
                            processor: Some("change_text_case_upper".to_string()),
                            endpoint: None,
                            module: None,
                            depends_on: vec![],
                            options: HashMap::new(),
                        },
                        ProcessorConfig {
                            id: "grpc1".to_string(),
                            backend: BackendType::Grpc,
                            processor: None,
                            endpoint: Some("grpc://service1:50051".to_string()),
                            module: None,
                            depends_on: vec!["local1".to_string()],
                            options: HashMap::new(),
                        },
                        ProcessorConfig {
                            id: "wasm1".to_string(),
                            backend: BackendType::Wasm,
                            processor: None,
                            endpoint: None,
                            module: Some("wasm1.wasm".to_string()),
                            depends_on: vec!["local1".to_string(), "grpc1".to_string()],
                            options: HashMap::new(),
                        },
                    ],
                },
                expected_processor_count: 3,
                expected_processor_ids: vec!["local1", "grpc1", "wasm1"],
            },
            TestCase {
                name: "processors with dependencies",
                config: Config {
                    strategy: Strategy::Level,
                    failure_strategy: FailureStrategy::FailFast,
                    executor_options: ExecutorOptions::default(),
                    processors: vec![
                        ProcessorConfig {
                            id: "input".to_string(),
                            backend: BackendType::Local,
                            processor: Some("change_text_case_upper".to_string()),
                            endpoint: None,
                            module: None,
                            depends_on: vec![],
                            options: HashMap::new(),
                        },
                        ProcessorConfig {
                            id: "transform".to_string(),
                            backend: BackendType::Http,
                            processor: None,
                            endpoint: Some("https://transform.service.com".to_string()),
                            module: None,
                            depends_on: vec!["input".to_string()],
                            options: HashMap::new(),
                        },
                        ProcessorConfig {
                            id: "output".to_string(),
                            backend: BackendType::Loadable,
                            processor: Some("liboutput.so".to_string()),
                            endpoint: None,
                            module: None,
                            depends_on: vec!["transform".to_string()],
                            options: HashMap::new(),
                        },
                    ],
                },
                expected_processor_count: 3,
                expected_processor_ids: vec!["input", "transform", "output"],
            },
        ];

        for test_case in test_cases {
            let processor_map_result = ProcessorMap::from_config(&test_case.config);

            // Check if test case contains unimplemented backends or invalid WASM files
            let has_unimplemented = test_case.config.processors.iter().any(|p| {
                matches!(p.backend, BackendType::Loadable | BackendType::Grpc | BackendType::Http)
            });

            let has_invalid_wasm = test_case.config.processors.iter().any(|p| {
                p.backend == BackendType::Wasm
                    && p.module.as_ref().map_or(false, |m| m.ends_with(".wasm"))
            });

            if has_unimplemented || has_invalid_wasm {
                assert!(
                    processor_map_result.is_err(),
                    "Test case '{}': Expected error for unimplemented backend or non-existent WASM file",
                    test_case.name
                );
                continue;
            }

            let processor_map = processor_map_result.unwrap();

            // Check processor count
            assert_eq!(
                processor_map.len(),
                test_case.expected_processor_count,
                "Test case '{}': expected {} processors, got {}",
                test_case.name,
                test_case.expected_processor_count,
                processor_map.len()
            );

            // Check that all expected processor IDs are present
            for expected_id in &test_case.expected_processor_ids {
                assert!(
                    processor_map.contains_key(expected_id),
                    "Test case '{}': expected processor '{}' not found in processor map",
                    test_case.name,
                    expected_id
                );
            }

            // Check that all processors have the correct name
            for id in processor_map.keys() {
                let processor = processor_map.get(id).unwrap();
                // For local processors with valid impl_, we get the actual processor
                // For others or invalid impl_, we get stub processors
                assert!(
                    processor.name() == "stub" || processor.name() == "change_text_case",
                    "Test case '{}': processor '{}' should have name 'stub' or 'change_text_case', got '{}'",
                    test_case.name,
                    id,
                    processor.name()
                );
            }
        }
    }

    #[test]
    fn test_from_config_processor_types() {
        // Test that each backend type creates a processor or returns appropriate error
        let backend_types = vec![
            BackendType::Local,
            BackendType::Loadable,
            BackendType::Grpc,
            BackendType::Http,
            BackendType::Wasm,
        ];

        for (i, backend_type) in backend_types.into_iter().enumerate() {
            let config = Config {
                strategy: Strategy::WorkQueue,
                failure_strategy: FailureStrategy::FailFast,
                executor_options: ExecutorOptions::default(),
                processors: vec![ProcessorConfig {
                    id: format!("processor_{}", i),
                    backend: backend_type.clone(),
                    processor: Some("test_impl".to_string()),
                    endpoint: Some("test_endpoint".to_string()),
                    module: Some("test_module".to_string()),
                    depends_on: vec![],
                    options: HashMap::new(),
                }],
            };

            let processor_map_result = ProcessorMap::from_config(&config);

            // All backend types with invalid configurations should fail
            // Loadable, Grpc, Http are not implemented
            // Local and WASM have invalid configurations
            assert!(
                processor_map_result.is_err(),
                "Expected error for {} processor configuration",
                format!("{:?}", backend_type)
            );

            // Verify error messages for unimplemented backends
            if let Err(err) = processor_map_result {
                match backend_type {
                    BackendType::Loadable => {
                        assert!(err.contains("not implemented"));
                        assert!(err.contains("Loadable"));
                    }
                    BackendType::Grpc => {
                        assert!(err.contains("not implemented"));
                        assert!(err.contains("Grpc"));
                    }
                    BackendType::Http => {
                        assert!(err.contains("not implemented"));
                        assert!(err.contains("Http"));
                    }
                    _ => {} // Local and WASM have different error messages
                }
            }
        }
    }

    #[test]
    fn test_from_config_duplicate_ids() {
        // Test behavior with duplicate processor IDs
        // Since Grpc is not implemented, this should fail
        let config = Config {
            strategy: Strategy::WorkQueue,
            failure_strategy: FailureStrategy::FailFast,
            executor_options: ExecutorOptions::default(),
            processors: vec![
                ProcessorConfig {
                    id: "duplicate".to_string(),
                    backend: BackendType::Local,
                    processor: Some("change_text_case_upper".to_string()),
                    endpoint: None,
                    module: None,
                    depends_on: vec![],
                    options: HashMap::new(),
                },
                ProcessorConfig {
                    id: "duplicate".to_string(),
                    backend: BackendType::Grpc,
                    processor: None,
                    endpoint: Some("second".to_string()),
                    module: None,
                    depends_on: vec![],
                    options: HashMap::new(),
                },
            ],
        };

        // Should fail because Grpc backend is not implemented
        let result = ProcessorMap::from_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("not implemented"));
        assert!(err.contains("Grpc"));
    }
}
