use std::collections::{HashMap, VecDeque, HashSet};
use std::rc::Rc;
use std::fmt;

struct Graph {
    adjacency_list: HashMap<Rc<String>, Vec<Rc<String>>>,
    in_degree: HashMap<Rc<String>, usize>,
}

impl Graph {
    fn new() -> Self {
        Self {
            adjacency_list: HashMap::new(),
            in_degree: HashMap::new(),
        }
    }

    fn add_edge(&mut self, from: Rc<String>, to: Rc<String>) {
        *self.in_degree.entry(to.clone()).or_insert(0) += 1;
        self.in_degree.entry(from.clone()).or_insert(0);
        self.adjacency_list.entry(from).or_insert_with(Vec::new).push(to);
    }
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

    println!("{}", g1);
    g1.visual_display();
    g1.show_diamonds();
}
