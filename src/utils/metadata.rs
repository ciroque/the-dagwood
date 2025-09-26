use std::collections::HashMap;
use crate::proto::processor_v1::ProcessorResponse;

/// URL-safe base64 encoding without padding for secure key generation
pub fn base64_url_safe_encode(input: &[u8]) -> String {
    // Simple base64url encoding implementation
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::new();
    
    for chunk in input.chunks(3) {
        let mut buf = [0u8; 3];
        for (i, &byte) in chunk.iter().enumerate() {
            buf[i] = byte;
        }
        
        let b = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);
        
        result.push(ALPHABET[((b >> 18) & 63) as usize] as char);
        result.push(ALPHABET[((b >> 12) & 63) as usize] as char);
        
        if chunk.len() > 1 {
            result.push(ALPHABET[((b >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            result.push(ALPHABET[(b & 63) as usize] as char);
        }
    }
    
    result
}

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
/// Combined metadata with dependency keys namespaced using secure base64 encoding
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
/// assert_eq!(merged.get("dep.cHJvY2Vzc29yMQ.cmVzdWx0"), Some(&"success".to_string()));
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
/// A collision-resistant namespaced key using URL-safe base64 encoding
/// 
/// This implementation is secure against collision attacks even when attackers
/// can control dependency IDs, by using proper encoding that eliminates delimiter confusion.
/// 
/// # Security
/// 
/// The previous scheme was vulnerable to attacks like:
/// - dependency_id='4:evil' + original_key='attack' vs dependency_id='4' + original_key='evil:attack'
/// 
/// This new scheme uses base64url encoding to make collisions cryptographically infeasible.
/// 
/// # Examples
/// 
/// ```rust
/// use the_dagwood::utils::metadata::create_namespaced_key;
/// 
/// // Normal case
/// let key1 = create_namespaced_key("proc1", "result");
/// let key2 = create_namespaced_key("proc2", "result");
/// assert_ne!(key1, key2);
/// 
/// // Attack-resistant: these cannot collide even with malicious dependency IDs
/// let key3 = create_namespaced_key("4:evil", "attack");
/// let key4 = create_namespaced_key("4", "evil:attack");
/// assert_ne!(key3, key4);
/// ```
pub fn create_namespaced_key(dependency_id: &str, original_key: &str) -> String {
    // Use URL-safe base64 encoding to eliminate any possibility of delimiter confusion
    let encoded_dep_id = base64_url_safe_encode(dependency_id.as_bytes());
    let encoded_key = base64_url_safe_encode(original_key.as_bytes());
    format!("dep.{}.{}", encoded_dep_id, encoded_key)
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
/// Combined metadata with dependency keys namespaced using secure base64 encoding
pub fn merge_metadata_from_responses(
    base_metadata: HashMap<String, String>,
    dependency_responses: &HashMap<String, ProcessorResponse>,
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
        
        // Verify dependency metadata is namespaced correctly using new secure format
        let proc1_result_key = create_namespaced_key("processor1", "result");
        let proc1_count_key = create_namespaced_key("processor1", "count");
        let proc2_result_key = create_namespaced_key("processor2", "result");
        let proc2_time_key = create_namespaced_key("processor2", "time");
        
        assert_eq!(merged.get(&proc1_result_key), Some(&"success".to_string()));
        assert_eq!(merged.get(&proc1_count_key), Some(&"42".to_string()));
        assert_eq!(merged.get(&proc2_result_key), Some(&"completed".to_string()));
        assert_eq!(merged.get(&proc2_time_key), Some(&"100ms".to_string()));
        
        // Verify no key conflicts (both dependencies had "result" key)
        assert!(merged.contains_key(&proc1_result_key));
        assert!(merged.contains_key(&proc2_result_key));
        assert_ne!(merged.get(&proc1_result_key), merged.get(&proc2_result_key));
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
        
        // Find the namespaced key (it will be base64 encoded)
        let expected_key = create_namespaced_key("dep1", "key");
        assert_eq!(merged.get(&expected_key), Some(&"value".to_string()));
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn test_merge_metadata_key_override() {
        let mut base = HashMap::new();
        // Use the actual namespaced key format
        let namespaced_key = create_namespaced_key("dep1", "result");
        base.insert(namespaced_key.clone(), "original".to_string());
        
        let mut dependency_metadata = HashMap::new();
        let mut dep_meta = HashMap::new();
        dep_meta.insert("result".to_string(), "from_dependency".to_string());
        dependency_metadata.insert("dep1".to_string(), dep_meta);
        
        let merged = merge_metadata_with_prefixes(base, &dependency_metadata);
        
        // Dependency metadata should override base metadata with same namespaced key
        assert_eq!(merged.get(&namespaced_key), Some(&"from_dependency".to_string()));
    }

    #[test]
    fn test_create_namespaced_key() {
        // Test that keys are generated in the new secure format
        let key1 = create_namespaced_key("proc1", "result");
        let key2 = create_namespaced_key("processor", "data");
        
        // Verify format: dep.{base64_dep_id}.{base64_key}
        assert!(key1.starts_with("dep."));
        assert!(key2.starts_with("dep."));
        assert_eq!(key1.matches('.').count(), 2);
        assert_eq!(key2.matches('.').count(), 2);
        
        // Verify different inputs produce different keys
        assert_ne!(key1, key2);
        
        // Edge cases that would cause collisions with old scheme are now safe
        let key3 = create_namespaced_key("user_profile", "data");
        let key4 = create_namespaced_key("user", "profile_data");
        assert_ne!(key3, key4);
        
        // Test attack resistance - these would collide in vulnerable schemes
        let attack1 = create_namespaced_key("4:evil", "attack");
        let attack2 = create_namespaced_key("4", "evil:attack");
        assert_ne!(attack1, attack2);
        
        // Verify colon-containing IDs produce different keys
        let colon1 = create_namespaced_key("ns:proc", "key");
        let colon2 = create_namespaced_key("ns", "proc:key");
        assert_ne!(colon1, colon2);
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
