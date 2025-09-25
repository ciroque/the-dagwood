use async_trait::async_trait;
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse};
use crate::errors::{ExecutionError, FailureStrategy};
use std::collections::HashMap;

use crate::config::{DependencyGraph, EntryPoints, ProcessorMap};

#[async_trait]
pub trait DagExecutor: Send + Sync {
    /// Execute a pipeline given processors and their dependency graph.
    ///
    /// - `processors`: registry mapping id -> processor instance
    /// - `graph`: adjacency list (id -> list of dependents)
    /// - `entrypoints`: processors with no dependencies
    /// - `input`: initial request payload
    /// - `failure_strategy`: how to handle processor failures (optional, defaults to FailFast)
    ///
    /// Returns a Result containing either:
    /// - Ok(HashMap): Successful execution results for all processors
    /// - Err(ExecutionError): Details about what went wrong during execution
    async fn execute(
        &self,
        processors: ProcessorMap,
        graph: DependencyGraph,
        entrypoints: EntryPoints,
        input: ProcessorRequest,
    ) -> Result<HashMap<String, ProcessorResponse>, ExecutionError> {
        self.execute_with_strategy(processors, graph, entrypoints, input, FailureStrategy::default()).await
    }

    /// Execute with a specific failure handling strategy
    async fn execute_with_strategy(
        &self,
        processors: ProcessorMap,
        graph: DependencyGraph,
        entrypoints: EntryPoints,
        input: ProcessorRequest,
        failure_strategy: FailureStrategy,
    ) -> Result<HashMap<String, ProcessorResponse>, ExecutionError>;
}
