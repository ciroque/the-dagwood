use super::*;
use crate::config::{ProcessorConfig, BackendType, ConflictResolution};
use crate::backends::local::factory::LocalProcessorFactory;
use crate::proto::processor_v1::{ProcessorRequest, processor_response::Outcome};
use std::collections::HashMap;
use serde_json;

/// Helper to create a test ProcessorRequest with serialized dependency results
fn create_collector_request(dependency_results: HashMap<String, CollectableResult>) -> ProcessorRequest {
    let serialized = serde_json::to_vec(&dependency_results).unwrap();
    ProcessorRequest {
        payload: serialized,
        metadata: HashMap::new(),
    }
}

/// Helper to create a successful CollectableResult
fn success_result(payload: &str) -> CollectableResult {
    CollectableResult {
        success: true,
        payload: Some(payload.as_bytes().to_vec()),
        error_code: None,
        error_message: None,
    }
}

/// Helper to create an error CollectableResult
fn error_result(code: i32, message: &str) -> CollectableResult {
    CollectableResult {
        success: false,
        payload: None,
        error_code: Some(code),
        error_message: Some(message.to_string()),
    }
}

#[tokio::test]
async fn test_first_available_collector_direct() {
    let collector = FirstAvailableCollector::new();
    
    let mut dependency_results = HashMap::new();
    dependency_results.insert("dep1".to_string(), error_result(400, "Failed"));
    dependency_results.insert("dep2".to_string(), success_result("success_data"));
    
    let request = create_collector_request(dependency_results);
    let response = collector.process(request).await;
    
    if let Some(Outcome::NextPayload(payload)) = response.outcome {
        assert_eq!(String::from_utf8(payload).unwrap(), "success_data");
    } else {
        panic!("Expected NextPayload outcome");
    }
}

#[tokio::test]
async fn test_first_available_collector_via_factory() {
    let config = ProcessorConfig {
        id: "test_collector".to_string(),
        backend: BackendType::Local,
        processor: Some("first_available_collector".to_string()),
        endpoint: None,
        module: None,
        depends_on: vec![],
        collection_strategy: None,
        options: HashMap::new(),
    };
    
    let processor = LocalProcessorFactory::create_processor(&config).unwrap();
    
    let mut dependency_results = HashMap::new();
    dependency_results.insert("dep1".to_string(), success_result("first_success"));
    dependency_results.insert("dep2".to_string(), success_result("second_success"));
    
    let request = create_collector_request(dependency_results);
    let response = processor.process(request).await;
    
    if let Some(Outcome::NextPayload(payload)) = response.outcome {
        let result = String::from_utf8(payload).unwrap();
        // Should get one of the success results
        assert!(result == "first_success" || result == "second_success");
    } else {
        panic!("Expected NextPayload outcome");
    }
}

#[tokio::test]
async fn test_metadata_merge_collector_direct() {
    let collector = MetadataMergeCollector::new(
        "primary".to_string(),
        vec!["secondary".to_string()],
    );
    
    let mut dependency_results = HashMap::new();
    dependency_results.insert("primary".to_string(), success_result("primary_data"));
    dependency_results.insert("secondary".to_string(), success_result("secondary_data"));
    
    let request = create_collector_request(dependency_results);
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
async fn test_metadata_merge_collector_via_factory() {
    let mut options = HashMap::new();
    options.insert("primary_source".to_string(), serde_yaml::Value::String("main".to_string()));
    options.insert("metadata_sources".to_string(), serde_yaml::Value::Sequence(vec![
        serde_yaml::Value::String("analysis".to_string()),
    ]));
    
    let config = ProcessorConfig {
        id: "test_collector".to_string(),
        backend: BackendType::Local,
        processor: Some("metadata_merge_collector".to_string()),
        endpoint: None,
        module: None,
        depends_on: vec![],
        collection_strategy: None,
        options,
    };
    
    let processor = LocalProcessorFactory::create_processor(&config).unwrap();
    
    let mut dependency_results = HashMap::new();
    dependency_results.insert("main".to_string(), success_result("main_content"));
    dependency_results.insert("analysis".to_string(), success_result("analysis_result"));
    
    let request = create_collector_request(dependency_results);
    let response = processor.process(request).await;
    
    if let Some(Outcome::NextPayload(payload)) = response.outcome {
        let result_str = String::from_utf8(payload).unwrap();
        let result_json: serde_json::Value = serde_json::from_str(&result_str).unwrap();
        
        assert!(result_json.get("primary_payload").is_some());
        assert!(result_json.get("metadata").is_some());
    } else {
        panic!("Expected NextPayload outcome");
    }
}

#[tokio::test]
async fn test_concatenate_collector_direct() {
    let collector = ConcatenateCollector::new(Some(" | ".to_string()));
    
    let mut dependency_results = HashMap::new();
    dependency_results.insert("dep1".to_string(), success_result("data1"));
    dependency_results.insert("dep2".to_string(), success_result("data2"));
    
    let request = create_collector_request(dependency_results);
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
async fn test_concatenate_collector_via_factory() {
    let mut options = HashMap::new();
    options.insert("separator".to_string(), serde_yaml::Value::String(" -> ".to_string()));
    
    let config = ProcessorConfig {
        id: "test_collector".to_string(),
        backend: BackendType::Local,
        processor: Some("concatenate_collector".to_string()),
        endpoint: None,
        module: None,
        depends_on: vec![],
        collection_strategy: None,
        options,
    };
    
    let processor = LocalProcessorFactory::create_processor(&config).unwrap();
    
    let mut dependency_results = HashMap::new();
    dependency_results.insert("a".to_string(), success_result("first"));
    dependency_results.insert("b".to_string(), success_result("second"));
    
    let request = create_collector_request(dependency_results);
    let response = processor.process(request).await;
    
    if let Some(Outcome::NextPayload(payload)) = response.outcome {
        let result = String::from_utf8(payload).unwrap();
        // Should be deterministically ordered by dependency ID
        assert_eq!(result, "first -> second");
    } else {
        panic!("Expected NextPayload outcome");
    }
}

#[tokio::test]
async fn test_json_merge_collector_direct() {
    let collector = JsonMergeCollector::new(true, ConflictResolution::Merge);
    
    let mut dependency_results = HashMap::new();
    dependency_results.insert("dep1".to_string(), success_result(r#"{"count": 5, "items": ["a"]}"#));
    dependency_results.insert("dep2".to_string(), success_result(r#"{"total": 10, "items": ["b"]}"#));
    
    let request = create_collector_request(dependency_results);
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
async fn test_json_merge_collector_via_factory() {
    let mut options = HashMap::new();
    options.insert("merge_arrays".to_string(), serde_yaml::Value::Bool(false));
    options.insert("conflict_resolution".to_string(), serde_yaml::Value::String("take_last".to_string()));
    
    let config = ProcessorConfig {
        id: "test_collector".to_string(),
        backend: BackendType::Local,
        processor: Some("json_merge_collector".to_string()),
        endpoint: None,
        module: None,
        depends_on: vec![],
        collection_strategy: None,
        options,
    };
    
    let processor = LocalProcessorFactory::create_processor(&config).unwrap();
    
    let mut dependency_results = HashMap::new();
    dependency_results.insert("dep1".to_string(), success_result(r#"{"value": "first"}"#));
    dependency_results.insert("dep2".to_string(), success_result(r#"{"value": "second"}"#));
    
    let request = create_collector_request(dependency_results);
    let response = processor.process(request).await;
    
    if let Some(Outcome::NextPayload(payload)) = response.outcome {
        let result = String::from_utf8(payload).unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        
        // With take_last conflict resolution, should have the second value
        assert_eq!(json.get("value").unwrap().as_str().unwrap(), "second");
    } else {
        panic!("Expected NextPayload outcome");
    }
}

#[tokio::test]
async fn test_custom_collector_via_factory() {
    let mut options = HashMap::new();
    options.insert("combiner_impl".to_string(), serde_yaml::Value::String("test_combiner".to_string()));
    
    let config = ProcessorConfig {
        id: "test_collector".to_string(),
        backend: BackendType::Local,
        processor: Some("custom_collector".to_string()),
        endpoint: None,
        module: None,
        depends_on: vec![],
        collection_strategy: None,
        options,
    };
    
    let processor = LocalProcessorFactory::create_processor(&config).unwrap();
    
    let mut dependency_results = HashMap::new();
    dependency_results.insert("dep1".to_string(), success_result("data"));
    
    let request = create_collector_request(dependency_results);
    let response = processor.process(request).await;
    
    // Custom collector should return an error since it's not implemented yet
    if let Some(Outcome::Error(error)) = response.outcome {
        assert!(error.message.contains("not implemented yet"));
    } else {
        panic!("Expected Error outcome for unimplemented custom collector");
    }
}

#[test]
fn test_factory_missing_required_options() {
    // Test metadata_merge_collector without primary_source
    let config = ProcessorConfig {
        id: "test_collector".to_string(),
        backend: BackendType::Local,
        processor: Some("metadata_merge_collector".to_string()),
        endpoint: None,
        module: None,
        depends_on: vec![],
        collection_strategy: None,
        options: HashMap::new(),
    };
    
    let result = LocalProcessorFactory::create_processor(&config);
    assert!(result.is_err());
    assert!(result.err().unwrap().contains("requires 'primary_source' option"));
}

#[test]
fn test_factory_list_includes_new_collectors() {
    let implementations = LocalProcessorFactory::list_available_implementations();
    
    assert!(implementations.contains(&"first_available_collector"));
    assert!(implementations.contains(&"metadata_merge_collector"));
    assert!(implementations.contains(&"concatenate_collector"));
    assert!(implementations.contains(&"json_merge_collector"));
    assert!(implementations.contains(&"custom_collector"));
}
