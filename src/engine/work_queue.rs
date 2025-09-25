use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json;

use crate::traits::executor::{DagExecutor, ProcessorMap, DependencyGraph, EntryPoints};
use crate::traits::processor::Processor;
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::config::CollectionStrategy;
use crate::backends::local::processors::{ResultCollectorProcessor, CollectableResult};
use crate::errors::{ExecutionError, FailureStrategy};

/// Work Queue executor that uses dependency counting to manage DAG execution.
/// 
/// This executor maintains a queue of ready-to-execute processors and tracks
/// the number of unresolved dependencies for each processor. When a processor
/// completes, it decrements the dependency count for all its dependents,
/// adding them to the work queue when their count reaches zero.
pub struct WorkQueueExecutor {
    /// Maximum number of concurrent processor executions
    max_concurrency: usize,
}

impl WorkQueueExecutor {
    /// Create a new Work Queue executor with the specified concurrency limit
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            max_concurrency: max_concurrency.max(1), // Ensure at least 1
        }
    }

    /// Create a new Work Queue executor with default concurrency (number of CPU cores)
    pub fn default() -> Self {
        let concurrency = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self::new(concurrency)
    }

    /// Build the dependency count map from the adjacency graph
    fn build_dependency_counts(&self, graph: &HashMap<String, Vec<String>>) -> HashMap<String, usize> {
        let mut dependency_counts = HashMap::new();
        
        // Initialize all processors with 0 dependencies
        for processor_id in graph.keys() {
            dependency_counts.insert(processor_id.clone(), 0);
        }
        
        // Count incoming dependencies for each processor
        for dependents in graph.values() {
            for dependent_id in dependents {
                *dependency_counts.entry(dependent_id.clone()).or_insert(0) += 1;
            }
        }
        
        dependency_counts
    }

    /// Build a reverse dependency map: processor_id -> list of processors it depends on
    /// TODO(steve): Should this be done while the registry is being created and provided then?
    fn build_reverse_dependencies(&self, graph: &HashMap<String, Vec<String>>) -> HashMap<String, Vec<String>> {
        let mut reverse_deps = HashMap::new();
        
        // Initialize all processors with empty dependency lists
        for processor_id in graph.keys() {
            reverse_deps.insert(processor_id.clone(), vec![]);
        }
        
        // Build reverse mapping
        for (processor_id, dependents) in graph {
            for dependent_id in dependents {
                reverse_deps.entry(dependent_id.clone())
                    .or_insert_with(Vec::new)
                    .push(processor_id.clone());
            }
        }
        
        reverse_deps
    }

    /// Find processors that are ready to execute (have no unresolved dependencies)
    #[cfg(test)]
    fn find_ready_processors(&self, dependency_counts: &HashMap<String, usize>) -> Vec<String> {
        dependency_counts
            .iter()
            .filter_map(|(id, &count)| if count == 0 { Some(id.clone()) } else { None })
            .collect()
    }

    /// Combine results from multiple dependencies using collection strategy
    async fn combine_dependency_results(
        dependencies: &[String],
        results_guard: &HashMap<String, ProcessorResponse>,
        fallback_input: ProcessorRequest,
    ) -> ProcessorRequest {
        // Convert ProcessorResponse to CollectableResult for each dependency
        let mut dependency_results = HashMap::new();
        
        for dep_id in dependencies {
            if let Some(dep_response) = results_guard.get(dep_id) {
                let collectable_result = match &dep_response.outcome {
                    Some(Outcome::NextPayload(payload)) => CollectableResult {
                        success: true,
                        payload: Some(payload.clone()),
                        error_code: None,
                        error_message: None,
                    },
                    Some(Outcome::Error(error)) => CollectableResult {
                        success: false,
                        payload: None,
                        error_code: Some(error.code),
                        error_message: Some(error.message.clone()),
                    },
                    None => CollectableResult {
                        success: false,
                        payload: None,
                        error_code: Some(500),
                        error_message: Some("No outcome in processor response".to_string()),
                    },
                };
                dependency_results.insert(dep_id.clone(), collectable_result);
            }
        }

        // If we have no successful dependency results, fallback to original input
        if dependency_results.is_empty() {
            return fallback_input;
        }

        // Use FirstAvailable strategy as default for now
        // TODO: In the future, this should be configurable per processor
        let collector = ResultCollectorProcessor::new(CollectionStrategy::FirstAvailable);
        
        // Serialize dependency results for the collector
        match serde_json::to_vec(&dependency_results) {
            Ok(serialized_deps) => {
                let collector_request = ProcessorRequest {
                    payload: serialized_deps,
                    metadata: fallback_input.metadata.clone(),
                };
                
                // Process the collection
                let collector_response = collector.process(collector_request).await;
                
                // Extract the combined result
                match collector_response.outcome {
                    Some(Outcome::NextPayload(combined_payload)) => ProcessorRequest {
                        payload: combined_payload,
                        metadata: fallback_input.metadata,
                    },
                    _ => fallback_input, // Fallback on collection failure
                }
            },
            Err(_) => fallback_input, // Fallback on serialization failure
        }
    }
}

#[async_trait]
impl DagExecutor for WorkQueueExecutor {
    async fn execute_with_strategy(
        &self,
        processors: ProcessorMap,
        graph: DependencyGraph,
        entrypoints: EntryPoints,
        input: ProcessorRequest,
        failure_strategy: FailureStrategy,
    ) -> Result<HashMap<String, ProcessorResponse>, ExecutionError> {
        // Validate all processors exist in registry
        for processor_id in graph.keys() {
            if !processors.contains_key(processor_id) {
                return Err(ExecutionError::ProcessorNotFound(processor_id.clone()));
            }
        }
        let _results: HashMap<String, ProcessorResponse> = HashMap::new();
        let dependency_counts = self.build_dependency_counts(&graph.0);
        let reverse_dependencies = self.build_reverse_dependencies(&graph.0);
        let mut work_queue = VecDeque::new();
        
        // Start with entrypoints (processors with no dependencies)
        for entrypoint in entrypoints.iter() {
            work_queue.push_back(entrypoint.clone());
        }
        
        // Track active tasks to respect concurrency limits
        let active_tasks = Arc::new(Mutex::new(0));
        let results_mutex = Arc::new(Mutex::new(HashMap::<String, ProcessorResponse>::new()));
        let dependency_counts_mutex = Arc::new(Mutex::new(dependency_counts));
        let work_queue_mutex = Arc::new(Mutex::new(work_queue));
        let failed_processors = Arc::new(Mutex::new(std::collections::HashSet::<String>::new()));
        let blocked_processors = Arc::new(Mutex::new(std::collections::HashSet::<String>::new()));
        
        // Process the work queue until all processors are complete
        loop {
            let next_processor_id = {
                let mut queue = work_queue_mutex.lock().await;
                let active_count = *active_tasks.lock().await;
                let failed = failed_processors.lock().await;
                
                // Check failure strategy
                match failure_strategy {
                    FailureStrategy::FailFast => {
                        if !failed.is_empty() {
                            // Return the first failure immediately
                            let first_failed = failed.iter().next().unwrap().clone();
                            return Err(ExecutionError::ProcessorFailed {
                                processor_id: first_failed,
                                error: "Processor execution failed".to_string(),
                            });
                        }
                    }
                    _ => {
                        // For ContinueOnError and BestEffort, we continue processing
                        // but skip blocked processors
                    }
                }
                
                // Check if we can start more tasks and have work to do
                if active_count < self.max_concurrency && !queue.is_empty() {
                    // Find a processor that isn't blocked
                    let blocked = blocked_processors.lock().await;
                    let mut next_id = None;
                    let mut remaining_queue = VecDeque::new();
                    
                    while let Some(id) = queue.pop_front() {
                        if !blocked.contains(&id) {
                            next_id = Some(id);
                            break;
                        } else {
                            remaining_queue.push_back(id);
                        }
                    }
                    
                    // Put back the blocked processors
                    while let Some(id) = remaining_queue.pop_front() {
                        queue.push_back(id);
                    }
                    
                    next_id
                } else {
                    None
                }
            };
            
            match next_processor_id {
                Some(processor_id) => {
                    // Increment active task count
                    {
                        let mut active = active_tasks.lock().await;
                        *active += 1;
                    }
                    
                    // Clone necessary data for the async task
                    let processor = match processors.get(&processor_id) {
                        Some(p) => p.clone(),
                        None => {
                            return Err(ExecutionError::ProcessorNotFound(processor_id));
                        }
                    };
                    let processor_id_clone = processor_id.clone();
                    let input_clone = input.clone();
                    let graph_clone = graph.0.clone();
                    let reverse_dependencies_clone = reverse_dependencies.clone();
                    let active_tasks_clone = active_tasks.clone();
                    let results_mutex_clone = results_mutex.clone();
                    let dependency_counts_mutex_clone = dependency_counts_mutex.clone();
                    let work_queue_mutex_clone = work_queue_mutex.clone();
                    let failed_processors_clone = failed_processors.clone();
                    let blocked_processors_clone = blocked_processors.clone();
                    
                    // Spawn async task to execute the processor
                    tokio::spawn(async move {
                        // Check if any dependencies failed
                        let should_block = if let Some(dependencies) = reverse_dependencies_clone.get(&processor_id_clone) {
                            let failed = failed_processors_clone.lock().await;
                            dependencies.iter().any(|dep| failed.contains(dep))
                        } else {
                            false
                        };
                        
                        if should_block {
                            // Mark this processor as blocked due to failed dependency
                            let mut blocked = blocked_processors_clone.lock().await;
                            blocked.insert(processor_id_clone.clone());
                            
                            // Also block all dependents of this processor
                            if let Some(dependents) = graph_clone.get(&processor_id_clone) {
                                for dependent in dependents {
                                    blocked.insert(dependent.clone());
                                }
                            }
                        } else {
                        // Determine the input for this processor
                        let processor_input = if let Some(dependencies) = reverse_dependencies_clone.get(&processor_id_clone) {
                            if dependencies.is_empty() {
                                // This is an entry point processor, use original input
                                input_clone
                            } else {
                                // This processor has dependencies, get input from dependency outputs
                                let results_guard = results_mutex_clone.lock().await;
                                
                                if dependencies.len() == 1 {
                                    // Single dependency: use its output directly
                                    let dep_id = &dependencies[0];
                                    if let Some(dep_response) = results_guard.get(dep_id) {
                                        if let Some(Outcome::NextPayload(payload)) = &dep_response.outcome {
                                            ProcessorRequest {
                                                payload: payload.clone(),
                                                ..input_clone
                                            }
                                        } else {
                                            input_clone // Fallback to original input
                                        }
                                    } else {
                                        input_clone // Fallback to original input
                                    }
                                } else {
                                    // Multiple dependencies: use collection strategy to combine results
                                    Self::combine_dependency_results(
                                        &dependencies,
                                        &results_guard,
                                        input_clone
                                    ).await
                                }
                            }
                        } else {
                            // No dependency information, use original input
                            input_clone
                        };
                        
                            // Execute the processor
                            let response = processor.process(processor_input).await;
                            
                            // Check if the processor execution was successful
                            let execution_successful = match &response.outcome {
                                Some(Outcome::NextPayload(_)) => true,
                                Some(_) => false, // Other outcomes might indicate failure
                                None => false, // No outcome indicates failure
                            };
                            
                            if !execution_successful {
                                // Mark processor as failed
                                let mut failed = failed_processors_clone.lock().await;
                                failed.insert(processor_id_clone.clone());
                                
                                // Block all dependents of this processor
                                if let Some(dependents) = graph_clone.get(&processor_id_clone) {
                                    let mut blocked = blocked_processors_clone.lock().await;
                                    for dependent in dependents {
                                        blocked.insert(dependent.clone());
                                    }
                                }
                            } else {
                                // Store the successful result
                                {
                                    let mut results = results_mutex_clone.lock().await;
                                    results.insert(processor_id_clone.clone(), response);
                                }
                                
                                // Update dependency counts for dependents
                                if let Some(dependents) = graph_clone.get(&processor_id_clone) {
                                    let mut dependency_counts = dependency_counts_mutex_clone.lock().await;
                                    let mut work_queue = work_queue_mutex_clone.lock().await;
                                    
                                    for dependent_id in dependents {
                                        if let Some(count) = dependency_counts.get_mut(dependent_id) {
                                            *count -= 1;
                                            
                                            // If dependency count reaches zero, add to work queue
                                            if *count == 0 {
                                                work_queue.push_back(dependent_id.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Decrement active task count
                        {
                            let mut active = active_tasks_clone.lock().await;
                            *active -= 1;
                        }
                    });
                }
                None => {
                    // No work available, check if we're done
                    let active_count = *active_tasks.lock().await;
                    let queue_empty = work_queue_mutex.lock().await.is_empty();
                    let failed = failed_processors.lock().await;
                    
                    if active_count == 0 && queue_empty {
                        // All work is complete, check for failures
                        match failure_strategy {
                            FailureStrategy::FailFast => {
                                // Should have already returned on first failure
                                break;
                            }
                            FailureStrategy::ContinueOnError | FailureStrategy::BestEffort => {
                                if !failed.is_empty() {
                                    // Collect all failures
                                    let failures: Vec<ExecutionError> = failed.iter()
                                        .map(|id| ExecutionError::ProcessorFailed {
                                            processor_id: id.clone(),
                                            error: "Processor execution failed".to_string(),
                                        })
                                        .collect();
                                    
                                    return Err(ExecutionError::MultipleFailed { failures });
                                }
                                break;
                            }
                        }
                    } else if active_count == 0 && !queue_empty {
                        // We have work but can't proceed - likely all remaining processors are blocked
                        let blocked = blocked_processors.lock().await;
                        let queue = work_queue_mutex.lock().await;
                        
                        if queue.iter().all(|id| blocked.contains(id)) {
                            // All remaining processors are blocked due to failed dependencies
                            let failures: Vec<ExecutionError> = failed.iter()
                                .map(|id| ExecutionError::ProcessorFailed {
                                    processor_id: id.clone(),
                                    error: "Processor execution failed".to_string(),
                                })
                                .collect();
                            
                            return Err(ExecutionError::MultipleFailed { failures });
                        }
                    } else {
                        // Wait a bit before checking again
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }
                }
            }
        }
        
        // Extract final results
        let final_results = results_mutex.lock().await;
        Ok(final_results.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse};
    use std::time::Duration;
    use tokio::time::sleep;

    // Mock processor for testing
    struct MockProcessor {
        delay_ms: u64,
        output_suffix: String,
    }

    impl MockProcessor {
        fn new(_name: &str, delay_ms: u64, output_suffix: &str) -> Self {
            Self {
                delay_ms,
                output_suffix: output_suffix.to_string(),
            }
        }
    }

    #[async_trait]
    impl Processor for MockProcessor {
        async fn process(&self, req: ProcessorRequest) -> ProcessorResponse {
            if self.delay_ms > 0 {
                sleep(Duration::from_millis(self.delay_ms)).await;
            }
            
            let input_text = String::from_utf8(req.payload).unwrap_or_default();
            let output_text = format!("{}{}", input_text, self.output_suffix);
            ProcessorResponse {
                outcome: Some(Outcome::NextPayload(output_text.into_bytes())),
            }
        }

        fn name(&self) -> &'static str {
            "MockProcessor"
        }
    }

    #[tokio::test]
    async fn test_single_processor() {
        let executor = WorkQueueExecutor::new(2);
        
        let mut processors = HashMap::new();
        processors.insert("proc1".to_string(), Arc::new(MockProcessor::new("proc1", 0, "-processed")) as Arc<dyn Processor>);
        
        let graph = HashMap::from([
            ("proc1".to_string(), vec![]),
        ]);
        
        let entrypoints = vec!["proc1".to_string()];
        let input = ProcessorRequest {
            payload: "test".to_string().into_bytes(),
            ..Default::default()
        };
        
        let results = executor.execute(ProcessorMap::from(processors), DependencyGraph::from(graph), EntryPoints::from(entrypoints), input).await.expect("DAG execution should succeed");
        
        assert_eq!(results.len(), 1);
        let response = results.get("proc1").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &response.outcome {
            assert_eq!(String::from_utf8(payload.clone()).unwrap(), "test-processed");
        } else {
            panic!("Expected success outcome");
        }
    }

    #[tokio::test]
    async fn test_linear_chain() {
        let executor = WorkQueueExecutor::new(2);
        
        let mut processors = HashMap::new();
        processors.insert("proc1".to_string(), Arc::new(MockProcessor::new("proc1", 0, "-1")) as Arc<dyn Processor>);
        processors.insert("proc2".to_string(), Arc::new(MockProcessor::new("proc2", 0, "-2")) as Arc<dyn Processor>);
        processors.insert("proc3".to_string(), Arc::new(MockProcessor::new("proc3", 0, "-3")) as Arc<dyn Processor>);
        
        let graph = HashMap::from([
            ("proc1".to_string(), vec!["proc2".to_string()]),
            ("proc2".to_string(), vec!["proc3".to_string()]),
            ("proc3".to_string(), vec![]),
        ]);
        
        let entrypoints = vec!["proc1".to_string()];
        let input = ProcessorRequest {
            payload: "test".to_string().into_bytes(),
            ..Default::default()
        };
        
        let results = executor.execute(ProcessorMap::from(processors), DependencyGraph::from(graph), EntryPoints::from(entrypoints), input).await.expect("DAG execution should succeed");
        
        assert_eq!(results.len(), 3);
        let response1 = results.get("proc1").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &response1.outcome {
            assert_eq!(String::from_utf8(payload.clone()).unwrap(), "test-1");
        } else {
            panic!("Expected success outcome for proc1");
        }
        let response2 = results.get("proc2").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &response2.outcome {
            assert_eq!(String::from_utf8(payload.clone()).unwrap(), "test-1-2");
        } else {
            panic!("Expected success outcome for proc2");
        }
        let response3 = results.get("proc3").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &response3.outcome {
            assert_eq!(String::from_utf8(payload.clone()).unwrap(), "test-1-2-3");
        } else {
            panic!("Expected success outcome for proc3");
        }
    }

    #[tokio::test]
    async fn test_diamond_dependency() {
        let executor = WorkQueueExecutor::new(4);
        
        let mut processors = HashMap::new();
        processors.insert("root".to_string(), Arc::new(MockProcessor::new("root", 0, "-root")) as Arc<dyn Processor>);
        processors.insert("left".to_string(), Arc::new(MockProcessor::new("left", 10, "-left")) as Arc<dyn Processor>);
        processors.insert("right".to_string(), Arc::new(MockProcessor::new("right", 5, "-right")) as Arc<dyn Processor>);
        processors.insert("merge".to_string(), Arc::new(MockProcessor::new("merge", 0, "-merge")) as Arc<dyn Processor>);
        
        let graph = HashMap::from([
            ("root".to_string(), vec!["left".to_string(), "right".to_string()]),
            ("left".to_string(), vec!["merge".to_string()]),
            ("right".to_string(), vec!["merge".to_string()]),
            ("merge".to_string(), vec![]),
        ]);
        
        let entrypoints = vec!["root".to_string()];
        let input = ProcessorRequest {
            payload: "test".to_string().into_bytes(),
            ..Default::default()
        };
        
        let results = executor.execute(ProcessorMap::from(processors), DependencyGraph::from(graph), EntryPoints::from(entrypoints), input).await.expect("DAG execution should succeed");
        
        assert_eq!(results.len(), 4);
        let response_root = results.get("root").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &response_root.outcome {
            assert_eq!(String::from_utf8(payload.clone()).unwrap(), "test-root");
        } else {
            panic!("Expected success outcome for root");
        }
        let response_left = results.get("left").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &response_left.outcome {
            assert_eq!(String::from_utf8(payload.clone()).unwrap(), "test-root-left");
        } else {
            panic!("Expected success outcome for left");
        }
        let response_right = results.get("right").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &response_right.outcome {
            assert_eq!(String::from_utf8(payload.clone()).unwrap(), "test-root-right");
        } else {
            panic!("Expected success outcome for right");
        }
        let response_merge = results.get("merge").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &response_merge.outcome {
            // Merge processor gets input from one of its dependencies (order may vary)
            let result = String::from_utf8(payload.clone()).unwrap();
            assert!(result == "test-root-left-merge" || result == "test-root-right-merge", 
                   "Expected merge result to be either 'test-root-left-merge' or 'test-root-right-merge', got: {}", result);
        } else {
            panic!("Expected success outcome for merge");
        }
    }

    #[tokio::test]
    async fn test_multiple_entrypoints() {
        let executor = WorkQueueExecutor::new(4);
        
        let mut processors = HashMap::new();
        processors.insert("entry1".to_string(), Arc::new(MockProcessor::new("entry1", 0, "-e1")) as Arc<dyn Processor>);
        processors.insert("entry2".to_string(), Arc::new(MockProcessor::new("entry2", 0, "-e2")) as Arc<dyn Processor>);
        processors.insert("merge".to_string(), Arc::new(MockProcessor::new("merge", 0, "-merge")) as Arc<dyn Processor>);
        
        let graph = HashMap::from([
            ("entry1".to_string(), vec!["merge".to_string()]),
            ("entry2".to_string(), vec!["merge".to_string()]),
            ("merge".to_string(), vec![]),
        ]);
        
        let entrypoints = vec!["entry1".to_string(), "entry2".to_string()];
        let input = ProcessorRequest {
            payload: "test".to_string().into_bytes(),
            ..Default::default()
        };
        
        let results = executor.execute(ProcessorMap::from(processors), DependencyGraph::from(graph), EntryPoints::from(entrypoints), input).await.expect("DAG execution should succeed");
        
        assert_eq!(results.len(), 3);
        let response_entry1 = results.get("entry1").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &response_entry1.outcome {
            assert_eq!(String::from_utf8(payload.clone()).unwrap(), "test-e1");
        } else {
            panic!("Expected success outcome for entry1");
        }
        let response_entry2 = results.get("entry2").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &response_entry2.outcome {
            assert_eq!(String::from_utf8(payload.clone()).unwrap(), "test-e2");
        } else {
            panic!("Expected success outcome for entry2");
        }
        let response_merge = results.get("merge").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &response_merge.outcome {
            // Merge processor gets input from one of its dependencies (order may vary)
            let result = String::from_utf8(payload.clone()).unwrap();
            assert!(result == "test-e1-merge" || result == "test-e2-merge", 
                   "Expected merge result to be either 'test-e1-merge' or 'test-e2-merge', got: {}", result);
        } else {
            panic!("Expected success outcome for merge");
        }
    }

    #[tokio::test]
    async fn test_build_dependency_counts() {
        let executor = WorkQueueExecutor::new(1);
        
        let graph = HashMap::from([
            ("a".to_string(), vec!["b".to_string(), "c".to_string()]),
            ("b".to_string(), vec!["d".to_string()]),
            ("c".to_string(), vec!["d".to_string()]),
            ("d".to_string(), vec![]),
        ]);
        
        let counts = executor.build_dependency_counts(&graph);
        
        assert_eq!(counts.get("a"), Some(&0)); // No dependencies
        assert_eq!(counts.get("b"), Some(&1)); // Depends on a
        assert_eq!(counts.get("c"), Some(&1)); // Depends on a
        assert_eq!(counts.get("d"), Some(&2)); // Depends on b and c
    }

    #[tokio::test]
    async fn test_find_ready_processors() {
        let executor = WorkQueueExecutor::new(1);
        
        let counts = HashMap::from([
            ("a".to_string(), 0),
            ("b".to_string(), 1),
            ("c".to_string(), 0),
            ("d".to_string(), 2),
        ]);
        
        let mut ready = executor.find_ready_processors(&counts);
        ready.sort(); // For deterministic testing
        
        assert_eq!(ready, vec!["a", "c"]);
    }

    #[tokio::test]
    async fn test_processor_not_found_error() {
        let executor = WorkQueueExecutor::new(2);
        
        // Create processors but don't include one referenced in the graph
        let processors = HashMap::from([
            ("proc1".to_string(), Arc::new(MockProcessor::new("proc1", 0, "-1")) as Arc<dyn Processor>),
        ]);
        
        // Graph references a processor that doesn't exist
        let graph = HashMap::from([
            ("proc1".to_string(), vec!["proc2".to_string()]),
            ("proc2".to_string(), vec![]), // proc2 not in processors map
        ]);
        
        let entrypoints = vec!["proc1".to_string()];
        let input = ProcessorRequest {
            payload: "test".to_string().into_bytes(),
            ..Default::default()
        };
        
        let result = executor.execute(ProcessorMap::from(processors), DependencyGraph::from(graph), EntryPoints::from(entrypoints), input).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ExecutionError::ProcessorNotFound(id) => {
                assert_eq!(id, "proc2");
            }
            _ => panic!("Expected ProcessorNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_processor_failure_with_fail_fast_strategy() {
        let executor = WorkQueueExecutor::new(2);
        
        // Create a failing processor (returns None outcome)
        struct FailingProcessor;
        
        #[async_trait]
        impl Processor for FailingProcessor {
            async fn process(&self, _req: ProcessorRequest) -> ProcessorResponse {
                ProcessorResponse {
                    outcome: None, // This indicates failure
                }
            }
            
            fn name(&self) -> &'static str {
                "failing_processor"
            }
        }
        
        let processors = HashMap::from([
            ("failing".to_string(), Arc::new(FailingProcessor) as Arc<dyn Processor>),
            ("dependent".to_string(), Arc::new(MockProcessor::new("dependent", 0, "-dep")) as Arc<dyn Processor>),
        ]);
        
        let graph = HashMap::from([
            ("failing".to_string(), vec!["dependent".to_string()]),
            ("dependent".to_string(), vec![]),
        ]);
        
        let entrypoints = vec!["failing".to_string()];
        let input = ProcessorRequest {
            payload: "test".to_string().into_bytes(),
            ..Default::default()
        };
        
        let result = executor.execute_with_strategy(
            ProcessorMap::from(processors), 
            DependencyGraph::from(graph), 
            EntryPoints::from(entrypoints), 
            input, 
            FailureStrategy::FailFast
        ).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ExecutionError::ProcessorFailed { processor_id, .. } => {
                assert_eq!(processor_id, "failing");
            }
            _ => panic!("Expected ProcessorFailed error"),
        }
    }

    #[tokio::test]
    async fn test_processor_failure_with_continue_on_error_strategy() {
        let executor = WorkQueueExecutor::new(2);
        
        // Create a failing processor and an independent successful processor
        struct FailingProcessor;
        
        #[async_trait]
        impl Processor for FailingProcessor {
            async fn process(&self, _req: ProcessorRequest) -> ProcessorResponse {
                ProcessorResponse {
                    outcome: None, // This indicates failure
                }
            }
            
            fn name(&self) -> &'static str {
                "failing_processor"
            }
        }
        
        let processors = HashMap::from([
            ("failing".to_string(), Arc::new(FailingProcessor) as Arc<dyn Processor>),
            ("dependent".to_string(), Arc::new(MockProcessor::new("dependent", 0, "-dep")) as Arc<dyn Processor>),
            ("independent".to_string(), Arc::new(MockProcessor::new("independent", 0, "-ind")) as Arc<dyn Processor>),
        ]);
        
        let graph = HashMap::from([
            ("failing".to_string(), vec!["dependent".to_string()]),
            ("dependent".to_string(), vec![]),
            ("independent".to_string(), vec![]),
        ]);
        
        let entrypoints = vec!["failing".to_string(), "independent".to_string()];
        let input = ProcessorRequest {
            payload: "test".to_string().into_bytes(),
            ..Default::default()
        };
        
        let result = executor.execute_with_strategy(
            ProcessorMap::from(processors), 
            DependencyGraph::from(graph), 
            EntryPoints::from(entrypoints), 
            input, 
            FailureStrategy::ContinueOnError
        ).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ExecutionError::MultipleFailed { failures } => {
                assert_eq!(failures.len(), 1);
                match &failures[0] {
                    ExecutionError::ProcessorFailed { processor_id, .. } => {
                        assert_eq!(processor_id, "failing");
                    }
                    _ => panic!("Expected ProcessorFailed error in failures list"),
                }
            }
            _ => panic!("Expected MultipleFailed error"),
        }
    }

    #[tokio::test]
    async fn test_dependency_blocking_on_failure() {
        let executor = WorkQueueExecutor::new(2);
        
        // Create a chain where the first processor fails
        struct FailingProcessor;
        
        #[async_trait]
        impl Processor for FailingProcessor {
            async fn process(&self, _req: ProcessorRequest) -> ProcessorResponse {
                ProcessorResponse {
                    outcome: None, // This indicates failure
                }
            }
            
            fn name(&self) -> &'static str {
                "failing_processor"
            }
        }
        
        let processors = HashMap::from([
            ("first".to_string(), Arc::new(FailingProcessor) as Arc<dyn Processor>),
            ("second".to_string(), Arc::new(MockProcessor::new("second", 0, "-2nd")) as Arc<dyn Processor>),
            ("third".to_string(), Arc::new(MockProcessor::new("third", 0, "-3rd")) as Arc<dyn Processor>),
        ]);
        
        let graph = HashMap::from([
            ("first".to_string(), vec!["second".to_string()]),
            ("second".to_string(), vec!["third".to_string()]),
            ("third".to_string(), vec![]),
        ]);
        
        let entrypoints = vec!["first".to_string()];
        let input = ProcessorRequest {
            payload: "test".to_string().into_bytes(),
            ..Default::default()
        };
        
        let result = executor.execute_with_strategy(
            ProcessorMap::from(processors), 
            DependencyGraph::from(graph), 
            EntryPoints::from(entrypoints), 
            input, 
            FailureStrategy::BestEffort
        ).await;
        
        // Should fail because the entire chain is blocked by the first processor failure
        assert!(result.is_err());
        match result.unwrap_err() {
            ExecutionError::MultipleFailed { failures } => {
                assert_eq!(failures.len(), 1);
                match &failures[0] {
                    ExecutionError::ProcessorFailed { processor_id, .. } => {
                        assert_eq!(processor_id, "first");
                    }
                    _ => panic!("Expected ProcessorFailed error in failures list"),
                }
            }
            _ => panic!("Expected MultipleFailed error"),
        }
    }
}
