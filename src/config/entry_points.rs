/// A type-safe wrapper for DAG entry points - processors with no dependencies.
///
/// Entry points are processors that can be executed immediately when a DAG starts,
/// as they don't depend on any other processors. In a typical DAG execution,
/// entry points are identified during dependency analysis and serve as the starting
/// nodes for topological traversal.
///
/// # Examples
///
/// ## Creating entry points from a vector
/// ```
/// use the_dagwood::config::EntryPoints;
/// 
/// let entry_points = EntryPoints::from(vec![
///     "input_processor".to_string(),
///     "config_loader".to_string()
/// ]);
/// 
/// assert_eq!(entry_points.0.len(), 2);
/// ```
///
/// ## Building entry points incrementally
/// ```
/// use the_dagwood::config::EntryPoints;
/// 
/// let mut entry_points = EntryPoints::new();
/// entry_points.add("data_ingestion".to_string());
/// entry_points.add("metadata_parser".to_string());
/// 
/// let processor_names: Vec<&String> = entry_points.iter().collect();
/// assert_eq!(processor_names.len(), 2);
/// assert!(processor_names.contains(&&"data_ingestion".to_string()));
/// ```
///
/// ## Converting back to Vec<String>
/// ```
/// use the_dagwood::config::EntryPoints;
/// 
/// let entry_points = EntryPoints::from(vec!["processor1".to_string()]);
/// let vec_form: Vec<String> = entry_points.into();
/// assert_eq!(vec_form, vec!["processor1".to_string()]);
/// ```
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
