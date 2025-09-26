use std::collections::{BinaryHeap, HashSet};
use std::cmp::Ordering;

/// Prioritized task for the work queue, ordered by topological rank and processor intent
#[derive(Debug, Clone)]
pub struct PrioritizedTask {
    pub processor_id: String,
    pub topological_rank: usize,
    pub is_transform: bool,
}

impl PrioritizedTask {
    pub fn new(processor_id: String, topological_rank: usize, is_transform: bool) -> Self {
        Self {
            processor_id,
            topological_rank,
            is_transform,
        }
    }
}

impl PartialEq for PrioritizedTask {
    fn eq(&self, other: &Self) -> bool {
        self.processor_id == other.processor_id
    }
}

impl Eq for PrioritizedTask {}

impl PartialOrd for PrioritizedTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedTask {
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
}

impl PriorityWorkQueue {
    /// Create a new empty priority work queue
    pub fn new() -> Self {
        Self { 
            heap: BinaryHeap::new() 
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
    /// This method implements a two-phase approach:
    /// 1. **Fast Path**: Check if the highest priority task is available (O(1))
    /// 2. **Slow Path**: Search through blocked tasks and restore them efficiently (O(n))
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
        // Fast path: check if top task is available (O(1) when no blocking)
        if let Some(top_task) = self.heap.peek() {
            if !blocked.contains(&top_task.processor_id) {
                return Some(self.heap.pop().unwrap().processor_id);
            }
        }
        
        // Slow path: find first non-blocked task (O(n) when blocking occurs)
        let mut temp_tasks = Vec::new();
        let mut result = None;
        
        while let Some(task) = self.heap.pop() {
            if !blocked.contains(&task.processor_id) {
                result = Some(task.processor_id);
                break;
            } else {
                temp_tasks.push(task);
            }
        }
        
        // Batch restore blocked tasks to minimize heap operations
        if !temp_tasks.is_empty() {
            self.heap.extend(temp_tasks);
        }
        
        result
    }
    
    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
    
    /// Peek at the highest priority task without removing it
    pub fn peek(&self) -> Option<&PrioritizedTask> {
        self.heap.peek()
    }
    
    /// Get the number of tasks in the queue
    pub fn len(&self) -> usize {
        self.heap.len()
    }
    
    /// Iterate over all tasks in the queue (for checking blocked status)
    pub fn iter(&self) -> std::collections::binary_heap::Iter<PrioritizedTask> {
        self.heap.iter()
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
}
