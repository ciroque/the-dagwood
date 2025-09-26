use std::collections::HashMap;
use std::sync::Arc;

use crate::traits::{Processor, DagExecutor};
use crate::config::{Config, BackendType, Strategy};
use crate::backends::stub::StubProcessor;
use crate::backends::local::LocalProcessorFactory;
use crate::engine::work_queue::WorkQueueExecutor;
use crate::errors::FailureStrategy;


/// Resolves processors from config into runtime instances
pub fn build_registry(cfg: &Config) -> HashMap<String, Arc<dyn Processor>> {
    let mut registry: HashMap<String, Arc<dyn Processor>> = HashMap::new();

    for p in &cfg.processors {
        let processor: Arc<dyn Processor> = match p.backend {
            BackendType::Local => {
                match LocalProcessorFactory::create_processor(p) {
                    Ok(processor) => processor,
                    Err(e) => {
                        eprintln!("Warning: Failed to create local processor '{}': {}. Using stub instead.", p.id, e);
                        Arc::new(StubProcessor::new(p.id.clone()))
                    }
                }
            }
            BackendType::Loadable => {
                // TODO: dynamic library loading
                Arc::new(StubProcessor::new(p.id.clone()))
            }
            BackendType::Grpc | BackendType::Http => {
                // TODO: build RPC client
                Arc::new(StubProcessor::new(p.id.clone()))
            }
            BackendType::Wasm => {
                // TODO: load WASM module
                Arc::new(StubProcessor::new(p.id.clone()))
            }
        };

        registry.insert(p.id.clone(), processor);
    }

    registry
}

/// Creates a DAG executor based on the configuration
pub fn build_executor(cfg: &Config) -> Box<dyn DagExecutor> {
    let max_concurrency = cfg.executor_options.max_concurrency
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        });

    match cfg.strategy {
        Strategy::WorkQueue => {
            Box::new(WorkQueueExecutor::new(max_concurrency))
        }
        Strategy::Level => {
            // TODO: Implement Level executor
            // For now, fallback to WorkQueue
            Box::new(WorkQueueExecutor::new(max_concurrency))
        }
        Strategy::Reactive => {
            // TODO: Implement Reactive executor
            // For now, fallback to WorkQueue
            Box::new(WorkQueueExecutor::new(max_concurrency))
        }
        Strategy::Hybrid => {
            // TODO: Implement Hybrid executor
            // For now, fallback to WorkQueue
            Box::new(WorkQueueExecutor::new(max_concurrency))
        }
    }
}

/// Builds both processor registry and executor from configuration
pub fn build_dag_runtime(cfg: &Config) -> (HashMap<String, Arc<dyn Processor>>, Box<dyn DagExecutor>, FailureStrategy) {
    let processors = build_registry(cfg);
    let executor = build_executor(cfg);
    let failure_strategy = cfg.failure_strategy;
    
    (processors, executor, failure_strategy)
}


#[cfg(test)]
mod tests {
    use super::*;

    use crate::config::ProcessorConfig;

    #[test]
    fn test_build_registry_table_driven() {
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
                    strategy: crate::config::Strategy::WorkQueue,
                    failure_strategy: crate::errors::FailureStrategy::FailFast,
                    executor_options: crate::config::ExecutorOptions::default(),
                    processors: vec![],
                },
                expected_processor_count: 0,
                expected_processor_ids: vec![],
            },
            TestCase {
                name: "single local processor",
                config: Config {
                    strategy: crate::config::Strategy::WorkQueue,
                    failure_strategy: crate::errors::FailureStrategy::FailFast,
                    executor_options: crate::config::ExecutorOptions::default(),
                    processors: vec![ProcessorConfig {
                        id: "local_proc".to_string(),
                        backend: BackendType::Local,
                        processor: Some("LocalImpl".to_string()),
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
                    strategy: crate::config::Strategy::Level,
                    failure_strategy: crate::errors::FailureStrategy::FailFast,
                    executor_options: crate::config::ExecutorOptions::default(),
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
                    strategy: crate::config::Strategy::Reactive,
                    failure_strategy: crate::errors::FailureStrategy::FailFast,
                    executor_options: crate::config::ExecutorOptions::default(),
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
                    strategy: crate::config::Strategy::Hybrid,
                    failure_strategy: crate::errors::FailureStrategy::FailFast,
                    executor_options: crate::config::ExecutorOptions::default(),
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
                    strategy: crate::config::Strategy::WorkQueue,
                    failure_strategy: crate::errors::FailureStrategy::FailFast,
                    executor_options: crate::config::ExecutorOptions::default(),
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
                    strategy: crate::config::Strategy::WorkQueue,
                    failure_strategy: crate::errors::FailureStrategy::FailFast,
                    executor_options: crate::config::ExecutorOptions::default(),
                    processors: vec![
                        ProcessorConfig {
                            id: "local1".to_string(),
                            backend: BackendType::Local,
                            processor: Some("LocalImpl1".to_string()),
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
                    strategy: crate::config::Strategy::Level,
                    failure_strategy: crate::errors::FailureStrategy::FailFast,
                    executor_options: crate::config::ExecutorOptions::default(),
                    processors: vec![
                        ProcessorConfig {
                            id: "input".to_string(),
                            backend: BackendType::Local,
                            processor: Some("InputProcessor".to_string()),
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
            let registry = build_registry(&test_case.config);
            
            // Check processor count
            assert_eq!(
                registry.len(),
                test_case.expected_processor_count,
                "Test case '{}': expected {} processors, got {}",
                test_case.name,
                test_case.expected_processor_count,
                registry.len()
            );

            // Check that all expected processor IDs are present
            for expected_id in &test_case.expected_processor_ids {
                assert!(
                    registry.contains_key(*expected_id),
                    "Test case '{}': expected processor '{}' not found in registry",
                    test_case.name,
                    expected_id
                );
            }

            // Check that all processors are StubProcessor instances (for now)
            // and have the correct name
            for (id, processor) in &registry {
                assert_eq!(
                    processor.name(),
                    "stub",
                    "Test case '{}': processor '{}' should have name 'stub'",
                    test_case.name,
                    id
                );
            }
        }
    }

    #[test]
    fn test_build_registry_processor_types() {
        // Test that each backend type creates a processor with the correct behavior
        let backend_types = vec![
            BackendType::Local,
            BackendType::Loadable,
            BackendType::Grpc,
            BackendType::Http,
            BackendType::Wasm,
        ];

        for (i, backend_type) in backend_types.into_iter().enumerate() {
            let config = Config {
                strategy: crate::config::Strategy::WorkQueue,
                failure_strategy: crate::errors::FailureStrategy::FailFast,
                executor_options: crate::config::ExecutorOptions::default(),
                processors: vec![ProcessorConfig {
                    id: format!("processor_{}", i),
                    backend: backend_type,
                    processor: Some("test_impl".to_string()),
                    endpoint: Some("test_endpoint".to_string()),
                    module: Some("test_module".to_string()),
                    depends_on: vec![],
                    options: HashMap::new(),
                }],
            };

            let registry = build_registry(&config);
            assert_eq!(registry.len(), 1);
            
            let processor_id = format!("processor_{}", i);
            let processor = registry.get(&processor_id).unwrap();
            assert_eq!(processor.name(), "stub");
        }
    }

    #[test]
    fn test_build_registry_duplicate_ids() {
        // Test behavior with duplicate processor IDs (last one should win)
        let config = Config {
            strategy: crate::config::Strategy::WorkQueue,
            failure_strategy: crate::errors::FailureStrategy::FailFast,
            executor_options: crate::config::ExecutorOptions::default(),
            processors: vec![
                ProcessorConfig {
                    id: "duplicate".to_string(),
                    backend: BackendType::Local,
                    processor: Some("first".to_string()),
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

        let registry = build_registry(&config);
        assert_eq!(registry.len(), 1);
        assert!(registry.contains_key("duplicate"));
    }
}