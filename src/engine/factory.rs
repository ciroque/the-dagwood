use crate::config::{Config, Strategy};
use crate::traits::DagExecutor;
use crate::engine::work_queue::WorkQueueExecutor;
use crate::engine::level_by_level::LevelByLevelExecutor;

/// Factory for creating DAG executors from configuration
pub struct ExecutorFactory;

impl ExecutorFactory {
    /// Create a DAG executor based on the configuration strategy
    pub fn from_config(cfg: &Config) -> Box<dyn DagExecutor> {
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
                Box::new(LevelByLevelExecutor::new(max_concurrency))
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ExecutorOptions, ProcessorConfig, BackendType};
    use crate::errors::FailureStrategy;

    fn create_test_config(strategy: Strategy) -> Config {
        Config {
            strategy,
            failure_strategy: FailureStrategy::FailFast,
            executor_options: ExecutorOptions::default(),
            processors: vec![ProcessorConfig {
                id: "test_processor".to_string(),
                backend: BackendType::Local,
                processor: Some("TestProcessor".to_string()),
                endpoint: None,
                module: None,
                depends_on: vec![],
                options: std::collections::HashMap::new(),
            }],
        }
    }

    #[test]
    fn test_factory_creates_work_queue_executor() {
        let config = create_test_config(Strategy::WorkQueue);
        let _executor = ExecutorFactory::from_config(&config);
        
        // Test passes if executor creation doesn't panic
        // The actual executor type verification would be done in integration tests
    }

    #[test]
    fn test_factory_creates_level_by_level_executor() {
        let config = create_test_config(Strategy::Level);
        let _executor = ExecutorFactory::from_config(&config);
        
        // Test passes if executor creation doesn't panic
    }

    #[test]
    fn test_factory_creates_reactive_executor_fallback() {
        let config = create_test_config(Strategy::Reactive);
        let _executor = ExecutorFactory::from_config(&config);
        
        // Currently falls back to WorkQueue - this will change when Reactive is implemented
        // Test passes if executor creation doesn't panic
    }

    #[test]
    fn test_factory_creates_hybrid_executor_fallback() {
        let config = create_test_config(Strategy::Hybrid);
        let _executor = ExecutorFactory::from_config(&config);
        
        // Currently falls back to WorkQueue - this will change when Hybrid is implemented
        // Test passes if executor creation doesn't panic
    }

    #[test]
    fn test_factory_respects_max_concurrency_option() {
        let mut config = create_test_config(Strategy::WorkQueue);
        config.executor_options.max_concurrency = Some(8);
        
        let _executor = ExecutorFactory::from_config(&config);
        
        // Executor should be created with the specified concurrency
        // The actual concurrency verification would be done in integration tests
        // Test passes if executor creation doesn't panic
    }

    #[test]
    fn test_factory_uses_default_concurrency_when_none_specified() {
        let config = create_test_config(Strategy::WorkQueue);
        // executor_options.max_concurrency is None by default
        
        let _executor = ExecutorFactory::from_config(&config);
        
        // Should use default concurrency (CPU cores or 4)
        // Test passes if executor creation doesn't panic
    }
}
