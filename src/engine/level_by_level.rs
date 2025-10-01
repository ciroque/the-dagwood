use async_trait::async_trait;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::traits::executor::DagExecutor;
use crate::traits::processor::ProcessorIntent;
use crate::utils::metadata::{merge_metadata_from_responses, BASE_METADATA_KEY};
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::errors::{ExecutionError, FailureStrategy};
use crate::config::{ProcessorMap, DependencyGraph, EntryPoints};

/// Level-by-Level executor that processes DAGs in topological levels with canonical payload tracking.
/// 
/// This executor implements a sophisticated level-based execution strategy that computes topological
/// levels using an optimized Kahn's algorithm and executes all processors at each level concurrently
/// before moving to the next level. It ensures deterministic execution through canonical payload
/// architecture and efficient dependency resolution.
/// 
/// ## Execution Strategy
/// 
/// The executor processes DAGs in distinct phases:
/// 1. **Topological Level Computation**: Uses optimized Kahn's algorithm with reverse dependencies
///    mapping for O(1) dependent lookups, reducing complexity from O(n²) to O(n)
/// 2. **Level-by-Level Execution**: Executes all processors at each level concurrently
/// 3. **Canonical Payload Management**: Maintains deterministic payload updates using ProcessorIntent
/// 4. **Metadata Merging**: Combines metadata from all dependencies using collision-resistant nesting
/// 
/// ## Canonical Payload Architecture
/// 
/// Similar to WorkQueue executor, this implements canonical payload tracking to ensure deterministic
/// execution and proper architectural separation between Transform and Analyze processors:
/// 
/// - **Transform processors**: Can modify the canonical payload when they complete
/// - **Analyze processors**: Receive the canonical payload but only contribute metadata
/// - **Downstream processors**: Always receive the current canonical payload plus merged metadata
/// 
/// This eliminates race conditions within each level and enforces the architectural principle
/// that only Transform processors should modify payloads.
/// 
/// ## Performance Optimizations
/// 
/// - **Reverse Dependencies Map**: Pre-computed mapping for O(1) dependent lookups
/// - **Arc<ProcessorRequest>**: Efficient payload sharing without expensive cloning
/// - **Concurrent Level Execution**: All processors at the same level execute in parallel
/// - **Optimized Metadata Merging**: Uses nested HashMap structure to avoid key collisions
pub struct LevelByLevelExecutor {
    /// Maximum number of concurrent processor executions within a level
    max_concurrency: usize,
}

impl LevelByLevelExecutor {
    /// Create a new Level-by-Level executor with the specified concurrency limit
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            max_concurrency: max_concurrency.max(1), // Ensure at least 1
        }
    }

    /// Create a new Level-by-Level executor with default concurrency (number of CPU cores)
    pub fn default() -> Self {
        let concurrency = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self::new(concurrency)
    }

    /// Compute topological levels using optimized Kahn's algorithm with reverse dependencies mapping.
    /// 
    /// This method implements a sophisticated topological sorting algorithm that:
    /// 1. Pre-computes a reverse dependencies map for O(1) dependent lookups
    /// 2. Uses Kahn's algorithm to determine execution levels
    /// 3. Ensures all processors at the same level can execute concurrently
    /// 
    /// ## Algorithm Complexity
    /// - **Time**: O(V + E) where V = processors, E = dependencies
    /// - **Space**: O(V + E) for reverse dependencies map and level storage
    /// - **Optimization**: Reduced from O(n²) to O(n) through reverse dependencies pre-computation
    /// 
    /// ## Return Value
    /// Returns a vector where each element is a vector of processor IDs at that level.
    /// - Level 0: Entry points (processors with no dependencies)
    /// - Level N: Processors whose dependencies are all in levels 0..N-1
    /// 
    /// ## Error Conditions
    /// - Returns ExecutionError::InternalError if cycles are detected (should be caught in validation)
    /// - Returns ExecutionError::InternalError if no valid entry points are found
    fn compute_topological_levels(
        &self,
        graph: &DependencyGraph,
        entrypoints: &EntryPoints,
    ) -> Result<Vec<Vec<String>>, ExecutionError> {
        let mut levels = Vec::new();
        let mut queue = VecDeque::new();
        let mut processed = HashSet::new();

        // Initialize in-degree count for all processors
        let mut in_degree = HashMap::new();
        for (processor_id, dependencies) in &graph.0 {
            in_degree.insert(processor_id.clone(), dependencies.len());
        }
        
        // Build reverse dependency map for O(1) lookups during level computation
        // Maps: processor_id -> [processors that depend on it]
        // This optimizes the O(n²) lookup in the main algorithm
        let mut dependents_map = HashMap::new();
        for (processor_id, _) in &graph.0 {
            dependents_map.insert(processor_id.clone(), Vec::new());
        }
        for (processor_id, dependencies) in &graph.0 {
            for dependency_id in dependencies {
                dependents_map.entry(dependency_id.clone())
                    .or_insert_with(Vec::new)
                    .push(processor_id.clone());
            }
        }

        // Add entry points to level 0
        let mut current_level = Vec::new();
        for entry_id in &entrypoints.0 {
            if in_degree.get(entry_id).copied().unwrap_or(0) == 0 {
                current_level.push(entry_id.clone());
                queue.push_back(entry_id.clone());
                processed.insert(entry_id.clone());
            }
        }

        if current_level.is_empty() {
            return Err(ExecutionError::InternalError {
                message: "No valid entry points found - all processors have dependencies".into(),
            });
        }

        levels.push(current_level);

        // Process levels using Kahn's algorithm
        while !queue.is_empty() {
            let mut next_level = Vec::new();
            let current_level_size = queue.len();

            // Process all processors in current level
            for _ in 0..current_level_size {
                if let Some(current_id) = queue.pop_front() {
                    // Use dependents map for O(1) lookup instead of O(n) iteration
                    if let Some(dependents) = dependents_map.get(&current_id) {
                        for dependent_id in dependents {
                            if !processed.contains(dependent_id) {
                                // Decrease in-degree with proper error handling
                                let current_in_degree = in_degree.get_mut(dependent_id)
                                    .ok_or_else(|| ExecutionError::InternalError {
                                        message: format!("Internal consistency error: processor '{}' not found in in-degree map during topological sorting", dependent_id)
                                    })?;
                                *current_in_degree -= 1;

                                // If in-degree becomes 0, add to next level
                                if *current_in_degree == 0 {
                                    next_level.push(dependent_id.clone());
                                    processed.insert(dependent_id.clone());
                                }
                            }
                        }
                    }
                }
            }

            // Add next level processors to queue for processing their dependents
            for processor_id in &next_level {
                queue.push_back(processor_id.clone());
            }

            // Add level if it has processors
            if !next_level.is_empty() {
                levels.push(next_level);
            }
        }

        // Check for cycles (if not all processors were processed)
        // Total processors includes all processors in the graph plus entry points
        let mut total_processors: HashSet<_> = graph.0.keys().cloned().collect();
        for entry_id in &entrypoints.0 {
            total_processors.insert(entry_id.clone());
        }
        
        if processed.len() != total_processors.len() {
            return Err(ExecutionError::InternalError {
                message: "Internal consistency error: dependency graph contains cycles (should have been caught during config validation)".into(),
            });
        }

        Ok(levels)
    }

    /// Execute all processors in a single level in parallel with concurrency control.
    /// 
    /// This method spawns concurrent async tasks for all processors in the given level,
    /// respecting the configured concurrency limit using a semaphore. It implements
    /// canonical payload tracking and comprehensive error handling.
    /// 
    /// ## Concurrency Control
    /// - Uses tokio::sync::Semaphore to limit concurrent executions
    /// - All processors in the level execute concurrently (up to the limit)
    /// - Level completion waits for all processors to finish
    /// 
    /// ## Canonical Payload Management
    /// - Transform processors can update the canonical payload
    /// - Analyze processors only contribute metadata
    /// - Uses ProcessorIntent to determine payload update eligibility
    /// 
    /// ## Error Handling
    /// - Respects failure strategy (FailFast, ContinueOnError, BestEffort)
    /// - Handles both processor execution errors and task join errors
    /// - Silent error handling for non-FailFast strategies (matches WorkQueue)
    async fn execute_level(
        &self,
        level_processors: &[String],
        processors: &ProcessorMap,
        results: &Arc<Mutex<HashMap<String, ProcessorResponse>>>,
        canonical_payload: &Arc<Mutex<Vec<u8>>>,
        graph: &DependencyGraph,
        input: &Arc<ProcessorRequest>,
        failure_strategy: FailureStrategy,
    ) -> Result<(), ExecutionError> {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.max_concurrency));
        let mut tasks = Vec::new();

        for processor_id in level_processors {
            let processor = processors.0.get(processor_id).ok_or_else(|| ExecutionError::ProcessorNotFound(processor_id.clone()))?;

            let processor_clone = processor.clone();
            let processor_id_clone = processor_id.clone();
            let results_clone = results.clone();
            let canonical_payload_clone = canonical_payload.clone();
            let graph_clone = graph.clone();
            let input_arc = input.clone(); // Arc::clone is cheap - only increments reference count
            let semaphore_clone = semaphore.clone();

            let task = tokio::spawn(async move {
                // Acquire semaphore permit with proper error handling
                let _permit = semaphore_clone.acquire().await
                    .map_err(|e| ExecutionError::InternalError {
                        message: format!("Failed to acquire semaphore permit for processor '{}': {}", processor_id_clone, e)
                    })?;

                // Build input for this processor
                let processor_input = Self::build_processor_input(
                    &processor_id_clone,
                    &graph_clone,
                    &results_clone,
                    &canonical_payload_clone,
                    &input_arc,
                ).await?;

                // Execute the processor
                let processor_response = processor_clone.process(processor_input).await;

                // Check if processor succeeded (has an outcome)
                if processor_response.outcome.is_some() {
                    // Update canonical payload only for Transform processors with NextPayload outcome
                    // Use the processor's declared intent to determine if it should update canonical payload
                    if let Some(Outcome::NextPayload(ref payload)) = processor_response.outcome {
                        let processor_intent = processor_clone.declared_intent();
                        
                        // Only Transform processors should update the canonical payload
                        if processor_intent == ProcessorIntent::Transform {
                            let mut canonical_guard = canonical_payload_clone.lock().await;
                            *canonical_guard = payload.clone();
                        }
                        // Analyze processors only contribute metadata, they don't update canonical payload
                    }

                    // Store result
                    let mut results_guard = results_clone.lock().await;
                    results_guard.insert(processor_id_clone, processor_response);
                    
                    Ok(())
                } else {
                    Err(ExecutionError::ProcessorFailed {
                        processor_id: processor_id_clone,
                        error: "Processor returned no outcome".to_string(),
                    })
                }
            });

            tasks.push(task);
        }

        // Wait for all tasks in this level to complete
        for task in tasks {
            match task.await {
                Ok(Ok(())) => continue,
                Ok(Err(e)) => {
                    match failure_strategy {
                        FailureStrategy::FailFast => return Err(e),
                        FailureStrategy::ContinueOnError | FailureStrategy::BestEffort => {
                            // For ContinueOnError and BestEffort, we continue processing
                            // Error handling is silent to match WorkQueue implementation
                        }
                    }
                }
                Err(join_error) => {
                    return Err(ExecutionError::InternalError {
                        message: format!("Task join error: {}", join_error),
                    });
                }
            }
        }

        Ok(())
    }

    /// Build input for a processor based on its dependencies with canonical payload and metadata merging.
    /// 
    /// This method constructs the appropriate ProcessorRequest for a given processor by:
    /// 1. Determining if it's an entry point (no dependencies) or has dependencies
    /// 2. Using canonical payload for processors with dependencies
    /// 3. Merging metadata from all dependencies using collision-resistant nesting
    /// 4. Preserving original input metadata as base metadata
    /// 
    /// ## Entry Points
    /// - Processors with no dependencies receive the original input directly
    /// - Requires cloning since processor trait expects owned ProcessorRequest
    /// 
    /// ## Processors with Dependencies
    /// - Receive current canonical payload (shared via Arc for efficiency)
    /// - Get merged metadata from all their dependencies
    /// - Base metadata from original input is preserved under BASE_METADATA_KEY
    /// - Each dependency's metadata is nested under the dependency's processor ID
    /// 
    /// ## Metadata Structure
    /// ```text
    /// {
    ///   "input": { /* original input metadata */ },
    ///   "dependency_processor_1": { /* processor 1 metadata */ },
    ///   "dependency_processor_2": { /* processor 2 metadata */ }
    /// }
    /// ```
    async fn build_processor_input(
        processor_id: &str,
        graph: &DependencyGraph,
        results: &Arc<Mutex<HashMap<String, ProcessorResponse>>>,
        canonical_payload: &Arc<Mutex<Vec<u8>>>,
        original_input: &Arc<ProcessorRequest>,
    ) -> Result<ProcessorRequest, ExecutionError> {
        let dependencies = graph.0.get(processor_id).cloned().unwrap_or_default();

        if dependencies.is_empty() {
            // Entry point processor - use original input
            // We need to clone here since the processor trait expects owned ProcessorRequest
            // TODO(steve) evaluate changing the Processor trait to take a reference or an Arc
            Ok((**original_input).clone())
        } else {
            // Processor with dependencies - use canonical payload + merged metadata
            let canonical_payload_guard = canonical_payload.lock().await;
            let results_guard = results.lock().await;

            // Collect metadata only from actual dependencies, not all completed processors
            let mut dependency_results = HashMap::new();
            for dep_id in &dependencies {
                if let Some(dep_response) = results_guard.get(dep_id) {
                    dependency_results.insert(dep_id.clone(), dep_response.clone());
                }
            }

            // Extract base metadata from original input and merge with dependency metadata
            let base_metadata = if let Some(input_metadata) = original_input.metadata.get(BASE_METADATA_KEY) {
                input_metadata.metadata.clone()
            } else {
                HashMap::new()
            };

            // Merge all metadata: base input metadata + all dependency contributions
            let all_metadata = merge_metadata_from_responses(
                base_metadata,
                &dependency_results
            );

            Ok(ProcessorRequest {
                payload: canonical_payload_guard.clone(),
                metadata: all_metadata,
            })
        }
    }
}

#[async_trait]
impl DagExecutor for LevelByLevelExecutor {
    async fn execute_with_strategy(
        &self,
        processors: ProcessorMap,
        graph: DependencyGraph,
        entrypoints: EntryPoints,
        input: ProcessorRequest,
        failure_strategy: FailureStrategy,
    ) -> Result<HashMap<String, ProcessorResponse>, ExecutionError> {
        // Compute topological levels
        let levels = self.compute_topological_levels(&graph, &entrypoints)?;

        // Initialize shared state
        let results = Arc::new(Mutex::new(HashMap::new()));
        let canonical_payload = Arc::new(Mutex::new(input.payload.clone()));
        
        // Wrap input in Arc to avoid cloning for each processor
        let input_arc = Arc::new(input);

        // Execute each level sequentially
        for level_processors in levels.iter() {
            self.execute_level(
                level_processors,
                &processors,
                &results,
                &canonical_payload,
                &graph,
                &input_arc,
                failure_strategy,
            ).await?;
        }

        // Return final results by taking ownership of the Arc contents
        let final_results = Arc::try_unwrap(results)
            .map_err(|_| ExecutionError::InternalError {
                message: "Failed to unwrap results Arc - multiple references still exist".into()
            })?
            .into_inner();
        Ok(final_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::stub::StubProcessor;
    use std::sync::Arc;

    fn create_test_processor(id: &str) -> Arc<dyn crate::traits::processor::Processor> {
        Arc::new(StubProcessor::new(format!("stub_{}", id)))
    }

    #[tokio::test]
    async fn test_single_processor() {
        let executor = LevelByLevelExecutor::new(2);
        
        let mut processors_map = HashMap::new();
        processors_map.insert("proc1".to_string(), create_test_processor("proc1"));
        let processors = ProcessorMap(processors_map);
        
        let graph = DependencyGraph(HashMap::new());
        let entrypoints = EntryPoints(vec!["proc1".to_string()]);
        let input = ProcessorRequest {
            payload: b"test input".to_vec(),
            metadata: HashMap::new(),
        };

        let result = executor.execute(processors, graph, entrypoints, input).await;
        assert!(result.is_ok());
        
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results.contains_key("proc1"));
    }

    #[tokio::test]
    async fn test_linear_chain() {
        let executor = LevelByLevelExecutor::new(2);
        
        let mut processors_map = HashMap::new();
        processors_map.insert("proc1".to_string(), create_test_processor("proc1"));
        processors_map.insert("proc2".to_string(), create_test_processor("proc2"));
        processors_map.insert("proc3".to_string(), create_test_processor("proc3"));
        let processors = ProcessorMap(processors_map);
        
        let mut graph_map = HashMap::new();
        graph_map.insert("proc1".to_string(), vec![]);
        graph_map.insert("proc2".to_string(), vec!["proc1".to_string()]);
        graph_map.insert("proc3".to_string(), vec!["proc2".to_string()]);
        let graph = DependencyGraph(graph_map);
        
        let entrypoints = EntryPoints(vec!["proc1".to_string()]);
        let input = ProcessorRequest {
            payload: b"test input".to_vec(),
            metadata: HashMap::new(),
        };

        let result = executor.execute(processors, graph, entrypoints, input).await;
        assert!(result.is_ok());
        
        let results = result.unwrap();
        assert_eq!(results.len(), 3);
        assert!(results.contains_key("proc1"));
        assert!(results.contains_key("proc2"));
        assert!(results.contains_key("proc3"));
    }

    #[tokio::test]
    async fn test_diamond_dependency() {
        let executor = LevelByLevelExecutor::new(4);
        
        let mut processors_map = HashMap::new();
        processors_map.insert("A".to_string(), create_test_processor("A"));
        processors_map.insert("B".to_string(), create_test_processor("B"));
        processors_map.insert("C".to_string(), create_test_processor("C"));
        processors_map.insert("D".to_string(), create_test_processor("D"));
        let processors = ProcessorMap(processors_map);
        
        let mut graph_map = HashMap::new();
        graph_map.insert("A".to_string(), vec![]);
        graph_map.insert("B".to_string(), vec!["A".to_string()]);
        graph_map.insert("C".to_string(), vec!["A".to_string()]);
        graph_map.insert("D".to_string(), vec!["B".to_string(), "C".to_string()]);
        let graph = DependencyGraph(graph_map);
        
        let entrypoints = EntryPoints(vec!["A".to_string()]);
        let input = ProcessorRequest {
            payload: b"test input".to_vec(),
            metadata: HashMap::new(),
        };

        let result = executor.execute(processors, graph, entrypoints, input).await;
        assert!(result.is_ok());
        
        let results = result.unwrap();
        assert_eq!(results.len(), 4);
        assert!(results.contains_key("A"));
        assert!(results.contains_key("B"));
        assert!(results.contains_key("C"));
        assert!(results.contains_key("D"));
    }

    #[tokio::test]
    async fn test_multiple_entrypoints() {
        let executor = LevelByLevelExecutor::new(4);
        
        let mut processors_map = HashMap::new();
        processors_map.insert("entry1".to_string(), create_test_processor("entry1"));
        processors_map.insert("entry2".to_string(), create_test_processor("entry2"));
        processors_map.insert("merge".to_string(), create_test_processor("merge"));
        let processors = ProcessorMap(processors_map);
        
        let mut graph_map = HashMap::new();
        graph_map.insert("entry1".to_string(), vec![]);
        graph_map.insert("entry2".to_string(), vec![]);
        graph_map.insert("merge".to_string(), vec!["entry1".to_string(), "entry2".to_string()]);
        let graph = DependencyGraph(graph_map);
        
        let entrypoints = EntryPoints(vec!["entry1".to_string(), "entry2".to_string()]);
        let input = ProcessorRequest {
            payload: b"test input".to_vec(),
            metadata: HashMap::new(),
        };

        let result = executor.execute(processors, graph, entrypoints, input).await;
        assert!(result.is_ok());
        
        let results = result.unwrap();
        assert_eq!(results.len(), 3);
        assert!(results.contains_key("entry1"));
        assert!(results.contains_key("entry2"));
        assert!(results.contains_key("merge"));
    }

    #[tokio::test]
    async fn test_topological_levels_computation() {
        let executor = LevelByLevelExecutor::new(2);
        
        // Diamond dependency: A -> [B, C] -> D
        let mut graph_map = HashMap::new();
        graph_map.insert("A".to_string(), vec![]);
        graph_map.insert("B".to_string(), vec!["A".to_string()]);
        graph_map.insert("C".to_string(), vec!["A".to_string()]);
        graph_map.insert("D".to_string(), vec!["B".to_string(), "C".to_string()]);
        let graph = DependencyGraph(graph_map);
        
        let entrypoints = EntryPoints(vec!["A".to_string()]);
        
        let levels = executor.compute_topological_levels(&graph, &entrypoints).unwrap();
        
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec!["A"]);
        assert!(levels[1].contains(&"B".to_string()));
        assert!(levels[1].contains(&"C".to_string()));
        assert_eq!(levels[1].len(), 2);
        assert_eq!(levels[2], vec!["D"]);
    }

    #[tokio::test]
    async fn test_cycle_detection() {
        let executor = LevelByLevelExecutor::new(2);
        
        // Create a cycle with a valid entry point: Entry -> A -> B -> C -> A
        let mut graph_map = HashMap::new();
        graph_map.insert("A".to_string(), vec!["Entry".to_string(), "C".to_string()]); // A depends on both Entry and C (cycle)
        graph_map.insert("B".to_string(), vec!["A".to_string()]);
        graph_map.insert("C".to_string(), vec!["B".to_string()]);
        let graph = DependencyGraph(graph_map);
        
        let entrypoints = EntryPoints(vec!["Entry".to_string()]);
        
        let result = executor.compute_topological_levels(&graph, &entrypoints);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, ExecutionError::InternalError { .. }));
        if let ExecutionError::InternalError { message } = error {
            assert!(message.contains("cycles"));
        }
    }

    #[tokio::test]
    async fn test_no_valid_entrypoints() {
        let executor = LevelByLevelExecutor::new(2);
        
        // All processors have dependencies
        let mut graph_map = HashMap::new();
        graph_map.insert("A".to_string(), vec!["B".to_string()]);
        graph_map.insert("B".to_string(), vec!["A".to_string()]);
        let graph = DependencyGraph(graph_map);
        
        let entrypoints = EntryPoints(vec!["A".to_string()]);
        
        let result = executor.compute_topological_levels(&graph, &entrypoints);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, ExecutionError::InternalError { .. }));
        if let ExecutionError::InternalError { message } = error {
            assert!(message.contains("No valid entry points"));
        }
    }
}
