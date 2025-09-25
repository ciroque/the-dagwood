use async_trait::async_trait;
use std::collections::HashMap;
use serde_json;
use crate::traits::Processor;
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, ErrorDetail};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::config::ConflictResolution;
use super::{ResultCollector, CollectableResult};

/// JsonMergeCollector intelligently merges JSON outputs from multiple dependencies
/// 
/// This collector parses JSON payloads from all successful dependencies and merges
/// them into a single JSON object. It handles conflicts using configurable resolution
/// strategies and can optionally merge arrays.
pub struct JsonMergeCollector {
    merge_arrays: bool,
    conflict_resolution: ConflictResolution,
}

impl JsonMergeCollector {
    pub fn new(merge_arrays: bool, conflict_resolution: ConflictResolution) -> Self {
        Self {
            merge_arrays,
            conflict_resolution,
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

    /// Helper to merge two JSON values
    fn merge_json_values(
        &self,
        existing: &serde_json::Value,
        new: &serde_json::Value,
    ) -> Option<serde_json::Value> {
        match (existing, new) {
            (serde_json::Value::Object(existing_obj), serde_json::Value::Object(new_obj)) => {
                let mut merged = existing_obj.clone();
                for (key, value) in new_obj {
                    merged.insert(key.clone(), value.clone());
                }
                Some(serde_json::Value::Object(merged))
            }
            (serde_json::Value::Array(existing_arr), serde_json::Value::Array(new_arr)) if self.merge_arrays => {
                let mut merged = existing_arr.clone();
                merged.extend(new_arr.clone());
                Some(serde_json::Value::Array(merged))
            }
            _ => None, // Can't merge, let caller decide
        }
    }
}

#[async_trait]
impl ResultCollector for JsonMergeCollector {
    async fn collect_results(
        &self,
        dependency_results: &HashMap<String, CollectableResult>,
        _request: &ProcessorRequest,
    ) -> ProcessorResponse {
        let mut merged_json = serde_json::Map::new();

        // Sort by dependency ID for deterministic ordering
        let mut sorted_deps: Vec<_> = dependency_results.iter().collect();
        sorted_deps.sort_by_key(|(id, _)| *id);

        for (dep_id, result) in sorted_deps {
            if result.success {
                if let Some(payload) = &result.payload {
                    if let Ok(payload_str) = String::from_utf8(payload.clone()) {
                        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&payload_str) {
                            if let serde_json::Value::Object(obj) = json_value {
                                for (key, value) in obj {
                                    match merged_json.get(&key) {
                                        None => {
                                            // No conflict, add the value
                                            merged_json.insert(key, value);
                                        }
                                        Some(existing_value) => {
                                            // Handle conflict based on resolution strategy
                                            let resolved_value = match self.conflict_resolution {
                                                ConflictResolution::TakeFirst => continue, // Keep existing
                                                ConflictResolution::TakeLast => value, // Use new value
                                                ConflictResolution::Merge => {
                                                    self.merge_json_values(existing_value, &value)
                                                        .unwrap_or(value)
                                                }
                                                ConflictResolution::Error => {
                                                    return self.error_response(&format!(
                                                        "JSON merge conflict for key '{}' from dependency '{}'", 
                                                        key, dep_id
                                                    ));
                                                }
                                            };
                                            merged_json.insert(key, resolved_value);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if merged_json.is_empty() {
            return self.error_response("No valid JSON results to merge");
        }

        let merged_result = serde_json::Value::Object(merged_json);
        match serde_json::to_string(&merged_result) {
            Ok(json_str) => ProcessorResponse {
                outcome: Some(Outcome::NextPayload(json_str.into_bytes())),
            },
            Err(e) => self.error_response(&format!("Failed to serialize merged JSON: {}", e)),
        }
    }
}

#[async_trait]
impl Processor for JsonMergeCollector {
    async fn process(&self, request: ProcessorRequest) -> ProcessorResponse {
        // The input payload should contain serialized dependency results
        match serde_json::from_slice::<HashMap<String, CollectableResult>>(&request.payload) {
            Ok(dependency_results) => self.collect_results(&dependency_results, &request).await,
            Err(e) => self.error_response(&format!("Failed to deserialize dependency results: {}", e)),
        }
    }

    fn name(&self) -> &'static str {
        "json_merge_collector"
    }
}
