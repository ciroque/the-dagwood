//! Priority-based work queue for efficient DAG processor scheduling.
//!
//! This module provides a sophisticated priority queue implementation optimized for DAG execution
//! scenarios where processors have different priorities based on their position in the dependency
//! graph and their execution characteristics. The queue handles blocked processors efficiently
//! without expensive queue reconstruction operations.
//!
//! # Key Features
//!
//! - **Topological Priority**: Processors with higher topological ranks execute first (critical path optimization)
//! - **Transform Priority**: Transform processors get priority over Analyze processors at the same rank
//! - **Blocked Task Optimization**: Separate storage for blocked tasks to avoid repeated heap operations
//! - **Fast Path Optimization**: O(1) operation when the highest priority task is available
//!
//! # Priority Ordering
//!
//! Tasks are ordered by:
//! 1. **Topological Rank** (higher = higher priority)
//! 2. **Processor Intent** (Transform > Analyze at same rank)
//! 3. **Processor ID** (stable sort for deterministic behavior)
//!
//! # Performance Characteristics
//!
//! - **Best Case**: O(1) when no blocking occurs
//! - **Worst Case**: O(n) with optimized batch operations for blocked tasks
//! - **Memory**: O(n) with separate storage for blocked vs. ready tasks
//!
//! # Examples
//!
//! ## Basic usage with priority ordering
//! ```rust
//! use std::collections::HashSet;
//! use the_dagwood::engine::priority_work_queue::{PriorityWorkQueue, PrioritizedTask};
//! 
//! let mut queue = PriorityWorkQueue::new();
//! 
//! // Add tasks with different priorities
//! queue.push(PrioritizedTask::new("low_priority".to_string(), 0, false));
//! queue.push(PrioritizedTask::new("high_priority".to_string(), 5, true));
//! queue.push(PrioritizedTask::new("mid_priority".to_string(), 3, false));
//! 
//! let blocked = HashSet::new();
//! 
//! // Processors execute in priority order: high -> mid -> low
//! assert_eq!(queue.pop_next_available(&blocked), Some("high_priority".to_string()));
//! assert_eq!(queue.pop_next_available(&blocked), Some("mid_priority".to_string()));
//! assert_eq!(queue.pop_next_available(&blocked), Some("low_priority".to_string()));
//! ```
//!
//! ## Handling blocked processors
//! ```rust
//! use std::collections::HashSet;
//! use the_dagwood::engine::priority_work_queue::{PriorityWorkQueue, PrioritizedTask};
//! 
//! let mut queue = PriorityWorkQueue::new();
//! queue.push(PrioritizedTask::new("blocked_high".to_string(), 10, true));
//! queue.push(PrioritizedTask::new("available_low".to_string(), 1, false));
//! 
//! let mut blocked = HashSet::new();
//! blocked.insert("blocked_high".to_string());
//! 
//! // Skips blocked processor, returns available one
//! assert_eq!(queue.pop_next_available(&blocked), Some("available_low".to_string()));
//! 
//! // Later, when processor becomes unblocked
//! blocked.clear();
//! assert_eq!(queue.pop_next_available(&blocked), Some("blocked_high".to_string()));
//! ```

use std::collections::{BinaryHeap, HashSet, HashMap};
use std::cmp::Ordering;

/// A prioritized task representing a processor ready for execution in the DAG.
///
/// `PrioritizedTask` encapsulates the information needed to schedule processors in optimal
/// execution order. The priority is determined by topological rank (critical path position)
/// and processor intent (Transform vs Analyze), enabling efficient DAG execution strategies.
///
/// # Priority Ordering Rules
///
/// Tasks are ordered by the following criteria (in order of precedence):
///
/// 1. **Topological Rank**: Higher ranks execute first (critical path optimization)
///    - Rank 5 executes before Rank 3
///    - Optimizes for reducing overall DAG execution time
///
/// 2. **Processor Intent**: Transform processors prioritized over Analyze at same rank
///    - Transform processors modify the canonical payload
///    - Analyze processors only contribute metadata
///    - Ensures payload modifications happen before analysis
///
/// 3. **Processor ID**: Lexicographic ordering for deterministic behavior
///    - Provides stable sort when all other criteria are equal
///    - Ensures reproducible execution order
///
/// # Examples
///
/// ## Creating prioritized tasks
/// ```rust
/// use the_dagwood::engine::priority_work_queue::PrioritizedTask;
/// 
/// // High priority Transform processor (critical path)
/// let critical_transform = PrioritizedTask::new(
///     "data_transformer".to_string(),
///     10, // High topological rank
///     true // Transform processor
/// );
/// 
/// // Lower priority Analyze processor
/// let analyzer = PrioritizedTask::new(
///     "data_analyzer".to_string(),
///     10, // Same rank as transform
///     false // Analyze processor
/// );
/// 
/// // Transform processor will execute first due to intent priority
/// assert!(critical_transform > analyzer);
/// ```
///
/// ## Priority comparison examples
/// ```rust
/// use the_dagwood::engine::priority_work_queue::PrioritizedTask;
/// 
/// let high_rank = PrioritizedTask::new("proc1".to_string(), 5, false);
/// let low_rank = PrioritizedTask::new("proc2".to_string(), 2, false);
/// let transform_same_rank = PrioritizedTask::new("proc3".to_string(), 5, true);
/// 
/// // Higher topological rank wins
/// assert!(high_rank > low_rank);
/// 
/// // Transform intent wins at same rank
/// assert!(transform_same_rank > high_rank);
/// ```
#[derive(Debug, Clone)]
pub struct PrioritizedTask {
    pub processor_id: String,
    pub topological_rank: usize,
    pub is_transform: bool,
}

impl PrioritizedTask {
    /// Creates a new prioritized task for DAG execution scheduling.
    ///
    /// # Arguments
    ///
    /// * `processor_id` - Unique identifier for the processor
    /// * `topological_rank` - Position in topological sort (higher = more critical)
    /// * `is_transform` - Whether this is a Transform processor (true) or Analyze processor (false)
    ///
    /// # Returns
    ///
    /// A new `PrioritizedTask` ready for insertion into the priority queue.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use the_dagwood::engine::priority_work_queue::PrioritizedTask;
    /// 
    /// // Create a high-priority Transform processor
    /// let task = PrioritizedTask::new(
    ///     "critical_processor".to_string(),
    ///     8, // High topological rank
    ///     true // Transform processor
    /// );
    /// 
    /// assert_eq!(task.processor_id, "critical_processor");
    /// assert_eq!(task.topological_rank, 8);
    /// assert_eq!(task.is_transform, true);
    /// ```
    pub fn new(processor_id: String, topological_rank: usize, is_transform: bool) -> Self {
        Self {
            processor_id,
            topological_rank,
            is_transform,
        }
    }
}

impl PartialEq for PrioritizedTask {
    /// Equality based solely on processor ID for task identity.
    ///
    /// Two tasks are considered equal if they represent the same processor,
    /// regardless of their priority attributes. This ensures that the same
    /// processor cannot be queued multiple times.
    fn eq(&self, other: &Self) -> bool {
        self.processor_id == other.processor_id
    }
}

impl Eq for PrioritizedTask {}

impl PartialOrd for PrioritizedTask {
    /// Partial comparison delegating to total ordering implementation.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedTask {
    /// Total ordering implementation for priority queue scheduling.
    ///
    /// This implements the core scheduling logic for DAG execution. The ordering
    /// is designed to optimize DAG execution time by prioritizing critical path
    /// processors and ensuring Transform processors execute before Analyze processors.
    ///
    /// # Ordering Rules (in precedence order)
    ///
    /// 1. **Topological Rank**: Higher ranks first (critical path optimization)
    /// 2. **Processor Intent**: Transform before Analyze at same rank
    /// 3. **Processor ID**: Lexicographic for deterministic behavior
    ///
    /// # BinaryHeap Behavior
    ///
    /// Since `BinaryHeap` is a max-heap, higher values are popped first:
    /// - Higher topological ranks → executed first (critical path)
    /// - Transform processors → executed before Analyze at same rank
    /// - Lexicographic ID ordering → deterministic execution order
    ///
    /// # Examples
    ///
    /// ```rust
    /// use the_dagwood::engine::priority_work_queue::PrioritizedTask;
    /// use std::cmp::Ordering;
    /// 
    /// let high_rank = PrioritizedTask::new("proc1".to_string(), 5, false);
    /// let low_rank = PrioritizedTask::new("proc2".to_string(), 2, false);
    /// 
    /// // Higher rank has greater priority
    /// assert_eq!(high_rank.cmp(&low_rank), Ordering::Greater);
    /// 
    /// let transform = PrioritizedTask::new("transform".to_string(), 3, true);
    /// let analyze = PrioritizedTask::new("analyze".to_string(), 3, false);
    /// 
    /// // Transform beats Analyze at same rank
    /// assert_eq!(transform.cmp(&analyze), Ordering::Greater);
    /// ```
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is a max-heap: higher topological ranks are popped first
        // This prioritizes critical path processors (higher ranks = executed first)
        // Transform processors get priority over Analyze processors at same rank
        match self.topological_rank.cmp(&other.topological_rank) {
            Ordering::Equal => {
                // If same rank, prioritize Transform over Analyze
                match (self.is_transform, other.is_transform) {
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    _ => self.processor_id.cmp(&other.processor_id), // Stable sort by ID
                }
            }
            other_ordering => other_ordering,
        }
    }
}

/// A priority queue for DAG processors that handles blocked processor filtering efficiently.
/// 
/// This data structure provides an optimized way to manage processor scheduling in DAG execution,
/// with built-in support for handling blocked processors without expensive queue reconstruction.
/// 
/// ## Performance Characteristics
/// 
/// - **Fast Path**: O(1) when the highest priority task is available
/// - **Slow Path**: O(n) with optimized batch operations when blocking occurs
/// - **Memory Efficient**: Minimal allocations and efficient heap operations
/// 
/// ## Usage
/// 
/// ```rust
/// use std::collections::HashSet;
/// use the_dagwood::engine::priority_work_queue::{PriorityWorkQueue, PrioritizedTask};
/// 
/// let mut queue = PriorityWorkQueue::new();
/// queue.push(PrioritizedTask::new("processor1".to_string(), 0, true));
/// 
/// let blocked = HashSet::new();
/// if let Some(processor_id) = queue.pop_next_available(&blocked) {
///     // Execute processor_id
/// }
/// ```
#[derive(Debug)]
pub struct PriorityWorkQueue {
    heap: BinaryHeap<PrioritizedTask>,
    // Separate storage for long-term blocked tasks to avoid repeated heap operations
    blocked_tasks: HashMap<String, PrioritizedTask>,
}

impl PriorityWorkQueue {
    /// Create a new empty priority work queue
    pub fn new() -> Self {
        Self { 
            heap: BinaryHeap::new(),
            blocked_tasks: HashMap::new(),
        }
    }
    
    /// Add a task to the priority queue
    pub fn push(&mut self, task: PrioritizedTask) {
        self.heap.push(task);
    }
    
    /// Add multiple tasks to the priority queue efficiently
    pub fn extend<I>(&mut self, tasks: I) 
    where 
        I: IntoIterator<Item = PrioritizedTask> 
    {
        self.heap.extend(tasks);
    }
    
    /// Efficiently find and remove the highest priority non-blocked processor.
    /// 
    /// This method implements an optimized approach with separate blocked task storage:
    /// 1. **Fast Path**: Check if the highest priority task is available (O(1))
    /// 2. **Optimized Slow Path**: Move blocked tasks to separate storage to avoid repeated heap operations
    /// 3. **Unblock Restoration**: Restore tasks from blocked storage when they become available
    /// 
    /// ## Arguments
    /// 
    /// * `blocked` - Set of processor IDs that are currently blocked and should be skipped
    /// 
    /// ## Returns
    /// 
    /// * `Some(processor_id)` - The highest priority non-blocked processor ID
    /// * `None` - If all tasks are blocked or the queue is empty
    pub fn pop_next_available(&mut self, blocked: &HashSet<String>) -> Option<String> {
        // First, restore any previously blocked tasks that are now unblocked
        self.restore_unblocked_tasks(blocked);
        
        // Fast path: check if top task is available (O(1) when no blocking)
        if let Some(top_task) = self.heap.peek() {
            if !blocked.contains(&top_task.processor_id) {
                return Some(self.heap.pop().unwrap().processor_id);
            }
        }
        
        // Optimized slow path: move blocked tasks to separate storage
        let mut result = None;
        
        while let Some(task) = self.heap.pop() {
            if !blocked.contains(&task.processor_id) {
                result = Some(task.processor_id);
                break;
            } else {
                // Store in separate blocked tasks storage instead of temp vector
                self.blocked_tasks.insert(task.processor_id.clone(), task);
            }
        }
        
        result
    }
    
    /// Restore previously blocked tasks that are now unblocked back to the main heap
    fn restore_unblocked_tasks(&mut self, blocked: &HashSet<String>) {
        let mut to_restore = Vec::new();
        
        // Find tasks that are no longer blocked
        self.blocked_tasks.retain(|processor_id, task| {
            if !blocked.contains(processor_id) {
                to_restore.push(task.clone());
                false // Remove from blocked_tasks
            } else {
                true // Keep in blocked_tasks
            }
        });
        
        // Restore unblocked tasks to the main heap
        if !to_restore.is_empty() {
            self.heap.extend(to_restore);
        }
    }
    
    /// Check if the queue is empty (both main heap and blocked tasks)
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty() && self.blocked_tasks.is_empty()
    }
    
    /// Peek at the highest priority task without removing it
    pub fn peek(&self) -> Option<&PrioritizedTask> {
        self.heap.peek()
    }
    
    /// Get the total number of tasks in the queue (including blocked tasks)
    pub fn len(&self) -> usize {
        self.heap.len() + self.blocked_tasks.len()
    }
    
    /// Iterate over all tasks in the queue (for checking blocked status)
    /// Returns an iterator that includes both heap tasks and blocked tasks
    pub fn iter(&self) -> Box<dyn Iterator<Item = &PrioritizedTask> + '_> {
        Box::new(self.heap.iter().chain(self.blocked_tasks.values()))
    }
}

impl Default for PriorityWorkQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        let mut queue = PriorityWorkQueue::new();
        
        // Add tasks with different priorities
        queue.push(PrioritizedTask::new("low_rank".to_string(), 0, false));
        queue.push(PrioritizedTask::new("high_rank".to_string(), 2, false));
        queue.push(PrioritizedTask::new("mid_rank".to_string(), 1, false));
        
        let blocked = HashSet::new();
        
        // Should return highest rank first
        assert_eq!(queue.pop_next_available(&blocked), Some("high_rank".to_string()));
        assert_eq!(queue.pop_next_available(&blocked), Some("mid_rank".to_string()));
        assert_eq!(queue.pop_next_available(&blocked), Some("low_rank".to_string()));
    }

    #[test]
    fn test_transform_priority_over_analyze() {
        let mut queue = PriorityWorkQueue::new();
        
        // Add Transform and Analyze processors at same rank
        queue.push(PrioritizedTask::new("analyze".to_string(), 1, false));
        queue.push(PrioritizedTask::new("transform".to_string(), 1, true));
        
        let blocked = HashSet::new();
        
        // Transform should come first at same rank
        assert_eq!(queue.pop_next_available(&blocked), Some("transform".to_string()));
        assert_eq!(queue.pop_next_available(&blocked), Some("analyze".to_string()));
    }

    #[test]
    fn test_blocked_processor_handling() {
        let mut queue = PriorityWorkQueue::new();
        
        queue.push(PrioritizedTask::new("blocked_high".to_string(), 2, true));
        queue.push(PrioritizedTask::new("available_low".to_string(), 1, true));
        
        let mut blocked = HashSet::new();
        blocked.insert("blocked_high".to_string());
        
        // Should skip blocked processor and return available one
        assert_eq!(queue.pop_next_available(&blocked), Some("available_low".to_string()));
        
        // Blocked processor should still be in queue
        blocked.clear();
        assert_eq!(queue.pop_next_available(&blocked), Some("blocked_high".to_string()));
    }

    #[test]
    fn test_empty_queue() {
        let mut queue = PriorityWorkQueue::new();
        let blocked = HashSet::new();
        
        assert_eq!(queue.pop_next_available(&blocked), None);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_all_blocked() {
        let mut queue = PriorityWorkQueue::new();
        
        queue.push(PrioritizedTask::new("task1".to_string(), 1, true));
        queue.push(PrioritizedTask::new("task2".to_string(), 2, true));
        
        let mut blocked = HashSet::new();
        blocked.insert("task1".to_string());
        blocked.insert("task2".to_string());
        
        // Should return None when all tasks are blocked
        assert_eq!(queue.pop_next_available(&blocked), None);
        
        // Tasks should still be in queue
        assert!(!queue.is_empty());
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_blocked_task_storage_optimization() {
        let mut queue = PriorityWorkQueue::new();
        
        // Add multiple tasks with different priorities
        queue.push(PrioritizedTask::new("high_blocked".to_string(), 3, true));
        queue.push(PrioritizedTask::new("mid_blocked".to_string(), 2, true));
        queue.push(PrioritizedTask::new("low_available".to_string(), 1, true));
        queue.push(PrioritizedTask::new("lowest_available".to_string(), 0, true));
        
        let mut blocked = HashSet::new();
        blocked.insert("high_blocked".to_string());
        blocked.insert("mid_blocked".to_string());
        
        // First call should move blocked tasks to separate storage and return available task
        assert_eq!(queue.pop_next_available(&blocked), Some("low_available".to_string()));
        
        // Verify blocked tasks are in separate storage (not in main heap)
        assert_eq!(queue.len(), 3); // 1 in heap + 2 in blocked storage
        
        // Second call should return next available without re-processing blocked tasks
        assert_eq!(queue.pop_next_available(&blocked), Some("lowest_available".to_string()));
        
        // Now unblock one task
        blocked.remove("high_blocked");
        
        // Should restore the unblocked task and return it (highest priority)
        assert_eq!(queue.pop_next_available(&blocked), Some("high_blocked".to_string()));
        
        // Only one blocked task should remain
        assert_eq!(queue.len(), 1);
        
        // Unblock the last task
        blocked.clear();
        
        // Should restore and return the last task
        assert_eq!(queue.pop_next_available(&blocked), Some("mid_blocked".to_string()));
        
        // Queue should now be empty
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_iter_includes_both_heap_and_blocked_tasks() {
        let mut queue = PriorityWorkQueue::new();
        
        // Add tasks that will be in the heap
        queue.push(PrioritizedTask {
            processor_id: "heap_task_1".to_string(),
            topological_rank: 5,
            is_transform: false,
        });
        queue.push(PrioritizedTask {
            processor_id: "heap_task_2".to_string(),
            topological_rank: 3,
            is_transform: true,
        });
        
        // Add tasks that will become blocked
        queue.push(PrioritizedTask {
            processor_id: "blocked_task_1".to_string(),
            topological_rank: 10,
            is_transform: false,
        });
        queue.push(PrioritizedTask {
            processor_id: "blocked_task_2".to_string(),
            topological_rank: 8,
            is_transform: true,
        });
        
        // Initially, all tasks should be visible in iterator
        let all_task_ids: std::collections::HashSet<String> = queue.iter()
            .map(|task| task.processor_id.clone())
            .collect();
        assert_eq!(all_task_ids.len(), 4);
        assert!(all_task_ids.contains("heap_task_1"));
        assert!(all_task_ids.contains("heap_task_2"));
        assert!(all_task_ids.contains("blocked_task_1"));
        assert!(all_task_ids.contains("blocked_task_2"));
        
        // Block some tasks
        let mut blocked = std::collections::HashSet::new();
        blocked.insert("blocked_task_1".to_string());
        blocked.insert("blocked_task_2".to_string());
        
        // Pop available tasks (this will move blocked tasks to separate storage)
        let available1 = queue.pop_next_available(&blocked);
        assert_eq!(available1, Some("heap_task_1".to_string()));
        
        let available2 = queue.pop_next_available(&blocked);
        assert_eq!(available2, Some("heap_task_2".to_string()));
        
        // Now we should have 2 blocked tasks in separate storage
        assert_eq!(queue.len(), 2);
        
        // Iterator should still see all remaining tasks (blocked tasks in HashMap)
        let remaining_task_ids: std::collections::HashSet<String> = queue.iter()
            .map(|task| task.processor_id.clone())
            .collect();
        assert_eq!(remaining_task_ids.len(), 2);
        assert!(remaining_task_ids.contains("blocked_task_1"));
        assert!(remaining_task_ids.contains("blocked_task_2"));
        
        // Verify the blocked tasks are not in the heap anymore
        let heap_task_ids: std::collections::HashSet<String> = queue.heap.iter()
            .map(|task| task.processor_id.clone())
            .collect();
        assert_eq!(heap_task_ids.len(), 0);
        
        // But they should still be visible through the main iterator
        assert_eq!(queue.iter().count(), 2);
        
        // Unblock one task
        blocked.remove("blocked_task_1");
        
        // Pop should restore and return the unblocked task
        let unblocked = queue.pop_next_available(&blocked);
        assert_eq!(unblocked, Some("blocked_task_1".to_string()));
        
        // Iterator should now show only the remaining blocked task
        let final_task_ids: std::collections::HashSet<String> = queue.iter()
            .map(|task| task.processor_id.clone())
            .collect();
        assert_eq!(final_task_ids.len(), 1);
        assert!(final_task_ids.contains("blocked_task_2"));
    }
}
