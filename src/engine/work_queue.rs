use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::traits::executor::DagExecutor;
use crate::traits::processor::ProcessorIntent;
use crate::config::{ProcessorMap, DependencyGraph, EntryPoints};
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::errors::{ExecutionError, FailureStrategy};
use crate::utils::metadata::merge_metadata_from_responses;

use super::priority_work_queue::{PriorityWorkQueue, PrioritizedTask};

/// Work Queue executor that uses dependency counting and canonical payload tracking.
/// 
/// This executor maintains a queue of ready-to-execute processors and tracks
/// the number of unresolved dependencies for each processor. When a processor
/// completes, it decrements the dependency count for all its dependents,
/// adding them to the work queue when their count reaches zero.
/// 
/// ## Canonical Payload Architecture
/// 
/// The executor implements a canonical payload approach to ensure deterministic
/// execution and proper architectural separation between Transform and Analyze processors:
/// 
/// - **Transform processors**: Modify the payload and update the canonical payload
/// - **Analyze processors**: Receive the canonical payload but only contribute metadata
/// - **Downstream processors**: Always receive the canonical payload from the last Transform
///   processor, plus merged metadata from all dependencies
/// 
/// This eliminates race conditions in diamond dependency patterns and enforces
/// the architectural principle that only Transform processors should modify payloads.
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


    /// Find processors that are ready to execute (have no unresolved dependencies)
    #[cfg(test)]
    fn find_ready_processors(&self, dependency_counts: &HashMap<String, usize>) -> Vec<String> {
        dependency_counts
            .iter()
            .filter_map(|(id, &count)| if count == 0 { Some(id.clone()) } else { None })
            .collect()
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
        let reverse_dependencies = graph.build_reverse_dependencies();
        
        // Efficiently compute both dependency counts and topological ranks together
        let (dependency_counts, topological_ranks) = graph.dependency_counts_and_ranks()
            .ok_or_else(|| ExecutionError::InternalError { 
                message: "Internal consistency error: dependency graph contains cycles (should have been caught during config validation)".into() 
            })?;
        
        let mut work_queue = PriorityWorkQueue::new();
        
        // Start with entrypoints (processors with no dependencies), prioritized by topological rank
        for entrypoint in entrypoints.iter() {
            let rank = topological_ranks.get(entrypoint).copied().unwrap_or(0);
            let is_transform = processors.get(entrypoint)
                .map(|p| p.declared_intent() == ProcessorIntent::Transform)
                .unwrap_or(false);
            work_queue.push(PrioritizedTask::new(entrypoint.clone(), rank, is_transform));
        }
        
        // Track active tasks to respect concurrency limits
        let active_tasks = Arc::new(Mutex::new(0));
        let results_mutex = Arc::new(Mutex::new(HashMap::<String, ProcessorResponse>::new()));
        let dependency_counts_mutex = Arc::new(Mutex::new(dependency_counts));
        let work_queue_mutex = Arc::new(Mutex::new(work_queue));
        let failed_processors = Arc::new(Mutex::new(std::collections::HashSet::<String>::new()));
        let blocked_processors = Arc::new(Mutex::new(std::collections::HashSet::<String>::new()));
        
        // Track canonical payload from Transform processors with topological ranking
        let canonical_payload_mutex = Arc::new(Mutex::new(input.payload.clone()));
        let highest_transform_rank_mutex = Arc::new(Mutex::new(None::<usize>));
        
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
                    // Use the newtype's efficient blocked processor handling
                    let blocked = blocked_processors.lock().await;
                    queue.pop_next_available(&blocked)
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
                    let processors_clone = processors.clone(); // Clone processors for combine_dependency_results
                    let canonical_payload_mutex_clone = canonical_payload_mutex.clone();
                    let highest_transform_rank_mutex_clone = highest_transform_rank_mutex.clone();
                    let topological_ranks_clone = topological_ranks.clone();
                    
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
                        // Determine the input for this processor using canonical payload approach
                        let processor_input = if let Some(dependencies) = reverse_dependencies_clone.get(&processor_id_clone) {
                            if dependencies.is_empty() {
                                // This is an entry point processor, use original input
                                input_clone
                            } else {
                                // This processor has dependencies, use canonical payload + collected metadata
                                let canonical_payload = canonical_payload_mutex_clone.lock().await.clone();
                                let results_guard = results_mutex_clone.lock().await;
                                
                                // Collect metadata only from actual dependencies, not all completed processors
                                let mut dependency_results = HashMap::new();
                                for dep_id in dependencies {
                                    if let Some(dep_response) = results_guard.get(dep_id) {
                                        dependency_results.insert(dep_id.clone(), dep_response.clone());
                                    }
                                }
                                
                                let all_metadata = merge_metadata_from_responses(
                                    input_clone.metadata.clone(),
                                    &dependency_results
                                );
                                
                                ProcessorRequest {
                                    payload: canonical_payload,
                                    metadata: all_metadata,
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
                                let response_clone = response.clone();
                                {
                                    let mut results = results_mutex_clone.lock().await;
                                    results.insert(processor_id_clone.clone(), response);
                                }
                                
                                // Update canonical payload if this is a Transform processor with higher topological rank
                                if let Some(processor) = processors_clone.get(&processor_id_clone) {
                                    if processor.declared_intent() == ProcessorIntent::Transform {
                                        if let Some(Outcome::NextPayload(new_payload)) = &response_clone.outcome {
                                            if let Some(&processor_rank) = topological_ranks_clone.get(&processor_id_clone) {
                                                let mut highest_rank = highest_transform_rank_mutex_clone.lock().await;
                                                
                                                // Update canonical payload if this processor has strictly higher rank
                                                // or if no Transform processor has completed yet (prevents race conditions)
                                                let should_update = match *highest_rank {
                                                    None => true, // First Transform processor
                                                    Some(current_highest) => processor_rank > current_highest,
                                                };
                                                
                                                if should_update {
                                                    let mut canonical_payload = canonical_payload_mutex_clone.lock().await;
                                                    *canonical_payload = new_payload.clone();
                                                    *highest_rank = Some(processor_rank);
                                                }
                                            }
                                        }
                                    }
                                }
                                
                                // Update dependency counts for dependents
                                if let Some(dependents) = graph_clone.get(&processor_id_clone) {
                                    let mut dependency_counts = dependency_counts_mutex_clone.lock().await;
                                    let mut work_queue = work_queue_mutex_clone.lock().await;
                                    
                                    for dependent_id in dependents {
                                        if let Some(count) = dependency_counts.get_mut(dependent_id) {
                                            *count -= 1;
                                            
                                            // If dependency count reaches zero, add to work queue with priority
                                            if *count == 0 {
                                                let rank = topological_ranks_clone.get(dependent_id).copied().unwrap_or(0);
                                                let is_transform = processors_clone.get(dependent_id)
                                                    .map(|p| p.declared_intent() == ProcessorIntent::Transform)
                                                    .unwrap_or(false);
                                                work_queue.push(PrioritizedTask::new(dependent_id.clone(), rank, is_transform));
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
                        
                        if queue.iter().all(|task| blocked.contains(&task.processor_id)) {
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
    use crate::traits::processor::Processor;
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
                metadata: HashMap::new(),
            }
        }

        fn name(&self) -> &'static str {
            "MockProcessor"
        }

        fn declared_intent(&self) -> ProcessorIntent {
            ProcessorIntent::Transform
        }
    }

    // Mock Analyze processor for testing canonical payload architecture
    struct MockAnalyzeProcessor {
        delay_ms: u64,
        metadata_suffix: String,
    }

    impl MockAnalyzeProcessor {
        fn new(_name: &str, delay_ms: u64, metadata_suffix: &str) -> Self {
            Self {
                delay_ms,
                metadata_suffix: metadata_suffix.to_string(),
            }
        }
    }

    #[async_trait]
    impl Processor for MockAnalyzeProcessor {
        async fn process(&self, req: ProcessorRequest) -> ProcessorResponse {
            if self.delay_ms > 0 {
                sleep(Duration::from_millis(self.delay_ms)).await;
            }
            
            // Analyze processors should NOT modify the payload, only add metadata
            let mut metadata = req.metadata.clone();
            metadata.insert("analysis".to_string(), self.metadata_suffix.clone());
            
            ProcessorResponse {
                outcome: Some(Outcome::NextPayload(req.payload)), // Pass through unchanged
                metadata,
            }
        }

        fn name(&self) -> &'static str {
            "MockAnalyzeProcessor"
        }

        fn declared_intent(&self) -> ProcessorIntent {
            ProcessorIntent::Analyze
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
        let graph = HashMap::from([
            ("a".to_string(), vec!["b".to_string(), "c".to_string()]),
            ("b".to_string(), vec!["d".to_string()]),
            ("c".to_string(), vec!["d".to_string()]),
            ("d".to_string(), vec![]),
        ]);
        
        let dependency_graph = DependencyGraph::from(graph);
        let counts = dependency_graph.build_dependency_counts();
        
        assert_eq!(counts.get("a"), Some(&0)); // No dependencies
        assert_eq!(counts.get("b"), Some(&1)); // Depends on a
        assert_eq!(counts.get("c"), Some(&1)); // Depends on a
        assert_eq!(counts.get("d"), Some(&2)); // Depends on b and c
    }

    #[tokio::test]
    async fn test_task_prioritization_based_on_topological_ranks() {
        let executor = WorkQueueExecutor::new(2); // Limited concurrency to test prioritization
        
        // Create a complex DAG to test prioritization:
        // entry1 -> [transform1, analyze1] -> transform2 -> final
        // entry2 -> transform3 -> final
        // This tests that higher-ranked Transform processors (transform2, transform3) get priority
        let processors = HashMap::from([
            ("entry1".to_string(), Arc::new(MockProcessor::new("entry1", 50, "-E1")) as Arc<dyn Processor>),
            ("entry2".to_string(), Arc::new(MockProcessor::new("entry2", 50, "-E2")) as Arc<dyn Processor>),
            ("transform1".to_string(), Arc::new(MockProcessor::new("transform1", 50, "-T1")) as Arc<dyn Processor>),
            ("analyze1".to_string(), Arc::new(MockAnalyzeProcessor::new("analyze1", 50, "-A1")) as Arc<dyn Processor>),
            ("transform2".to_string(), Arc::new(MockProcessor::new("transform2", 10, "-T2")) as Arc<dyn Processor>),
            ("transform3".to_string(), Arc::new(MockProcessor::new("transform3", 10, "-T3")) as Arc<dyn Processor>),
            ("final".to_string(), Arc::new(MockProcessor::new("final", 10, "-FINAL")) as Arc<dyn Processor>),
        ]);
        
        let graph = HashMap::from([
            ("entry1".to_string(), vec!["transform1".to_string(), "analyze1".to_string()]),
            ("entry2".to_string(), vec!["transform3".to_string()]),
            ("transform1".to_string(), vec!["transform2".to_string()]),
            ("analyze1".to_string(), vec!["transform2".to_string()]),
            ("transform2".to_string(), vec!["final".to_string()]),
            ("transform3".to_string(), vec!["final".to_string()]),
            ("final".to_string(), vec![]),
        ]);
        
        let entrypoints = vec!["entry1".to_string(), "entry2".to_string()];
        let input = ProcessorRequest {
            payload: "start".to_string().into_bytes(),
            ..Default::default()
        };
        
        let result = executor.execute_with_strategy(
            ProcessorMap::from(processors), 
            DependencyGraph::from(graph), 
            EntryPoints::from(entrypoints), 
            input,
            FailureStrategy::FailFast
        ).await.unwrap();
        
        // Verify all processors completed successfully
        assert_eq!(result.len(), 7);
        
        // Verify the final processor received the canonical payload from the highest-ranked Transform
        let final_result = result.get("final").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &final_result.outcome {
            let result_str = String::from_utf8(payload.clone()).unwrap();
            // Due to prioritization, either transform2 or transform3 should have set the canonical payload
            // The final result should show the canonical payload path
            assert!(result_str.contains("start") && (result_str.contains("T2") || result_str.contains("T3")));
        } else {
            panic!("Expected NextPayload outcome for final processor");
        }
        
        // Verify Transform processors completed (they should be prioritized)
        assert!(result.contains_key("transform1"));
        assert!(result.contains_key("transform2"));
        assert!(result.contains_key("transform3"));
        
        // Verify Analyze processor completed but didn't affect canonical payload
        assert!(result.contains_key("analyze1"));
    }

    #[tokio::test]
    async fn test_topological_rank_based_canonical_payload_updates() {
        let executor = WorkQueueExecutor::new(4);
        
        // Create Transform processors with different topological ranks
        // Graph: transform1 -> transform2 -> analyze1
        // transform1 (rank 0) -> transform2 (rank 1) -> analyze1 (rank 2)
        let processors = HashMap::from([
            ("transform1".to_string(), Arc::new(MockProcessor::new("transform1", 10, "-T1")) as Arc<dyn Processor>),
            ("transform2".to_string(), Arc::new(MockProcessor::new("transform2", 10, "-T2")) as Arc<dyn Processor>),
            ("analyze1".to_string(), Arc::new(MockAnalyzeProcessor::new("analyze1", 10, "-A1")) as Arc<dyn Processor>),
        ]);
        
        let graph = HashMap::from([
            ("transform1".to_string(), vec!["transform2".to_string()]),
            ("transform2".to_string(), vec!["analyze1".to_string()]),
            ("analyze1".to_string(), vec![]),
        ]);
        
        let entrypoints = vec!["transform1".to_string()];
        let input = ProcessorRequest {
            payload: "initial".to_string().into_bytes(),
            ..Default::default()
        };
        
        let result = executor.execute_with_strategy(
            ProcessorMap::from(processors), 
            DependencyGraph::from(graph), 
            EntryPoints::from(entrypoints), 
            input,
            FailureStrategy::FailFast
        ).await.unwrap();
        
        // Verify that the final analyze processor received the canonical payload
        // from the highest-ranked Transform processor (transform2)
        let analyze_result = result.get("analyze1").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &analyze_result.outcome {
            let result_str = String::from_utf8(payload.clone()).unwrap();
            // Should be: "initial" -> transform1 -> "initial-T1" -> transform2 -> "initial-T1-T2"
            // analyze1 should receive "initial-T1-T2" and add "-A1" metadata only
            assert_eq!(result_str, "initial-T1-T2");
        } else {
            panic!("Expected NextPayload outcome for analyze1");
        }
        
        // Verify that transform2 has the expected output (canonical payload source)
        let transform2_result = result.get("transform2").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &transform2_result.outcome {
            let result_str = String::from_utf8(payload.clone()).unwrap();
            assert_eq!(result_str, "initial-T1-T2");
        } else {
            panic!("Expected NextPayload outcome for transform2");
        }
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
                    metadata: HashMap::new(),
                }
            }
            
            fn name(&self) -> &'static str {
                "failing_processor"
            }

            fn declared_intent(&self) -> ProcessorIntent {
                ProcessorIntent::Transform
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
                    metadata: HashMap::new(),
                }
            }
            
            fn name(&self) -> &'static str {
                "failing_processor"
            }

            fn declared_intent(&self) -> ProcessorIntent {
                ProcessorIntent::Transform
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
    async fn test_metadata_isolation_between_unrelated_processors() {
        let executor = WorkQueueExecutor::new(4);
        
        // Create a DAG where processors should only receive metadata from their dependencies
        // Graph: entry1 -> proc1 -> final
        //        entry2 -> proc2 (unrelated to final)
        // final should only get metadata from proc1, not from proc2
        let processors = HashMap::from([
            ("entry1".to_string(), Arc::new(MockAnalyzeProcessor::new("entry1", 10, "E1_META")) as Arc<dyn Processor>),
            ("entry2".to_string(), Arc::new(MockAnalyzeProcessor::new("entry2", 10, "E2_META")) as Arc<dyn Processor>),
            ("proc1".to_string(), Arc::new(MockAnalyzeProcessor::new("proc1", 10, "P1_META")) as Arc<dyn Processor>),
            ("proc2".to_string(), Arc::new(MockAnalyzeProcessor::new("proc2", 10, "P2_META")) as Arc<dyn Processor>),
            ("final".to_string(), Arc::new(MockAnalyzeProcessor::new("final", 10, "FINAL_META")) as Arc<dyn Processor>),
        ]);
        
        let graph = HashMap::from([
            ("entry1".to_string(), vec!["proc1".to_string()]),
            ("entry2".to_string(), vec!["proc2".to_string()]),
            ("proc1".to_string(), vec!["final".to_string()]),
            ("proc2".to_string(), vec![]), // proc2 is unrelated to final
            ("final".to_string(), vec![]),
        ]);
        
        let entrypoints = vec!["entry1".to_string(), "entry2".to_string()];
        let input = ProcessorRequest {
            payload: "test".to_string().into_bytes(),
            metadata: {
                let mut m = HashMap::new();
                m.insert("original".to_string(), "INPUT_META".to_string());
                m
            },
        };
        
        let result = executor.execute_with_strategy(
            ProcessorMap::from(processors), 
            DependencyGraph::from(graph), 
            EntryPoints::from(entrypoints), 
            input,
            FailureStrategy::FailFast
        ).await.unwrap();
        
        // Verify all processors completed
        assert_eq!(result.len(), 5);
        
        // Check that final processor only has metadata from its dependencies (proc1)
        // and NOT from unrelated processors (proc2, entry2)
        let final_result = result.get("final").unwrap();
        
        // Verify metadata isolation: final should only have metadata from its direct dependency (proc1)
        
        // Should have original metadata
        assert!(final_result.metadata.contains_key("original"));
        assert_eq!(final_result.metadata.get("original"), Some(&"INPUT_META".to_string()));
        
        // Should have metadata from proc1 (direct dependency)
        assert!(final_result.metadata.contains_key("dep:5:proc1:analysis"));
        assert_eq!(final_result.metadata.get("dep:5:proc1:analysis"), Some(&"P1_META".to_string()));
        
        // Should NOT have metadata from proc2 (unrelated processor)
        assert!(!final_result.metadata.contains_key("dep:5:proc2:analysis"));
        
        // Should NOT have metadata from entry2 (unrelated processor)
        assert!(!final_result.metadata.contains_key("dep:6:entry2:analysis"));
        
        // Note: entry1 metadata should NOT be directly present in final because
        // final only depends on proc1, not entry1. The metadata chain is:
        // entry1 -> proc1 (proc1 gets entry1's metadata)
        // proc1 -> final (final gets proc1's metadata, but not entry1's directly)
        
        // Verify proc2 completed successfully but is isolated
        let proc2_result = result.get("proc2").unwrap();
        assert!(proc2_result.metadata.contains_key("analysis"));
        assert_eq!(proc2_result.metadata.get("analysis"), Some(&"P2_META".to_string()));
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
                    metadata: HashMap::new(),
                }
            }
            
            fn name(&self) -> &'static str {
                "failing_processor"
            }

            fn declared_intent(&self) -> ProcessorIntent {
                ProcessorIntent::Transform
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
