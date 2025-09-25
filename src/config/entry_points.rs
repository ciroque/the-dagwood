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

impl From<EntryPoints> for Vec<String>{
    fn from(value: EntryPoints) -> Self {
        value.0
    }
}
