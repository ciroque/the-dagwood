use async_trait::async_trait;
use std::collections::HashMap;
use serde_json;
use crate::traits::Processor;
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, ErrorDetail};
use crate::proto::processor_v1::processor_response::Outcome;
use super::{ResultCollector, CollectableResult};

/// MetadataMergeCollector combines a primary payload with metadata from secondary sources
/// 
/// This collector takes one dependency as the primary source (its payload becomes the output)
/// and treats other dependencies as metadata sources (their payloads become metadata).
/// This is useful for enriching a main data stream with analysis results.
pub struct MetadataMergeCollector {
    primary_source: String,
    metadata_sources: Vec<String>,
}

impl MetadataMergeCollector {
    pub fn new(primary_source: String, metadata_sources: Vec<String>) -> Self {
        Self {
            primary_source,
            metadata_sources,
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
impl ResultCollector for MetadataMergeCollector {
    async fn collect_results(
        &self,
        dependency_results: &HashMap<String, CollectableResult>,
        _request: &ProcessorRequest,
    ) -> ProcessorResponse {
        // Get primary payload
        let primary_payload = if let Some(primary_result) = dependency_results.get(&self.primary_source) {
            if primary_result.success {
                if let Some(payload) = &primary_result.payload {
                    payload.clone()
                } else {
                    return self.error_response(&format!("Primary source '{}' has no payload", self.primary_source));
                }
            } else {
                return self.error_response(&format!("Primary source '{}' failed", self.primary_source));
            }
        } else {
            return self.error_response(&format!("Primary source '{}' not found in dependency results", self.primary_source));
        };

        // Collect metadata from secondary sources
        let mut metadata_map = HashMap::new();
        for source in &self.metadata_sources {
            if let Some(result) = dependency_results.get(source) {
                if result.success {
                    if let Some(payload) = &result.payload {
                        // Convert payload to string for metadata
                        if let Ok(payload_str) = String::from_utf8(payload.clone()) {
                            metadata_map.insert(format!("{}_result", source), payload_str);
                        }
                    }
                }
            }
        }

        // Since ProcessorResponse now has metadata field, we can use it properly
        // But for backward compatibility, we also create a JSON structure
        // that includes both the primary payload and metadata
        let combined_result = serde_json::json!({
            "primary_payload": String::from_utf8_lossy(&primary_payload),
            "metadata": metadata_map
        });

        match serde_json::to_string(&combined_result) {
            Ok(json_str) => ProcessorResponse {
                outcome: Some(Outcome::NextPayload(json_str.into_bytes())),
            },
            Err(e) => self.error_response(&format!("Failed to serialize metadata result: {}", e)),
        }
    }
}

#[async_trait]
impl Processor for MetadataMergeCollector {
    async fn process(&self, request: ProcessorRequest) -> ProcessorResponse {
        // The input payload should contain serialized dependency results
        match serde_json::from_slice::<HashMap<String, CollectableResult>>(&request.payload) {
            Ok(dependency_results) => self.collect_results(&dependency_results, &request).await,
            Err(e) => self.error_response(&format!("Failed to deserialize dependency results: {}", e)),
        }
    }

    fn name(&self) -> &'static str {
        "metadata_merge_collector"
    }
}
