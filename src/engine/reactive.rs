use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;

use crate::traits::executor::DagExecutor;
use crate::traits::processor::ProcessorIntent;
use crate::config::{ProcessorMap, DependencyGraph, EntryPoints};
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::errors::{ExecutionError, FailureStrategy};
use crate::engine::metadata::{merge_dependency_metadata_for_execution, BASE_METADATA_KEY};

/// Reactive/Event-Driven executor that uses async channels for processor communication.
///
/// This executor implements an event-driven approach where processors are notified
/// immediately when their dependencies complete, enabling natural parallelism and
/// low-latency execution without artificial batching or level computation.
///
/// ## Event-Driven Architecture
///
/// The executor builds a notification network using async channels where:
/// - Each processor has a receiver channel for dependency completion events
/// - When a processor completes, it sends events to all its dependents
/// - Processors execute immediately when all dependencies are satisfied
///
/// ## Canonical Payload Architecture
///
/// Like WorkQueue and LevelByLevel executors, this implements canonical payload tracking:
/// - **Transform processors**: Update the canonical payload when they complete
/// - **Analyze processors**: Receive canonical payload but only contribute metadata
/// - **Downstream processors**: Always receive canonical payload + merged metadata
///
/// This ensures deterministic execution and proper architectural separation.
pub struct ReactiveExecutor {
    /// Maximum number of concurrent processor executions
    max_concurrency: usize,
}

/// Event sent between processors in the reactive execution network
#[derive(Debug, Clone)]
enum ProcessorEvent {
    /// Notification that a dependency has completed
    DependencyCompleted {
        dependency_id: String,
        metadata: HashMap<String, crate::proto::processor_v1::Metadata>,
    },
    /// Initial trigger for entry point processors
    Execute {
        metadata: HashMap<String, crate::proto::processor_v1::Metadata>,
    },
}

/// Internal state for tracking processor execution in the reactive network
struct ProcessorNode {
    /// Channel receiver for incoming dependency completion events
    receiver: mpsc::UnboundedReceiver<ProcessorEvent>,
    /// List of processor IDs that depend on this processor (for notifications)
    dependents: Vec<String>,
    /// Number of dependencies this processor is waiting for
    pending_dependencies: usize,
    /// Collected dependency results for metadata merging
    dependency_results: HashMap<String, ProcessorResponse>,
}

impl ReactiveExecutor {
    /// Create a new Reactive executor with the specified concurrency limit
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            max_concurrency: max_concurrency.max(1), // Ensure at least 1
        }
    }

    /// Create a new Reactive executor with default concurrency (number of CPU cores)
    pub fn default() -> Self {
        let concurrency = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self::new(concurrency)
    }

    /// Build the event-driven notification network using DependencyGraph methods
    ///
    /// This uses the forward graph (graph.0) for dependents and build_dependency_counts()
    /// for initial pending dependency counts.
    fn build_notification_network(
        &self,
        graph: &DependencyGraph,
    ) -> Result<(HashMap<String, mpsc::UnboundedSender<ProcessorEvent>>, HashMap<String, ProcessorNode>), ExecutionError> {
        eprintln!("[REACTIVE_DEBUG] Building notification network for {} processors", graph.keys().count());
        // Get dependency counts for initial pending dependencies
        let dependency_counts = graph.build_dependency_counts();

        let mut senders = HashMap::new();
        let mut nodes = HashMap::new();

        // Create channels for each processor
        for processor_id in graph.keys() {
            let (sender, receiver) = mpsc::unbounded_channel();

            // Use forward graph (graph.0) to get dependents for notification network
            let dependents = graph.0.get(processor_id)
                .cloned()
                .unwrap_or_default();

            let pending_dependencies = dependency_counts.get(processor_id)
                .copied()
                .unwrap_or(0);

            eprintln!("[REACTIVE_DEBUG] Processor '{}': {} pending deps, {} dependents: {:?}", 
                processor_id, pending_dependencies, dependents.len(), dependents);

            senders.insert(processor_id.clone(), sender);
            nodes.insert(processor_id.clone(), ProcessorNode {
                receiver,
                dependents,
                pending_dependencies,
                dependency_results: HashMap::new(),
            });
        }

        Ok((senders, nodes))
    }

    /// Spawn an async task for a processor in the reactive network
    ///
    /// This reuses the canonical payload architecture and declared_intent() pattern
    /// from the existing executors to maintain consistency.
    async fn spawn_processor_task(
        processor_id: String,
        mut node: ProcessorNode,
        processors: Arc<ProcessorMap>,
        canonical_payload_mutex: Arc<Mutex<Vec<u8>>>,
        results_mutex: Arc<Mutex<HashMap<String, ProcessorResponse>>>,
        senders: Arc<HashMap<String, mpsc::UnboundedSender<ProcessorEvent>>>,
        failure_strategy: FailureStrategy,
        semaphore: Arc<tokio::sync::Semaphore>,
        cancellation_token: CancellationToken,
    ) -> Result<(), ExecutionError> {
        eprintln!("[REACTIVE_DEBUG] Task started for processor '{}' with {} pending deps", 
            processor_id, node.pending_dependencies);
        
        // Wait for all dependencies to complete
        while node.pending_dependencies > 0 {
            eprintln!("[REACTIVE_DEBUG] Processor '{}' waiting for {} dependencies", 
                processor_id, node.pending_dependencies);
            
            tokio::select! {
                // Check for cancellation first
                _ = cancellation_token.cancelled() => {
                    eprintln!("[REACTIVE_DEBUG] Processor '{}' cancelled while waiting for dependencies", processor_id);
                    return Err(ExecutionError::InternalError {
                        message: format!("Processor '{}' cancelled due to failure in another processor", processor_id),
                    });
                }
                // Wait for dependency events
                event_result = node.receiver.recv() => {
                    if let Some(event) = event_result {
                        match event {
                            ProcessorEvent::DependencyCompleted { dependency_id, metadata } => {
                                eprintln!("[REACTIVE_DEBUG] Processor '{}' received completion from '{}', {} deps remaining", 
                                    processor_id, dependency_id, node.pending_dependencies - 1);
                                
                                // Store dependency result for metadata merging
                                let dependency_response = ProcessorResponse {
                                    outcome: Some(Outcome::NextPayload(vec![])), // Payload not used in metadata merging
                                    metadata,
                                };
                                node.dependency_results.insert(dependency_id, dependency_response);
                                node.pending_dependencies -= 1;
                            }
                            ProcessorEvent::Execute { metadata } => {
                                eprintln!("[REACTIVE_DEBUG] Processor '{}' received Execute event with {} pending deps", 
                                    processor_id, node.pending_dependencies);
                                
                                // Entry point execution - store as base metadata
                                let base_response = ProcessorResponse {
                                    outcome: Some(Outcome::NextPayload(vec![])),
                                    metadata,
                                };
                                node.dependency_results.insert(BASE_METADATA_KEY.to_string(), base_response);
                                
                                // Validate that entry points have no pending dependencies
                                if node.pending_dependencies == 0 {
                                    eprintln!("[REACTIVE_DEBUG] Entry point processor '{}' ready to execute", processor_id);
                                    break; // Exit the loop for entry points
                                } else {
                                    return Err(ExecutionError::InternalError {
                                        message: format!(
                                            "Received Execute event for processor '{}' with pending_dependencies = {} (expected 0)",
                                            processor_id, node.pending_dependencies
                                        ),
                                    });
                                }
                            }
                        }
                    } else {
                        eprintln!("[REACTIVE_DEBUG] Channel closed for processor '{}'", processor_id);
                        return Err(ExecutionError::InternalError {
                            message: format!("Channel closed for processor '{}'", processor_id),
                        });
                    }
                }
            }
        }

        eprintln!("[REACTIVE_DEBUG] Processor '{}' all dependencies satisfied, acquiring semaphore", processor_id);
        
        // Acquire semaphore permit for concurrency control
        let _permit = semaphore.acquire().await
            .map_err(|e| ExecutionError::InternalError {
                message: format!("Failed to acquire semaphore permit for processor '{}': {}", processor_id, e),
            })?;

        eprintln!("[REACTIVE_DEBUG] Processor '{}' acquired semaphore, getting processor instance", processor_id);
        
        // Get processor instance
        let processor = processors.get(&processor_id)
            .ok_or_else(|| ExecutionError::ProcessorNotFound(processor_id.clone()))?;

        // Build processor input using canonical payload and merged metadata
        eprintln!("[REACTIVE_DEBUG] Processor '{}' acquiring canonical payload lock", processor_id);
        let canonical_payload = {
            let guard = canonical_payload_mutex.lock().await;
            eprintln!("[REACTIVE_DEBUG] Processor '{}' acquired canonical payload lock, payload size: {}", 
                processor_id, guard.len());
            guard.clone()
        };
        eprintln!("[REACTIVE_DEBUG] Processor '{}' released canonical payload lock", processor_id);

        // Use existing metadata merging utility (same as other executors)
        // Extract base metadata from original input and merge with dependency metadata
        let base_metadata = node
            .dependency_results
            .get(BASE_METADATA_KEY)
            .and_then(|input_metadata| input_metadata.metadata.get(BASE_METADATA_KEY))
            .map(|base_meta| base_meta.metadata.clone())
            .unwrap_or_default();

        let all_metadata = merge_dependency_metadata_for_execution(
            base_metadata,
            &node.dependency_results,
        );

        let processor_input = ProcessorRequest {
            payload: canonical_payload, // All processors get canonical payload (correct!)
            metadata: all_metadata,
        };

        // Execute processor
        eprintln!("[REACTIVE_DEBUG] Processor '{}' starting execution", processor_id);
        let processor_response = processor.process(processor_input).await;
        eprintln!("[REACTIVE_DEBUG] Processor '{}' finished execution", processor_id);

        // Handle processor execution result based on failure strategy
        match &processor_response.outcome {
            Some(Outcome::NextPayload(_)) => {
                eprintln!("[REACTIVE_DEBUG] Processor '{}' succeeded", processor_id);
                
                // Success case - update canonical payload if this is a Transform processor
                if processor.declared_intent() == ProcessorIntent::Transform {
                    if let Some(Outcome::NextPayload(new_payload)) = &processor_response.outcome {
                        eprintln!("[REACTIVE_DEBUG] Transform processor '{}' updating canonical payload", processor_id);
                        let mut canonical_guard = canonical_payload_mutex.lock().await;
                        *canonical_guard = new_payload.clone();
                        eprintln!("[REACTIVE_DEBUG] Transform processor '{}' updated canonical payload", processor_id);
                    }
                }

                // Store successful result
                eprintln!("[REACTIVE_DEBUG] Processor '{}' storing result", processor_id);
                {
                    let mut results_guard = results_mutex.lock().await;
                    results_guard.insert(processor_id.clone(), processor_response.clone());
                }
                eprintln!("[REACTIVE_DEBUG] Processor '{}' stored result", processor_id);

                // Notify all dependents (event-driven core)
                eprintln!("[REACTIVE_DEBUG] Processor '{}' notifying {} dependents: {:?}", 
                    processor_id, node.dependents.len(), node.dependents);
                for dependent_id in &node.dependents {
                    if let Some(sender) = senders.get(dependent_id) {
                        eprintln!("[REACTIVE_DEBUG] Processor '{}' notifying dependent '{}'", 
                            processor_id, dependent_id);
                        if let Err(e) = sender.send(ProcessorEvent::DependencyCompleted {
                            dependency_id: processor_id.clone(),
                            metadata: processor_response.metadata.clone(),
                        }) {
                            eprintln!(
                                "[REACTIVE_DEBUG] Failed to notify dependent '{}' from processor '{}': {}",
                                dependent_id, processor_id, e
                            );
                        } else {
                            eprintln!("[REACTIVE_DEBUG] Processor '{}' successfully notified dependent '{}'", 
                                processor_id, dependent_id);
                        }
                    } else {
                        eprintln!("[REACTIVE_DEBUG] No sender found for dependent '{}' from processor '{}'", 
                            dependent_id, processor_id);
                    }
                }
            }
            Some(Outcome::Error(error_detail)) => {
                eprintln!("[REACTIVE_DEBUG] Processor '{}' failed with error: {}", 
                    processor_id, error_detail.message);
                
                // Processor failed - apply failure strategy
                match failure_strategy {
                    FailureStrategy::FailFast => {
                        eprintln!("[REACTIVE_DEBUG] Processor '{}' failing fast - cancelling all other tasks", processor_id);
                        // Cancel all other tasks before failing
                        cancellation_token.cancel();
                        // Fail immediately on first error
                        return Err(ExecutionError::ProcessorFailed {
                            processor_id: processor_id.clone(),
                            error: error_detail.message.clone(),
                        });
                    }
                    FailureStrategy::ContinueOnError | FailureStrategy::BestEffort => {
                        eprintln!("[REACTIVE_DEBUG] Processor '{}' continuing despite error", processor_id);
                        // Continue processing despite error - store failed result but don't notify dependents
                        // This prevents cascade failures while still recording the error
                        let mut results_guard = results_mutex.lock().await;
                        results_guard.insert(processor_id.clone(), processor_response.clone());
                        
                        // For ContinueOnError and BestEffort, we don't propagate to dependents
                        // This stops the error from cascading through the DAG
                    }
                }
            }
            None => {
                eprintln!("[REACTIVE_DEBUG] Processor '{}' returned no outcome", processor_id);
                
                // Processor returned no outcome - treat as error
                let error_msg = "Processor returned no outcome".to_string();
                match failure_strategy {
                    FailureStrategy::FailFast => {
                        eprintln!("[REACTIVE_DEBUG] Processor '{}' failing fast (no outcome) - cancelling all other tasks", processor_id);
                        // Cancel all other tasks before failing
                        cancellation_token.cancel();
                        return Err(ExecutionError::ProcessorFailed {
                            processor_id: processor_id.clone(),
                            error: error_msg.clone(),
                        });
                    }
                    FailureStrategy::ContinueOnError | FailureStrategy::BestEffort => {
                        eprintln!("[REACTIVE_DEBUG] Processor '{}' continuing despite no outcome", processor_id);
                        // Store error result but don't notify dependents
                        let error_response = ProcessorResponse {
                            outcome: Some(Outcome::Error(crate::proto::processor_v1::ErrorDetail {
                                code: 500,
                                message: error_msg,
                            })),
                            metadata: HashMap::new(),
                        };
                        let mut results_guard = results_mutex.lock().await;
                        results_guard.insert(processor_id.clone(), error_response);
                    }
                }
            }
        }

        eprintln!("[REACTIVE_DEBUG] Processor '{}' task completed successfully", processor_id);
        Ok(())
    }
}

#[async_trait]
impl DagExecutor for ReactiveExecutor {
    async fn execute_with_strategy(
        &self,
        processors: ProcessorMap,
        graph: DependencyGraph,
        entrypoints: EntryPoints,
        input: ProcessorRequest,
        failure_strategy: FailureStrategy,
    ) -> Result<HashMap<String, ProcessorResponse>, ExecutionError> {
        eprintln!("[REACTIVE_DEBUG] Starting reactive execution with {} processors, {} entry points", 
            processors.0.len(), entrypoints.0.len());
        eprintln!("[REACTIVE_DEBUG] Entry points: {:?}", entrypoints.0);
        
        // Validate dependency graph (reuse existing validation)
        let (_dependency_counts, _topological_ranks) = graph.dependency_counts_and_ranks()
            .ok_or_else(|| ExecutionError::InternalError {
                message: "Internal consistency error: dependency graph contains cycles (should have been caught during config validation)".into(),
            })?;

        // Build notification network using corrected approach
        eprintln!("[REACTIVE_DEBUG] Building notification network");
        let (senders, mut nodes) = self.build_notification_network(&graph)?;
        eprintln!("[REACTIVE_DEBUG] Notification network built successfully");

        // Initialize canonical payload with input payload
        let canonical_payload_mutex = Arc::new(Mutex::new(input.payload.clone()));
        let results_mutex = Arc::new(Mutex::new(HashMap::new()));
        let senders_arc = Arc::new(senders);
        let processors_arc = Arc::new(processors);
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.max_concurrency));
        let cancellation_token = CancellationToken::new();

        // Spawn tasks for all processors
        eprintln!("[REACTIVE_DEBUG] Spawning {} processor tasks", nodes.len());
        let mut tasks = Vec::new();
        for (processor_id, node) in nodes.drain() {
            eprintln!("[REACTIVE_DEBUG] Spawning task for processor '{}'", processor_id);
            let task = tokio::spawn(Self::spawn_processor_task(
                processor_id.clone(),
                node,
                processors_arc.clone(),
                canonical_payload_mutex.clone(),
                results_mutex.clone(),
                senders_arc.clone(),
                failure_strategy,
                semaphore.clone(),
                cancellation_token.clone(),
            ));
            tasks.push(task);
        }
        eprintln!("[REACTIVE_DEBUG] All {} processor tasks spawned", tasks.len());

        // Trigger entry point processors
        eprintln!("[REACTIVE_DEBUG] Triggering {} entry point processors", entrypoints.0.len());
        for entrypoint in entrypoints.iter() {
            eprintln!("[REACTIVE_DEBUG] Triggering entry point processor '{}'", entrypoint);
            if let Some(sender) = senders_arc.get(entrypoint) {
                if let Err(e) = sender.send(ProcessorEvent::Execute {
                    metadata: input.metadata.clone(),
                }) {
                    eprintln!(
                        "[REACTIVE_DEBUG] Failed to trigger entry point processor '{}': {}",
                        entrypoint, e
                    );
                } else {
                    eprintln!("[REACTIVE_DEBUG] Successfully triggered entry point processor '{}'", entrypoint);
                }
            } else {
                eprintln!("[REACTIVE_DEBUG] No sender found for entry point processor '{}'", entrypoint);
            }
        }
        eprintln!("[REACTIVE_DEBUG] All entry points triggered");

        // Wait for all tasks to complete
        eprintln!("[REACTIVE_DEBUG] Waiting for {} tasks to complete", tasks.len());
        let mut processor_error = None;
        let mut other_errors = Vec::new();
        
        for (i, task) in tasks.into_iter().enumerate() {
            eprintln!("[REACTIVE_DEBUG] Waiting for task {} to complete", i);
            match task.await {
                Ok(Ok(())) => {
                    eprintln!("[REACTIVE_DEBUG] Task {} completed successfully", i);
                }
                Ok(Err(e)) => {
                    eprintln!("[REACTIVE_DEBUG] Task {} failed with error: {:?}", i, e);
                    match &e {
                        ExecutionError::ProcessorFailed { .. } => {
                            // Prioritize actual processor failures over cancellation errors
                            if processor_error.is_none() {
                                processor_error = Some(e);
                            }
                        }
                        _ => {
                            // Collect other errors (like cancellation) as fallback
                            other_errors.push(e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[REACTIVE_DEBUG] Task {} join failed: {}", i, e);
                    other_errors.push(ExecutionError::InternalError {
                        message: format!("Task join failed: {}", e),
                    });
                }
            }
        }
        eprintln!("[REACTIVE_DEBUG] All tasks completed");
        
        // Return processor error first, then any other error, prioritizing actual failures
        if let Some(error) = processor_error {
            eprintln!("[REACTIVE_DEBUG] Returning processor error: {:?}", error);
            return Err(error);
        } else if let Some(error) = other_errors.into_iter().next() {
            eprintln!("[REACTIVE_DEBUG] Returning other error: {:?}", error);
            return Err(error);
        }

        // Return final results
        eprintln!("[REACTIVE_DEBUG] Extracting final results");
        let final_results = Arc::try_unwrap(results_mutex)
            .map_err(|_| ExecutionError::InternalError {
                message: "Failed to unwrap results Arc - multiple references still exist".into(),
            })?
            .into_inner();

        eprintln!("[REACTIVE_DEBUG] Execution completed with {} results: {:?}", 
            final_results.len(), final_results.keys().collect::<Vec<_>>());
        Ok(final_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::stub::StubProcessor;
    use crate::traits::Processor;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_reactive_executor_creation() {
        let executor = ReactiveExecutor::new(4);
        assert_eq!(executor.max_concurrency, 4);
    }

    #[tokio::test]
    async fn test_reactive_executor_default() {
        let executor = ReactiveExecutor::default();
        assert!(executor.max_concurrency >= 1);
    }

    #[tokio::test]
    async fn test_single_processor() {
        let executor = ReactiveExecutor::new(2);

        let mut processor_map: HashMap<String, Arc<dyn Processor>> = HashMap::new();
        processor_map.insert("test_proc".to_string(), Arc::new(StubProcessor::new("test_proc".to_string())));

        let mut dependency_graph = HashMap::new();
        dependency_graph.insert("test_proc".to_string(), vec![]);

        let entry_points = vec!["test_proc".to_string()];

        let input = ProcessorRequest {
            payload: b"test input".to_vec(),
            metadata: HashMap::new(),
        };

        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            FailureStrategy::FailFast,
        ).await;

        assert!(result.is_ok());
        let responses = result.unwrap();
        assert_eq!(responses.len(), 1);
        assert!(responses.contains_key("test_proc"));
    }

    #[tokio::test]
    async fn test_linear_dependency_chain() {
        let executor = ReactiveExecutor::new(2);

        let mut processor_map: HashMap<String, Arc<dyn Processor>> = HashMap::new();
        processor_map.insert("proc1".to_string(), Arc::new(StubProcessor::new("proc1".to_string())));
        processor_map.insert("proc2".to_string(), Arc::new(StubProcessor::new("proc2".to_string())));
        processor_map.insert("proc3".to_string(), Arc::new(StubProcessor::new("proc3".to_string())));

        let mut dependency_graph = HashMap::new();
        dependency_graph.insert("proc1".to_string(), vec!["proc2".to_string()]);
        dependency_graph.insert("proc2".to_string(), vec!["proc3".to_string()]);
        dependency_graph.insert("proc3".to_string(), vec![]);

        let entry_points = vec!["proc1".to_string()];

        let input = ProcessorRequest {
            payload: b"test input".to_vec(),
            metadata: HashMap::new(),
        };

        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            FailureStrategy::FailFast,
        ).await;

        assert!(result.is_ok());
        let responses = result.unwrap();
        assert_eq!(responses.len(), 3);
        assert!(responses.contains_key("proc1"));
        assert!(responses.contains_key("proc2"));
        assert!(responses.contains_key("proc3"));
    }

    #[tokio::test]
    async fn test_diamond_dependency_pattern() {
        let executor = ReactiveExecutor::new(4);

        let mut processor_map: HashMap<String, Arc<dyn Processor>> = HashMap::new();
        processor_map.insert("entry".to_string(), Arc::new(StubProcessor::new("entry".to_string())));
        processor_map.insert("left".to_string(), Arc::new(StubProcessor::new("left".to_string())));
        processor_map.insert("right".to_string(), Arc::new(StubProcessor::new("right".to_string())));
        processor_map.insert("merge".to_string(), Arc::new(StubProcessor::new("merge".to_string())));

        let mut dependency_graph = HashMap::new();
        dependency_graph.insert("entry".to_string(), vec!["left".to_string(), "right".to_string()]);
        dependency_graph.insert("left".to_string(), vec!["merge".to_string()]);
        dependency_graph.insert("right".to_string(), vec!["merge".to_string()]);
        dependency_graph.insert("merge".to_string(), vec![]);

        let entry_points = vec!["entry".to_string()];

        let input = ProcessorRequest {
            payload: b"test input".to_vec(),
            metadata: HashMap::new(),
        };

        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            FailureStrategy::FailFast,
        ).await;

        assert!(result.is_ok());
        let responses = result.unwrap();
        assert_eq!(responses.len(), 4);
        assert!(responses.contains_key("entry"));
        assert!(responses.contains_key("left"));
        assert!(responses.contains_key("right"));
        assert!(responses.contains_key("merge"));
    }

    #[tokio::test]
    async fn test_failure_strategy_fail_fast() {
        use crate::backends::stub::FailingProcessor;

        let executor = ReactiveExecutor::new(2);

        let mut processor_map: HashMap<String, Arc<dyn Processor>> = HashMap::new();
        processor_map.insert("entry".to_string(), Arc::new(StubProcessor::new("entry".to_string())));
        processor_map.insert("failing".to_string(), Arc::new(FailingProcessor::new("failing".to_string())));
        processor_map.insert("dependent".to_string(), Arc::new(StubProcessor::new("dependent".to_string())));

        let mut dependency_graph = HashMap::new();
        dependency_graph.insert("entry".to_string(), vec!["failing".to_string()]);
        dependency_graph.insert("failing".to_string(), vec!["dependent".to_string()]);
        dependency_graph.insert("dependent".to_string(), vec![]);

        let entry_points = vec!["entry".to_string()];

        let input = ProcessorRequest {
            payload: b"test input".to_vec(),
            metadata: HashMap::new(),
        };

        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            FailureStrategy::FailFast,
        ).await;

        // Should fail fast on first processor failure
        assert!(result.is_err());
        match result.unwrap_err() {
            ExecutionError::ProcessorFailed { processor_id, .. } => {
                assert_eq!(processor_id, "failing");
            }
            _ => panic!("Expected ProcessorFailed error"),
        }
    }

    #[tokio::test]
    async fn test_failure_strategy_continue_on_error() {
        use crate::backends::stub::FailingProcessor;

        let executor = ReactiveExecutor::new(2);

        let mut processor_map: HashMap<String, Arc<dyn Processor>> = HashMap::new();
        processor_map.insert("entry".to_string(), Arc::new(StubProcessor::new("entry".to_string())));
        processor_map.insert("failing".to_string(), Arc::new(FailingProcessor::new("failing".to_string())));
        processor_map.insert("independent".to_string(), Arc::new(StubProcessor::new("independent".to_string())));

        let mut dependency_graph = HashMap::new();
        dependency_graph.insert("entry".to_string(), vec!["failing".to_string(), "independent".to_string()]);
        dependency_graph.insert("failing".to_string(), vec![]);
        dependency_graph.insert("independent".to_string(), vec![]);

        let entry_points = vec!["entry".to_string()];

        let input = ProcessorRequest {
            payload: b"test input".to_vec(),
            metadata: HashMap::new(),
        };

        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            FailureStrategy::ContinueOnError,
        ).await;

        // Should continue execution despite failure
        assert!(result.is_ok());
        let responses = result.unwrap();

        // Should have results for entry and independent, plus failed result for failing
        assert_eq!(responses.len(), 3);
        assert!(responses.contains_key("entry"));
        assert!(responses.contains_key("failing"));
        assert!(responses.contains_key("independent"));

        // Verify the failing processor has an error outcome
        let failing_response = responses.get("failing").unwrap();
        match &failing_response.outcome {
            Some(Outcome::Error(_)) => {}, // Expected
            _ => panic!("Expected Error outcome for failing processor"),
        }
    }

    #[tokio::test]
    async fn test_entry_point_validation() {
        // This test verifies that entry points with non-zero pending_dependencies are caught
        // We can't easily create this scenario with the current setup since dependency counting
        // is done correctly, but this test documents the expected behavior

        let executor = ReactiveExecutor::new(1);

        let mut processor_map: HashMap<String, Arc<dyn Processor>> = HashMap::new();
        processor_map.insert("entry".to_string(), Arc::new(StubProcessor::new("entry".to_string())));

        let mut dependency_graph = HashMap::new();
        dependency_graph.insert("entry".to_string(), vec![]);

        let entry_points = vec!["entry".to_string()];

        let input = ProcessorRequest {
            payload: b"test input".to_vec(),
            metadata: HashMap::new(),
        };

        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            FailureStrategy::FailFast,
        ).await;

        // Should succeed for properly configured entry point
        assert!(result.is_ok());
        let responses = result.unwrap();
        assert_eq!(responses.len(), 1);
        assert!(responses.contains_key("entry"));
    }

    #[tokio::test]
    async fn test_processor_no_outcome_handling() {
        use crate::backends::stub::NoOutcomeProcessor;

        let executor = ReactiveExecutor::new(1);

        let mut processor_map: HashMap<String, Arc<dyn Processor>> = HashMap::new();
        processor_map.insert("no_outcome".to_string(), Arc::new(NoOutcomeProcessor::new("no_outcome".to_string())));

        let mut dependency_graph = HashMap::new();
        dependency_graph.insert("no_outcome".to_string(), vec![]);

        let entry_points = vec!["no_outcome".to_string()];

        let input = ProcessorRequest {
            payload: b"test input".to_vec(),
            metadata: HashMap::new(),
        };

        // Test FailFast behavior
        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map.clone()),
            DependencyGraph(dependency_graph.clone()),
            EntryPoints(entry_points.clone()),
            input.clone(),
            FailureStrategy::FailFast,
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ExecutionError::ProcessorFailed { processor_id, error } => {
                assert_eq!(processor_id, "no_outcome");
                assert_eq!(error, "Processor returned no outcome");
            }
            _ => panic!("Expected ProcessorFailed error"),
        }

        // Test ContinueOnError behavior
        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            FailureStrategy::ContinueOnError,
        ).await;

        assert!(result.is_ok());
        let responses = result.unwrap();
        assert_eq!(responses.len(), 1);

        let no_outcome_response = responses.get("no_outcome").unwrap();
        match &no_outcome_response.outcome {
            Some(Outcome::Error(error_detail)) => {
                assert_eq!(error_detail.message, "Processor returned no outcome");
                assert_eq!(error_detail.code, 500);
            }
            _ => panic!("Expected Error outcome for no outcome processor"),
        }
    }
}
