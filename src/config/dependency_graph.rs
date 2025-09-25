use std::collections::HashMap;

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

impl From<DependencyGraph> for HashMap<String, Vec<String>> {
    fn from(graph: DependencyGraph) -> Self {
        graph.0
    }
}
