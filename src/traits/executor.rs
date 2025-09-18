use async_trait::async_trait;
use crate::traits::processor::Processor;
use crate::proto::processor::v1::{ProcessorRequest, ProcessorResponse};
use std::sync::Arc;
use std::collections::HashMap;

#[async_trait]
pub trait DagExecutor: Send + Sync {
    /// Execute a pipeline given processors and their dependency graph.
    ///
    /// - `processors`: registry mapping id -> processor instance
    /// - `graph`: adjacency list (id -> list of dependents)
    /// - `entrypoints`: processors with no dependencies
    /// - `input`: initial request payload
    async fn execute(
        &self,
        processors: HashMap<String, Arc<dyn Processor>>,
        graph: HashMap<String, Vec<String>>,
        entrypoints: Vec<String>,
        input: ProcessorRequest,
    ) -> HashMap<String, ProcessorResponse>;
}
