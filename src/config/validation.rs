// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Configuration validation for DAG integrity and correctness.
//!
//! This module provides comprehensive validation for DAG configurations, ensuring that
//! processor dependency graphs are valid, acyclic, and executable. The validation system
//! performs multiple checks in a specific order to provide meaningful error messages and
//! prevent invalid DAG execution attempts.
//!
//! # Validation Pipeline
//!
//! The validation process follows a three-stage pipeline:
//!
//! 1. **Uniqueness Validation**: Ensures all processor IDs are unique
//! 2. **Reference Validation**: Verifies all dependencies point to existing processors  
//! 3. **Cycle Detection**: Uses DFS to detect circular dependencies
//!
//! This ordering is important because cycle detection requires a valid graph structure,
//! so reference validation must pass first.
//!
//! # Algorithms
//!
//! ## Cycle Detection Algorithm
//! Uses **Depth-First Search (DFS) with recursion stack** to detect cycles:
//! - **Time Complexity**: O(V + E) where V = processors, E = dependencies
//! - **Space Complexity**: O(V) for visited set and recursion stack
//! - **Advantage**: Provides the actual cycle path for debugging
//! - **Detection Method**: Tracks nodes in current recursion path (gray nodes)
//!
//! ## Reference Validation
//! Uses **HashSet lookup** for efficient dependency resolution:
//! - **Time Complexity**: O(V + E) where V = processors, E = dependencies
//! - **Space Complexity**: O(V) for processor ID set
//! - **Method**: Build processor ID set, then validate all dependency references
//!
//! # Examples
//!
//! ## Basic validation usage
//! ```rust
//! use the_dagwood::config::{validate_dependency_graph, Config, Strategy, ProcessorConfig, BackendType, ExecutorOptions};
//! use the_dagwood::errors::FailureStrategy;
//! use std::collections::HashMap;
//!
//! // Create a sample configuration
//! let config = Config {
//!     strategy: Strategy::WorkQueue,
//!     failure_strategy: FailureStrategy::FailFast,
//!     executor_options: ExecutorOptions::default(),
//!     processors: vec![
//!         ProcessorConfig {
//!             id: "processor1".to_string(),
//!             backend: BackendType::Local,
//!             processor: Some("test".to_string()),
//!             endpoint: None,
//!             module: None,
//!             depends_on: vec![],
//!             options: HashMap::new(),
//!         }
//!     ],
//! };
//!
//! // Validate the configuration
//! match validate_dependency_graph(&config) {
//!     Ok(()) => println!("Configuration is valid"),
//!     Err(errors) => {
//!         for error in errors {
//!             eprintln!("Validation error: {}", error);
//!         }
//!     }
//! }
//! ```
//!
//! ## Handling specific validation errors
//! ```rust
//! use the_dagwood::config::{validate_dependency_graph, Config, Strategy, ProcessorConfig, BackendType, ExecutorOptions};
//! use the_dagwood::errors::{ValidationError, FailureStrategy};
//! use std::collections::HashMap;
//!
//! // Create a configuration with validation errors
//! let config = Config {
//!     strategy: Strategy::WorkQueue,
//!     failure_strategy: FailureStrategy::FailFast,
//!     executor_options: ExecutorOptions::default(),
//!     processors: vec![
//!         ProcessorConfig {
//!             id: "processor1".to_string(),
//!             backend: BackendType::Local,
//!             processor: Some("test".to_string()),
//!             endpoint: None,
//!             module: None,
//!             depends_on: vec!["nonexistent".to_string()], // This will cause an error
//!             options: HashMap::new(),
//!         }
//!     ],
//! };
//!
//! if let Err(errors) = validate_dependency_graph(&config) {
//!     for error in errors {
//!         match error {
//!             ValidationError::CyclicDependency { cycle } => {
//!                 eprintln!("Cycle detected: {}", cycle.join(" -> "));
//!             }
//!             ValidationError::UnresolvedDependency { processor_id, missing_dependency } => {
//!                 eprintln!("Processor '{}' depends on missing processor '{}'",
//!                          processor_id, missing_dependency);
//!             }
//!             ValidationError::DuplicateProcessorId { processor_id } => {
//!                 eprintln!("Duplicate processor ID: '{}'", processor_id);
//!             }
//!             ValidationError::DiamondPatternWarning { convergence_processor, parallel_paths } => {
//!                 eprintln!("Warning: Diamond pattern at '{}' may cause non-deterministic behavior",
//!                          convergence_processor);
//!             }
//!         }
//!     }
//! }
//! ```

use crate::config::{Config, WasmConfig};
use crate::errors::ValidationError;
use std::collections::{HashMap, HashSet};

/// Validates a configuration's dependency graph for structural integrity and executability.
///
/// This is the main validation entry point that orchestrates all validation checks in the
/// correct order. The validation pipeline ensures that:
///
/// 1. **Processor IDs are unique** - No duplicate processor identifiers
/// 2. **Dependencies are resolvable** - All `depends_on` references point to existing processors
/// 3. **Graph is acyclic** - No circular dependencies that would prevent execution
///
/// The validation order is important: cycle detection requires a structurally valid graph,
/// so uniqueness and reference validation must pass first.
///
/// # Arguments
///
/// * `config` - The configuration to validate
///
/// # Returns
///
/// * `Ok(())` - Configuration is valid and ready for execution
/// * `Err(Vec<ValidationError>)` - List of all validation errors found
///
/// # Examples
///
/// ```rust
/// use the_dagwood::config::{validate_dependency_graph, Config, Strategy, ProcessorConfig, BackendType, ExecutorOptions};
/// use the_dagwood::errors::FailureStrategy;
/// use std::collections::HashMap;
///
/// // Create a valid configuration
/// let config = Config {
///     strategy: Strategy::WorkQueue,
///     failure_strategy: FailureStrategy::FailFast,
///     executor_options: ExecutorOptions::default(),
///     processors: vec![
///         ProcessorConfig {
///             id: "input".to_string(),
///             backend: BackendType::Local,
///             processor: Some("test".to_string()),
///             endpoint: None,
///             module: None,
///             depends_on: vec![],
///             options: HashMap::new(),
///         },
///         ProcessorConfig {
///             id: "output".to_string(),
///             backend: BackendType::Local,
///             processor: Some("test".to_string()),
///             endpoint: None,
///             module: None,
///             depends_on: vec!["input".to_string()],
///             options: HashMap::new(),
///         }
///     ],
/// };
///
/// // Validate before execution
/// match validate_dependency_graph(&config) {
///     Ok(()) => {
///         // Safe to proceed with DAG execution
///         println!("Configuration validated successfully");
///     }
///     Err(errors) => {
///         // Handle validation failures
///         for error in errors {
///             eprintln!("Validation failed: {}", error);
///         }
///         return;
///     }
/// }
/// ```
///
/// # Error Accumulation
///
/// This function accumulates multiple errors when possible, allowing users to see all
/// validation issues at once rather than fixing them one by one. However, cycle detection
/// is skipped if there are reference errors, since cycle detection requires a valid graph.
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

    // Check for diamond patterns (warnings only, don't prevent execution)
    // Note: Diamond patterns are structural warnings, not execution-blocking errors
    if errors.is_empty() {
        if let Err(diamond_warnings) = validate_diamond_patterns(config) {
            // For now, we'll log warnings but not fail validation
            // In the future, this could be configurable (strict vs permissive mode)
            for warning in diamond_warnings {
                eprintln!("Warning: {}", warning);
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validates that all processor IDs are unique within the configuration.
///
/// Processor IDs must be unique because they serve as the primary key for:
/// - Dependency resolution (`depends_on` references)
/// - Execution tracking and result storage
/// - Error reporting and debugging
///
/// This validation uses a `HashSet` to efficiently detect duplicates in O(n) time.
///
/// # Arguments
///
/// * `config` - The configuration to validate
///
/// # Returns
///
/// * `Ok(())` - All processor IDs are unique
/// * `Err(Vec<ValidationError>)` - List of duplicate processor IDs found
///
/// # Algorithm
///
/// 1. Create empty `HashSet` to track seen IDs
/// 2. Iterate through all processors
/// 3. For each processor, attempt to insert ID into set
/// 4. If insertion fails (ID already exists), record as duplicate
/// 5. Return all duplicates found
///
/// **Time Complexity**: O(n) where n = number of processors
/// **Space Complexity**: O(n) for the HashSet
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

/// Validates that all dependency references point to existing processors.
///
/// This ensures that every processor ID listed in `depends_on` fields corresponds to
/// an actual processor in the configuration. Unresolved dependencies would cause
/// runtime failures during DAG execution when the executor tries to wait for
/// non-existent processors to complete.
///
/// # Arguments
///
/// * `config` - The configuration to validate
///
/// # Returns
///
/// * `Ok(())` - All dependency references are valid
/// * `Err(Vec<ValidationError>)` - List of unresolved dependency references
///
/// # Algorithm
///
/// 1. Build `HashSet` of all processor IDs for O(1) lookup
/// 2. Iterate through all processors and their dependencies
/// 3. For each dependency, check if it exists in the processor ID set
/// 4. Record any dependencies that don't have corresponding processors
/// 5. Return all unresolved dependencies found
///
/// **Time Complexity**: O(n + d) where n = processors, d = total dependencies
/// **Space Complexity**: O(n) for the processor ID set
///
/// # Example Error Scenarios
///
/// - Processor "transform" depends on "input", but "input" processor doesn't exist
/// - Typo in dependency name: "proces1" instead of "process1"
/// - Processor removed from config but dependencies not updated
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

/// Validates that the dependency graph is acyclic using DFS-based cycle detection.
///
/// Cyclic dependencies make DAG execution impossible because processors would wait
/// indefinitely for each other to complete. This function uses a sophisticated
/// DFS algorithm with recursion stack tracking to detect cycles and provide the
/// exact cycle path for debugging.
///
/// # Arguments
///
/// * `config` - The configuration to validate (must have valid references)
///
/// # Returns
///
/// * `Ok(())` - Graph is acyclic and executable
/// * `Err(Vec<ValidationError>)` - Contains cycle information if found
///
/// # Algorithm: DFS with Recursion Stack
///
/// Uses the **"Three Colors" DFS approach**:
/// - **White (unvisited)**: Node not yet explored
/// - **Gray (in recursion stack)**: Node currently being explored
/// - **Black (visited)**: Node fully explored
///
/// **Cycle Detection**: If we encounter a gray node during DFS, we've found a cycle.
///
/// ## Steps:
/// 1. Build forward adjacency list (dependency → [dependents])
/// 2. Initialize all nodes as white (unvisited)
/// 3. For each unvisited node, start DFS:
///    - Mark node as gray (add to recursion stack)
///    - Recursively visit all neighbors
///    - If neighbor is gray → cycle detected, extract cycle path
///    - If neighbor is white → continue DFS
///    - Mark node as black (remove from recursion stack)
///
/// **Time Complexity**: O(V + E) where V = processors, E = dependencies
/// **Space Complexity**: O(V) for visited set, recursion stack, and call stack
///
/// # Cycle Path Extraction
///
/// When a cycle is detected, the algorithm extracts the exact cycle path by:
/// 1. Finding where the cycle starts in the current DFS path
/// 2. Taking the path segment from cycle start to current node
/// 3. Adding the back edge to close the cycle
///
/// This provides developers with precise information about which processors
/// form the circular dependency.
///
/// # Example Cycle Scenarios
///
/// - **Self-dependency**: Processor A depends on itself
/// - **Simple cycle**: A → B → A
/// - **Complex cycle**: A → B → C → D → B (cycle is B → C → D → B)
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

/// Performs depth-first search with cycle detection and path tracking.
///
/// This is the core cycle detection algorithm that implements the "three colors" DFS
/// approach with explicit recursion stack tracking. It not only detects cycles but
/// also extracts the exact cycle path for detailed error reporting.
///
/// # Arguments
///
/// * `node` - Current node being explored
/// * `graph` - Forward adjacency list representation of dependencies
/// * `visited` - Set of fully explored nodes (black nodes)
/// * `rec_stack` - Set of nodes in current recursion path (gray nodes)
/// * `path` - Current DFS path for cycle extraction
///
/// # Returns
///
/// * `Some(Vec<String>)` - Cycle path if cycle detected
/// * `None` - No cycle found in this DFS branch
///
/// # Algorithm Details
///
/// ## State Transitions
/// 1. **White → Gray**: Mark node as visited and add to recursion stack
/// 2. **Gray → Black**: Remove from recursion stack when DFS completes
/// 3. **Cycle Detection**: If we visit a gray node, cycle found
///
/// ## Cycle Path Construction
/// When a back edge is found (current node → gray node):
/// 1. Find position of gray node in current path
/// 2. Extract path segment from gray node to current position
/// 3. Add back edge to close the cycle
/// 4. Return complete cycle path
///
/// ## Example Execution
/// For graph A → B → C → A:
/// 1. Start DFS at A: path = [A], rec_stack = {A}
/// 2. Visit B: path = [A, B], rec_stack = {A, B}
/// 3. Visit C: path = [A, B, C], rec_stack = {A, B, C}
/// 4. Try to visit A: A is in rec_stack → cycle detected!
/// 5. Extract cycle: A is at position 0, current path = [A, B, C]
/// 6. Cycle = [A, B, C, A] (path[0..] + back edge)
///
/// **Time Complexity**: O(V + E) amortized across all calls
/// **Space Complexity**: O(V) for recursion stack and path tracking
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
                if let Some(cycle) = dfs_cycle_detection(neighbor, graph, visited, rec_stack, path)
                {
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

/// Validates for diamond dependency patterns that may cause non-deterministic behavior.
///
/// Diamond patterns occur when multiple processors depend on the same upstream processor
/// and then converge at a downstream processor. In the reactive executor, if any of the
/// parallel processors are Transform type, they may race to update the canonical payload,
/// leading to non-deterministic behavior.
///
/// This validation detects such patterns and issues warnings to help users understand
/// potential non-determinism in their DAG configurations.
///
/// # Arguments
///
/// * `config` - The configuration to validate (must have valid references and be acyclic)
///
/// # Returns
///
/// * `Ok(())` - No diamond patterns detected
/// * `Err(Vec<ValidationError>)` - Contains diamond pattern warnings
///
/// # Algorithm
///
/// 1. Build forward adjacency list (dependency → [dependents])
/// 2. For each processor, check if it has multiple dependencies
/// 3. For processors with multiple dependencies, trace back to find common ancestors
/// 4. If common ancestors exist with multiple paths to the current processor, it's a diamond
/// 5. Extract the parallel paths for detailed warning information
///
/// # Example Diamond Patterns
///
/// - **Simple Diamond**: `A → [B, C] → D` (A is common ancestor, B and C are parallel, D converges)
/// - **Complex Diamond**: `A → B → [C, D] → E` (B is common ancestor, C and D are parallel, E converges)
/// - **Nested Diamonds**: Multiple diamond patterns within the same DAG
fn validate_diamond_patterns(config: &Config) -> Result<(), Vec<ValidationError>> {
    // Build forward adjacency list (dependency -> [dependents])
    let mut graph: HashMap<&String, Vec<&String>> = HashMap::new();

    // Initialize all processors in the graph
    for processor in &config.processors {
        graph.insert(&processor.id, Vec::new());
    }

    // Add edges (dependencies -> dependents)
    for processor in &config.processors {
        for dependency in &processor.depends_on {
            graph.get_mut(dependency).unwrap().push(&processor.id);
        }
    }

    let mut warnings = Vec::new();

    // Check each processor for diamond patterns
    for processor in &config.processors {
        if processor.depends_on.len() >= 2 {
            // This processor has multiple dependencies - potential diamond convergence point
            let diamond_paths = find_diamond_paths(&processor.id, &processor.depends_on, &graph);
            if !diamond_paths.is_empty() {
                warnings.push(ValidationError::DiamondPatternWarning {
                    convergence_processor: processor.id.clone(),
                    parallel_paths: diamond_paths,
                });
            }
        }
    }

    if warnings.is_empty() {
        Ok(())
    } else {
        Err(warnings)
    }
}

/// Finds diamond paths for a processor with multiple dependencies.
///
/// This function traces back from the convergence processor through its dependencies
/// to identify parallel execution paths that may cause race conditions.
///
/// # Arguments
///
/// * `convergence_processor` - The processor where paths converge
/// * `dependencies` - The direct dependencies of the convergence processor
/// * `graph` - Forward adjacency list representation
///
/// # Returns
///
/// * `Vec<Vec<String>>` - List of parallel paths, empty if no diamond detected
fn find_diamond_paths(
    convergence_processor: &str,
    dependencies: &[String],
    graph: &HashMap<&String, Vec<&String>>,
) -> Vec<Vec<String>> {
    let mut parallel_paths = Vec::new();

    // For simplicity, we'll detect the most common diamond pattern:
    // Multiple direct dependencies that don't depend on each other
    // More complex diamond detection would require full path analysis

    for dep in dependencies {
        // Check if this dependency has any path to other dependencies
        let mut has_path_to_other_deps = false;
        for other_dep in dependencies {
            if dep != other_dep && has_path_between(dep, other_dep, graph) {
                has_path_to_other_deps = true;
                break;
            }
        }

        // If this dependency doesn't connect to other dependencies,
        // it forms a parallel path in a potential diamond
        if !has_path_to_other_deps {
            parallel_paths.push(vec![dep.clone(), convergence_processor.to_string()]);
        }
    }

    // Only return paths if we have multiple parallel paths (diamond pattern)
    if parallel_paths.len() >= 2 {
        parallel_paths
    } else {
        Vec::new()
    }
}

/// Checks if there's a path between two processors in the dependency graph.
///
/// Uses BFS to determine if `from` processor can reach `to` processor
/// through the dependency graph.
fn has_path_between(from: &str, to: &str, graph: &HashMap<&String, Vec<&String>>) -> bool {
    if from == to {
        return true;
    }

    let mut visited = HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(from.to_string());
    visited.insert(from.to_string());

    while let Some(current) = queue.pop_front() {
        if let Some(neighbors) = graph.get(&current) {
            for &neighbor in neighbors {
                if neighbor == to {
                    return true;
                }
                if !visited.contains(neighbor) {
                    visited.insert(neighbor.to_string());
                    queue.push_back(neighbor.to_string());
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BackendType, ProcessorConfig, Strategy};

    fn create_test_processor(id: &str, depends_on: Vec<&str>) -> ProcessorConfig {
        ProcessorConfig {
            id: id.to_string(),
            backend: BackendType::Local,
            processor: Some("test".to_string()),
            endpoint: None,
            module: None,
            depends_on: depends_on.iter().map(|s| s.to_string()).collect(),
            options: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_valid_empty_config() {
        let config = Config {
            strategy: Strategy::WorkQueue,
            failure_strategy: crate::errors::FailureStrategy::FailFast,
            executor_options: crate::config::ExecutorOptions::default(),
            wasm: WasmConfig::default(),
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
            wasm: WasmConfig::default(),
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
            wasm: WasmConfig::default(),
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
            wasm: WasmConfig::default(),
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
            wasm: WasmConfig::default(),
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
            wasm: WasmConfig::default(),
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
            wasm: WasmConfig::default(),
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
            wasm: WasmConfig::default(),
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
            wasm: WasmConfig::default(),
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
            wasm: WasmConfig::default(),
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

    #[test]
    fn test_diamond_pattern_detection() {
        let config = Config {
            strategy: Strategy::WorkQueue,
            failure_strategy: crate::errors::FailureStrategy::FailFast,
            executor_options: crate::config::ExecutorOptions::default(),
            wasm: WasmConfig::default(),
            processors: vec![
                create_test_processor("entry", vec![]),
                create_test_processor("left", vec!["entry"]),
                create_test_processor("right", vec!["entry"]),
                create_test_processor("merge", vec!["left", "right"]), // Diamond convergence
            ],
        };

        // Diamond patterns are now warnings, not errors - validation should succeed
        let result = validate_dependency_graph(&config);
        assert!(
            result.is_ok(),
            "Diamond patterns should be warnings, not blocking errors"
        );

        // Test the diamond detection function directly to verify it detects the pattern
        let diamond_result = validate_diamond_patterns(&config);
        assert!(diamond_result.is_err());
        let warnings = diamond_result.unwrap_err();
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            warnings[0],
            ValidationError::DiamondPatternWarning { .. }
        ));

        if let ValidationError::DiamondPatternWarning {
            convergence_processor,
            parallel_paths,
        } = &warnings[0]
        {
            assert_eq!(convergence_processor, "merge");
            assert_eq!(parallel_paths.len(), 2);
        }
    }

    #[test]
    fn test_no_diamond_pattern_linear_chain() {
        let config = Config {
            strategy: Strategy::WorkQueue,
            failure_strategy: crate::errors::FailureStrategy::FailFast,
            executor_options: crate::config::ExecutorOptions::default(),
            wasm: WasmConfig::default(),
            processors: vec![
                create_test_processor("a", vec![]),
                create_test_processor("b", vec!["a"]),
                create_test_processor("c", vec!["b"]),
            ],
        };

        let result = validate_dependency_graph(&config);
        assert!(result.is_ok()); // Linear chain should not trigger diamond warning
    }

    #[test]
    fn test_no_diamond_pattern_single_dependency() {
        let config = Config {
            strategy: Strategy::WorkQueue,
            failure_strategy: crate::errors::FailureStrategy::FailFast,
            executor_options: crate::config::ExecutorOptions::default(),
            wasm: WasmConfig::default(),
            processors: vec![
                create_test_processor("a", vec![]),
                create_test_processor("b", vec!["a"]),
                create_test_processor("c", vec!["a"]),
                create_test_processor("d", vec!["b"]), // Only depends on b, not both b and c
            ],
        };

        let result = validate_dependency_graph(&config);
        assert!(result.is_ok()); // No convergence point, so no diamond
    }
}
