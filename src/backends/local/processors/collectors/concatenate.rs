use async_trait::async_trait;
use std::collections::HashMap;
use crate::traits::Processor;
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, ErrorDetail};
use crate::proto::processor_v1::processor_response::Outcome;
use super::{ResultCollector, CollectableResult};

/// ConcatenateCollector combines all dependency outputs into a single payload
/// 
/// This collector concatenates the payloads from all successful dependencies
/// using a configurable separator. Results are ordered deterministically
/// by dependency ID to ensure consistent output.
pub struct ConcatenateCollector {
    separator: String,
}

impl ConcatenateCollector {
    pub fn new(separator: Option<String>) -> Self {
        Self {
            separator: separator.unwrap_or_default(),
        }
    }

    /// Helper to create error responses
    fn error_response(&self, message: &str) -> ProcessorResponse {
        ProcessorResponse {
            outcome: Some(Outcome::Error(ErrorDetail {
                code: 500,
                message: message.to_string(),
            })),
        }
    }
}

#[async_trait]
impl ResultCollector for ConcatenateCollector {
    async fn collect_results(
        &self,
        dependency_results: &HashMap<String, CollectableResult>,
        _request: &ProcessorRequest,
    ) -> ProcessorResponse {
        let mut combined_parts = Vec::new();

        // Sort by dependency ID for deterministic ordering
        let mut sorted_deps: Vec<_> = dependency_results.iter().collect();
        sorted_deps.sort_by_key(|(id, _)| *id);

        for (_, result) in sorted_deps {
            if result.success {
                if let Some(payload) = &result.payload {
                    if let Ok(payload_str) = String::from_utf8(payload.clone()) {
                        combined_parts.push(payload_str);
                    }
                }
            }
        }

        if combined_parts.is_empty() {
            return self.error_response("No successful dependency results to concatenate");
        }

        let combined = combined_parts.join(&self.separator);
        ProcessorResponse {
            outcome: Some(Outcome::NextPayload(combined.into_bytes())),
        }
    }
}

#[async_trait]
impl Processor for ConcatenateCollector {
    async fn process(&self, request: ProcessorRequest) -> ProcessorResponse {
        // The input payload should contain serialized dependency results
        match serde_json::from_slice::<HashMap<String, CollectableResult>>(&request.payload) {
            Ok(dependency_results) => self.collect_results(&dependency_results, &request).await,
            Err(e) => self.error_response(&format!("Failed to deserialize dependency results: {}", e)),
        }
    }

    fn name(&self) -> &'static str {
        "concatenate_collector"
    }
}
