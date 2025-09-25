use async_trait::async_trait;
use crate::traits::processor::Processor;
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse};
use crate::errors::{ExecutionError, FailureStrategy};
use std::sync::Arc;
use std::collections::HashMap;

/// Newtype wrapper for processor registry providing type safety
#[derive(Clone)]
pub struct ProcessorMap(pub HashMap<String, Arc<dyn Processor>>);

impl ProcessorMap {
    /// Create a new empty processor map
    pub fn new() -> Self {
        Self(HashMap::new())
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

impl Into<HashMap<String, Arc<dyn Processor>>> for ProcessorMap {
    fn into(self) -> HashMap<String, Arc<dyn Processor>> {
        self.0
    }
}

/// Newtype wrapper for dependency graph providing type safety
#[derive(Debug, Clone)]
pub struct DependencyGraph(pub HashMap<String, Vec<String>>);

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self(HashMap::new())
    }
    
    /// Add a dependency relationship
    pub fn add_dependency(&mut self, processor_id: String, dependents: Vec<String>) {
        self.0.insert(processor_id, dependents);
    }
    
    /// Get dependents for a processor
    pub fn get_dependents(&self, processor_id: &str) -> Option<&Vec<String>> {
        self.0.get(processor_id)
    }
    
    /// Get all processor IDs in the graph
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.keys()
    }
    
    /// Get all values in the graph
    pub fn values(&self) -> impl Iterator<Item = &Vec<String>> {
        self.0.values()
    }
}

impl From<HashMap<String, Vec<String>>> for DependencyGraph {
    fn from(graph: HashMap<String, Vec<String>>) -> Self {
        Self(graph)
    }
}

impl Into<HashMap<String, Vec<String>>> for DependencyGraph {
    fn into(self) -> HashMap<String, Vec<String>> {
        self.0
    }
}

/// Newtype wrapper for entrypoints providing type safety
#[derive(Debug, Clone)]
pub struct EntryPoints(pub Vec<String>);

impl EntryPoints {
    /// Create a new empty entrypoints list
    pub fn new() -> Self {
        Self(Vec::new())
    }
    
    /// Add an entrypoint
    pub fn add(&mut self, processor_id: String) {
        self.0.push(processor_id);
    }
    
    /// Get iterator over entrypoints
    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.0.iter()
    }
}

impl From<Vec<String>> for EntryPoints {
    fn from(entrypoints: Vec<String>) -> Self {
        Self(entrypoints)
    }
}

impl Into<Vec<String>> for EntryPoints {
    fn into(self) -> Vec<String> {
        self.0
    }
}

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
