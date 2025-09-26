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

    /// Build the dependency count map from the adjacency graph.
    /// Returns a map of processor_id -> number of incoming dependencies.
    pub fn build_dependency_counts(&self) -> HashMap<String, usize> {
        let mut dependency_counts = HashMap::new();
        
        // Initialize all processors with 0 dependencies
        for processor_id in self.0.keys() {
            dependency_counts.insert(processor_id.clone(), 0);
        }
        
        // Count incoming dependencies for each processor
        for dependents in self.0.values() {
            for dependent_id in dependents {
                *dependency_counts.entry(dependent_id.clone()).or_insert(0) += 1;
            }
        }
        
        dependency_counts
    }

    /// Build a reverse dependency map: processor_id -> list of processors it depends on.
    /// This is useful for determining input sources for each processor.
    pub fn build_reverse_dependencies(&self) -> HashMap<String, Vec<String>> {
        let mut reverse_deps = HashMap::new();
        
        // Initialize all processors with empty dependency lists
        for processor_id in self.0.keys() {
            reverse_deps.insert(processor_id.clone(), vec![]);
        }
        
        // Build reverse mapping
        for (processor_id, dependents) in &self.0 {
            for dependent_id in dependents {
                reverse_deps.entry(dependent_id.clone())
                    .or_insert_with(Vec::new)
                    .push(processor_id.clone());
            }
        }
        
        reverse_deps
    }

    /// Compute a topological sort order using the provided dependency counts.
    /// Returns a vector of processor IDs in topological order, or None if the graph has cycles.
    /// Uses Kahn's algorithm for topological sorting.
    /// 
    /// This is a more efficient version that reuses pre-computed dependency counts.
    pub fn topological_sort_with_counts(&self, mut dependency_counts: HashMap<String, usize>) -> Option<Vec<String>> {
        let mut result = Vec::new();
        let mut queue = Vec::new();
        
        // Find all processors with no dependencies (in-degree 0)
        for (processor_id, &count) in &dependency_counts {
            if count == 0 {
                queue.push(processor_id.clone());
            }
        }
        
        // Process processors in topological order
        while let Some(processor_id) = queue.pop() {
            result.push(processor_id.clone());
            
            // For each dependent of this processor
            if let Some(dependents) = self.0.get(&processor_id) {
                for dependent_id in dependents {
                    // Decrease the dependency count
                    if let Some(count) = dependency_counts.get_mut(dependent_id) {
                        *count -= 1;
                        
                        // If dependency count reaches zero, add to queue
                        if *count == 0 {
                            queue.push(dependent_id.clone());
                        }
                    }
                }
            }
        }
        
        // Check if all processors were processed (no cycles)
        if result.len() == self.0.len() {
            Some(result)
        } else {
            None // Graph has cycles
        }
    }

    /// Compute a topological sort order of the processors in the graph.
    /// Returns a vector of processor IDs in topological order, or None if the graph has cycles.
    /// Uses Kahn's algorithm for topological sorting.
    pub fn topological_sort(&self) -> Option<Vec<String>> {
        let dependency_counts = self.build_dependency_counts();
        self.topological_sort_with_counts(dependency_counts)
    }

    /// Efficiently compute both dependency counts and topological ranks together.
    /// Returns a tuple of (dependency_counts, topological_ranks) or None if the graph has cycles.
    /// This is more efficient than calling build_dependency_counts() and topological_ranks() separately.
    pub fn dependency_counts_and_ranks(&self) -> Option<(HashMap<String, usize>, HashMap<String, usize>)> {
        let dependency_counts = self.build_dependency_counts();
        // topological_sort_with_counts takes ownership and modifies the HashMap, so clone for the call
        let sorted_processors = self.topological_sort_with_counts(dependency_counts.clone())?;
        
        let ranks = sorted_processors
            .iter()
            .enumerate()
            .map(|(rank, processor_id)| (processor_id.clone(), rank))
            .collect();
            
        Some((dependency_counts, ranks))
    }

    /// Get topological ranks for all processors in the graph.
    /// Returns a map of processor_id -> rank, where rank 0 is earliest in topological order.
    /// Returns None if the graph has cycles.
    pub fn topological_ranks(&self) -> Option<HashMap<String, usize>> {
        self.topological_sort().map(|sorted_processors| {
            sorted_processors
                .iter()
                .enumerate()
                .map(|(rank, processor_id)| (processor_id.clone(), rank))
                .collect()
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_dependency_counts() {
        // Create a diamond dependency graph: a -> [b, c] -> d
        let mut graph = HashMap::new();
        graph.insert("a".to_string(), vec!["b".to_string(), "c".to_string()]);
        graph.insert("b".to_string(), vec!["d".to_string()]);
        graph.insert("c".to_string(), vec!["d".to_string()]);
        graph.insert("d".to_string(), vec![]);
        
        let dependency_graph = DependencyGraph::from(graph);
        let counts = dependency_graph.build_dependency_counts();
        
        assert_eq!(counts.get("a"), Some(&0)); // No dependencies
        assert_eq!(counts.get("b"), Some(&1)); // Depends on a
        assert_eq!(counts.get("c"), Some(&1)); // Depends on a
        assert_eq!(counts.get("d"), Some(&2)); // Depends on b and c
    }

    #[test]
    fn test_build_reverse_dependencies() {
        // Create a linear chain: a -> b -> c
        let mut graph = HashMap::new();
        graph.insert("a".to_string(), vec!["b".to_string()]);
        graph.insert("b".to_string(), vec!["c".to_string()]);
        graph.insert("c".to_string(), vec![]);
        
        let dependency_graph = DependencyGraph::from(graph);
        let reverse_deps = dependency_graph.build_reverse_dependencies();
        
        assert_eq!(reverse_deps.get("a"), Some(&vec![])); // No dependencies
        assert_eq!(reverse_deps.get("b"), Some(&vec!["a".to_string()])); // Depends on a
        assert_eq!(reverse_deps.get("c"), Some(&vec!["b".to_string()])); // Depends on b
    }

    #[test]
    fn test_topological_sort_valid_graph() {
        // Create a diamond dependency graph: a -> [b, c] -> d
        let mut graph = HashMap::new();
        graph.insert("a".to_string(), vec!["b".to_string(), "c".to_string()]);
        graph.insert("b".to_string(), vec!["d".to_string()]);
        graph.insert("c".to_string(), vec!["d".to_string()]);
        graph.insert("d".to_string(), vec![]);
        
        let dependency_graph = DependencyGraph::from(graph);
        let topo_order = dependency_graph.topological_sort().unwrap();
        
        // Verify the order respects dependencies
        let a_pos = topo_order.iter().position(|x| x == "a").unwrap();
        let b_pos = topo_order.iter().position(|x| x == "b").unwrap();
        let c_pos = topo_order.iter().position(|x| x == "c").unwrap();
        let d_pos = topo_order.iter().position(|x| x == "d").unwrap();
        
        assert!(a_pos < b_pos); // a comes before b
        assert!(a_pos < c_pos); // a comes before c
        assert!(b_pos < d_pos); // b comes before d
        assert!(c_pos < d_pos); // c comes before d
        assert_eq!(topo_order.len(), 4); // All processors included
    }

    #[test]
    fn test_topological_sort_cyclic_graph() {
        // Create a cyclic graph: a -> b -> c -> a
        let mut graph = HashMap::new();
        graph.insert("a".to_string(), vec!["b".to_string()]);
        graph.insert("b".to_string(), vec!["c".to_string()]);
        graph.insert("c".to_string(), vec!["a".to_string()]);
        
        let dependency_graph = DependencyGraph::from(graph);
        let topo_order = dependency_graph.topological_sort();
        
        assert!(topo_order.is_none()); // Should return None for cyclic graphs
    }

    #[test]
    fn test_topological_ranks() {
        // Create a linear chain: a -> b -> c
        let mut graph = HashMap::new();
        graph.insert("a".to_string(), vec!["b".to_string()]);
        graph.insert("b".to_string(), vec!["c".to_string()]);
        graph.insert("c".to_string(), vec![]);
        
        let dependency_graph = DependencyGraph::from(graph);
        let ranks = dependency_graph.topological_ranks().unwrap();
        
        assert_eq!(ranks.get("a"), Some(&0)); // First in order
        assert_eq!(ranks.get("b"), Some(&1)); // Second in order
        assert_eq!(ranks.get("c"), Some(&2)); // Third in order
    }

    #[test]
    fn test_topological_ranks_cyclic_graph() {
        // Create a cyclic graph: a -> b -> a
        let mut graph = HashMap::new();
        graph.insert("a".to_string(), vec!["b".to_string()]);
        graph.insert("b".to_string(), vec!["a".to_string()]);
        
        let dependency_graph = DependencyGraph::from(graph);
        let ranks = dependency_graph.topological_ranks();
        
        assert!(ranks.is_none()); // Should return None for cyclic graphs
    }

    #[test]
    fn test_empty_graph() {
        let dependency_graph = DependencyGraph::new();
        
        let counts = dependency_graph.build_dependency_counts();
        let reverse_deps = dependency_graph.build_reverse_dependencies();
        let topo_order = dependency_graph.topological_sort().unwrap();
        let ranks = dependency_graph.topological_ranks().unwrap();
        
        assert!(counts.is_empty());
        assert!(reverse_deps.is_empty());
        assert!(topo_order.is_empty());
        assert!(ranks.is_empty());
    }

    #[test]
    fn test_single_processor() {
        let mut graph = HashMap::new();
        graph.insert("a".to_string(), vec![]);
        
        let dependency_graph = DependencyGraph::from(graph);
        let counts = dependency_graph.build_dependency_counts();
        let reverse_deps = dependency_graph.build_reverse_dependencies();
        let topo_order = dependency_graph.topological_sort().unwrap();
        let ranks = dependency_graph.topological_ranks().unwrap();
        
        assert_eq!(counts.get("a"), Some(&0));
        assert_eq!(reverse_deps.get("a"), Some(&vec![]));
        assert_eq!(topo_order, vec!["a"]);
        assert_eq!(ranks.get("a"), Some(&0));
    }

    #[test]
    fn test_multiple_entrypoints() {
        // Create a graph with multiple entry points: [a, b] -> c
        let mut graph = HashMap::new();
        graph.insert("a".to_string(), vec!["c".to_string()]);
        graph.insert("b".to_string(), vec!["c".to_string()]);
        graph.insert("c".to_string(), vec![]);
        
        let dependency_graph = DependencyGraph::from(graph);
        let counts = dependency_graph.build_dependency_counts();
        let reverse_deps = dependency_graph.build_reverse_dependencies();
        let topo_order = dependency_graph.topological_sort().unwrap();
        
        assert_eq!(counts.get("a"), Some(&0)); // No dependencies
        assert_eq!(counts.get("b"), Some(&0)); // No dependencies
        assert_eq!(counts.get("c"), Some(&2)); // Depends on a and b
        
        assert_eq!(reverse_deps.get("a"), Some(&vec![])); // No dependencies
        assert_eq!(reverse_deps.get("b"), Some(&vec![])); // No dependencies
        let c_deps = reverse_deps.get("c").unwrap();
        assert_eq!(c_deps.len(), 2);
        assert!(c_deps.contains(&"a".to_string()));
        assert!(c_deps.contains(&"b".to_string()));
        
        assert_eq!(topo_order.len(), 3);
        let c_pos = topo_order.iter().position(|x| x == "c").unwrap();
        let a_pos = topo_order.iter().position(|x| x == "a").unwrap();
        let b_pos = topo_order.iter().position(|x| x == "b").unwrap();
        assert!(a_pos < c_pos); // a comes before c
        assert!(b_pos < c_pos); // b comes before c
    }

    #[test]
    fn test_dependency_counts_and_ranks_efficiency() {
        let mut graph = HashMap::new();
        graph.insert("a".to_string(), vec!["b".to_string(), "c".to_string()]);
        graph.insert("b".to_string(), vec!["d".to_string()]);
        graph.insert("c".to_string(), vec!["d".to_string()]);
        graph.insert("d".to_string(), vec![]);
        
        let dependency_graph = DependencyGraph::from(graph);
        
        // Test the efficient combined method
        let result = dependency_graph.dependency_counts_and_ranks();
        assert!(result.is_some());
        
        let (counts, ranks) = result.unwrap();
        
        // Verify dependency counts
        assert_eq!(counts.get("a"), Some(&0)); // No dependencies
        assert_eq!(counts.get("b"), Some(&1)); // Depends on a
        assert_eq!(counts.get("c"), Some(&1)); // Depends on a
        assert_eq!(counts.get("d"), Some(&2)); // Depends on b and c
        
        // Verify topological ranks
        assert_eq!(ranks.get("a"), Some(&0)); // First in topological order
        assert_eq!(ranks.get("d"), Some(&3)); // Last in topological order
        
        // Verify that separate method calls produce same results
        let separate_counts = dependency_graph.build_dependency_counts();
        let separate_ranks = dependency_graph.topological_ranks().unwrap();
        
        assert_eq!(counts, separate_counts);
        assert_eq!(ranks, separate_ranks);
    }

    #[test]
    fn test_dependency_counts_and_ranks_cyclic_graph() {
        let mut graph = HashMap::new();
        graph.insert("a".to_string(), vec!["b".to_string()]);
        graph.insert("b".to_string(), vec!["a".to_string()]); // Cycle
        
        let dependency_graph = DependencyGraph::from(graph);
        let result = dependency_graph.dependency_counts_and_ranks();
        
        assert!(result.is_none()); // Should return None for cyclic graph
    }
}
