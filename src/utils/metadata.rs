use std::collections::HashMap;
use crate::proto::processor_v1::{ProcessorResponse, Metadata};

/// Merge metadata from dependency responses with base metadata.
/// Simple approach: collect all metadata from dependencies and merge with base.
pub fn merge_metadata_from_responses(
    base_metadata: HashMap<String, String>,
    dependency_responses: &HashMap<String, ProcessorResponse>,
) -> HashMap<String, Metadata> {
    let mut result = HashMap::new();
    
    // Add base metadata under "input" key if not empty
    if !base_metadata.is_empty() {
        result.insert("input".to_string(), Metadata {
            metadata: base_metadata,
        });
    }
    
    // Add metadata from each dependency response
    for (_dep_id, response) in dependency_responses {
        // Copy all metadata from this dependency response
        for (processor_name, metadata) in &response.metadata {
            result.insert(processor_name.clone(), metadata.clone());
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_metadata_from_responses() {
        let mut base = HashMap::new();
        base.insert("original".to_string(), "INPUT_META".to_string());
        
        let mut responses = HashMap::new();
        
        // Create a response with metadata
        let mut response = ProcessorResponse::default();
        let mut proc_metadata = HashMap::new();
        proc_metadata.insert("analysis".to_string(), "PROCESSED".to_string());
        response.metadata.insert("processor1".to_string(), Metadata {
            metadata: proc_metadata,
        });
        responses.insert("dep1".to_string(), response);
        
        let merged = merge_metadata_from_responses(base, &responses);
        
        // Verify base metadata is under "input"
        assert_eq!(merged.get("input").unwrap().metadata.get("original"), Some(&"INPUT_META".to_string()));
        
        // Verify dependency metadata is preserved
        assert_eq!(merged.get("processor1").unwrap().metadata.get("analysis"), Some(&"PROCESSED".to_string()));
    }
}
