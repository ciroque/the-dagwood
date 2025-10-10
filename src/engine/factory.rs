// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use crate::config::{Config, Strategy};
use crate::engine::level_by_level::LevelByLevelExecutor;
use crate::engine::reactive::ReactiveExecutor;
use crate::engine::work_queue::WorkQueueExecutor;
use crate::traits::DagExecutor;

/// Factory for creating DAG executors from configuration
pub struct ExecutorFactory;

impl ExecutorFactory {
    /// Create a DAG executor based on the configuration strategy
    pub fn from_config(cfg: &Config) -> Box<dyn DagExecutor> {
        let max_concurrency = cfg.executor_options.max_concurrency.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        });

        match cfg.strategy {
            Strategy::WorkQueue => Box::new(WorkQueueExecutor::new(max_concurrency)),
            Strategy::Level => Box::new(LevelByLevelExecutor::new(max_concurrency)),
            Strategy::Reactive => Box::new(ReactiveExecutor::new(max_concurrency)),
            Strategy::Hybrid => {
                // TODO: Implement Hybrid executor
                // For now, fallback to WorkQueue
                Box::new(WorkQueueExecutor::new(max_concurrency))
            }
        }
    }
}
