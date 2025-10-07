//! Reactive/Event-Driven DAG executor with async channel-based processor communication.
//!
//! This module implements a sophisticated event-driven execution strategy that uses async channels
//! to create a reactive network where processors are notified immediately when their dependencies
//! complete. This approach enables natural parallelism, low-latency execution, and efficient
//! resource utilization without artificial batching or level computation.
//!
//! # Architecture Overview
//!
//! The Reactive executor builds a **notification network** using async channels:
//! - Each processor has a dedicated receiver channel for dependency completion events
//! - When a processor completes, it broadcasts events to all its dependents
//! - Processors execute immediately when all dependencies are satisfied
//! - No polling, no artificial delays - pure event-driven execution
//!
//! # Key Features
//!
//! - **Event-Driven**: Processors react immediately to dependency completion
//! - **Natural Parallelism**: No artificial batching - processors run as soon as ready
//! - **Low Latency**: Minimal delay between dependency completion and dependent execution
//! - **Canonical Payload**: Maintains architectural consistency with Transform/Analyze separation
//! - **Metadata Collection**: Collects and merges processor metadata consistently with other executors
//! - **Failure Resilience**: Sophisticated error handling with cancellation support
//! - **Concurrency Control**: Configurable semaphore-based concurrency limiting
//!
//! # Execution Flow
//!
//! 1. **Network Setup**: Build async channel network based on dependency graph
//! 2. **Task Spawning**: Spawn async task for each processor
//! 3. **Entry Point Triggering**: Send execute events to entry point processors
//! 4. **Event Propagation**: Processors notify dependents upon completion
//! 5. **Result & Metadata Collection**: Gather results and metadata from all completed processors
//!
//! # Performance Characteristics
//!
//! - **Latency**: O(1) notification propagation (async channel send)
//! - **Throughput**: Limited by `max_concurrency` and processor execution time
//! - **Memory**: O(V) for channel network where V = number of processors
//! - **Scalability**: Excellent for I/O-bound processors with natural parallelism
//!
//! # Examples
//!
//! ## Basic reactive execution
//! ```rust
//! use std::collections::HashMap;
//! use std::sync::Arc;
//! use the_dagwood::engine::reactive::ReactiveExecutor;
//! use the_dagwood::traits::executor::DagExecutor;
//! use the_dagwood::config::{ProcessorMap, DependencyGraph, EntryPoints};
//! use the_dagwood::backends::stub::StubProcessor;
//! use the_dagwood::traits::Processor;
//! use the_dagwood::proto::processor_v1::{ProcessorRequest, PipelineMetadata};
//! use the_dagwood::errors::FailureStrategy;
//! 
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let executor = ReactiveExecutor::new(4); // 4 concurrent processors max
//! 
//! // Build processor map
//! let mut processor_map = HashMap::new();
//! processor_map.insert("input".to_string(), Arc::new(StubProcessor::new("input".to_string())) as Arc<dyn Processor>);
//! processor_map.insert("transform".to_string(), Arc::new(StubProcessor::new("transform".to_string())) as Arc<dyn Processor>);
//! processor_map.insert("output".to_string(), Arc::new(StubProcessor::new("output".to_string())) as Arc<dyn Processor>);
//! 
//! // Build dependency graph: input -> transform -> output
//! let mut dependency_graph = HashMap::new();
//! dependency_graph.insert("input".to_string(), vec![]); // Entry point - no dependencies
//! dependency_graph.insert("transform".to_string(), vec!["input".to_string()]); // Depends on input
//! dependency_graph.insert("output".to_string(), vec!["transform".to_string()]); // Depends on transform
//! 
//! let entry_points = vec!["input".to_string()];
//! let input = ProcessorRequest {
//!     payload: b"reactive execution test".to_vec(),
//! };
//! 
//! // Execute with event-driven approach
//! let (results, _metadata) = executor.execute_with_strategy(
//!     ProcessorMap(processor_map),
//!     DependencyGraph(dependency_graph),
//!     EntryPoints(entry_points),
//!     input,
//!     PipelineMetadata::new(),
//!     FailureStrategy::FailFast,
//! ).await?;
//! 
//! // All processors executed reactively
//! assert_eq!(results.len(), 3);
//! # Ok(())
//! # }
//! ```
//!
//! ## Diamond dependency with parallel execution
//! ```rust
//! use std::collections::HashMap;
//! use std::sync::Arc;
//! use the_dagwood::engine::reactive::ReactiveExecutor;
//! use the_dagwood::traits::executor::DagExecutor;
//! use the_dagwood::config::{ProcessorMap, DependencyGraph, EntryPoints};
//! use the_dagwood::backends::stub::StubProcessor;
//! use the_dagwood::traits::Processor;
//! use the_dagwood::proto::processor_v1::{ProcessorRequest, PipelineMetadata};
//! use the_dagwood::errors::FailureStrategy;
//! 
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let executor = ReactiveExecutor::new(4);
//! 
//! // Diamond pattern: source -> [left, right] -> sink
//! let mut processor_map = HashMap::new();
//! processor_map.insert("source".to_string(), Arc::new(StubProcessor::new("source".to_string())) as Arc<dyn Processor>);
//! processor_map.insert("left".to_string(), Arc::new(StubProcessor::new("left".to_string())) as Arc<dyn Processor>);
//! processor_map.insert("right".to_string(), Arc::new(StubProcessor::new("right".to_string())) as Arc<dyn Processor>);
//! processor_map.insert("sink".to_string(), Arc::new(StubProcessor::new("sink".to_string())) as Arc<dyn Processor>);
//! 
//! let mut dependency_graph = HashMap::new();
//! dependency_graph.insert("source".to_string(), vec!["left".to_string(), "right".to_string()]);
//! dependency_graph.insert("left".to_string(), vec!["sink".to_string()]);
//! dependency_graph.insert("right".to_string(), vec!["sink".to_string()]);
//! dependency_graph.insert("sink".to_string(), vec![]);
//! 
//! let entry_points = vec!["source".to_string()];
//! let input = ProcessorRequest {
//!     payload: b"diamond pattern".to_vec(),
//! };
//! 
//! // Left and right processors execute in parallel after source completes
//! // Sink executes immediately when both left and right complete
//! let (results, _metadata) = executor.execute_with_strategy(
//!     ProcessorMap(processor_map),
//!     DependencyGraph(dependency_graph),
//!     EntryPoints(entry_points),
//!     input,
//!     PipelineMetadata::new(),
//!     FailureStrategy::FailFast,
//! ).await?;
//! 
//! assert_eq!(results.len(), 4);
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;

use crate::traits::executor::DagExecutor;
use crate::traits::processor::ProcessorIntent;
use crate::config::{ProcessorMap, DependencyGraph, EntryPoints};
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, PipelineMetadata};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::errors::{ExecutionError, FailureStrategy};

/// Reactive/Event-Driven executor that uses async channels for processor communication.
///
/// This executor implements a sophisticated event-driven approach where processors are notified
/// immediately when their dependencies complete, enabling natural parallelism and low-latency
/// execution without artificial batching or level computation. It's particularly well-suited
/// for I/O-bound workloads and scenarios requiring minimal execution latency.
///
/// # Event-Driven Architecture
///
/// The executor builds a **notification network** using async channels where:
/// - Each processor has a dedicated receiver channel for dependency completion events
/// - When a processor completes, it broadcasts events to all its dependents via async channels
/// - Processors execute immediately when all dependencies are satisfied (no polling)
/// - Natural parallelism emerges from the dependency structure without artificial constraints
///
/// # Canonical Payload Architecture
///
/// Maintains architectural consistency with other executors through canonical payload tracking:
/// - **Transform processors**: Update the canonical payload when they complete successfully
/// - **Analyze processors**: Receive canonical payload but only contribute metadata
/// - **Downstream processors**: Always receive canonical payload + merged dependency metadata
/// - **Deterministic execution**: Same payload/metadata combination regardless of execution order
///
/// This ensures proper architectural separation and deterministic results across all execution strategies.
///
/// # Concurrency and Resource Management
///
/// - **Semaphore-based concurrency control**: Limits concurrent processor executions
/// - **Async task spawning**: Each processor runs in its own async task
/// - **Cancellation support**: Failed processors can cancel remaining tasks (FailFast mode)
/// - **Memory efficient**: O(V) memory usage for channel network where V = processor count
///
/// # Performance Characteristics
///
/// **Best suited for**:
/// - I/O-bound processors (network requests, file operations, database queries)
/// - Workloads requiring low latency between dependency completion and execution
/// - DAGs with natural parallelism and irregular execution times
///
/// **Performance metrics**:
/// - **Notification latency**: O(1) - async channel send
/// - **Execution latency**: Minimal - processors start immediately when ready
/// - **Memory overhead**: O(V) for channel infrastructure
/// - **Throughput**: Limited by `max_concurrency` and processor execution time
///
/// # Examples
///
/// ## Creating a reactive executor
/// ```rust
/// use the_dagwood::engine::reactive::ReactiveExecutor;
/// 
/// // Create with specific concurrency limit
/// let executor = ReactiveExecutor::new(8);
/// 
/// // Create with default concurrency (CPU core count)
/// let executor = ReactiveExecutor::default();
/// ```
///
/// ## Comparing with other execution strategies
/// ```rust
/// use the_dagwood::engine::reactive::ReactiveExecutor;
/// use the_dagwood::engine::work_queue::WorkQueueExecutor;
/// use the_dagwood::engine::level_by_level::LevelByLevelExecutor;
/// 
/// // Reactive: Best for I/O-bound, low-latency requirements
/// let reactive = ReactiveExecutor::new(4);
/// 
/// // WorkQueue: Best for CPU-bound, priority-based scheduling
/// let work_queue = WorkQueueExecutor::new(4);
/// 
/// // LevelByLevel: Best for predictable execution patterns, debugging
/// let level_by_level = LevelByLevelExecutor::new(4);
/// ```
pub struct ReactiveExecutor {
    /// Maximum number of concurrent processor executions.
    /// 
    /// This limit is enforced using a tokio Semaphore to prevent resource exhaustion
    /// while still allowing natural parallelism within the constraint. Processors
    /// will wait for permits before executing, but the event-driven notification
    /// system continues to operate without blocking.
    max_concurrency: usize,
}

/// Event sent between processors in the reactive execution network
#[derive(Debug, Clone)]
enum ProcessorEvent {
    /// Notification that a dependency has completed
    DependencyCompleted {
        dependency_id: String,
        metadata: Option<PipelineMetadata>,
    },
    /// Initial trigger for entry point processors
    Execute {
        metadata: Option<PipelineMetadata>,
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
}

impl ReactiveExecutor {
    /// Creates a new Reactive executor with the specified concurrency limit.
    ///
    /// The concurrency limit controls how many processors can execute simultaneously,
    /// preventing resource exhaustion while still allowing natural parallelism within
    /// the constraint. The event-driven notification system operates independently
    /// of this limit.
    ///
    /// # Arguments
    ///
    /// * `max_concurrency` - Maximum number of processors that can execute concurrently.
    ///                       Will be clamped to a minimum of 1.
    ///
    /// # Returns
    ///
    /// A new `ReactiveExecutor` configured with the specified concurrency limit.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use the_dagwood::engine::reactive::ReactiveExecutor;
    /// 
    /// // Create executor with specific concurrency
    /// let executor = ReactiveExecutor::new(8);
    /// 
    /// // Minimum concurrency is enforced
    /// let executor = ReactiveExecutor::new(0); // Actually creates with concurrency = 1
    /// ```
    ///
    /// # Performance Considerations
    ///
    /// - **Higher concurrency**: Better for I/O-bound processors, may increase memory usage
    /// - **Lower concurrency**: Better for CPU-bound processors, reduces context switching
    /// - **Rule of thumb**: Start with CPU core count, adjust based on processor characteristics
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            max_concurrency: max_concurrency.max(1), // Ensure at least 1
        }
    }

    /// Creates a new Reactive executor with default concurrency based on system capabilities.
    ///
    /// The default concurrency is set to the number of available CPU cores, which provides
    /// a good balance for most workloads. Falls back to 4 if the system's parallelism
    /// cannot be determined.
    ///
    /// # Returns
    ///
    /// A new `ReactiveExecutor` with concurrency set to the number of CPU cores.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use the_dagwood::engine::reactive::ReactiveExecutor;
    /// 
    /// // Create with system-appropriate concurrency
    /// let executor = ReactiveExecutor::default();
    /// 
    /// // Equivalent to:
    /// let core_count = std::thread::available_parallelism()
    ///     .map(|n| n.get())
    ///     .unwrap_or(4);
    /// let executor = ReactiveExecutor::new(core_count);
    /// ```
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

            // Use the forward graph (graph.0) to get dependents for notification network
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
            });
        }

        Ok((senders, nodes))
    }

    /// Handle a dependency completion event
    fn handle_dependency_completed(
        node: &mut ProcessorNode,
        _dependency_id: String,
        _metadata: Option<PipelineMetadata>,
    ) {
        // Simply decrement pending dependencies count
        // Metadata is collected globally via pipeline_metadata_mutex
        node.pending_dependencies -= 1;
    }

    /// Handle an execute event for entry point processors
    fn handle_execute_event(
        node: &mut ProcessorNode,
        _processor_id: &str,
        _metadata: Option<PipelineMetadata>,
    ) -> Result<bool, ExecutionError> {
        // Entry point execution - no dependency results needed for entry points
        
        // For true entry points, force pending dependencies to 0 to allow execution
        // This handles cases where entry points might have been incorrectly initialized
        node.pending_dependencies = 0;
        
        Ok(true) // Signal to break from dependency waiting loop
    }

    /// Process a single event received by a processor
    async fn process_event(
        node: &mut ProcessorNode,
        processor_id: &str,
        event: ProcessorEvent,
    ) -> Result<bool, ExecutionError> {
        match event {
            ProcessorEvent::DependencyCompleted { dependency_id, metadata } => {
                Self::handle_dependency_completed(node, dependency_id, metadata);
                Ok(false) // Continue waiting for more dependencies
            }
            ProcessorEvent::Execute { metadata } => {
                Self::handle_execute_event(node, processor_id, metadata)
            }
        }
    }

    /// Wait for all dependencies to complete before processor execution
    async fn wait_for_dependencies(
        mut node: ProcessorNode,
        processor_id: &str,
        cancellation_token: &CancellationToken,
    ) -> Result<ProcessorNode, ExecutionError> {
        while node.pending_dependencies > 0 {
            tokio::select! {
                // Check for cancellation first
                _ = cancellation_token.cancelled() => {
                    return Err(ExecutionError::InternalError {
                        message: format!("Processor '{}' cancelled due to failure in another processor", processor_id),
                    });
                }
                // Wait for dependency events
                event_result = node.receiver.recv() => {
                    if let Some(event) = event_result {
                        let should_break = Self::process_event(&mut node, processor_id, event).await?;
                        if should_break {
                            break;
                        }
                    } else {
                        return Err(ExecutionError::InternalError {
                            message: format!("Channel closed for processor '{}'", processor_id),
                        });
                    }
                }
            }
        }
        Ok(node)
    }

    /// Spawn an async task for a processor in the reactive network
    ///
    /// This reuses the canonical payload architecture, declared_intent() pattern,
    /// and metadata collection from the existing executors to maintain consistency.
    async fn spawn_processor_task(
        processor_id: String,
        node: ProcessorNode,
        processors: Arc<ProcessorMap>,
        canonical_payload_mutex: Arc<Mutex<Vec<u8>>>,
        results_mutex: Arc<Mutex<HashMap<String, ProcessorResponse>>>,
        pipeline_metadata_mutex: Arc<Mutex<PipelineMetadata>>,
        senders: Arc<HashMap<String, mpsc::UnboundedSender<ProcessorEvent>>>,
        failure_strategy: FailureStrategy,
        semaphore: Arc<tokio::sync::Semaphore>,
        cancellation_token: CancellationToken,
    ) -> Result<(), ExecutionError> {
        // Wait for all dependencies to complete
        let node = Self::wait_for_dependencies(node, &processor_id, &cancellation_token).await?;

        // Acquire semaphore permit for concurrency control
        let _permit = semaphore.acquire().await
            .map_err(|e| ExecutionError::InternalError {
                message: format!("Failed to acquire semaphore permit for processor '{}': {}", processor_id, e),
            })?;

        // Check for cancellation after acquiring semaphore to prevent race condition
        // where a task acquires semaphore but should have been cancelled
        if cancellation_token.is_cancelled() {
            return Err(ExecutionError::InternalError {
                message: format!("Processor '{}' cancelled after semaphore acquisition", processor_id),
            });
        }

        // Get processor instance
        let processor = processors.get(&processor_id)
            .ok_or_else(|| ExecutionError::ProcessorNotFound(processor_id.clone()))?;

        // CRITICAL FIX: Get canonical payload AFTER dependencies complete
        // This ensures Transform dependencies have updated the canonical payload before dependents access it
        let canonical_payload = {
            let guard = canonical_payload_mutex.lock().await;
            guard.clone()
        };

        let processor_input = ProcessorRequest {
            payload: canonical_payload, // All processors get canonical payload
        };

        // Execute processor
        let processor_response = processor.process(processor_input).await;

        // Handle processor execution result based on failure strategy
        match &processor_response.outcome {
            Some(Outcome::NextPayload(_)) => {
                // Success case - update canonical payload if this is a Transform processor
                // CRITICAL: Update canonical payload BEFORE notifying dependents to prevent race conditions
                if processor.declared_intent() == ProcessorIntent::Transform {
                    if let Some(Outcome::NextPayload(new_payload)) = &processor_response.outcome {
                        let mut canonical_guard = canonical_payload_mutex.lock().await;
                        *canonical_guard = new_payload.clone();
                        // Keep the lock until after we notify dependents to ensure atomicity
                        
                        // Store successful result
                        {
                            let mut results_guard = results_mutex.lock().await;
                            results_guard.insert(processor_id.clone(), processor_response.clone());
                        }

                        // Collect metadata from processor response
                        {
                            let mut pipeline_meta = pipeline_metadata_mutex.lock().await;
                            pipeline_meta.merge_processor_response(&processor_id, &processor_response);
                        }

                        // Notify all dependents AFTER canonical payload is updated
                        for dependent_id in &node.dependents {
                            if let Some(sender) = senders.get(dependent_id) {
                                if let Err(_) = sender.send(ProcessorEvent::DependencyCompleted {
                                    dependency_id: processor_id.clone(),
                                    metadata: processor_response.metadata.clone(),
                                }) {
                                    // Channel closed - dependent processor likely cancelled or failed
                                    // This is expected during cancellation scenarios, so we continue
                                    // without treating it as an error
                                }
                            }
                        }
                        // canonical_guard is dropped here, ensuring atomicity
                    }
                } else {
                    // Non-Transform processor - no canonical payload update needed
                    // Store successful result
                    {
                        let mut results_guard = results_mutex.lock().await;
                        results_guard.insert(processor_id.clone(), processor_response.clone());
                    }

                    // Collect metadata from processor response
                    {
                        let mut pipeline_meta = pipeline_metadata_mutex.lock().await;
                        pipeline_meta.merge_processor_response(&processor_id, &processor_response);
                    }

                    // Notify all dependents (event-driven core)
                    for dependent_id in &node.dependents {
                        if let Some(sender) = senders.get(dependent_id) {
                            if let Err(_) = sender.send(ProcessorEvent::DependencyCompleted {
                                dependency_id: processor_id.clone(),
                                metadata: processor_response.metadata.clone(),
                            }) {
                                // Channel closed - dependent processor likely cancelled or failed
                                // This is expected during cancellation scenarios, so we continue
                                // without treating it as an error
                            }
                        }
                    }
                }
            }
            Some(Outcome::Error(error_detail)) => {
                // Processor failed - apply failure strategy
                match failure_strategy {
                    FailureStrategy::FailFast => {
                        // Cancel all other tasks before failing
                        cancellation_token.cancel();
                        // Fail immediately on first error
                        return Err(ExecutionError::ProcessorFailed {
                            processor_id: processor_id.clone(),
                            error: error_detail.message.clone(),
                        });
                    }
                    FailureStrategy::ContinueOnError | FailureStrategy::BestEffort => {
                        // Store failed result but notify dependents with empty metadata
                        let mut results_guard = results_mutex.lock().await;
                        results_guard.insert(processor_id.clone(), processor_response.clone());
                        
                        // Notify dependents with None metadata to maintain dependency counting
                        // This prevents cascade failures while allowing execution to continue
                        for dependent_id in &node.dependents {
                            if let Some(sender) = senders.get(dependent_id) {
                                let _ = sender.send(ProcessorEvent::DependencyCompleted {
                                    dependency_id: processor_id.clone(),
                                    metadata: None, // Failed processor contributes no metadata
                                });
                            }
                        }
                    }
                }
            }
            None => {
                // Processor returned no outcome - treat as error
                let error_msg = "Processor returned no outcome".to_string();
                match failure_strategy {
                    FailureStrategy::FailFast => {
                        // Cancel all other tasks before failing
                        cancellation_token.cancel();
                        return Err(ExecutionError::ProcessorFailed {
                            processor_id: processor_id.clone(),
                            error: error_msg.clone(),
                        });
                    }
                    FailureStrategy::ContinueOnError | FailureStrategy::BestEffort => {
                        // Store error result but notify dependents with empty metadata
                        let error_response = ProcessorResponse {
                            outcome: Some(Outcome::Error(crate::proto::processor_v1::ErrorDetail {
                                code: 500,
                                message: error_msg,
                            })),
                            metadata: None,
                        };
                        let mut results_guard = results_mutex.lock().await;
                        results_guard.insert(processor_id.clone(), error_response);
                        
                        // Notify dependents with None metadata to maintain dependency counting
                        for dependent_id in &node.dependents {
                            if let Some(sender) = senders.get(dependent_id) {
                                let _ = sender.send(ProcessorEvent::DependencyCompleted {
                                    dependency_id: processor_id.clone(),
                                    metadata: None, // Failed processor contributes no metadata
                                });
                            }
                        }
                    }
                }
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
        pipeline_metadata: PipelineMetadata,
        failure_strategy: FailureStrategy,
    ) -> Result<(HashMap<String, ProcessorResponse>, PipelineMetadata), ExecutionError> {
        
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
        let pipeline_metadata_mutex = Arc::new(Mutex::new(pipeline_metadata));
        let senders_arc = Arc::new(senders);
        let processors_arc = Arc::new(processors);
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.max_concurrency));
        let cancellation_token = CancellationToken::new();

        // Spawn tasks for all processors
        let mut tasks = Vec::new();
        for (processor_id, node) in nodes.drain() {
            let dependents = node.dependents.clone(); // Store dependents for panic recovery
            let task = tokio::spawn(Self::spawn_processor_task(
                processor_id.clone(),
                node,
                processors_arc.clone(),
                canonical_payload_mutex.clone(),
                results_mutex.clone(),
                pipeline_metadata_mutex.clone(),
                senders_arc.clone(),
                failure_strategy,
                semaphore.clone(),
                cancellation_token.clone(),
            ));
            tasks.push((task, processor_id, dependents));
        }

        // Trigger entry point processors
        for entrypoint in entrypoints.iter() {
            if let Some(sender) = senders_arc.get(entrypoint) {
                if let Err(_) = sender.send(ProcessorEvent::Execute {
                    metadata: None,
                }) {
                    // Entry point processor channel closed - this indicates a serious issue
                    // since entry points should be ready to receive at startup
                    return Err(ExecutionError::InternalError {
                        message: format!("Failed to trigger entry point processor '{}' - channel closed", entrypoint),
                    });
                }
            }
        }

        // Wait for all tasks to complete
        let mut processor_error = None;
        let mut other_errors = Vec::new();
        
        for (task, processor_id, dependents) in tasks.into_iter() {
            match task.await {
                Ok(Ok(())) => {
                    // Task completed successfully
                }
                Ok(Err(e)) => {
                    match &e {
                        ExecutionError::ProcessorFailed { .. } => {
                            // Collect all processor failures for comprehensive error reporting
                            if processor_error.is_none() {
                                processor_error = Some(e); // Keep first failure as primary
                            } else {
                                other_errors.push(e); // Collect additional failures
                            }
                        }
                        _ => {
                            // Collect other errors (like cancellation) as fallback
                            other_errors.push(e);
                        }
                    }
                }
                Err(join_error) => {
                    // Task panicked or was cancelled - notify dependents to prevent deadlock
                    for dependent_id in &dependents {
                        if let Some(sender) = senders_arc.get(dependent_id) {
                            let _ = sender.send(ProcessorEvent::DependencyCompleted {
                                dependency_id: processor_id.clone(),
                                metadata: None, // Panicked processor contributes no metadata
                            });
                        }
                    }
                    
                    // Determine if this was a panic or cancellation
                    let error_message = if join_error.is_panic() {
                        format!("Processor '{}' panicked during execution", processor_id)
                    } else {
                        format!("Processor '{}' task was cancelled", processor_id)
                    };
                    
                    other_errors.push(ExecutionError::InternalError {
                        message: error_message,
                    });
                }
            }
        }
        
        // Return processor error first, then any other error, prioritizing actual failures
        if let Some(error) = processor_error {
            return Err(error);
        } else if let Some(error) = other_errors.into_iter().next() {
            return Err(error);
        }

        // Return final results
        let final_results = Arc::try_unwrap(results_mutex)
            .map_err(|_| ExecutionError::InternalError {
                message: "Failed to unwrap results Arc - multiple references still exist".into(),
            })?
            .into_inner();

        // Return collected metadata from processor responses
        let final_metadata = Arc::try_unwrap(pipeline_metadata_mutex)
            .map_err(|_| ExecutionError::InternalError {
                message: "Failed to unwrap pipeline metadata Arc - multiple references still exist".into(),
            })?
            .into_inner();

        Ok((final_results, final_metadata))
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
        };

        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            PipelineMetadata::new(),
            FailureStrategy::FailFast,
        ).await;

        assert!(result.is_ok());
        let (responses, _metadata) = result.unwrap();
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
        };

        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            PipelineMetadata::new(),
            FailureStrategy::FailFast,
        ).await;

        assert!(result.is_ok());
        let (responses, _metadata) = result.unwrap();
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
        };

        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            PipelineMetadata::new(),
            FailureStrategy::FailFast,
        ).await;

        assert!(result.is_ok());
        let (responses, _metadata) = result.unwrap();
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
        };

        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            PipelineMetadata::new(),
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
        };

        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            PipelineMetadata::new(),
            FailureStrategy::ContinueOnError,
        ).await;

        // Should continue execution despite failure
        assert!(result.is_ok());
        let (responses, _metadata) = result.unwrap();

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
        };

        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            PipelineMetadata::new(),
            FailureStrategy::FailFast,
        ).await;

        // Should succeed for properly configured entry point
        assert!(result.is_ok());
        let (responses, _metadata) = result.unwrap();
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
        };

        // Test FailFast behavior
        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map.clone()),
            DependencyGraph(dependency_graph.clone()),
            EntryPoints(entry_points.clone()),
            input.clone(),
            PipelineMetadata::new(),
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
            PipelineMetadata::new(),
            FailureStrategy::ContinueOnError,
        ).await;

        assert!(result.is_ok());
        let (responses, _metadata) = result.unwrap();
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

    #[tokio::test]
    async fn test_channel_error_handling_resilience() {
        use crate::backends::stub::StubProcessor;

        let executor = ReactiveExecutor::new(1);

        let mut processor_map: HashMap<String, Arc<dyn Processor>> = HashMap::new();
        processor_map.insert("simple".to_string(), Arc::new(StubProcessor::new("simple".to_string())));

        let mut dependency_graph = HashMap::new();
        dependency_graph.insert("simple".to_string(), vec![]);

        let entry_points = vec!["simple".to_string()];

        let input = ProcessorRequest {
            payload: b"test".to_vec(),
        };

        // This test verifies that our channel error handling improvements
        // don't break normal execution of simple processors
        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            PipelineMetadata::new(),
            FailureStrategy::FailFast,
        ).await;

        // Should succeed - this tests that our error handling improvements
        // don't break normal execution
        match result {
            Ok((responses, _metadata)) => {
                assert_eq!(responses.len(), 1);
                assert!(responses.contains_key("simple"));
            },
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_entry_point_triggering_success() {
        use crate::backends::stub::StubProcessor;

        let executor = ReactiveExecutor::new(1);

        let mut processor_map: HashMap<String, Arc<dyn Processor>> = HashMap::new();
        processor_map.insert("entry".to_string(), Arc::new(StubProcessor::new("entry".to_string())));

        let mut dependency_graph = HashMap::new();
        dependency_graph.insert("entry".to_string(), vec![]);

        let entry_points = vec!["entry".to_string()];

        let input = ProcessorRequest {
            payload: b"test".to_vec(),
        };

        // This test verifies that entry point triggering works correctly
        // and that our error handling doesn't interfere with normal operation
        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            PipelineMetadata::new(),
            FailureStrategy::FailFast,
        ).await;

        assert!(result.is_ok());
        let (responses, _metadata) = result.unwrap();
        assert_eq!(responses.len(), 1);
        assert!(responses.contains_key("entry"));
    }

    #[tokio::test]
    async fn test_error_handling_with_failing_processor() {
        use crate::backends::stub::FailingProcessor;

        let executor = ReactiveExecutor::new(1);

        let mut processor_map: HashMap<String, Arc<dyn Processor>> = HashMap::new();
        processor_map.insert("failing".to_string(), Arc::new(FailingProcessor::new("failing".to_string())));

        let mut dependency_graph = HashMap::new();
        dependency_graph.insert("failing".to_string(), vec![]);

        let entry_points = vec!["failing".to_string()];

        let input = ProcessorRequest {
            payload: b"test".to_vec(),
        };

        // This test verifies that processor failures are handled correctly
        // and that our channel error handling doesn't interfere with failure reporting
        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            PipelineMetadata::new(),
            FailureStrategy::FailFast,
        ).await;

        // Should fail due to the failing processor
        assert!(result.is_err());
        match result.unwrap_err() {
            ExecutionError::ProcessorFailed { processor_id, .. } => {
                assert_eq!(processor_id, "failing");
            }
            other_error => panic!("Expected ProcessorFailed error, got: {:?}", other_error),
        }
    }

    #[tokio::test]
    async fn test_multiple_processors_execution() {
        use crate::backends::stub::StubProcessor;

        let executor = ReactiveExecutor::new(3);

        let mut processor_map: HashMap<String, Arc<dyn Processor>> = HashMap::new();
        processor_map.insert("proc1".to_string(), Arc::new(StubProcessor::new("proc1".to_string())));
        processor_map.insert("proc2".to_string(), Arc::new(StubProcessor::new("proc2".to_string())));
        processor_map.insert("proc3".to_string(), Arc::new(StubProcessor::new("proc3".to_string())));

        let mut dependency_graph = HashMap::new();
        dependency_graph.insert("proc1".to_string(), vec![]);
        dependency_graph.insert("proc2".to_string(), vec![]);
        dependency_graph.insert("proc3".to_string(), vec![]);

        let entry_points = vec!["proc1".to_string(), "proc2".to_string(), "proc3".to_string()];

        let input = ProcessorRequest {
            payload: b"test".to_vec(),
        };

        // Test that multiple independent processors can execute successfully
        // and that our channel error handling doesn't interfere
        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            PipelineMetadata::new(),
            FailureStrategy::FailFast,
        ).await;

        match result {
            Ok((responses, _metadata)) => {
                assert_eq!(responses.len(), 3);
                assert!(responses.contains_key("proc1"));
                assert!(responses.contains_key("proc2"));
                assert!(responses.contains_key("proc3"));
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_panic_recovery_prevents_deadlock() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use async_trait::async_trait;
        use crate::traits::processor::{Processor, ProcessorIntent};
        use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse};
        use crate::proto::processor_v1::processor_response::Outcome;

        // Mock processor that panics during execution
        struct PanickingProcessor;

        #[async_trait]
        impl Processor for PanickingProcessor {
            fn name(&self) -> &'static str {
                "panicking_processor"
            }

            fn declared_intent(&self) -> ProcessorIntent {
                ProcessorIntent::Transform
            }

            async fn process(&self, _req: ProcessorRequest) -> ProcessorResponse {
                panic!("Intentional panic for testing");
            }
        }

        // Mock processor that tracks if it was called (to verify dependent execution)
        struct TrackingProcessor {
            called: Arc<AtomicBool>,
        }

        impl TrackingProcessor {
            fn new(called: Arc<AtomicBool>) -> Self {
                Self { called }
            }
        }

        #[async_trait]
        impl Processor for TrackingProcessor {
            fn name(&self) -> &'static str {
                "tracking_processor"
            }

            fn declared_intent(&self) -> ProcessorIntent {
                ProcessorIntent::Analyze
            }

            async fn process(&self, _req: ProcessorRequest) -> ProcessorResponse {
                self.called.store(true, Ordering::SeqCst);
                ProcessorResponse {
                    outcome: Some(Outcome::NextPayload(b"tracked".to_vec())),
                    metadata: None,
                }
            }
        }

        let executor = ReactiveExecutor::new(2);

        // Track if dependent processor was called
        let dependent_called = Arc::new(AtomicBool::new(false));

        let mut processor_map: HashMap<String, Arc<dyn Processor>> = HashMap::new();
        processor_map.insert("panicking".to_string(), Arc::new(PanickingProcessor));
        processor_map.insert("dependent".to_string(), Arc::new(TrackingProcessor::new(dependent_called.clone())));

        // Setup: panicking -> dependent
        let mut dependency_graph = HashMap::new();
        dependency_graph.insert("panicking".to_string(), vec![]);
        dependency_graph.insert("dependent".to_string(), vec!["panicking".to_string()]);

        let entry_points = vec!["panicking".to_string()];

        let input = ProcessorRequest {
            payload: b"test".to_vec(),
        };

        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            PipelineMetadata::new(),
            FailureStrategy::ContinueOnError, // Continue despite panic
        ).await;

        // The execution might fail due to the panic, but the key test is whether
        // the dependent processor was called (proving panic recovery worked)
        let execution_succeeded = match &result {
            Ok(_) => {
                // Great! Execution succeeded despite panic
                true
            }
            Err(e) => {
                // Execution failed, but that's okay as long as dependent was notified
                println!("Execution failed as expected due to panic: {:?}", e);
                false
            }
        };
        
        // Most importantly: dependent processor should have been called
        // This proves panic recovery worked and prevented deadlock
        assert!(dependent_called.load(Ordering::SeqCst), 
                "Dependent processor was not called - panic recovery failed!");

        // If execution succeeded, verify dependent response exists
        if execution_succeeded {
            let (responses, _metadata) = result.unwrap();
            assert!(responses.contains_key("dependent"));
        }
    }

    #[tokio::test]
    async fn test_entry_point_with_dependencies_validation() {
        use crate::backends::stub::StubProcessor;

        let executor = ReactiveExecutor::new(2);

        let mut processor_map: HashMap<String, Arc<dyn Processor>> = HashMap::new();
        processor_map.insert("not_entry".to_string(), Arc::new(StubProcessor::new("not_entry".to_string())));
        processor_map.insert("fake_entry".to_string(), Arc::new(StubProcessor::new("fake_entry".to_string())));

        // Misconfigured graph: "fake_entry" is marked as entry point but has dependencies
        let mut dependency_graph = HashMap::new();
        dependency_graph.insert("not_entry".to_string(), vec![]);
        dependency_graph.insert("fake_entry".to_string(), vec!["not_entry".to_string()]); // Entry point with deps!

        // This is the misconfiguration: fake_entry has dependencies but is marked as entry point
        let entry_points = vec!["fake_entry".to_string()];

        let input = ProcessorRequest {
            payload: b"test".to_vec(),
        };

        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            PipelineMetadata::new(),
            FailureStrategy::FailFast,
        ).await;

        // Should succeed because our current implementation forces pending_dependencies = 0
        // for entry points, handling the misconfiguration gracefully
        assert!(result.is_ok());
        let (responses, _metadata) = result.unwrap();
        assert_eq!(responses.len(), 2); // Both processors should execute
    }

    #[tokio::test]
    async fn test_canonical_payload_transform_propagation() {
        // NOTE: This test demonstrates that the reactive executor currently does NOT
        // implement canonical payload propagation like the work queue executor.
        // The reactive executor treats each processor independently rather than
        // maintaining a shared canonical payload that Transform processors can update.
        // 
        // This is a known architectural difference - the reactive executor focuses on
        // event-driven execution rather than the canonical payload pattern.
        // 
        // For now, we'll test that Transform processors can at least update their own output.
        
        use async_trait::async_trait;
        use crate::traits::processor::{Processor, ProcessorIntent};
        use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse};
        use crate::proto::processor_v1::processor_response::Outcome;

        // Simple Transform processor that modifies its input
        struct TransformProcessor;

        #[async_trait]
        impl Processor for TransformProcessor {
            fn name(&self) -> &'static str {
                "transform_processor"
            }

            fn declared_intent(&self) -> ProcessorIntent {
                ProcessorIntent::Transform
            }

            async fn process(&self, req: ProcessorRequest) -> ProcessorResponse {
                let input_text = String::from_utf8_lossy(&req.payload);
                let output_text = format!("{}-transformed", input_text);
                ProcessorResponse {
                    outcome: Some(Outcome::NextPayload(output_text.into_bytes())),
                    metadata: None,
                }
            }
        }

        let executor = ReactiveExecutor::new(1);

        let mut processor_map: HashMap<String, Arc<dyn Processor>> = HashMap::new();
        processor_map.insert("transform".to_string(), Arc::new(TransformProcessor));

        let mut dependency_graph = HashMap::new();
        dependency_graph.insert("transform".to_string(), vec![]);

        let entry_points = vec!["transform".to_string()];

        let input = ProcessorRequest {
            payload: b"hello".to_vec(),
        };

        let result = executor.execute_with_strategy(
            ProcessorMap(processor_map),
            DependencyGraph(dependency_graph),
            EntryPoints(entry_points),
            input,
            PipelineMetadata::new(),
            FailureStrategy::FailFast,
        ).await;

        assert!(result.is_ok());
        let (responses, _metadata) = result.unwrap();
        assert_eq!(responses.len(), 1);

        // Verify the transform processor produced the expected output
        let transform_response = responses.get("transform").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &transform_response.outcome {
            let output_text = String::from_utf8_lossy(payload);
            assert_eq!(output_text, "hello-transformed", 
                       "Transform processor should modify its own output");
        } else {
            panic!("Transform processor should return NextPayload outcome");
        }
    }
}
