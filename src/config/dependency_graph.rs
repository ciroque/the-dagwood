use std::collections::HashMap;

/// A type-safe wrapper for DAG dependency relationships with graph algorithms.
///
/// The `DependencyGraph` represents processor dependencies as a directed acyclic graph (DAG)
/// where each processor maps to a list of processors that depend on it. This forward adjacency
/// representation enables efficient topological sorting and dependency analysis for DAG execution.
///
/// The internal structure is `HashMap<String, Vec<String>>` where:
/// - **Key**: Processor ID that produces output
/// - **Value**: List of processor IDs that consume this processor's output
///
/// This forward representation (A → [B, C]) is optimal for:
/// - **Topological sorting**: Efficiently traverse dependents during Kahn's algorithm
/// - **Dependency counting**: Count incoming edges for each processor
/// - **Cycle detection**: DFS-based cycle detection with forward edges
/// - **Execution planning**: Determine which processors become ready when dependencies complete
///
/// # Graph Algorithms
///
/// The `DependencyGraph` provides multiple algorithms for different use cases:
///
/// ## Topological Sorting
/// - **`topological_sort()`**: Standard Kahn's algorithm for dependency ordering
/// - **`topological_sort_dfs()`**: DFS-based sorting that preserves original state
/// - **`topological_sort_with_counts()`**: Efficient sorting with pre-computed dependency counts
///
/// ## Dependency Analysis
/// - **`build_dependency_counts()`**: Count incoming dependencies for each processor
/// - **`build_reverse_dependencies()`**: Create reverse mapping (processor → dependencies)
/// - **`dependency_counts_and_ranks()`**: Efficiently compute both counts and topological ranks
///
/// # Examples
///
/// ## Creating a simple linear dependency chain
/// ```
/// use std::collections::HashMap;
/// use the_dagwood::config::DependencyGraph;
/// 
/// // Create chain: input → transform → output
/// let mut graph = HashMap::new();
/// graph.insert("input".to_string(), vec!["transform".to_string()]);
/// graph.insert("transform".to_string(), vec!["output".to_string()]);
/// graph.insert("output".to_string(), vec![]);
/// 
/// let dependency_graph = DependencyGraph::from(graph);
/// 
/// // Get topological order for execution
/// let execution_order = dependency_graph.topological_sort().unwrap();
/// assert_eq!(execution_order, vec!["input", "transform", "output"]);
/// ```
///
/// ## Creating a diamond dependency pattern
/// ```
/// use std::collections::HashMap;
/// use the_dagwood::config::DependencyGraph;
/// 
/// // Create diamond: source → [left, right] → sink
/// let mut graph = HashMap::new();
/// graph.insert("source".to_string(), vec!["left".to_string(), "right".to_string()]);
/// graph.insert("left".to_string(), vec!["sink".to_string()]);
/// graph.insert("right".to_string(), vec!["sink".to_string()]);
/// graph.insert("sink".to_string(), vec![]);
/// 
/// let dependency_graph = DependencyGraph::from(graph);
/// 
/// // Analyze dependency structure
/// let dependency_counts = dependency_graph.build_dependency_counts();
/// assert_eq!(dependency_counts.get("source"), Some(&0)); // No dependencies
/// assert_eq!(dependency_counts.get("sink"), Some(&2));   // Depends on left + right
/// ```
///
/// ## Building dependency analysis for execution planning
/// ```
/// use std::collections::HashMap;
/// use the_dagwood::config::DependencyGraph;
/// 
/// let mut graph = HashMap::new();
/// graph.insert("data_loader".to_string(), vec!["validator".to_string(), "transformer".to_string()]);
/// graph.insert("validator".to_string(), vec!["merger".to_string()]);
/// graph.insert("transformer".to_string(), vec!["merger".to_string()]);
/// graph.insert("merger".to_string(), vec![]);
/// 
/// let dependency_graph = DependencyGraph::from(graph);
/// 
/// // Get both dependency counts and topological ranks efficiently
/// let (counts, ranks) = dependency_graph.dependency_counts_and_ranks().unwrap();
/// 
/// // Use for execution planning
/// assert_eq!(ranks.get("data_loader"), Some(&0)); // Execute first
/// assert_eq!(ranks.get("merger"), Some(&3));      // Execute last
/// assert_eq!(counts.get("merger"), Some(&2));     // Wait for 2 dependencies
/// ```
///
/// ## Reverse dependency mapping for input resolution
/// ```
/// use std::collections::HashMap;
/// use the_dagwood::config::DependencyGraph;
/// 
/// let mut graph = HashMap::new();
/// graph.insert("input1".to_string(), vec!["processor".to_string()]);
/// graph.insert("input2".to_string(), vec!["processor".to_string()]);
/// graph.insert("processor".to_string(), vec![]);
/// 
/// let dependency_graph = DependencyGraph::from(graph);
/// 
/// // Find what each processor depends on (for input resolution)
/// let reverse_deps = dependency_graph.build_reverse_dependencies();
/// let processor_inputs = reverse_deps.get("processor").unwrap();
/// 
/// assert_eq!(processor_inputs.len(), 2);
/// assert!(processor_inputs.contains(&"input1".to_string()));
/// assert!(processor_inputs.contains(&"input2".to_string()));
/// ```
///
/// ## Cycle detection for validation
/// ```
/// use std::collections::HashMap;
/// use the_dagwood::config::DependencyGraph;
/// 
/// // Create cyclic graph: A → B → C → A
/// let mut cyclic_graph = HashMap::new();
/// cyclic_graph.insert("A".to_string(), vec!["B".to_string()]);
/// cyclic_graph.insert("B".to_string(), vec!["C".to_string()]);
/// cyclic_graph.insert("C".to_string(), vec!["A".to_string()]);
/// 
/// let dependency_graph = DependencyGraph::from(cyclic_graph);
/// 
/// // Detect cycles during validation
/// assert!(dependency_graph.topological_sort().is_none()); // Returns None for cycles
/// assert!(dependency_graph.topological_sort_dfs().is_none()); // DFS also detects cycles
/// ```
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

    /// Compute a topological sort using DFS with cycle detection.
    /// Returns Some(order) if acyclic, or None if a cycle is detected.
    ///
    /// This variant does not require or mutate dependency counts, making it
    /// suitable for scenarios where we also need to preserve the original
    /// indegree map (e.g., when returning both counts and ranks together).
    ///
    /// # When to use this DFS-based approach vs. Kahn's algorithm
    /// - Use this DFS-based method when you need to preserve the original indegree map,
    ///   as it does not require mutating or maintaining dependency counts.
    /// - This approach is suitable for scenarios where you want to avoid side effects
    ///   on the graph's state, or when you need to return both the topological order
    ///   and the original dependency counts together.
    /// - Kahn's algorithm (if implemented elsewhere) is generally more efficient for
    ///   large graphs and is the standard approach for topological sorting, but it
    ///   requires mutating or copying the indegree map.
    ///
    /// Choose the method that best fits your use case: use DFS for immutability and
    /// preservation of state, or Kahn's algorithm for performance on large graphs.
    pub fn topological_sort_dfs(&self) -> Option<Vec<String>> {
        // 0 = unvisited, 1 = visiting, 2 = visited
        let mut state: HashMap<&str, u8> = HashMap::new();
        for k in self.0.keys() {
            state.insert(k.as_str(), 0);
        }

        let mut order: Vec<String> = Vec::with_capacity(self.0.len());

        const UNVISITED: u8 = 0;
        const VISITING: u8 = 1;
        const VISITED: u8 = 2;

        fn dfs<'a>(
            graph: &'a HashMap<String, Vec<String>>,
            node: &'a str,
            state: &mut HashMap<&'a str, u8>,
            order: &mut Vec<String>,
        ) -> bool {
            match state.get(node).copied().unwrap_or(UNVISITED) {
                1 => return false, // back edge: cycle
                2 => return true,  // already processed
                _ => {}
            }

            state.insert(node, VISITING);
            if let Some(neighbors) = graph.get(node) {
                for dep in neighbors {
                    if !dfs(graph, dep.as_str(), state, order) {
                        return false;
                    }
                }
            }
            state.insert(node, VISITED);
            order.push(node.to_string());
            true
        }

        for node in self.0.keys() {
            if state.get(node.as_str()).copied().unwrap_or(0) == 0 {
                if !dfs(&self.0, node.as_str(), &mut state, &mut order) {
                    return None; // cycle detected
                }
            }
        }

        order.reverse();
        Some(order)
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
        // topological_sort_with_counts takes ownership and mutates the map for Kahn's algorithm,
        // so clone here to preserve the original counts we return.
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
