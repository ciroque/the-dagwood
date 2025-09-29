use std::collections::HashMap;
use std::sync::Arc;

use crate::backends::local::factory::LocalProcessorFactory;
use crate::config::{BackendType, ProcessorConfig};
use crate::engine::WorkQueueExecutor;
use crate::proto::processor_v1::processor_response::Outcome;
use crate::proto::processor_v1::ProcessorRequest;
use crate::traits::{DagExecutor, Processor, ProcessorMap, DependencyGraph, EntryPoints};

/// Integration tests for the Work Queue executor using real local processors
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_change_text_case_to_reverse_text_pipeline() {
        let executor = WorkQueueExecutor::new(2);
        
        // Create processor configurations
        let uppercase_config = ProcessorConfig {
            id: "uppercase".to_string(),
            backend: BackendType::Local,
            processor: Some("change_text_case_upper".to_string()),
            endpoint: None,
            module: None,
            depends_on: vec![],
            options: HashMap::new(),
        };
        
        let reverse_config = ProcessorConfig {
            id: "reverse".to_string(),
            backend: BackendType::Local,
            processor: Some("reverse_text".to_string()),
            endpoint: None,
            module: None,
            depends_on: vec!["uppercase".to_string()],
            options: HashMap::new(),
        };
        
        // Create processors using the factory
        let uppercase_processor = LocalProcessorFactory::create_processor(&uppercase_config)
            .expect("Failed to create uppercase processor");
        let reverse_processor = LocalProcessorFactory::create_processor(&reverse_config)
            .expect("Failed to create reverse processor");
        
        // Build the processor registry
        let mut processors = HashMap::new();
        processors.insert("uppercase".to_string(), uppercase_processor);
        processors.insert("reverse".to_string(), reverse_processor);
        
        // Build the dependency graph
        let graph = HashMap::from([
            ("uppercase".to_string(), vec!["reverse".to_string()]),
            ("reverse".to_string(), vec![]),
        ]);
        
        let entrypoints = vec!["uppercase".to_string()];
        let input = ProcessorRequest {
            payload: "hello world".to_string().into_bytes(),
            ..Default::default()
        };
        
        // Execute the DAG
        let results = executor.execute(ProcessorMap::from(processors), DependencyGraph::from(graph), EntryPoints::from(entrypoints), input).await.expect("DAG execution should succeed");
        
        // Verify results
        assert_eq!(results.len(), 2);
        
        // Check uppercase processor result
        let uppercase_result = results.get("uppercase").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &uppercase_result.outcome {
            assert_eq!(String::from_utf8(payload.clone()).unwrap(), "HELLO WORLD");
        } else {
            panic!("Expected NextPayload outcome for uppercase processor");
        }
        
        // Check reverse processor result
        let reverse_result = results.get("reverse").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &reverse_result.outcome {
            assert_eq!(String::from_utf8(payload.clone()).unwrap(), "DLROW OLLEH");
        } else {
            panic!("Expected NextPayload outcome for reverse processor");
        }
    }

    #[tokio::test]
    async fn test_complex_text_processing_dag() {
        let executor = WorkQueueExecutor::new(4);
        
        // Create a more complex DAG:
        // input -> [uppercase, lowercase] -> token_counter -> word_frequency
        let configs = vec![
            ProcessorConfig {
                id: "uppercase".to_string(),
                backend: BackendType::Local,
                processor: Some("change_text_case_upper".to_string()),
                endpoint: None,
                module: None,
                depends_on: vec![],
                options: HashMap::new(),
            },
            ProcessorConfig {
                id: "lowercase".to_string(),
                backend: BackendType::Local,
                processor: Some("change_text_case_lower".to_string()),
                endpoint: None,
                module: None,
                depends_on: vec![],
                options: HashMap::new(),
            },
            ProcessorConfig {
                id: "token_counter".to_string(),
                backend: BackendType::Local,
                processor: Some("token_counter".to_string()),
                endpoint: None,
                module: None,
                depends_on: vec!["uppercase".to_string(), "lowercase".to_string()],
                options: HashMap::new(),
            },
            ProcessorConfig {
                id: "word_frequency".to_string(),
                backend: BackendType::Local,
                processor: Some("word_frequency_analyzer".to_string()),
                endpoint: None,
                module: None,
                depends_on: vec!["token_counter".to_string()],
                options: HashMap::new(),
            },
        ];
        
        // Create processors using the factory
        let mut processors = HashMap::new();
        for config in &configs {
            let processor = LocalProcessorFactory::create_processor(config)
                .expect(&format!("Failed to create processor: {}", config.id));
            processors.insert(config.id.clone(), processor);
        }
        
        // Build the dependency graph
        let graph = HashMap::from([
            ("uppercase".to_string(), vec!["token_counter".to_string()]),
            ("lowercase".to_string(), vec!["token_counter".to_string()]),
            ("token_counter".to_string(), vec!["word_frequency".to_string()]),
            ("word_frequency".to_string(), vec![]),
        ]);
        
        let entrypoints = vec!["uppercase".to_string(), "lowercase".to_string()];
        let input = ProcessorRequest {
            payload: "hello world hello rust".to_string().into_bytes(),
            ..Default::default()
        };
        
        // Execute the DAG
        let results = executor.execute(ProcessorMap::from(processors), DependencyGraph::from(graph), EntryPoints::from(entrypoints), input).await.expect("DAG execution should succeed");
        
        // Verify all processors executed
        assert_eq!(results.len(), 4);
        assert!(results.contains_key("uppercase"));
        assert!(results.contains_key("lowercase"));
        assert!(results.contains_key("token_counter"));
        assert!(results.contains_key("word_frequency"));
        
        // Verify each processor produced a NextPayload outcome
        for (processor_id, response) in &results {
            match &response.outcome {
                Some(Outcome::NextPayload(_)) => {
                    // Expected outcome
                }
                Some(Outcome::Error(error)) => {
                    panic!("Processor {} failed with error: {}", processor_id, error.message);
                }
                None => {
                    panic!("Processor {} produced no outcome", processor_id);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_prefix_suffix_adder_chain() {
        let executor = WorkQueueExecutor::new(2);
        
        // Create processors directly with specific configurations
        // Since the factory doesn't support custom config yet, we'll create them manually
        use crate::backends::local::processors::prefix_suffix_adder::{PrefixSuffixAdderProcessor, PrefixSuffixConfig};
        
        let prefix_processor = Arc::new(PrefixSuffixAdderProcessor::new(PrefixSuffixConfig {
            prefix: Some("[START] ".to_string()),
            suffix: None,
        })) as Arc<dyn Processor>;
        
        let suffix_processor = Arc::new(PrefixSuffixAdderProcessor::new(PrefixSuffixConfig {
            prefix: None,
            suffix: Some(" [END]".to_string()),
        })) as Arc<dyn Processor>;
        
        let mut processors = HashMap::new();
        processors.insert("add_prefix".to_string(), prefix_processor);
        processors.insert("add_suffix".to_string(), suffix_processor);
        
        // Build the dependency graph
        let graph = HashMap::from([
            ("add_prefix".to_string(), vec!["add_suffix".to_string()]),
            ("add_suffix".to_string(), vec![]),
        ]);
        
        let entrypoints = vec!["add_prefix".to_string()];
        let input = ProcessorRequest {
            payload: "Hello World".to_string().into_bytes(),
            ..Default::default()
        };
        
        // Execute the DAG
        let results = executor.execute(ProcessorMap::from(processors), DependencyGraph::from(graph), EntryPoints::from(entrypoints), input).await.expect("DAG execution should succeed");
        
        // Verify results
        assert_eq!(results.len(), 2);
        
        // Check final result
        let final_result = results.get("add_suffix").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &final_result.outcome {
            assert_eq!(String::from_utf8(payload.clone()).unwrap(), "[START] Hello World [END]");
        } else {
            panic!("Expected NextPayload outcome for final processor");
        }
    }

    #[tokio::test]
    async fn test_parallel_collection_with_first_available_strategy() {
        // Test the new parallel collection functionality
        // Create a DAG where a processor depends on multiple parallel processors
        let executor = WorkQueueExecutor::new(4);
        
        // Two parallel processors that both process the same input
        let upper_case = LocalProcessorFactory::create_processor(&ProcessorConfig {
            id: "upper_case".to_string(),
            backend: BackendType::Local,
            processor: Some("change_text_case_upper".to_string()),
            endpoint: None,
            module: None,
            depends_on: vec![],
            options: HashMap::new(),
        }).unwrap();
        
        let lower_case = LocalProcessorFactory::create_processor(&ProcessorConfig {
            id: "lower_case".to_string(),
            backend: BackendType::Local,
            processor: Some("change_text_case_lower".to_string()),
            endpoint: None,
            module: None,
            depends_on: vec![],
            options: HashMap::new(),
        }).unwrap();
        
        // A processor that depends on both parallel processors
        // This will trigger the ResultCollector with FirstAvailable strategy
        let reverse_text = LocalProcessorFactory::create_processor(&ProcessorConfig {
            id: "reverse_text".to_string(),
            backend: BackendType::Local,
            processor: Some("reverse_text".to_string()),
            endpoint: None,
            module: None,
            depends_on: vec!["upper_case".to_string(), "lower_case".to_string()],
            options: HashMap::new(),
        }).unwrap();
        
        let mut processors = HashMap::new();
        processors.insert("upper_case".to_string(), upper_case);
        processors.insert("lower_case".to_string(), lower_case);
        processors.insert("reverse_text".to_string(), reverse_text);
        
        // Build dependency graph
        let graph = HashMap::from([
            ("upper_case".to_string(), vec!["reverse_text".to_string()]),
            ("lower_case".to_string(), vec!["reverse_text".to_string()]),
            ("reverse_text".to_string(), vec![]),
        ]);
        
        let entrypoints = vec!["upper_case".to_string(), "lower_case".to_string()];
        let input = ProcessorRequest {
            payload: "Hello World".as_bytes().to_vec(),
            metadata: HashMap::new(),
        };
        
        let results = executor.execute(ProcessorMap::from(processors), DependencyGraph::from(graph), EntryPoints::from(entrypoints), input).await.expect("DAG execution should succeed");
        
        // Verify all processors executed successfully
        assert!(results.contains_key("upper_case"));
        assert!(results.contains_key("lower_case"));
        assert!(results.contains_key("reverse_text"));
        
        // Check that upper_case and lower_case both produced results
        let upper_result = results.get("upper_case").unwrap();
        let lower_result = results.get("lower_case").unwrap();
        
        if let Some(Outcome::NextPayload(payload)) = &upper_result.outcome {
            let upper_str = String::from_utf8(payload.clone()).unwrap();
            assert_eq!(upper_str, "HELLO WORLD");
        } else {
            panic!("Expected successful outcome from upper_case");
        }
        
        if let Some(Outcome::NextPayload(payload)) = &lower_result.outcome {
            let lower_str = String::from_utf8(payload.clone()).unwrap();
            assert_eq!(lower_str, "hello world");
        } else {
            panic!("Expected successful outcome from lower_case");
        }
        
        // Check that reverse_text received input from ResultCollector and processed it
        let reverse_result = results.get("reverse_text").unwrap();
        if let Some(Outcome::NextPayload(payload)) = &reverse_result.outcome {
            let reverse_str = String::from_utf8(payload.clone()).unwrap();
            // The ResultCollector should have used FirstAvailable strategy
            // So reverse_text should have received either "HELLO WORLD" or "hello world"
            // and reversed it
            assert!(reverse_str == "DLROW OLLEH" || reverse_str == "dlrow olleh");
            println!("Parallel collection result: {}", reverse_str);
        } else {
            panic!("Expected successful outcome from reverse_text");
        }
    }
}
