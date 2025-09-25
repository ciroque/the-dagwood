use std::collections::{HashMap, HashSet};
use crate::config::Config;
use crate::errors::ValidationError;

/// Validates a configuration's dependency graph for cycles and unresolved references
pub fn validate_dependency_graph(config: &Config) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();
    
    // Check for duplicate processor IDs
    if let Err(duplicate_errors) = validate_unique_processor_ids(config) {
        errors.extend(duplicate_errors);
    }
    
    // Check for unresolved dependencies
    if let Err(unresolved_errors) = validate_dependency_references(config) {
        errors.extend(unresolved_errors);
    }
    
    // Check for cycles (only if no unresolved dependencies, as cycles detection needs valid graph)
    if errors.is_empty() {
        if let Err(cycle_errors) = validate_acyclic_graph(config) {
            errors.extend(cycle_errors);
        }
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validates that all processor IDs are unique
fn validate_unique_processor_ids(config: &Config) -> Result<(), Vec<ValidationError>> {
    let mut seen_ids = HashSet::new();
    let mut errors = Vec::new();
    
    for processor in &config.processors {
        if !seen_ids.insert(&processor.id) {
            errors.push(ValidationError::DuplicateProcessorId {
                processor_id: processor.id.clone(),
            });
        }
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validates that all dependency references point to existing processors
fn validate_dependency_references(config: &Config) -> Result<(), Vec<ValidationError>> {
    let processor_ids: HashSet<&String> = config.processors.iter().map(|p| &p.id).collect();
    let mut errors = Vec::new();
    
    for processor in &config.processors {
        for dependency in &processor.depends_on {
            if !processor_ids.contains(dependency) {
                errors.push(ValidationError::UnresolvedDependency {
                    processor_id: processor.id.clone(),
                    missing_dependency: dependency.clone(),
                });
            }
        }
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validates that the dependency graph is acyclic using DFS-based cycle detection
fn validate_acyclic_graph(config: &Config) -> Result<(), Vec<ValidationError>> {
    // Build adjacency list representation of the dependency graph
    let mut graph: HashMap<&String, Vec<&String>> = HashMap::new();
    
    // Initialize all processors in the graph
    for processor in &config.processors {
        graph.insert(&processor.id, Vec::new());
    }
    
    // Add edges (dependencies)
    for processor in &config.processors {
        for dependency in &processor.depends_on {
            // Add edge from dependency to processor (dependency -> dependent)
            graph.get_mut(dependency).unwrap().push(&processor.id);
        }
    }
    
    // Use DFS to detect cycles
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();
    
    for processor_id in graph.keys() {
        if !visited.contains(*processor_id) {
            if let Some(cycle) = dfs_cycle_detection(
                processor_id,
                &graph,
                &mut visited,
                &mut rec_stack,
                &mut path,
            ) {
                return Err(vec![ValidationError::CyclicDependency { cycle }]);
            }
        }
    }
    
    Ok(())
}

/// DFS-based cycle detection that returns the cycle path if found
fn dfs_cycle_detection(
    node: &str,
    graph: &HashMap<&String, Vec<&String>>,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    visited.insert(node.to_string());
    rec_stack.insert(node.to_string());
    path.push(node.to_string());
    
    if let Some(neighbors) = graph.get(&node.to_string()) {
        for &neighbor in neighbors {
            if !visited.contains(neighbor) {
                if let Some(cycle) = dfs_cycle_detection(neighbor, graph, visited, rec_stack, path) {
                    return Some(cycle);
                }
            } else if rec_stack.contains(neighbor) {
                // Found a cycle - extract the cycle path
                let cycle_start = path.iter().position(|x| x == neighbor).unwrap();
                let mut cycle = path[cycle_start..].to_vec();
                cycle.push(neighbor.to_string()); // Close the cycle
                return Some(cycle);
            }
        }
    }
    
    rec_stack.remove(node);
    path.pop();
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Strategy, BackendType, ProcessorConfig};

    fn create_test_processor(id: &str, depends_on: Vec<&str>) -> ProcessorConfig {
        ProcessorConfig {
            id: id.to_string(),
            backend: BackendType::Local,
            processor: Some("test".to_string()),
            endpoint: None,
            module: None,
            depends_on: depends_on.iter().map(|s| s.to_string()).collect(),
            collection_strategy: None,
            options: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_valid_empty_config() {
        let config = Config {
            strategy: Strategy::WorkQueue,
            failure_strategy: crate::errors::FailureStrategy::FailFast,
            executor_options: crate::config::ExecutorOptions::default(),
            processors: vec![],
        };
        
        assert!(validate_dependency_graph(&config).is_ok());
    }

    #[test]
    fn test_valid_single_processor() {
        let config = Config {
            strategy: Strategy::WorkQueue,
            failure_strategy: crate::errors::FailureStrategy::FailFast,
            executor_options: crate::config::ExecutorOptions::default(),
            processors: vec![create_test_processor("a", vec![])],
        };
        
        assert!(validate_dependency_graph(&config).is_ok());
    }

    #[test]
    fn test_valid_linear_chain() {
        let config = Config {
            strategy: Strategy::WorkQueue,
            failure_strategy: crate::errors::FailureStrategy::FailFast,
            executor_options: crate::config::ExecutorOptions::default(),
            processors: vec![
                create_test_processor("a", vec![]),
                create_test_processor("b", vec!["a"]),
                create_test_processor("c", vec!["b"]),
            ],
        };
        
        assert!(validate_dependency_graph(&config).is_ok());
    }

    #[test]
    fn test_valid_diamond_dependency() {
        let config = Config {
            strategy: Strategy::WorkQueue,
            failure_strategy: crate::errors::FailureStrategy::FailFast,
            executor_options: crate::config::ExecutorOptions::default(),
            processors: vec![
                create_test_processor("a", vec![]),
                create_test_processor("b", vec!["a"]),
                create_test_processor("c", vec!["a"]),
                create_test_processor("d", vec!["b", "c"]),
            ],
        };
        
        assert!(validate_dependency_graph(&config).is_ok());
    }

    #[test]
    fn test_duplicate_processor_ids() {
        let config = Config {
            strategy: Strategy::WorkQueue,
            failure_strategy: crate::errors::FailureStrategy::FailFast,
            executor_options: crate::config::ExecutorOptions::default(),
            processors: vec![
                create_test_processor("a", vec![]),
                create_test_processor("a", vec![]), // Duplicate
            ],
        };
        
        let result = validate_dependency_graph(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::DuplicateProcessorId { .. }
        ));
    }

    #[test]
    fn test_unresolved_dependency() {
        let config = Config {
            strategy: Strategy::WorkQueue,
            failure_strategy: crate::errors::FailureStrategy::FailFast,
            executor_options: crate::config::ExecutorOptions::default(),
            processors: vec![
                create_test_processor("a", vec![]),
                create_test_processor("b", vec!["nonexistent"]),
            ],
        };
        
        let result = validate_dependency_graph(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::UnresolvedDependency { .. }
        ));
    }

    #[test]
    fn test_simple_cycle() {
        let config = Config {
            strategy: Strategy::WorkQueue,
            failure_strategy: crate::errors::FailureStrategy::FailFast,
            executor_options: crate::config::ExecutorOptions::default(),
            processors: vec![
                create_test_processor("a", vec!["b"]),
                create_test_processor("b", vec!["a"]),
            ],
        };
        
        let result = validate_dependency_graph(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::CyclicDependency { .. }
        ));
    }

    #[test]
    fn test_self_dependency_cycle() {
        let config = Config {
            strategy: Strategy::WorkQueue,
            failure_strategy: crate::errors::FailureStrategy::FailFast,
            executor_options: crate::config::ExecutorOptions::default(),
            processors: vec![create_test_processor("a", vec!["a"])],
        };
        
        let result = validate_dependency_graph(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::CyclicDependency { .. }
        ));
    }

    #[test]
    fn test_complex_cycle() {
        let config = Config {
            strategy: Strategy::WorkQueue,
            failure_strategy: crate::errors::FailureStrategy::FailFast,
            executor_options: crate::config::ExecutorOptions::default(),
            processors: vec![
                create_test_processor("a", vec!["b"]),
                create_test_processor("b", vec!["c"]),
                create_test_processor("c", vec!["d"]),
                create_test_processor("d", vec!["b"]), // Creates cycle b -> c -> d -> b
            ],
        };
        
        let result = validate_dependency_graph(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::CyclicDependency { .. }
        ));
    }

    #[test]
    fn test_multiple_errors() {
        let config = Config {
            strategy: Strategy::WorkQueue,
            failure_strategy: crate::errors::FailureStrategy::FailFast,
            executor_options: crate::config::ExecutorOptions::default(),
            processors: vec![
                create_test_processor("a", vec!["nonexistent"]),
                create_test_processor("a", vec![]), // Duplicate ID
                create_test_processor("b", vec!["missing"]),
            ],
        };
        
        let result = validate_dependency_graph(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.len() >= 2); // Should have multiple errors
    }
}
