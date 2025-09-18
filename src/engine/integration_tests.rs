use std::collections::HashMap;
use std::sync::Arc;

use crate::backends::local::factory::LocalProcessorFactory;
use crate::config::{BackendType, ProcessorConfig};
use crate::engine::WorkQueueExecutor;
use crate::proto::processor_v1::processor_response::Outcome;
use crate::proto::processor_v1::ProcessorRequest;
use crate::traits::{DagExecutor, Processor};

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
            impl_: Some("change_text_case_upper".to_string()),
            endpoint: None,
            module: None,
            depends_on: vec![],
        };
        
        let reverse_config = ProcessorConfig {
            id: "reverse".to_string(),
            backend: BackendType::Local,
            impl_: Some("reverse_text".to_string()),
            endpoint: None,
            module: None,
            depends_on: vec!["uppercase".to_string()],
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
        let results = executor.execute(processors, graph, entrypoints, input).await;
        
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
                impl_: Some("change_text_case_upper".to_string()),
                endpoint: None,
                module: None,
                depends_on: vec![],
            },
            ProcessorConfig {
                id: "lowercase".to_string(),
                backend: BackendType::Local,
                impl_: Some("change_text_case_lower".to_string()),
                endpoint: None,
                module: None,
                depends_on: vec![],
            },
            ProcessorConfig {
                id: "token_counter".to_string(),
                backend: BackendType::Local,
                impl_: Some("token_counter".to_string()),
                endpoint: None,
                module: None,
                depends_on: vec!["uppercase".to_string(), "lowercase".to_string()],
            },
            ProcessorConfig {
                id: "word_frequency".to_string(),
                backend: BackendType::Local,
                impl_: Some("word_frequency_analyzer".to_string()),
                endpoint: None,
                module: None,
                depends_on: vec!["token_counter".to_string()],
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
        let results = executor.execute(processors, graph, entrypoints, input).await;
        
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
        let results = executor.execute(processors, graph, entrypoints, input).await;
        
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
}
