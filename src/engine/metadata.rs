use std::collections::HashMap;
use crate::proto::processor_v1::{ProcessorResponse, Metadata};

/// Key used to store base/original input metadata in the metadata map.
/// 
/// When merging metadata from dependency responses, the original input metadata
/// is preserved under this special key to maintain traceability of the initial
/// request context throughout DAG execution.
pub const BASE_METADATA_KEY: &str = "input";

/// Merges metadata from dependency processor responses for DAG execution.
///
/// This function is specifically designed for DAG executors that need to combine
/// metadata from multiple dependency processors when preparing input for a downstream
/// processor. It preserves the original input metadata under a special key while
/// collecting all metadata from dependency responses.
///
/// # Use Cases
/// - Work Queue executor combining metadata from parallel dependencies
/// - Level-by-level executor merging metadata from previous level
/// - Any executor that needs to maintain metadata lineage through DAG execution
///
/// # Arguments
/// * `base_metadata` - Original input metadata to preserve (stored under BASE_METADATA_KEY)
/// * `dependency_responses` - Map of dependency processor responses containing metadata
///
/// # Returns
/// A HashMap where:
/// - Key: Processor name (either BASE_METADATA_KEY or dependency processor names)
/// - Value: Metadata struct containing the processor's metadata key-value pairs
///
/// # Example
/// ```rust
/// use std::collections::HashMap;
/// use the_dagwood::engine::metadata::merge_dependency_metadata_for_execution;
/// use the_dagwood::proto::processor_v1::{ProcessorResponse, Metadata};
///
/// let mut base_metadata = HashMap::new();
/// base_metadata.insert("request_id".to_string(), "req_123".to_string());
///
/// let mut dependency_responses = HashMap::new();
/// let mut response = ProcessorResponse::default();
/// let mut proc_metadata = HashMap::new();
/// proc_metadata.insert("result".to_string(), "processed".to_string());
/// response.metadata.insert("analyzer".to_string(), Metadata {
///     metadata: proc_metadata,
/// });
/// dependency_responses.insert("dep1".to_string(), response);
///
/// let merged = merge_dependency_metadata_for_execution(base_metadata, &dependency_responses);
/// 
/// // Access original input metadata
/// assert!(merged.contains_key("input"));
/// // Access dependency processor metadata
/// assert!(merged.contains_key("analyzer"));
/// ```
pub fn merge_dependency_metadata_for_execution(
    base_metadata: HashMap<String, String>,
    dependency_responses: &HashMap<String, ProcessorResponse>,
) -> HashMap<String, Metadata> {
    let mut result = HashMap::new();
    
    // Preserve base metadata under special key if not empty
    // This maintains traceability of the original request context
    if !base_metadata.is_empty() {
        result.insert(BASE_METADATA_KEY.to_string(), Metadata {
            metadata: base_metadata,
        });
    }
    
    // Collect metadata from all dependency responses
    // Each dependency processor's metadata is preserved under its processor name
    for (_dependency_id, response) in dependency_responses {
        // Copy all metadata from this dependency response
        // Note: If multiple dependencies have processors with the same name,
        // the last one will overwrite previous ones (HashMap behavior)
        for (processor_name, metadata) in &response.metadata {
            result.insert(processor_name.clone(), metadata.clone());
        }
    }
    
    result
}

/// Extracts metadata for a specific processor from a merged metadata map.
///
/// This is a convenience function for accessing metadata from a specific processor
/// in the merged metadata structure created by `merge_dependency_metadata_for_execution`.
///
/// # Arguments
/// * `merged_metadata` - The merged metadata map from `merge_dependency_metadata_for_execution`
/// * `processor_name` - Name of the processor whose metadata to extract
///
/// # Returns
/// Option containing the processor's metadata HashMap, or None if not found
///
/// # Example
/// ```rust
/// use std::collections::HashMap;
/// use the_dagwood::engine::metadata::{merge_dependency_metadata_for_execution, extract_processor_metadata};
/// use the_dagwood::proto::processor_v1::{ProcessorResponse, Metadata};
/// 
/// let base_metadata = HashMap::new();
/// let dependency_responses = HashMap::new();
/// let merged_metadata = merge_dependency_metadata_for_execution(base_metadata, &dependency_responses);
/// 
/// if let Some(analyzer_metadata) = extract_processor_metadata(&merged_metadata, "analyzer") {
///     if let Some(result) = analyzer_metadata.get("result") {
///         println!("Analyzer result: {}", result);
///     }
/// }
/// ```
pub fn extract_processor_metadata<'a>(
    merged_metadata: &'a HashMap<String, Metadata>,
    processor_name: &str,
) -> Option<&'a HashMap<String, String>> {
    merged_metadata.get(processor_name).map(|metadata| &metadata.metadata)
}

/// Checks if the merged metadata contains metadata from the original input.
///
/// This is a convenience function to check if base metadata was preserved
/// during the metadata merging process.
///
/// # Arguments
/// * `merged_metadata` - The merged metadata map from `merge_dependency_metadata_for_execution`
///
/// # Returns
/// true if base metadata is present, false otherwise
pub fn has_base_metadata(merged_metadata: &HashMap<String, Metadata>) -> bool {
    merged_metadata.contains_key(BASE_METADATA_KEY)
}

/// Gets the original input metadata from a merged metadata map.
///
/// This is a convenience function for accessing the original input metadata
/// that was preserved during the merging process.
///
/// # Arguments
/// * `merged_metadata` - The merged metadata map from `merge_dependency_metadata_for_execution`
///
/// # Returns
/// Option containing the original input metadata HashMap, or None if not present
pub fn get_base_metadata<'a>(merged_metadata: &'a HashMap<String, Metadata>) -> Option<&'a HashMap<String, String>> {
    extract_processor_metadata(merged_metadata, BASE_METADATA_KEY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_dependency_metadata_for_execution_basic() {
        let mut base = HashMap::new();
        base.insert("request_id".to_string(), "req_123".to_string());
        base.insert("user_id".to_string(), "user_456".to_string());
        
        let mut responses = HashMap::new();
        
        // Create a response with metadata from one dependency
        let mut response = ProcessorResponse::default();
        let mut proc_metadata = HashMap::new();
        proc_metadata.insert("analysis_result".to_string(), "POSITIVE".to_string());
        proc_metadata.insert("confidence".to_string(), "0.95".to_string());
        response.metadata.insert("sentiment_analyzer".to_string(), Metadata {
            metadata: proc_metadata,
        });
        responses.insert("analyzer_dep".to_string(), response);
        
        let merged = merge_dependency_metadata_for_execution(base, &responses);
        
        // Verify base metadata is preserved under BASE_METADATA_KEY
        assert!(merged.contains_key(BASE_METADATA_KEY));
        let base_meta = merged.get(BASE_METADATA_KEY).unwrap();
        assert_eq!(base_meta.metadata.get("request_id"), Some(&"req_123".to_string()));
        assert_eq!(base_meta.metadata.get("user_id"), Some(&"user_456".to_string()));
        
        // Verify dependency metadata is preserved under processor name
        assert!(merged.contains_key("sentiment_analyzer"));
        let analyzer_meta = merged.get("sentiment_analyzer").unwrap();
        assert_eq!(analyzer_meta.metadata.get("analysis_result"), Some(&"POSITIVE".to_string()));
        assert_eq!(analyzer_meta.metadata.get("confidence"), Some(&"0.95".to_string()));
    }

    #[test]
    fn test_merge_dependency_metadata_for_execution_empty_base() {
        let base = HashMap::new(); // Empty base metadata
        
        let mut responses = HashMap::new();
        let mut response = ProcessorResponse::default();
        let mut proc_metadata = HashMap::new();
        proc_metadata.insert("result".to_string(), "SUCCESS".to_string());
        response.metadata.insert("processor1".to_string(), Metadata {
            metadata: proc_metadata,
        });
        responses.insert("dep1".to_string(), response);
        
        let merged = merge_dependency_metadata_for_execution(base, &responses);
        
        // Base metadata should not be present when empty
        assert!(!merged.contains_key(BASE_METADATA_KEY));
        
        // Dependency metadata should still be present
        assert!(merged.contains_key("processor1"));
        assert_eq!(merged.get("processor1").unwrap().metadata.get("result"), Some(&"SUCCESS".to_string()));
    }

    #[test]
    fn test_merge_dependency_metadata_for_execution_multiple_dependencies() {
        let mut base = HashMap::new();
        base.insert("session_id".to_string(), "sess_789".to_string());
        
        let mut responses = HashMap::new();
        
        // First dependency with multiple processors
        let mut response1 = ProcessorResponse::default();
        let mut proc1_metadata = HashMap::new();
        proc1_metadata.insert("tokens".to_string(), "42".to_string());
        response1.metadata.insert("tokenizer".to_string(), Metadata {
            metadata: proc1_metadata,
        });
        
        let mut proc2_metadata = HashMap::new();
        proc2_metadata.insert("language".to_string(), "en".to_string());
        response1.metadata.insert("language_detector".to_string(), Metadata {
            metadata: proc2_metadata,
        });
        responses.insert("text_analysis_dep".to_string(), response1);
        
        // Second dependency
        let mut response2 = ProcessorResponse::default();
        let mut proc3_metadata = HashMap::new();
        proc3_metadata.insert("score".to_string(), "8.5".to_string());
        response2.metadata.insert("quality_scorer".to_string(), Metadata {
            metadata: proc3_metadata,
        });
        responses.insert("quality_dep".to_string(), response2);
        
        let merged = merge_dependency_metadata_for_execution(base, &responses);
        
        // Should have base metadata + 3 processor metadata entries
        assert_eq!(merged.len(), 4);
        assert!(merged.contains_key(BASE_METADATA_KEY));
        assert!(merged.contains_key("tokenizer"));
        assert!(merged.contains_key("language_detector"));
        assert!(merged.contains_key("quality_scorer"));
        
        // Verify all metadata is correctly preserved
        assert_eq!(get_base_metadata(&merged).unwrap().get("session_id"), Some(&"sess_789".to_string()));
        assert_eq!(extract_processor_metadata(&merged, "tokenizer").unwrap().get("tokens"), Some(&"42".to_string()));
        assert_eq!(extract_processor_metadata(&merged, "language_detector").unwrap().get("language"), Some(&"en".to_string()));
        assert_eq!(extract_processor_metadata(&merged, "quality_scorer").unwrap().get("score"), Some(&"8.5".to_string()));
    }

    #[test]
    fn test_extract_processor_metadata() {
        let mut merged = HashMap::new();
        let mut metadata = HashMap::new();
        metadata.insert("key1".to_string(), "value1".to_string());
        metadata.insert("key2".to_string(), "value2".to_string());
        merged.insert("test_processor".to_string(), Metadata { metadata });
        
        let extracted = extract_processor_metadata(&merged, "test_processor");
        assert!(extracted.is_some());
        assert_eq!(extracted.unwrap().get("key1"), Some(&"value1".to_string()));
        assert_eq!(extracted.unwrap().get("key2"), Some(&"value2".to_string()));
        
        let missing = extract_processor_metadata(&merged, "nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_has_base_metadata() {
        let mut merged_with_base = HashMap::new();
        let mut base_metadata = HashMap::new();
        base_metadata.insert("original".to_string(), "data".to_string());
        merged_with_base.insert(BASE_METADATA_KEY.to_string(), Metadata { metadata: base_metadata });
        
        assert!(has_base_metadata(&merged_with_base));
        
        let merged_without_base = HashMap::new();
        assert!(!has_base_metadata(&merged_without_base));
    }

    #[test]
    fn test_get_base_metadata() {
        let mut merged = HashMap::new();
        let mut base_metadata = HashMap::new();
        base_metadata.insert("original_key".to_string(), "original_value".to_string());
        merged.insert(BASE_METADATA_KEY.to_string(), Metadata { metadata: base_metadata });
        
        let base = get_base_metadata(&merged);
        assert!(base.is_some());
        assert_eq!(base.unwrap().get("original_key"), Some(&"original_value".to_string()));
        
        let empty_merged = HashMap::new();
        let no_base = get_base_metadata(&empty_merged);
        assert!(no_base.is_none());
    }

    #[test]
    fn test_processor_name_collision() {
        // Test behavior when multiple dependencies have processors with the same name
        let base = HashMap::new();
        let mut responses = HashMap::new();
        
        // First dependency with processor "analyzer"
        let mut response1 = ProcessorResponse::default();
        let mut proc1_metadata = HashMap::new();
        proc1_metadata.insert("version".to_string(), "1.0".to_string());
        response1.metadata.insert("analyzer".to_string(), Metadata {
            metadata: proc1_metadata,
        });
        responses.insert("dep1".to_string(), response1);
        
        // Second dependency with processor "analyzer" (same name)
        let mut response2 = ProcessorResponse::default();
        let mut proc2_metadata = HashMap::new();
        proc2_metadata.insert("version".to_string(), "2.0".to_string());
        response2.metadata.insert("analyzer".to_string(), Metadata {
            metadata: proc2_metadata,
        });
        responses.insert("dep2".to_string(), response2);
        
        let merged = merge_dependency_metadata_for_execution(base, &responses);
        
        // Should have only one "analyzer" entry (last one wins due to HashMap behavior)
        assert_eq!(merged.len(), 1);
        assert!(merged.contains_key("analyzer"));
        
        // The version should be from the last processor (HashMap insertion order dependent)
        let analyzer_meta = extract_processor_metadata(&merged, "analyzer").unwrap();
        let version = analyzer_meta.get("version").unwrap();
        assert!(version == "1.0" || version == "2.0"); // Either could win depending on iteration order
    }
}
