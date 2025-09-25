pub mod first_available;
pub mod metadata_merge;
pub mod concatenate;
pub mod json_merge;
pub mod custom;

#[cfg(test)]
mod tests;

pub use first_available::FirstAvailableCollector;
pub use metadata_merge::MetadataMergeCollector;
pub use concatenate::ConcatenateCollector;
pub use json_merge::JsonMergeCollector;
pub use custom::CustomCollector;

use async_trait::async_trait;
use std::collections::HashMap;
use crate::traits::Processor;
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse};
use serde::{Serialize, Deserialize};

/// Serializable representation of a processor result for collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectableResult {
    pub success: bool,
    pub payload: Option<Vec<u8>>,
    pub error_code: Option<i32>,
    pub error_message: Option<String>,
}

/// Base trait for all result collectors
/// 
/// Result collectors are specialized processors that combine outputs from multiple
/// dependencies using different strategies. This trait extends the base Processor
/// trait with collection-specific functionality.
#[async_trait]
pub trait ResultCollector: Processor {
    /// Collect multiple processor results into a single response
    /// 
    /// This method takes a map of dependency results and combines them according
    /// to the collector's specific strategy.
    async fn collect_results(
        &self,
        dependency_results: &HashMap<String, CollectableResult>,
        request: &ProcessorRequest,
    ) -> ProcessorResponse;
}
