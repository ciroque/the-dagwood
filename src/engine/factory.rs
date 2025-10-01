use crate::config::{Config, Strategy};
use crate::traits::DagExecutor;
use crate::engine::work_queue::WorkQueueExecutor;

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
}
