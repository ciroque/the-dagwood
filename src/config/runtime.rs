// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use crate::config::{Config, ProcessorMap};
use crate::engine::factory::ExecutorFactory;
use crate::errors::FailureStrategy;
use crate::traits::DagExecutor;

/// DAG runtime builder - orchestrates processor map and executor creation from configuration.
///
/// The `RuntimeBuilder` provides a clean interface for creating complete DAG runtime
/// environments from configuration. It coordinates the creation of both the processor
/// registry and the execution engine, ensuring they're properly configured and compatible.
///
/// # Examples
///
/// ## Building runtime from configuration
/// ```
/// use the_dagwood::config::{RuntimeBuilder, load_config};
///
/// # // This is a mock example since we can't load actual files in doctests
/// # let config = the_dagwood::config::Config {
/// #     strategy: the_dagwood::config::Strategy::WorkQueue,
/// #     failure_strategy: the_dagwood::errors::FailureStrategy::FailFast,
/// #     executor_options: the_dagwood::config::ExecutorOptions::default(),
/// #     processors: vec![],
/// # };
///
/// let (processors, executor, failure_strategy) = RuntimeBuilder::from_config(&config).unwrap();
///
/// // Runtime is ready for DAG execution
/// assert_eq!(failure_strategy, the_dagwood::errors::FailureStrategy::FailFast);
/// ```
pub struct RuntimeBuilder;

impl RuntimeBuilder {
    /// Build complete DAG runtime from configuration.
    ///
    /// Creates and returns:
    /// - `ProcessorMap`: Registry of all configured processors
    /// - `Box<dyn DagExecutor>`: Executor configured per strategy
    /// - `FailureStrategy`: How to handle processor failures
    ///
    /// # Arguments
    /// * `cfg` - Configuration containing processor definitions, execution strategy, and options
    ///
    /// # Returns
    /// A tuple of (ProcessorMap, DagExecutor, FailureStrategy) ready for DAG execution
    pub fn from_config(cfg: &Config) -> Result<(ProcessorMap, Box<dyn DagExecutor>, FailureStrategy), String> {
        let processors = ProcessorMap::from_config(cfg)?;
        let executor = ExecutorFactory::from_config(cfg);
        Ok((processors, executor, cfg.failure_strategy))
    }
}
