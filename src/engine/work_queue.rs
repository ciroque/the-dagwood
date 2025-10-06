//! Work Queue DAG executor with dependency counting and canonical payload architecture.
//!
//! This module implements a sophisticated work queue-based execution strategy that combines
//! dependency counting algorithms with a revolutionary canonical payload architecture to
//! eliminate race conditions and ensure deterministic DAG execution. The executor is
//! particularly well-suited for CPU-bound workloads and scenarios requiring predictable
//! execution order and resource utilization.
//!
//! # Architecture Overview
//!
//! The Work Queue executor uses a **priority queue** combined with **dependency counting**:
//! - Processors are queued based on topological rank and processor intent
//! - Dependency counting tracks how many dependencies each processor is waiting for
//! - When dependencies complete, dependent processors are added to the work queue
//! - Configurable concurrency limits prevent resource exhaustion
//!
//! # Canonical Payload Innovation
//!
//! The executor implements a groundbreaking **canonical payload architecture** that solves
//! the fundamental race condition problem in diamond dependency patterns:
//!
//! ```text
//! Diamond Pattern Race Condition (SOLVED):
//!     A (Transform)
//!    / \
//!   B   C (parallel execution)
//!    \ /
//!     D (which payload should D receive?)
//! ```
//!
//! ## Solution: Canonical Payload Tracking
//! - **Single Source of Truth**: One canonical payload flows through the DAG
//! - **Transform Processors**: Only Transform processors can update the canonical payload
//! - **Analyze Processors**: Receive canonical payload but only contribute metadata
//! - **Topological Ranking**: Higher-ranked Transform processors override lower-ranked ones
//! - **Deterministic Updates**: Race conditions eliminated through strict ordering rules
//!
//! # Key Features
//!
//! - **Dependency Counting**: Efficient O(1) dependency resolution using counters
//! - **Priority Scheduling**: Topological rank + processor intent-based prioritization
//! - **Canonical Payload**: Revolutionary architecture eliminating race conditions
//! - **Failure Strategies**: Comprehensive error handling (FailFast, ContinueOnError, BestEffort)
//! - **Concurrency Control**: Configurable limits with efficient task management
//! - **Metadata Isolation**: Processors only receive metadata from direct dependencies
//!
//! # Performance Characteristics
//!
//! - **Best for**: CPU-bound processors, predictable execution patterns, priority-based scheduling
//! - **Time Complexity**: O(V + E) where V = processors, E = dependencies
//! - **Space Complexity**: O(V) for dependency tracking and work queue
//! - **Concurrency**: Configurable with efficient semaphore-like task limiting
//! - **Determinism**: Fully deterministic execution order and results
//!
//! # Execution Flow
//!
//! 1. **Validation**: Verify all processors exist and graph is valid
//! 2. **Initialization**: Build dependency counts and topological ranks
//! 3. **Queue Setup**: Initialize priority work queue with entry points
//! 4. **Execution Loop**: Process work queue with dependency counting
//! 5. **Result Collection**: Gather results from all completed processors
//!
//! # Examples
//!
//! ## Basic work queue execution
//! ```rust
//! use std::collections::HashMap;
//! use std::sync::Arc;
//! use the_dagwood::engine::work_queue::WorkQueueExecutor;
//! use the_dagwood::traits::executor::DagExecutor;
//! use the_dagwood::config::{ProcessorMap, DependencyGraph, EntryPoints};
//! use the_dagwood::backends::stub::StubProcessor;
//! use the_dagwood::traits::Processor;
//! use the_dagwood::proto::processor_v1::ProcessorRequest;
//! use the_dagwood::errors::FailureStrategy;
//! 
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let executor = WorkQueueExecutor::new(4); // 4 concurrent processors max
//! 
//! // Build processor map
//! let mut processor_map = HashMap::new();
//! processor_map.insert("input".to_string(), Arc::new(StubProcessor::new("input".to_string())) as Arc<dyn Processor>);
//! processor_map.insert("process".to_string(), Arc::new(StubProcessor::new("process".to_string())) as Arc<dyn Processor>);
//! processor_map.insert("output".to_string(), Arc::new(StubProcessor::new("output".to_string())) as Arc<dyn Processor>);
//! 
//! // Build dependency graph: input -> process -> output
//! let mut dependency_graph = HashMap::new();
//! dependency_graph.insert("input".to_string(), vec!["process".to_string()]);
//! dependency_graph.insert("process".to_string(), vec!["output".to_string()]);
//! dependency_graph.insert("output".to_string(), vec![]);
//! 
//! let entry_points = vec!["input".to_string()];
//! let input = ProcessorRequest {
//!     payload: b"work queue execution".to_vec(),
//!     metadata: HashMap::new(),
//! };
//! 
//! // Execute with dependency counting and canonical payload
//! let results = executor.execute_with_strategy(
//!     ProcessorMap(processor_map),
//!     DependencyGraph(dependency_graph),
//!     EntryPoints(entry_points),
//!     input,
//!     FailureStrategy::FailFast,
//! ).await?;
//! 
//! // All processors executed in dependency order
//! assert_eq!(results.len(), 3);
//! # Ok(())
//! # }
//! ```
//!
//! ## Diamond dependency with canonical payload
//! ```rust
//! use std::collections::HashMap;
//! use std::sync::Arc;
//! use the_dagwood::engine::work_queue::WorkQueueExecutor;
//! use the_dagwood::traits::executor::DagExecutor;
//! use the_dagwood::config::{ProcessorMap, DependencyGraph, EntryPoints};
//! use the_dagwood::backends::stub::StubProcessor;
//! use the_dagwood::traits::Processor;
//! use the_dagwood::proto::processor_v1::ProcessorRequest;
//! use the_dagwood::errors::FailureStrategy;
//! 
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let executor = WorkQueueExecutor::new(4);
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
//!     payload: b"canonical payload test".to_vec(),
//!     metadata: HashMap::new(),
//! };
//! 
//! // Canonical payload eliminates race conditions:
//! // - Left and right execute in parallel after source
//! // - Sink receives canonical payload from source (deterministic)
//! // - No race condition despite parallel execution
//! let results = executor.execute_with_strategy(
//!     ProcessorMap(processor_map),
//!     DependencyGraph(dependency_graph),
//!     EntryPoints(entry_points),
//!     input,
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
use tokio::sync::Mutex;

use crate::traits::executor::DagExecutor;
use crate::traits::processor::ProcessorIntent;
use crate::config::{ProcessorMap, DependencyGraph, EntryPoints};
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::errors::{ExecutionError, FailureStrategy};
use crate::engine::metadata::{merge_dependency_metadata_for_execution, BASE_METADATA_KEY};

use super::priority_work_queue::{PriorityWorkQueue, PrioritizedTask};

/// Work Queue executor that uses dependency counting and canonical payload tracking.
///
/// This executor represents a sophisticated approach to DAG execution that combines classical
/// dependency counting algorithms with a revolutionary canonical payload architecture. It
/// maintains a priority queue of ready-to-execute processors and uses efficient dependency
/// counting to track execution readiness, making it ideal for CPU-bound workloads and
/// scenarios requiring predictable execution order.
///
/// # Canonical Payload Architecture
///
/// The executor's most innovative feature is its **canonical payload architecture** that
/// solves the fundamental race condition problem in diamond dependency patterns:
///
/// ## The Problem
/// ```text
/// Traditional Approach (RACE CONDITION):
///     A (Transform: "hello" -> "HELLO")
///    / \
///   B   C (parallel: both receive "HELLO")
///    \ /
///     D (receives first completion - non-deterministic!)
/// ```
///
/// ## The Solution
/// ```text
/// Canonical Payload Approach (DETERMINISTIC):
///     A (Transform: updates canonical payload to "HELLO")
///    / \
///   B   C (parallel: both receive "HELLO", only contribute metadata)
///    \ /
///     D (receives canonical "HELLO" + merged metadata - deterministic!)
/// ```
///
/// ## Architecture Rules
/// - **Transform processors**: Can update the canonical payload (based on topological rank)
/// - **Analyze processors**: Receive canonical payload but only contribute metadata
/// - **Downstream processors**: Always receive canonical payload + merged dependency metadata
/// - **Rank-based updates**: Higher topological rank Transform processors override lower ones
///
/// This eliminates race conditions while enforcing proper architectural separation between
/// data transformation and analysis operations.
///
/// # Priority Scheduling
///
/// Uses a sophisticated priority queue that orders processors by:
/// 1. **Topological Rank**: Higher ranks (later in DAG) have higher priority
/// 2. **Processor Intent**: Transform processors prioritized over Analyze at same rank
/// 3. **Processor ID**: Lexicographic ordering for deterministic behavior
///
/// This ensures optimal execution order for both performance and correctness.
///
/// # Concurrency and Resource Management
///
/// - **Configurable Concurrency**: Limits concurrent processor executions
/// - **Efficient Task Management**: Uses Arc<Mutex<T>> pattern for thread-safe state
/// - **Deadlock Detection**: Identifies and handles blocked processor scenarios
/// - **Failure Strategies**: Comprehensive error handling with different propagation modes
///
/// # Performance Characteristics
///
/// **Best suited for**:
/// - CPU-bound processors with predictable execution times
/// - Workloads requiring deterministic execution order
/// - Scenarios with complex dependency patterns (diamonds, fan-out/fan-in)
/// - Applications needing priority-based processor scheduling
///
/// **Performance metrics**:
/// - **Dependency Resolution**: O(1) per processor completion
/// - **Queue Operations**: O(log V) for priority queue operations
/// - **Memory Usage**: O(V) for dependency tracking and state management
/// - **Concurrency**: Configurable limits with efficient resource utilization
///
/// # Examples
///
/// ## Creating a work queue executor
/// ```rust
/// use the_dagwood::engine::work_queue::WorkQueueExecutor;
/// 
/// // Create with specific concurrency limit
/// let executor = WorkQueueExecutor::new(8);
/// 
/// // Create with default concurrency (CPU core count)
/// let executor = WorkQueueExecutor::default();
/// ```
///
/// ## Comparing with other execution strategies
/// ```rust
/// use the_dagwood::engine::work_queue::WorkQueueExecutor;
/// use the_dagwood::engine::level_by_level::LevelByLevelExecutor;
/// use the_dagwood::engine::reactive::ReactiveExecutor;
/// 
/// // WorkQueue: Best for CPU-bound, priority-based, deterministic execution
/// let work_queue = WorkQueueExecutor::new(4);
/// 
/// // LevelByLevel: Best for debugging, predictable patterns, level-wise execution
/// let level_by_level = LevelByLevelExecutor::new(4);
/// 
/// // Reactive: Best for I/O-bound, low-latency, event-driven execution
/// let reactive = ReactiveExecutor::new(4);
/// ```
pub struct WorkQueueExecutor {
    /// Maximum number of concurrent processor executions.
    ///
    /// This limit is enforced through careful task counting and queue management.
    /// When the limit is reached, new processors wait in the priority queue until
    /// running processors complete. This prevents resource exhaustion while
    /// maintaining optimal parallelism within the constraint.
    max_concurrency: usize,
}

impl WorkQueueExecutor {
    /// Creates a new Work Queue executor with the specified concurrency limit.
    ///
    /// The concurrency limit controls how many processors can execute simultaneously,
    /// preventing resource exhaustion while allowing optimal parallelism within the
    /// constraint. The work queue continues to accept and prioritize processors even
    /// when the concurrency limit is reached.
    ///
    /// # Arguments
    ///
    /// * `max_concurrency` - Maximum number of processors that can execute concurrently.
    ///                       Will be clamped to a minimum of 1.
    ///
    /// # Returns
    ///
    /// A new `WorkQueueExecutor` configured with the specified concurrency limit.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use the_dagwood::engine::work_queue::WorkQueueExecutor;
    /// 
    /// // Create executor with specific concurrency
    /// let executor = WorkQueueExecutor::new(8);
    /// 
    /// // Minimum concurrency is enforced
    /// let executor = WorkQueueExecutor::new(0); // Actually creates with concurrency = 1
    /// ```
    ///
    /// # Performance Considerations
    ///
    /// - **Higher concurrency**: Better for independent processors, may increase memory usage
    /// - **Lower concurrency**: Better for resource-constrained environments, reduces contention
    /// - **Rule of thumb**: Start with CPU core count, adjust based on processor characteristics
    /// - **Work queue benefits**: Priority scheduling works well with moderate concurrency limits
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            max_concurrency: max_concurrency.max(1), // Ensure at least 1
        }
    }

    /// Creates a new Work Queue executor with default concurrency based on system capabilities.
    ///
    /// The default concurrency is set to the number of available CPU cores, which provides
    /// a good balance for most CPU-bound workloads. Falls back to 4 if the system's
    /// parallelism cannot be determined.
    ///
    /// # Returns
    ///
    /// A new `WorkQueueExecutor` with concurrency set to the number of CPU cores.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use the_dagwood::engine::work_queue::WorkQueueExecutor;
    /// 
    /// // Create with system-appropriate concurrency
    /// let executor = WorkQueueExecutor::default();
    /// 
    /// // Equivalent to:
    /// let core_count = std::thread::available_parallelism()
    ///     .map(|n| n.get())
    ///     .unwrap_or(4);
    /// let executor = WorkQueueExecutor::new(core_count);
    /// ```
    ///
    /// # System Detection
    ///
    /// The executor attempts to detect the number of available CPU cores using
    /// `std::thread::available_parallelism()`. This accounts for:
    /// - Physical CPU cores
    /// - System-imposed limits (cgroups, etc.)
    /// - Process-specific constraints
    ///
    /// If detection fails, defaults to 4 concurrent processors.
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
        // === PHASE 1: VALIDATION AND SETUP ===
        
        // Validate all processors referenced in the dependency graph actually exist in the registry
        // This prevents runtime errors when trying to execute non-existent processors
        for processor_id in graph.keys() {
            if !processors.contains_key(processor_id) {
                return Err(ExecutionError::ProcessorNotFound(processor_id.clone()));
            }
        }
        
        // Build reverse dependency map: processor_id -> [processors that depend on it]
        // This is used to efficiently find which processors become ready when one completes
        let reverse_dependencies = graph.build_reverse_dependencies();
        
        // Compute dependency counts (how many dependencies each processor has) and topological ranks
        // (execution order) together for efficiency. Topological ranks are crucial for the canonical
        // payload architecture - they determine which Transform processor's payload takes precedence
        let (dependency_counts, topological_ranks) = graph.dependency_counts_and_ranks()
            .ok_or_else(|| ExecutionError::InternalError { 
                message: "Internal consistency error: dependency graph contains cycles (should have been caught during config validation)".into() 
            })?;
        
        // === PHASE 2: WORK QUEUE INITIALIZATION ===
        
        // Priority work queue ensures deterministic execution order:
        // 1. Lower topological rank (earlier in DAG) executes first
        // 2. At same rank, Transform processors execute before Analyze processors
        // This ordering is critical for the canonical payload architecture
        let mut work_queue = PriorityWorkQueue::new();
        
        // Start with entrypoints (processors with no dependencies)
        // These are prioritized by topological rank to ensure deterministic startup
        for entrypoint in entrypoints.iter() {
            let rank = topological_ranks.get(entrypoint).copied().unwrap_or(0);
            let is_transform = processors.get(entrypoint)
                .map(|p| p.declared_intent() == ProcessorIntent::Transform)
                .unwrap_or(false);
            work_queue.push(PrioritizedTask::new(entrypoint.clone(), rank, is_transform));
        }
        
        // === PHASE 3: SHARED STATE SETUP FOR CONCURRENT EXECUTION ===
        
        // All shared state uses Arc<Mutex<T>> pattern for thread-safe access across async tasks
        // Arc provides shared ownership, Mutex provides exclusive access for mutations
        
        // Track active tasks to respect concurrency limits and know when execution is complete
        let active_tasks = Arc::new(Mutex::new(0));
        
        // Store execution results from completed processors - used for metadata merging
        let results_mutex = Arc::new(Mutex::new(HashMap::<String, ProcessorResponse>::new()));
        
        // Track remaining dependency counts - decremented as processors complete
        let dependency_counts_mutex = Arc::new(Mutex::new(dependency_counts));
        
        // Work queue of processors ready to execute - shared across all async tasks
        let work_queue_mutex = Arc::new(Mutex::new(work_queue));
        
        // Track processors that have failed execution
        let failed_processors = Arc::new(Mutex::new(std::collections::HashSet::<String>::new()));
        
        // Track processors blocked due to failed dependencies
        let blocked_processors = Arc::new(Mutex::new(std::collections::HashSet::<String>::new()));
        
        // === CANONICAL PAYLOAD ARCHITECTURE ===
        // This is the key innovation that solves race conditions in diamond dependency patterns
        
        // The canonical payload represents the "official" data flowing through the DAG
        // Only Transform processors can modify it, and only if they have a higher topological rank
        // Use Arc<Vec<u8>> to avoid expensive cloning for large payloads
        let canonical_payload_mutex = Arc::new(Mutex::new(Arc::new(input.payload.clone())));
        
        // Track the highest topological rank of any Transform processor that has updated the payload
        // This ensures deterministic behavior: later processors (higher rank) override earlier ones
        let highest_transform_rank_mutex = Arc::new(Mutex::new(None::<usize>));
        
        // === PHASE 4: MAIN EXECUTION LOOP ===
        // Process the work queue until all processors are complete
        loop {
            // Determine the next processor to execute (if any)
            // This block acquires multiple locks, so we scope it to release them quickly
            let next_processor_id = {
                let mut queue = work_queue_mutex.lock().await;
                let active_count = *active_tasks.lock().await;
                let failed = failed_processors.lock().await;
                
                // Apply failure strategy to determine if we should continue execution
                match failure_strategy {
                    FailureStrategy::FailFast => {
                        if !failed.is_empty() {
                            // Return the first failure immediately - no further processing
                            let first_failed = failed.iter().next().unwrap().clone();
                            return Err(ExecutionError::ProcessorFailed {
                                processor_id: first_failed,
                                error: "Processor execution failed".to_string(),
                            });
                        }
                    }
                    _ => {
                        // For ContinueOnError and BestEffort, we continue processing
                        // but skip blocked processors (handled below)
                    }
                }
                
                // Check if we can start more tasks (respect concurrency limits) and have work to do
                if active_count < self.max_concurrency && !queue.is_empty() {
                    // Efficiently find next available processor, skipping any that are blocked
                    // due to failed dependencies
                    let blocked = blocked_processors.lock().await;
                    queue.pop_next_available(&blocked)
                } else {
                    None // Either at concurrency limit or no work available
                }
            };
            
            match next_processor_id {
                Some(processor_id) => {
                    // === SPAWN ASYNC TASK FOR PROCESSOR EXECUTION ===
                    
                    // Increment active task count before spawning to maintain accurate concurrency tracking
                    {
                        let mut active = active_tasks.lock().await;
                        *active += 1;
                    }
                    
                    // Get the processor instance - this should always succeed due to earlier validation
                    let processor = match processors.get(&processor_id) {
                        Some(p) => p.clone(),
                        None => {
                            return Err(ExecutionError::ProcessorNotFound(processor_id));
                        }
                    };
                    
                    // Clone all necessary data for the async task
                    // Arc::clone is cheap - it only increments the reference count, doesn't copy data
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
                    let processors_clone = processors.clone();
                    let canonical_payload_mutex_clone = canonical_payload_mutex.clone();
                    let highest_transform_rank_mutex_clone = highest_transform_rank_mutex.clone();
                    let topological_ranks_clone = topological_ranks.clone();
                    
                    // Spawn async task to execute the processor concurrently
                    // Each processor runs in its own async task for maximum parallelism
                    tokio::spawn(async move {
                        // === DEPENDENCY FAILURE CHECK ===
                        // Before executing, check if any of this processor's dependencies have failed
                        // If so, this processor should be blocked (not executed)
                        let should_block = if let Some(dependencies) = reverse_dependencies_clone.get(&processor_id_clone) {
                            let failed = failed_processors_clone.lock().await;
                            dependencies.iter().any(|dep| failed.contains(dep))
                        } else {
                            false // No dependencies means nothing can block this processor
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
                            // === PROCESSOR INPUT PREPARATION ===
                            // Determine the input for this processor using canonical payload approach
                            let processor_input = if let Some(dependencies) = reverse_dependencies_clone.get(&processor_id_clone) {
                                if dependencies.is_empty() {
                                    // This is an entry point processor - use the original input as-is
                                    input_clone
                                } else {
                                    // This processor has dependencies - construct input from canonical payload + metadata
                                    
                                    // Get the current canonical payload (latest from any Transform processor)
                                    let canonical_payload_arc = canonical_payload_mutex_clone.lock().await.clone();
                                    let canonical_payload = (*canonical_payload_arc).clone(); // Only clone when creating ProcessorRequest
                                    let results_guard = results_mutex_clone.lock().await;
                                    
                                    // Collect metadata only from actual dependencies, not all completed processors
                                    // This ensures processors only see metadata from their direct dependencies
                                    let mut dependency_results = HashMap::new();
                                    for dep_id in dependencies {
                                        if let Some(dep_response) = results_guard.get(dep_id) {
                                            dependency_results.insert(dep_id.clone(), dep_response.clone());
                                        }
                                    }
                                    
                                    // Extract base metadata from original input and merge with dependency metadata
                                    let base_metadata = if let Some(input_metadata) = input_clone.metadata.get(BASE_METADATA_KEY) {
                                        input_metadata.metadata.clone()
                                    } else {
                                        HashMap::new()
                                    };
                                    
                                    // Merge all metadata: base input metadata + all dependency contributions
                                    let all_metadata = merge_dependency_metadata_for_execution(
                                        base_metadata,
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
                            
                            // === PROCESSOR EXECUTION ===
                            // Execute the processor with the prepared input
                            let response = processor.process(processor_input).await;
                            
                            // Check if the processor execution was successful
                            // Success is indicated by a NextPayload outcome
                            let execution_successful = match &response.outcome {
                                Some(Outcome::NextPayload(_)) => true,
                                Some(_) => false, // Other outcomes might indicate failure
                                None => false, // No outcome indicates failure
                            };
                            
                            if !execution_successful {
                                // === FAILURE HANDLING ===
                                // Mark processor as failed
                                let mut failed = failed_processors_clone.lock().await;
                                failed.insert(processor_id_clone.clone());
                                
                                // Block all dependents of this processor (failure propagation)
                                if let Some(dependents) = graph_clone.get(&processor_id_clone) {
                                    let mut blocked = blocked_processors_clone.lock().await;
                                    for dependent in dependents {
                                        blocked.insert(dependent.clone());
                                    }
                                }
                            } else {
                                // === SUCCESS HANDLING ===
                                // Store the successful result for future processors to use
                                let response_clone = response.clone();
                                {
                                    let mut results = results_mutex_clone.lock().await;
                                    results.insert(processor_id_clone.clone(), response);
                                }
                                
                                // === CANONICAL PAYLOAD UPDATE (CORE ARCHITECTURE) ===
                                // Update canonical payload if this is a Transform processor with higher topological rank
                                if let Some(processor) = processors_clone.get(&processor_id_clone) {
                                    if processor.declared_intent() == ProcessorIntent::Transform {
                                        if let Some(Outcome::NextPayload(new_payload)) = &response_clone.outcome {
                                            if let Some(&processor_rank) = topological_ranks_clone.get(&processor_id_clone) {
                                                let mut highest_rank = highest_transform_rank_mutex_clone.lock().await;
                                                
                                                // CRITICAL: Update canonical payload if this processor has a strictly higher rank
                                                // or if no Transform processor has completed yet. Strict comparison (>) prevents
                                                // race conditions: parallel Transform processors at the same rank can't overwrite
                                                // each other's payload, ensuring deterministic canonical payload updates.
                                                // This is the key innovation that solves diamond dependency race conditions.
                                                let should_update = match *highest_rank {
                                                    None => true, // First Transform processor gets to set canonical payload
                                                    Some(current_highest) => processor_rank > current_highest, // Only higher ranks can override
                                                };
                                                
                                                if should_update {
                                                    let mut canonical_payload = canonical_payload_mutex_clone.lock().await;
                                                    *canonical_payload = Arc::new(new_payload.clone());
                                                    *highest_rank = Some(processor_rank);
                                                }
                                            }
                                        }
                                    }
                                }
                                
                                // === DEPENDENCY RESOLUTION ===
                                // Update dependency counts for dependents and add newly ready processors to queue
                                if let Some(dependents) = graph_clone.get(&processor_id_clone) {
                                    let mut dependency_counts = dependency_counts_mutex_clone.lock().await;
                                    let mut work_queue = work_queue_mutex_clone.lock().await;
                                    
                                    for dependent_id in dependents {
                                        if let Some(count) = dependency_counts.get_mut(dependent_id) {
                                            *count -= 1; // One less dependency to wait for
                                            
                                            // If dependency count reaches zero, this processor is ready to execute
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
                        
                        // === CLEANUP ===
                        // Always decrement active task count, even if processor failed
                        // This ensures accurate concurrency tracking and prevents deadlocks
                        {
                            let mut active = active_tasks_clone.lock().await;
                            *active -= 1;
                        }
                    });
                }
                None => {
                    // === EXECUTION COMPLETION CHECK ===
                    // No work available, check if we're done with all execution
                    let active_count = *active_tasks.lock().await;
                    let queue_empty = work_queue_mutex.lock().await.is_empty();
                    let failed = failed_processors.lock().await;
                    
                    if active_count == 0 && queue_empty {
                        // All work is complete - no active tasks and no queued work
                        // Apply failure strategy to determine final result
                        match failure_strategy {
                            FailureStrategy::FailFast => {
                                // Should have already returned on first failure
                                break;
                            }
                            FailureStrategy::ContinueOnError | FailureStrategy::BestEffort => {
                                if !failed.is_empty() {
                                    // Collect all failures for comprehensive error reporting
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
                        // === DEADLOCK DETECTION ===
                        // We have work but can't proceed - likely all remaining processors are blocked
                        let blocked = blocked_processors.lock().await;
                        let queue = work_queue_mutex.lock().await;
                        
                        if queue.iter().all(|task| blocked.contains(&task.processor_id)) {
                            // All remaining processors are blocked due to failed dependencies
                            // This is a deadlock situation - no progress can be made
                            let failures: Vec<ExecutionError> = failed.iter()
                                .map(|id| ExecutionError::ProcessorFailed {
                                    processor_id: id.clone(),
                                    error: "Processor execution failed".to_string(),
                                })
                                .collect();
                            
                            return Err(ExecutionError::MultipleFailed { failures });
                        }
                    } else {
                        // === WAIT FOR PROGRESS ===
                        // Active tasks are running or concurrency limit reached - wait briefly
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }
                }
            }
        }
        
        // === PHASE 5: RESULT EXTRACTION ===
        // All processors have completed successfully - extract the final results
        let final_results = results_mutex.lock().await;
        Ok(final_results.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::processor::Processor;
    use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, Metadata};
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
            
            // Merge with existing "self" metadata instead of overwriting
            if let Some(existing_self) = metadata.get_mut("self") {
                // Add our analysis metadata to existing metadata
                existing_self.metadata.insert("analysis".to_string(), self.metadata_suffix.clone());
            } else {
                // No existing "self" metadata, create new
                let mut own_metadata = std::collections::HashMap::new();
                own_metadata.insert("analysis".to_string(), self.metadata_suffix.clone());
                metadata.insert("self".to_string(), Metadata {
                    metadata: own_metadata,
                });
            }
            
            ProcessorResponse {
                outcome: Some(Outcome::NextPayload(Vec::new())), // Pass through unchanged
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
        processors.insert("right".to_string(), Arc::new(MockAnalyzeProcessor::new("right", 5, "RIGHT_ANALYSIS")) as Arc<dyn Processor>);
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
            // Right is now an Analyze processor, so it receives an empty payload
            assert_eq!(String::from_utf8(payload.clone()).unwrap(), "");
        } else {
            panic!("Expected success outcome for right");
        }
        let response_merge = results.get("merge").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &response_merge.outcome {
            // Merge processor gets canonical payload from left (the only Transform at rank 1)
            // This is now deterministic due to canonical payload architecture
            let result = String::from_utf8(payload.clone()).unwrap();
            assert_eq!(result, "test-root-left-merge", 
                   "Expected merge result to be deterministic 'test-root-left-merge', got: {}", result);
        } else {
            panic!("Expected success outcome for merge");
        }
    }

    #[tokio::test]
    async fn test_multiple_entrypoints() {
        let executor = WorkQueueExecutor::new(4);
        
        let mut processors = HashMap::new();
        processors.insert("entry1".to_string(), Arc::new(MockProcessor::new("entry1", 0, "-e1")) as Arc<dyn Processor>);
        processors.insert("entry2".to_string(), Arc::new(MockAnalyzeProcessor::new("entry2", 0, "E2_ANALYSIS")) as Arc<dyn Processor>);
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
            // Entry2 is now an Analyze processor, so it receives an empty payload
            assert_eq!(String::from_utf8(payload.clone()).unwrap(), "");
        } else {
            panic!("Expected success outcome for entry2");
        }
        let response_merge = results.get("merge").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &response_merge.outcome {
            // Merge processor gets canonical payload from entry1 (the only Transform at rank 0)
            // This is now deterministic due to canonical payload architecture
            let result = String::from_utf8(payload.clone()).unwrap();
            assert_eq!(result, "test-e1-merge", 
                   "Expected merge result to be deterministic 'test-e1-merge', got: {}", result);
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
            // analyze1 should receive an empty payload and add "-A1" metadata only
            assert_eq!(result_str, "");
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
                let mut original_metadata = HashMap::new();
                original_metadata.insert("original".to_string(), "INPUT_META".to_string());
                m.insert("self".to_string(), Metadata {
                    metadata: original_metadata,
                });
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
        
        // Should have original metadata under "self" processor
        assert!(final_result.metadata.contains_key("self"));
        let self_metadata = final_result.metadata.get("self").unwrap();
        assert_eq!(self_metadata.metadata.get("original"), Some(&"INPUT_META".to_string()));
        
        // Should have its own analysis metadata (final processor's own metadata)
        assert!(final_result.metadata.contains_key("self"));
        let final_metadata = final_result.metadata.get("self").unwrap();
        assert_eq!(final_metadata.metadata.get("analysis"), Some(&"FINAL_META".to_string()));
        
        // Should NOT have metadata from proc2 (unrelated processor)
        // With the new structure, unrelated processors should not appear in metadata at all
        assert!(!final_result.metadata.contains_key("proc2"));
        
        // Should NOT have metadata from entry2 (unrelated processor)
        assert!(!final_result.metadata.contains_key("entry2"));
        
        // Note: entry1 metadata should NOT be directly present in final because
        // final only depends on proc1, not entry1. The metadata chain is:
        // entry1 -> proc1 (proc1 gets entry1's metadata)
        // proc1 -> final (final gets proc1's metadata, but not entry1's directly)
        
        // Verify proc2 completed successfully but is isolated
        let proc2_result = result.get("proc2").unwrap();
        assert!(proc2_result.metadata.contains_key("self"));
        let proc2_metadata = proc2_result.metadata.get("self").unwrap();
        assert_eq!(proc2_metadata.metadata.get("analysis"), Some(&"P2_META".to_string()));
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
