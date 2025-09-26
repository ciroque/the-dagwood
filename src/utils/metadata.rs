use std::collections::HashMap;

/// Merge metadata from multiple sources with prefixed keys to avoid conflicts.
/// 
/// This utility function provides consistent metadata merging behavior across
/// the DAGwood system, ensuring that dependency metadata is properly namespaced
/// to prevent key collisions.
/// 
/// # Arguments
/// 
/// * `base_metadata` - The base metadata to start with (e.g., from input or processor)
/// * `dependency_metadata` - Map of dependency_id -> metadata to merge with prefixes
/// 
/// # Returns
/// 
/// Combined metadata with dependency keys prefixed as "dependency_id_original_key"
/// 
/// # Example
/// 
/// ```rust
/// use std::collections::HashMap;
/// use the_dagwood::utils::merge_metadata_with_prefixes;
/// 
/// let mut base = HashMap::new();
/// base.insert("own_key".to_string(), "own_value".to_string());
/// 
/// let mut dep_metadata = HashMap::new();
/// let mut dep1_meta = HashMap::new();
/// dep1_meta.insert("result".to_string(), "success".to_string());
/// dep_metadata.insert("processor1".to_string(), dep1_meta);
/// 
/// let merged = merge_metadata_with_prefixes(base, &dep_metadata);
/// 
/// assert_eq!(merged.get("own_key"), Some(&"own_value".to_string()));
/// assert_eq!(merged.get("processor1_result"), Some(&"success".to_string()));
/// ```
pub fn merge_metadata_with_prefixes(
    mut base_metadata: HashMap<String, String>,
    dependency_metadata: &HashMap<String, HashMap<String, String>>,
) -> HashMap<String, String> {
    // Merge dependency metadata with prefixed keys to avoid conflicts
    for (dep_id, dep_metadata) in dependency_metadata {
        for (key, value) in dep_metadata {
            let prefixed_key = format!("{}_{}", dep_id, key);
            base_metadata.insert(prefixed_key, value.clone());
        }
    }
    
    base_metadata
}

/// Merge metadata from dependency responses with prefixed keys.
/// 
/// This is a convenience function specifically for merging metadata from
/// ProcessorResponse objects, commonly used in DAG execution.
/// 
/// # Arguments
/// 
/// * `base_metadata` - The base metadata to start with
/// * `dependency_responses` - Map of dependency_id -> ProcessorResponse
/// 
/// # Returns
/// 
/// Combined metadata with dependency keys prefixed as "dependency_id_original_key"
pub fn merge_metadata_from_responses(
    base_metadata: HashMap<String, String>,
    dependency_responses: &HashMap<String, crate::proto::processor_v1::ProcessorResponse>,
) -> HashMap<String, String> {
    let mut dependency_metadata = HashMap::new();
    
    for (dep_id, response) in dependency_responses {
        dependency_metadata.insert(dep_id.clone(), response.metadata.clone());
    }
    
    merge_metadata_with_prefixes(base_metadata, &dependency_metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_metadata_with_prefixes() {
        let mut base = HashMap::new();
        base.insert("own_key".to_string(), "own_value".to_string());
        
        let mut dependency_metadata = HashMap::new();
        
        // Add metadata from dependency 1
        let mut dep1_meta = HashMap::new();
        dep1_meta.insert("result".to_string(), "success".to_string());
        dep1_meta.insert("count".to_string(), "42".to_string());
        dependency_metadata.insert("processor1".to_string(), dep1_meta);
        
        // Add metadata from dependency 2
        let mut dep2_meta = HashMap::new();
        dep2_meta.insert("result".to_string(), "completed".to_string()); // Same key, different value
        dep2_meta.insert("time".to_string(), "100ms".to_string());
        dependency_metadata.insert("processor2".to_string(), dep2_meta);
        
        let merged = merge_metadata_with_prefixes(base, &dependency_metadata);
        
        // Verify base metadata is preserved
        assert_eq!(merged.get("own_key"), Some(&"own_value".to_string()));
        
        // Verify dependency metadata is prefixed correctly
        assert_eq!(merged.get("processor1_result"), Some(&"success".to_string()));
        assert_eq!(merged.get("processor1_count"), Some(&"42".to_string()));
        assert_eq!(merged.get("processor2_result"), Some(&"completed".to_string()));
        assert_eq!(merged.get("processor2_time"), Some(&"100ms".to_string()));
        
        // Verify no key conflicts (both dependencies had "result" key)
        assert!(merged.contains_key("processor1_result"));
        assert!(merged.contains_key("processor2_result"));
        assert_ne!(merged.get("processor1_result"), merged.get("processor2_result"));
    }

    #[test]
    fn test_merge_metadata_empty_dependencies() {
        let mut base = HashMap::new();
        base.insert("key".to_string(), "value".to_string());
        
        let dependency_metadata = HashMap::new();
        
        let merged = merge_metadata_with_prefixes(base.clone(), &dependency_metadata);
        
        assert_eq!(merged, base);
    }

    #[test]
    fn test_merge_metadata_empty_base() {
        let base = HashMap::new();
        
        let mut dependency_metadata = HashMap::new();
        let mut dep_meta = HashMap::new();
        dep_meta.insert("key".to_string(), "value".to_string());
        dependency_metadata.insert("dep1".to_string(), dep_meta);
        
        let merged = merge_metadata_with_prefixes(base, &dependency_metadata);
        
        assert_eq!(merged.get("dep1_key"), Some(&"value".to_string()));
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn test_merge_metadata_key_override() {
        let mut base = HashMap::new();
        base.insert("dep1_result".to_string(), "original".to_string());
        
        let mut dependency_metadata = HashMap::new();
        let mut dep_meta = HashMap::new();
        dep_meta.insert("result".to_string(), "from_dependency".to_string());
        dependency_metadata.insert("dep1".to_string(), dep_meta);
        
        let merged = merge_metadata_with_prefixes(base, &dependency_metadata);
        
        // Dependency metadata should override base metadata with same prefixed key
        assert_eq!(merged.get("dep1_result"), Some(&"from_dependency".to_string()));
    }
}
