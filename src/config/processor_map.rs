use std::collections::HashMap;
use std::sync::Arc;
use crate::traits::Processor;

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

impl From<ProcessorMap> for HashMap<String, Arc<dyn Processor>> {
    fn from(map: ProcessorMap) -> Self {
        map.0
    }
}
