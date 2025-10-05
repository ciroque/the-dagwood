use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

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
        _failure_strategy: FailureStrategy,
        semaphore: Arc<tokio::sync::Semaphore>,
    ) -> Result<(), ExecutionError> {
        // Wait for all dependencies to complete
        while node.pending_dependencies > 0 {
            if let Some(event) = node.receiver.recv().await {
                match event {
                    ProcessorEvent::DependencyCompleted { dependency_id, metadata } => {
                        // Store dependency result for metadata merging
                        let dependency_response = ProcessorResponse {
                            outcome: Some(Outcome::NextPayload(vec![])), // Payload not used in metadata merging
                            metadata,
                        };
                        node.dependency_results.insert(dependency_id, dependency_response);
                        node.pending_dependencies -= 1;
                    }
                    ProcessorEvent::Execute { metadata } => {
                        // Entry point execution - store as base metadata
                        let base_response = ProcessorResponse {
                            outcome: Some(Outcome::NextPayload(vec![])),
                            metadata,
                        };
                        node.dependency_results.insert(BASE_METADATA_KEY.to_string(), base_response);
                        // For entry points, pending_dependencies should already be 0
                        break; // Exit the loop for entry points
                    }
                }
            } else {
                return Err(ExecutionError::InternalError {
                    message: format!("Channel closed for processor '{}'", processor_id),
                });
            }
        }

        // Acquire semaphore permit for concurrency control
        let _permit = semaphore.acquire().await
            .map_err(|e| ExecutionError::InternalError {
                message: format!("Failed to acquire semaphore permit for processor '{}': {}", processor_id, e),
            })?;

        // Get processor instance
        let processor = processors.get(&processor_id)
            .ok_or_else(|| ExecutionError::ProcessorNotFound(processor_id.clone()))?;

        // Build processor input using canonical payload and merged metadata
        let canonical_payload = {
            let guard = canonical_payload_mutex.lock().await;
            guard.clone()
        };

        // Use existing metadata merging utility (same as other executors)
        // Extract base metadata from original input and merge with dependency metadata
        let base_metadata = if let Some(input_metadata) = node.dependency_results.get(BASE_METADATA_KEY) {
            if let Some(base_meta) = input_metadata.metadata.get(BASE_METADATA_KEY) {
                base_meta.metadata.clone()
            } else {
                HashMap::new()
            }
        } else {
            HashMap::new()
        };

        let all_metadata = merge_dependency_metadata_for_execution(
            base_metadata,
            &node.dependency_results,
        );

        let processor_input = ProcessorRequest {
            payload: canonical_payload, // All processors get canonical payload (correct!)
            metadata: all_metadata,
        };

        // Execute processor
        let processor_response = processor.process(processor_input).await;

        // Update canonical payload if this is a Transform processor (same pattern as other executors)
        if processor.declared_intent() == ProcessorIntent::Transform {
            if let Some(Outcome::NextPayload(new_payload)) = &processor_response.outcome {
                let mut canonical_guard = canonical_payload_mutex.lock().await;
                *canonical_guard = new_payload.clone();
            }
        }

        // Store result
        {
            let mut results_guard = results_mutex.lock().await;
            results_guard.insert(processor_id.clone(), processor_response.clone());
        }

        // Notify all dependents (event-driven core)
        // Use the dependents list from the forward graph (fixed notification network)
        for dependent_id in &node.dependents {
            if let Some(sender) = senders.get(dependent_id) {
                let _ = sender.send(ProcessorEvent::DependencyCompleted {
                    dependency_id: processor_id.clone(),
                    metadata: processor_response.metadata.clone(),
                });
            }
        }

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
        // Validate dependency graph (reuse existing validation)
        let (_dependency_counts, _topological_ranks) = graph.dependency_counts_and_ranks()
            .ok_or_else(|| ExecutionError::InternalError {
                message: "Internal consistency error: dependency graph contains cycles (should have been caught during config validation)".into(),
            })?;

        // Build notification network using corrected approach
        let (senders, mut nodes) = self.build_notification_network(&graph)?;

        // Initialize canonical payload with input payload
        let canonical_payload_mutex = Arc::new(Mutex::new(input.payload.clone()));
        let results_mutex = Arc::new(Mutex::new(HashMap::new()));
        let senders_arc = Arc::new(senders);
        let processors_arc = Arc::new(processors);
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.max_concurrency));

        // Spawn tasks for all processors
        let mut tasks = Vec::new();
        for (processor_id, node) in nodes.drain() {
            let task = tokio::spawn(Self::spawn_processor_task(
                processor_id,
                node,
                processors_arc.clone(),
                canonical_payload_mutex.clone(),
                results_mutex.clone(),
                senders_arc.clone(),
                failure_strategy,
                semaphore.clone(),
            ));
            tasks.push(task);
        }

        // Trigger entry point processors
        for entrypoint in entrypoints.iter() {
            if let Some(sender) = senders_arc.get(entrypoint) {
                let _ = sender.send(ProcessorEvent::Execute {
                    metadata: input.metadata.clone(),
                });
            }
        }

        // Wait for all tasks to complete
        for task in tasks {
            if let Err(e) = task.await {
                return Err(ExecutionError::InternalError {
                    message: format!("Task execution failed: {}", e),
                });
            }
        }

        // Return final results
        let final_results = Arc::try_unwrap(results_mutex)
            .map_err(|_| ExecutionError::InternalError {
                message: "Failed to unwrap results Arc - multiple references still exist".into(),
            })?
            .into_inner();

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
}
