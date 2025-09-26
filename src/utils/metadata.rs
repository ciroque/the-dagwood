use std::collections::HashMap;

/// Merge metadata from multiple sources with namespaced keys to avoid conflicts.
/// 
/// This utility function provides consistent metadata merging behavior across
/// the DAGwood system, ensuring that dependency metadata is properly namespaced
/// to prevent key collisions even when dependency IDs contain special characters.
/// 
/// Uses a robust namespacing scheme with URL-style encoding and length prefixes
/// to guarantee collision-free key generation.
/// 
/// # Arguments
/// 
/// * `base_metadata` - The base metadata to start with (e.g., from input or processor)
/// * `dependency_metadata` - Map of dependency_id -> metadata to merge with namespaces
/// 
/// # Returns
/// 
/// Combined metadata with dependency keys namespaced as "dep:<len>:<dependency_id>:<original_key>"
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
/// assert_eq!(merged.get("dep:10:processor1:result"), Some(&"success".to_string()));
/// ```
pub fn merge_metadata_with_prefixes(
    mut base_metadata: HashMap<String, String>,
    dependency_metadata: &HashMap<String, HashMap<String, String>>,
) -> HashMap<String, String> {
    // Merge dependency metadata with collision-resistant namespaced keys
    for (dep_id, dep_metadata) in dependency_metadata {
        for (key, value) in dep_metadata {
            let namespaced_key = create_namespaced_key(dep_id, key);
            base_metadata.insert(namespaced_key, value.clone());
        }
    }
    
    base_metadata
}

/// Create a collision-resistant namespaced key for dependency metadata.
/// 
/// Uses a robust scheme: "dep:<len>:<dependency_id>:<original_key>"
/// where <len> is the byte length of the dependency_id. This prevents
/// collisions even when dependency IDs contain colons, underscores, or other
/// special characters.
/// 
/// # Arguments
/// 
/// * `dependency_id` - The ID of the dependency processor
/// * `original_key` - The original metadata key from the dependency
/// 
/// # Returns
/// 
/// A collision-resistant namespaced key
/// 
/// # Examples
/// 
/// ```rust
/// use the_dagwood::utils::metadata::create_namespaced_key;
/// 
/// // Normal case
/// assert_eq!(create_namespaced_key("proc1", "result"), "dep:5:proc1:result");
/// 
/// // Edge case with underscores and colons
/// assert_eq!(create_namespaced_key("user_profile", "data"), "dep:12:user_profile:data");
/// assert_eq!(create_namespaced_key("user", "profile_data"), "dep:4:user:profile_data");
/// 
/// // These produce different keys despite similar content
/// assert_ne!(create_namespaced_key("user_profile", "data"), 
///           create_namespaced_key("user", "profile_data"));
/// ```
pub fn create_namespaced_key(dependency_id: &str, original_key: &str) -> String {
    format!("dep:{}:{}:{}", dependency_id.len(), dependency_id, original_key)
}

/// Merge metadata from dependency responses with namespaced keys.
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
/// Combined metadata with dependency keys namespaced as "dep:<len>:<dependency_id>:<original_key>"
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
        
        // Verify dependency metadata is namespaced correctly
        assert_eq!(merged.get("dep:10:processor1:result"), Some(&"success".to_string()));
        assert_eq!(merged.get("dep:10:processor1:count"), Some(&"42".to_string()));
        assert_eq!(merged.get("dep:10:processor2:result"), Some(&"completed".to_string()));
        assert_eq!(merged.get("dep:10:processor2:time"), Some(&"100ms".to_string()));
        
        // Verify no key conflicts (both dependencies had "result" key)
        assert!(merged.contains_key("dep:10:processor1:result"));
        assert!(merged.contains_key("dep:10:processor2:result"));
        assert_ne!(merged.get("dep:10:processor1:result"), merged.get("dep:10:processor2:result"));
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
        
        assert_eq!(merged.get("dep:4:dep1:key"), Some(&"value".to_string()));
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn test_merge_metadata_key_override() {
        let mut base = HashMap::new();
        base.insert("dep:4:dep1:result".to_string(), "original".to_string());
        
        let mut dependency_metadata = HashMap::new();
        let mut dep_meta = HashMap::new();
        dep_meta.insert("result".to_string(), "from_dependency".to_string());
        dependency_metadata.insert("dep1".to_string(), dep_meta);
        
        let merged = merge_metadata_with_prefixes(base, &dependency_metadata);
        
        // Dependency metadata should override base metadata with same namespaced key
        assert_eq!(merged.get("dep:4:dep1:result"), Some(&"from_dependency".to_string()));
    }

    #[test]
    fn test_create_namespaced_key() {
        // Normal cases
        assert_eq!(create_namespaced_key("proc1", "result"), "dep:5:proc1:result");
        assert_eq!(create_namespaced_key("processor", "data"), "dep:9:processor:data");
        
        // Edge cases that would cause collisions with underscore prefixing
        assert_eq!(create_namespaced_key("user_profile", "data"), "dep:12:user_profile:data");
        assert_eq!(create_namespaced_key("user", "profile_data"), "dep:4:user:profile_data");
        
        // Verify these produce different keys (would collide with "user_profile_data")
        assert_ne!(create_namespaced_key("user_profile", "data"), 
                  create_namespaced_key("user", "profile_data"));
        
        // Test with colons in dependency ID (another potential collision source)
        assert_eq!(create_namespaced_key("ns:proc", "key"), "dep:7:ns:proc:key");
        assert_eq!(create_namespaced_key("ns", "proc:key"), "dep:2:ns:proc:key");
        
        // Verify colon-containing IDs produce different keys
        assert_ne!(create_namespaced_key("ns:proc", "key"),
                  create_namespaced_key("ns", "proc:key"));
    }

    #[test]
    fn test_collision_resistance_comprehensive() {
        // Test various edge cases that could cause collisions
        let test_cases = vec![
            // (dep_id1, key1, dep_id2, key2) - should all produce different namespaced keys
            ("user_profile", "data", "user", "profile_data"),
            ("a_b", "c", "a", "b_c"),
            ("ns:proc", "key", "ns", "proc:key"),
            ("dep:5:test", "key", "dep", "5:test:key"),
            ("", "full_key", "full", "_key"),
            ("123", "456", "12", "3456"),
        ];
        
        for (dep_id1, key1, dep_id2, key2) in test_cases {
            let namespaced1 = create_namespaced_key(dep_id1, key1);
            let namespaced2 = create_namespaced_key(dep_id2, key2);
            
            assert_ne!(namespaced1, namespaced2, 
                      "Collision detected: '{}' + '{}' vs '{}' + '{}' both produce '{}'",
                      dep_id1, key1, dep_id2, key2, namespaced1);
        }
    }
}
