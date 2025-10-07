use async_trait::async_trait;
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, PipelineMetadata};
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
    /// - `pipeline_metadata`: metadata accumulator for the entire pipeline
    /// - `failure_strategy`: how to handle processor failures
    ///
    /// Returns a Result containing either:
    /// - Ok((HashMap, PipelineMetadata)): Successful execution results and accumulated metadata
    /// - Err(ExecutionError): Details about what went wrong during execution
    async fn execute_with_strategy(
        &self,
        processors: ProcessorMap,
        graph: DependencyGraph,
        entrypoints: EntryPoints,
        input: ProcessorRequest,
        mut pipeline_metadata: PipelineMetadata,
        failure_strategy: FailureStrategy,
    ) -> Result<(HashMap<String, ProcessorResponse>, PipelineMetadata), ExecutionError>;

    /// Test convenience method that uses the default failure strategy (FailFast).
    /// Production code should use `execute_with_strategy` to explicitly specify failure handling.
    #[cfg(test)]
    async fn execute(
        &self,
        processors: ProcessorMap,
        graph: DependencyGraph,
        entrypoints: EntryPoints,
        input: ProcessorRequest,
    ) -> Result<HashMap<String, ProcessorResponse>, ExecutionError> {
        let pipeline_metadata = PipelineMetadata::new();
        let (results, _metadata) = self.execute_with_strategy(processors, graph, entrypoints, input, pipeline_metadata, FailureStrategy::default()).await?;
        Ok(results)
    }
}
