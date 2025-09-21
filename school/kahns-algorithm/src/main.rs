use std::collections::{HashMap, VecDeque, HashSet};
use std::rc::Rc;
use std::fmt;

struct Graph {
    adjacency_list: HashMap<Rc<String>, Vec<Rc<String>>>,
    in_degree: HashMap<Rc<String>, usize>,
}

impl Graph {
    /// Creates a new, empty graph.
    ///
    /// # Return Value
    ///
    /// A new, empty graph.
    ///
    /// # Examples
    ///
    ///
    fn new() -> Self {
        Self {
            adjacency_list: HashMap::new(),
            in_degree: HashMap::new(),
        }
    }

    /// Adds an edge to the graph.
    ///
    /// The edge is added from the node `from` to the node `to`. The in-degree of
    /// `to` is incremented, and the in-degree of `from` is set to 0 if it
    /// was not already present in the graph.
    ///
    /// # Parameters
    ///
    /// * `from`: The source node of the edge.
    /// * `to`: The target node of the edge.
    fn add_edge(&mut self, from: Rc<String>, to: Rc<String>) {
        *self.in_degree.entry(to.clone()).or_insert(0) += 1;
        self.in_degree.entry(from.clone()).or_insert(0);
        self.adjacency_list.entry(from).or_insert_with(Vec::new).push(to);
    }
    /// Performs a topological sort on the graph.
    ///
    /// A topological sort is a linear ordering of the graph's nodes such that
    /// for every edge (u, v), node u comes before node v in the ordering.
    ///
    /// If the graph contains a cycle, then this function will return an error
    /// string. Otherwise, it will return a vector of nodes in a valid topological
    /// sort order.
    ///
    /// # Return Value
    ///
    /// A Result containing either a vector of nodes in a valid topological sort
    /// order or an error string if the graph contains a cycle.
    fn topological_sort(&self) -> Result<Vec<Rc<String>>, String> {
        let mut queue: VecDeque<Rc<String>> = VecDeque::new();
        let mut result: Vec<Rc<String>> = Vec::new();
        let mut in_degree = self.in_degree.clone();

        for (node, in_degree) in in_degree.iter_mut() {
            if *in_degree == 0 {
                queue.push_back(node.clone());
            }
        }

        while let Some(node) = queue.pop_front() {
            result.push(node.clone());
            for neighbor in self.adjacency_list.get(&node).unwrap_or(&Vec::new()) {
                let neighbor_degree = in_degree.get_mut(neighbor).unwrap();
                *neighbor_degree -= 1;
                if *neighbor_degree == 0 {
                    queue.push_back(neighbor.clone());
                }
            }
        }

        if result.len() != self.in_degree.len() {
            return Err("Cycle detected in graph".to_string());
        }

        Ok(result)
    }

    /// Prints out the structure of the graph in a human-readable format.
    ///
    /// The graph will be printed out as a series of subtrees, with each subtree
    /// rooted at a node with no incoming edges (a "root" node). Each subtree will
    /// be indented to show its hierarchy.
    ///
    /// # Example
    ///
    /// If the graph contains the nodes {a, b, c, d, e} with the edges
    /// {(a, b), (b, c), (c, d), (d, e)}, then the output of this function
    /// will be:
    ///
    /// Graph Structure:
    ///   a
    ///   ├─ b
    ///   │   ├─ c
    ///   │   │   ├─ d
    ///   │   │   │   ├─ e
    fn visual_display(&self) {
        println!("Graph Structure:");

        // Find nodes with no incoming edges (roots)
        let roots: Vec<_> = self.in_degree.iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(node, _)| node)
            .collect();

        for root in roots {
            self.print_subtree(root, 0, &mut HashSet::new());
        }
    }

    /// Recursively prints out the subtree rooted at the given node.
    ///
    /// This function is used by `visual_display` to print out the
    /// structure of the graph.
    ///
    /// # Parameters
    ///
    /// * `node`: The node at which to start printing the subtree.
    /// * `depth`: The current depth of the subtree being printed.
    /// * `visited`: A set of nodes that have already been visited.
    ///
    /// # Return Value
    ///
    /// None.
    ///
    /// # Side Effects
    ///
    /// Prints out the subtree rooted at `node` to the console.
    fn print_subtree(&self, node: &Rc<String>, depth: usize, visited: &mut HashSet<Rc<String>>) {
        let indent = "  ".repeat(depth);

        if visited.contains(node) {
            println!("{}├─ {} (already visited)", indent, node);
            return;
        }

        println!("{}├─ {}", indent, node);
        visited.insert(node.clone());

        if let Some(neighbors) = self.adjacency_list.get(node) {
            for neighbor in neighbors {
                self.print_subtree(neighbor, depth + 1, visited);
            }
        }
    }

    /// Prints out all diamond patterns detected in the graph.
    ///
    /// A diamond pattern is a pattern where a node has two neighbors
    /// that share a common target. For example, if a node A has
    /// neighbors B and C, and both B and C have a neighbor D, then
    /// there is a diamond pattern A → B → D and A → C → D.
    ///
    /// This function will print out all diamond patterns detected in
    /// the graph. Each pattern will be printed out as two lines,
    /// with the format "  {} → [{}] → {}", where the first node is
    /// the source of the diamond, the second node is one of the
    /// neighbors, and the third node is the target of the diamond.
    fn show_diamonds(&self) {
        println!("Diamond patterns detected:");
        for (node, neighbors) in &self.adjacency_list {
            if neighbors.len() >= 2 {
                // Check if any two neighbors share a common target
                for i in 0..neighbors.len() {
                    for j in i+1..neighbors.len() {
                        let n1 = &neighbors[i];
                        let n2 = &neighbors[j];
                        if let (Some(n1_neighbors), Some(n2_neighbors)) =
                            (self.adjacency_list.get(n1), self.adjacency_list.get(n2)) {
                            for target in n1_neighbors {
                                if n2_neighbors.contains(target) {
                                    println!("  {} → [{}] → {}", node, n1, target);
                                    println!("  {} → [{}] → {}", node, n2, target);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl fmt::Display for Graph {
/// Format the graph as a string.
///
/// The string will contain the graph's nodes and edges, as well as the
/// in-degrees of each node.
///
/// # Examples
///
///
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Graph:")?;
        writeln!(f, "  Nodes: {}", self.in_degree.len())?;
        writeln!(f, "  Edges:")?;

        for (from, neighbors) in &self.adjacency_list {
            for to in neighbors {
                writeln!(f, "    {} → {}", from, to)?;
            }
        }

        writeln!(f, "  In-degrees:")?;
        for (node, degree) in &self.in_degree {
            writeln!(f, "    {}: {}", node, degree)?;
        }

        Ok(())
    }
}


/// Example usage of the Graph data structure
///
/// This example creates a graph with nodes {a, b, c, d, e, f}
/// and edges {(a, b), (b, c), (a, c), (b, d), (a, e), (e, f)}
/// and then prints out the graph's structure, visualizes it, and shows
/// any diamond subgraphs it contains.
fn main() {
    let mut g1 = Graph::new();
    g1.add_edge(Rc::new("a".to_string()), Rc::new("b".to_string()));
    g1.add_edge(Rc::new("b".to_string()), Rc::new("c".to_string()));
    g1.add_edge(Rc::new("a".to_string()), Rc::new("c".to_string()));
    g1.add_edge(Rc::new("b".to_string()), Rc::new("d".to_string()));
    g1.add_edge(Rc::new("a".to_string()), Rc::new("e".to_string()));
    g1.add_edge(Rc::new("e".to_string()), Rc::new("f".to_string()));
    // g1.add_edge(Rc::new("f".to_string()), Rc::new("f".to_string()));

    let wtf = g1.topological_sort();

    match wtf {
        Ok(sorted_nodes) => {
            println!("Topological sort result: {:?}", sorted_nodes);
        }
        Err(error) => {
            println!("Error: {}", error);
        }
    }
    println!("{}", g1);
    g1.visual_display();
    g1.show_diamonds();
    
}
