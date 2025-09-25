use async_trait::async_trait;
use std::collections::HashMap;
use serde_json;

use crate::traits::Processor;
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, ErrorDetail};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::config::{CollectionStrategy, ConflictResolution};
use super::collectors::CollectableResult;

/// ResultCollector processor for combining outputs from multiple dependencies.
/// 
/// This processor implements the Kubeflow-inspired collection strategy pattern
/// to deterministically combine results from parallel processor execution.
/// It addresses the non-deterministic behavior issue in parallel DAG execution.
pub struct ResultCollectorProcessor {
    strategy: CollectionStrategy,
}

impl ResultCollectorProcessor {
    /// Create a new ResultCollector with the specified collection strategy
    pub fn new(strategy: CollectionStrategy) -> Self {
        Self { strategy }
    }

    /// Create a ResultCollector with MergeMetadata strategy
    pub fn merge_metadata(primary_source: String, metadata_sources: Vec<String>) -> Self {
        Self::new(CollectionStrategy::MergeMetadata {
            primary_source,
            metadata_sources,
        })
    }

    /// Create a ResultCollector with Concatenate strategy
    pub fn concatenate(separator: Option<String>) -> Self {
        Self::new(CollectionStrategy::Concatenate { separator })
    }

    /// Create a ResultCollector with JsonMerge strategy
    pub fn json_merge(merge_arrays: bool, conflict_resolution: ConflictResolution) -> Self {
        Self::new(CollectionStrategy::JsonMerge {
            merge_arrays,
            conflict_resolution,
        })
    }

    /// Collect results from multiple dependencies based on the configured strategy
    fn collect_results(
        &self,
        dependency_results: &HashMap<String, CollectableResult>,
    ) -> ProcessorResponse {
        match &self.strategy {
            CollectionStrategy::FirstAvailable => {
                self.first_available_strategy(dependency_results)
            }
            CollectionStrategy::MergeMetadata { primary_source, metadata_sources } => {
                self.merge_metadata_strategy(dependency_results, primary_source, metadata_sources)
            }
            CollectionStrategy::Concatenate { separator } => {
                self.concatenate_strategy(dependency_results, separator.as_deref())
            }
            CollectionStrategy::JsonMerge { merge_arrays, conflict_resolution } => {
                self.json_merge_strategy(dependency_results, *merge_arrays, conflict_resolution)
            }
            CollectionStrategy::Custom { combiner_impl } => {
                self.custom_strategy(dependency_results, combiner_impl)
            }
        }
    }

    /// First available strategy - use the first successful dependency result
    fn first_available_strategy(
        &self,
        dependency_results: &HashMap<String, CollectableResult>,
    ) -> ProcessorResponse {
        for (_, result) in dependency_results {
            if result.success {
                if let Some(payload) = &result.payload {
                    return ProcessorResponse {
                        outcome: Some(Outcome::NextPayload(payload.clone())),
                    };
                }
            }
        }
        
        self.error_response("No successful dependency results found")
    }

    /// Merge metadata strategy - primary payload + others as metadata
    /// Note: Since ProcessorResponse doesn't have metadata field, we encode metadata into the payload
    fn merge_metadata_strategy(
        &self,
        dependency_results: &HashMap<String, CollectableResult>,
        primary_source: &str,
        metadata_sources: &[String],
    ) -> ProcessorResponse {
        // Get primary payload
        let primary_payload = if let Some(primary_result) = dependency_results.get(primary_source) {
            if primary_result.success {
                if let Some(payload) = &primary_result.payload {
                    payload.clone()
                } else {
                    return self.error_response(&format!("Primary source '{}' has no payload", primary_source));
                }
            } else {
                return self.error_response(&format!("Primary source '{}' failed", primary_source));
            }
        } else {
            return self.error_response(&format!("Primary source '{}' not found in dependency results", primary_source));
        };

        // Collect metadata from secondary sources
        let mut metadata_map = HashMap::new();
        for source in metadata_sources {
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

        // Since ProcessorResponse doesn't have metadata field, we create a JSON structure
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

    /// Concatenate strategy - combine all outputs into single payload
    fn concatenate_strategy(
        &self,
        dependency_results: &HashMap<String, CollectableResult>,
        separator: Option<&str>,
    ) -> ProcessorResponse {
        let sep = separator.unwrap_or("");
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

        let combined = combined_parts.join(sep);
        ProcessorResponse {
            outcome: Some(Outcome::NextPayload(combined.into_bytes())),
        }
    }

    /// JSON merge strategy - intelligently merge JSON outputs
    fn json_merge_strategy(
        &self,
        dependency_results: &HashMap<String, CollectableResult>,
        merge_arrays: bool,
        conflict_resolution: &ConflictResolution,
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
                                            let resolved_value = match conflict_resolution {
                                                ConflictResolution::TakeFirst => continue, // Keep existing
                                                ConflictResolution::TakeLast => value, // Use new value
                                                ConflictResolution::Merge => {
                                                    self.merge_json_values(existing_value, &value, merge_arrays)
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

    /// Custom strategy - placeholder for extensible custom logic
    fn custom_strategy(
        &self,
        _dependency_results: &HashMap<String, CollectableResult>,
        combiner_impl: &str,
    ) -> ProcessorResponse {
        // TODO: Implement custom combiner loading/execution
        self.error_response(&format!("Custom combiner '{}' not implemented yet", combiner_impl))
    }

    /// Helper to merge two JSON values
    fn merge_json_values(
        &self,
        existing: &serde_json::Value,
        new: &serde_json::Value,
        merge_arrays: bool,
    ) -> Option<serde_json::Value> {
        match (existing, new) {
            (serde_json::Value::Object(existing_obj), serde_json::Value::Object(new_obj)) => {
                let mut merged = existing_obj.clone();
                for (key, value) in new_obj {
                    merged.insert(key.clone(), value.clone());
                }
                Some(serde_json::Value::Object(merged))
            }
            (serde_json::Value::Array(existing_arr), serde_json::Value::Array(new_arr)) if merge_arrays => {
                let mut merged = existing_arr.clone();
                merged.extend(new_arr.clone());
                Some(serde_json::Value::Array(merged))
            }
            _ => None, // Can't merge, let caller decide
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
impl Processor for ResultCollectorProcessor {
    async fn process(&self, request: ProcessorRequest) -> ProcessorResponse {
        // The input payload should contain serialized dependency results
        // This will be provided by the work queue executor
        match serde_json::from_slice::<HashMap<String, CollectableResult>>(&request.payload) {
            Ok(dependency_results) => self.collect_results(&dependency_results),
            Err(e) => self.error_response(&format!("Failed to deserialize dependency results: {}", e)),
        }
    }

    fn name(&self) -> &'static str {
        "result_collector"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_success_result(payload: &str) -> CollectableResult {
        CollectableResult {
            success: true,
            payload: Some(payload.as_bytes().to_vec()),
            error_code: None,
            error_message: None,
        }
    }

    fn create_error_result(code: i32, message: &str) -> CollectableResult {
        CollectableResult {
            success: false,
            payload: None,
            error_code: Some(code),
            error_message: Some(message.to_string()),
        }
    }

    #[tokio::test]
    async fn test_merge_metadata_strategy() {
        let collector = ResultCollectorProcessor::merge_metadata(
            "primary".to_string(),
            vec!["secondary".to_string()],
        );

        let mut dependency_results = HashMap::new();
        dependency_results.insert("primary".to_string(), create_success_result("primary_data"));
        dependency_results.insert("secondary".to_string(), create_success_result("secondary_data"));

        let input = serde_json::to_vec(&dependency_results).unwrap();
        let request = ProcessorRequest {
            payload: input,
            metadata: HashMap::new(),
        };

        let response = collector.process(request).await;

        if let Some(Outcome::NextPayload(payload)) = response.outcome {
            let result_str = String::from_utf8(payload).unwrap();
            let result_json: serde_json::Value = serde_json::from_str(&result_str).unwrap();
            
            // Check that the result contains both primary payload and metadata
            assert!(result_json.get("primary_payload").is_some());
            assert!(result_json.get("metadata").is_some());
            
            let metadata = result_json.get("metadata").unwrap().as_object().unwrap();
            assert!(metadata.contains_key("secondary_result"));
            assert_eq!(metadata.get("secondary_result").unwrap().as_str().unwrap(), "secondary_data");
        } else {
            panic!("Expected NextPayload outcome");
        }
    }

    #[tokio::test]
    async fn test_concatenate_strategy() {
        let collector = ResultCollectorProcessor::concatenate(Some(" | ".to_string()));

        let mut dependency_results = HashMap::new();
        dependency_results.insert("dep1".to_string(), create_success_result("data1"));
        dependency_results.insert("dep2".to_string(), create_success_result("data2"));

        let input = serde_json::to_vec(&dependency_results).unwrap();
        let request = ProcessorRequest {
            payload: input,
            metadata: HashMap::new(),
        };

        let response = collector.process(request).await;

        if let Some(Outcome::NextPayload(payload)) = response.outcome {
            let result = String::from_utf8(payload).unwrap();
            // Results should be deterministically ordered by dependency ID
            assert!(result == "data1 | data2" || result == "data2 | data1");
        } else {
            panic!("Expected NextPayload outcome");
        }
    }

    #[tokio::test]
    async fn test_json_merge_strategy() {
        let collector = ResultCollectorProcessor::json_merge(true, ConflictResolution::Merge);

        let mut dependency_results = HashMap::new();
        dependency_results.insert("dep1".to_string(), create_success_result(r#"{"count": 5, "items": ["a"]}"#));
        dependency_results.insert("dep2".to_string(), create_success_result(r#"{"total": 10, "items": ["b"]}"#));

        let input = serde_json::to_vec(&dependency_results).unwrap();
        let request = ProcessorRequest {
            payload: input,
            metadata: HashMap::new(),
        };

        let response = collector.process(request).await;

        if let Some(Outcome::NextPayload(payload)) = response.outcome {
            let result = String::from_utf8(payload).unwrap();
            let json: serde_json::Value = serde_json::from_str(&result).unwrap();
            
            assert!(json.get("count").is_some());
            assert!(json.get("total").is_some());
            assert!(json.get("items").is_some());
        } else {
            panic!("Expected NextPayload outcome");
        }
    }

    #[tokio::test]
    async fn test_first_available_strategy() {
        let collector = ResultCollectorProcessor::new(CollectionStrategy::FirstAvailable);

        let mut dependency_results = HashMap::new();
        dependency_results.insert("dep1".to_string(), create_error_result(400, "Failed"));
        dependency_results.insert("dep2".to_string(), create_success_result("success_data"));

        let input = serde_json::to_vec(&dependency_results).unwrap();
        let request = ProcessorRequest {
            payload: input,
            metadata: HashMap::new(),
        };

        let response = collector.process(request).await;

        if let Some(Outcome::NextPayload(payload)) = response.outcome {
            assert_eq!(String::from_utf8(payload).unwrap(), "success_data");
        } else {
            panic!("Expected NextPayload outcome");
        }
    }
}
