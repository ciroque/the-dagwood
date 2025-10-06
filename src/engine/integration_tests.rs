use std::collections::HashMap;
use std::sync::Arc;

use crate::backends::local::factory::LocalProcessorFactory;
use crate::config::{BackendType, ProcessorConfig};
use crate::engine::{WorkQueueExecutor, LevelByLevelExecutor, ReactiveExecutor};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::proto::processor_v1::ProcessorRequest;
use crate::traits::{DagExecutor, Processor};
use crate::config::{ProcessorMap, DependencyGraph, EntryPoints};

/// Integration tests for the Work Queue executor using real local processors
#[cfg(test)]
use serde_yaml::Value;

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

    /// Integration tests for the Level-by-Level executor using real local processors
    #[tokio::test]
    async fn test_level_by_level_change_text_case_to_reverse_text_pipeline() {
        let executor = LevelByLevelExecutor::new(2);
        
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
        let mut processors_map = HashMap::new();
        processors_map.insert("uppercase".to_string(), uppercase_processor);
        processors_map.insert("reverse".to_string(), reverse_processor);
        let processors = ProcessorMap(processors_map);
        
        // Build the dependency graph (forward dependencies: uppercase -> reverse)
        let mut graph_map = HashMap::new();
        graph_map.insert("uppercase".to_string(), vec!["reverse".to_string()]);
        graph_map.insert("reverse".to_string(), vec![]);
        let graph = DependencyGraph(graph_map);
        
        let entrypoints = EntryPoints(vec!["uppercase".to_string()]);
        
        // Create input
        let input = ProcessorRequest {
            payload: b"hello world".to_vec(),
            metadata: HashMap::new(),
        };
        
        // Execute the pipeline
        let result = executor.execute(processors, graph, entrypoints, input).await;
        assert!(result.is_ok(), "Pipeline execution failed: {:?}", result.err());
        
        let results = result.unwrap();
        assert_eq!(results.len(), 2);
        
        // Verify uppercase processor result
        let uppercase_result = results.get("uppercase").expect("uppercase result not found");
        if let Some(Outcome::NextPayload(payload)) = &uppercase_result.outcome {
            let output = String::from_utf8(payload.clone()).expect("Invalid UTF-8");
            assert_eq!(output, "HELLO WORLD");
        } else {
            panic!("Expected successful outcome from uppercase");
        }
        
        // Verify reverse processor result
        let reverse_result = results.get("reverse").expect("reverse result not found");
        if let Some(Outcome::NextPayload(payload)) = &reverse_result.outcome {
            let output = String::from_utf8(payload.clone()).expect("Invalid UTF-8");
            assert_eq!(output, "DLROW OLLEH");
        } else {
            panic!("Expected successful outcome from reverse_text");
        }
    }

    #[tokio::test]
    async fn test_level_by_level_diamond_dependency_with_real_processors() {
        let executor = LevelByLevelExecutor::new(4);
        
        // Create processor configurations for diamond pattern: case_change -> [token_counter, word_frequency] -> merge
        let case_change_config = ProcessorConfig {
            id: "case_change".to_string(),
            backend: BackendType::Local,
            processor: Some("change_text_case_upper".to_string()),
            endpoint: None,
            module: None,
            depends_on: vec![],
            options: HashMap::new(),
        };
        
        let token_counter_config = ProcessorConfig {
            id: "token_counter".to_string(),
            backend: BackendType::Local,
            processor: Some("token_counter".to_string()),
            endpoint: None,
            module: None,
            depends_on: vec!["case_change".to_string()],
            options: HashMap::new(),
        };
        
        let word_frequency_config = ProcessorConfig {
            id: "word_frequency".to_string(),
            backend: BackendType::Local,
            processor: Some("word_frequency_analyzer".to_string()),
            endpoint: None,
            module: None,
            depends_on: vec!["case_change".to_string()],
            options: HashMap::new(),
        };
        
        let prefix_suffix_config = ProcessorConfig {
            id: "prefix_suffix".to_string(),
            backend: BackendType::Local,
            processor: Some("prefix_suffix_adder".to_string()),
            endpoint: None,
            module: None,
            depends_on: vec!["token_counter".to_string(), "word_frequency".to_string()],
            options: {
                let mut opts = HashMap::new();
                opts.insert("prefix".to_string(), Value::String("PROCESSED: ".to_string()));
                opts.insert("suffix".to_string(), Value::String(" [DONE]".to_string()));
                opts
            },
        };
        
        // Create processors using the factory
        let case_change_processor = LocalProcessorFactory::create_processor(&case_change_config)
            .expect("Failed to create case_change processor");
        let token_counter_processor = LocalProcessorFactory::create_processor(&token_counter_config)
            .expect("Failed to create token_counter processor");
        let word_frequency_processor = LocalProcessorFactory::create_processor(&word_frequency_config)
            .expect("Failed to create word_frequency processor");
        let prefix_suffix_processor = LocalProcessorFactory::create_processor(&prefix_suffix_config)
            .expect("Failed to create prefix_suffix processor");
        
        // Build the processor registry
        let mut processors_map = HashMap::new();
        processors_map.insert("case_change".to_string(), case_change_processor);
        processors_map.insert("token_counter".to_string(), token_counter_processor);
        processors_map.insert("word_frequency".to_string(), word_frequency_processor);
        processors_map.insert("prefix_suffix".to_string(), prefix_suffix_processor);
        let processors = ProcessorMap(processors_map);
        
        // Build the dependency graph (diamond pattern - forward dependencies)
        let mut graph_map = HashMap::new();
        graph_map.insert("case_change".to_string(), vec!["token_counter".to_string(), "word_frequency".to_string()]);
        graph_map.insert("token_counter".to_string(), vec!["prefix_suffix".to_string()]);
        graph_map.insert("word_frequency".to_string(), vec!["prefix_suffix".to_string()]);
        graph_map.insert("prefix_suffix".to_string(), vec![]);
        let graph = DependencyGraph(graph_map);
        
        let entrypoints = EntryPoints(vec!["case_change".to_string()]);
        
        // Create input
        let input = ProcessorRequest {
            payload: b"hello world test".to_vec(),
            metadata: HashMap::new(),
        };
        
        // Execute the pipeline
        let result = executor.execute(processors, graph, entrypoints, input).await;
        assert!(result.is_ok(), "Pipeline execution failed: {:?}", result.err());
        
        let results = result.unwrap();
        assert_eq!(results.len(), 4);
        
        // Verify all processors completed successfully
        assert!(results.contains_key("case_change"));
        assert!(results.contains_key("token_counter"));
        assert!(results.contains_key("word_frequency"));
        assert!(results.contains_key("prefix_suffix"));
        
        // Verify final result has prefix and suffix
        let final_result = results.get("prefix_suffix").expect("prefix_suffix result not found");
        if let Some(Outcome::NextPayload(payload)) = &final_result.outcome {
            let output = String::from_utf8(payload.clone()).expect("Invalid UTF-8");
            assert!(output.starts_with("PROCESSED: "));
            assert!(output.ends_with(" [DONE]"));
            // Should contain the uppercase text from case_change processor
            assert!(output.contains("HELLO WORLD TEST"));
        } else {
            panic!("Expected successful outcome from prefix_suffix");
        }
    }

    #[tokio::test]
    async fn test_level_by_level_multiple_entrypoints() {
        let executor = LevelByLevelExecutor::new(4);
        
        // Create two entry processors and one merge processor
        let entry1_config = ProcessorConfig {
            id: "entry1".to_string(),
            backend: BackendType::Local,
            processor: Some("prefix_suffix_adder".to_string()),
            endpoint: None,
            module: None,
            depends_on: vec![],
            options: {
                let mut opts = HashMap::new();
                opts.insert("prefix".to_string(), Value::String("ENTRY1: ".to_string()));
                opts.insert("suffix".to_string(), Value::String("".to_string()));
                opts
            },
        };
        
        let entry2_config = ProcessorConfig {
            id: "entry2".to_string(),
            backend: BackendType::Local,
            processor: Some("prefix_suffix_adder".to_string()),
            endpoint: None,
            module: None,
            depends_on: vec![],
            options: {
                let mut opts = HashMap::new();
                opts.insert("prefix".to_string(), Value::String("ENTRY2: ".to_string()));
                opts.insert("suffix".to_string(), Value::String("".to_string()));
                opts
            },
        };
        
        let merge_config = ProcessorConfig {
            id: "merge".to_string(),
            backend: BackendType::Local,
            processor: Some("token_counter".to_string()),
            endpoint: None,
            module: None,
            depends_on: vec!["entry1".to_string(), "entry2".to_string()],
            options: HashMap::new(),
        };
        
        // Create processors using the factory
        let entry1_processor = LocalProcessorFactory::create_processor(&entry1_config)
            .expect("Failed to create entry1 processor");
        let entry2_processor = LocalProcessorFactory::create_processor(&entry2_config)
            .expect("Failed to create entry2 processor");
        let merge_processor = LocalProcessorFactory::create_processor(&merge_config)
            .expect("Failed to create merge processor");
        
        // Build the processor registry
        let mut processors_map = HashMap::new();
        processors_map.insert("entry1".to_string(), entry1_processor);
        processors_map.insert("entry2".to_string(), entry2_processor);
        processors_map.insert("merge".to_string(), merge_processor);
        let processors = ProcessorMap(processors_map);
        
        // Build the dependency graph (forward dependencies: [entry1, entry2] -> merge)
        let mut graph_map = HashMap::new();
        graph_map.insert("entry1".to_string(), vec!["merge".to_string()]);
        graph_map.insert("entry2".to_string(), vec!["merge".to_string()]);
        graph_map.insert("merge".to_string(), vec![]);
        let graph = DependencyGraph(graph_map);
        
        let entrypoints = EntryPoints(vec!["entry1".to_string(), "entry2".to_string()]);
        
        // Create input
        let input = ProcessorRequest {
            payload: b"test input".to_vec(),
            metadata: HashMap::new(),
        };
        
        // Execute the pipeline
        let result = executor.execute(processors, graph, entrypoints, input).await;
        assert!(result.is_ok(), "Pipeline execution failed: {:?}", result.err());
        
        let results = result.unwrap();
        assert_eq!(results.len(), 3);
        
        // Verify all processors completed successfully
        assert!(results.contains_key("entry1"));
        assert!(results.contains_key("entry2"));
        assert!(results.contains_key("merge"));
        
        // The merge processor should receive the canonical payload and have metadata from both entries
        let merge_result = results.get("merge").expect("merge result not found");
        assert!(merge_result.outcome.is_some());
        
        // Check that merge processor has metadata from both dependencies
        assert!(!merge_result.metadata.is_empty(), "Merge processor should have metadata from dependencies");
    }

    /// Test that compares all three executors (WorkQueue, LevelByLevel, Reactive) 
    /// with the same DAG to ensure they produce identical results
    #[tokio::test]
    async fn test_executor_comparison_identical_results() {
        // Create processor configurations for a simple linear pipeline
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
        
        // Helper function to create processor map
        let create_processor_map = || -> ProcessorMap {
            let uppercase_processor = LocalProcessorFactory::create_processor(&uppercase_config)
                .expect("Failed to create uppercase processor");
            let reverse_processor = LocalProcessorFactory::create_processor(&reverse_config)
                .expect("Failed to create reverse processor");
            
            let mut processors: HashMap<String, Arc<dyn Processor>> = HashMap::new();
            processors.insert("uppercase".to_string(), uppercase_processor);
            processors.insert("reverse".to_string(), reverse_processor);
            
            ProcessorMap(processors)
        };
        
        // Create dependency graph
        let mut graph_map = HashMap::new();
        graph_map.insert("uppercase".to_string(), vec!["reverse".to_string()]);
        graph_map.insert("reverse".to_string(), vec![]);
        let graph = DependencyGraph(graph_map);
        
        let entrypoints = EntryPoints(vec!["uppercase".to_string()]);
        
        // Create input
        let input = ProcessorRequest {
            payload: b"hello world".to_vec(),
            metadata: HashMap::new(),
        };
        
        // Execute with WorkQueue executor
        let work_queue_executor = WorkQueueExecutor::new(2);
        let work_queue_result = work_queue_executor
            .execute_with_strategy(
                create_processor_map(),
                graph.clone(),
                entrypoints.clone(),
                input.clone(),
                crate::errors::FailureStrategy::FailFast,
            )
            .await
            .expect("WorkQueue execution failed");
        
        // Execute with LevelByLevel executor  
        let level_executor = LevelByLevelExecutor::new(2);
        let level_result = level_executor
            .execute_with_strategy(
                create_processor_map(),
                graph.clone(),
                entrypoints.clone(),
                input.clone(),
                crate::errors::FailureStrategy::FailFast,
            )
            .await
            .expect("LevelByLevel execution failed");
        
        // Execute with Reactive executor
        let reactive_executor = ReactiveExecutor::new(2);
        let reactive_result = reactive_executor
            .execute_with_strategy(
                create_processor_map(),
                graph.clone(),
                entrypoints.clone(),
                input.clone(),
                crate::errors::FailureStrategy::FailFast,
            )
            .await
            .expect("Reactive execution failed");
        
        // Verify all executors produced the same results
        assert_eq!(work_queue_result.len(), 2);
        assert_eq!(level_result.len(), 2);
        assert_eq!(reactive_result.len(), 2);
        
        // Check that all executors have the same processor results
        for processor_id in ["uppercase", "reverse"] {
            let work_queue_response = work_queue_result.get(processor_id)
                .unwrap_or_else(|| panic!("WorkQueue missing {}", processor_id));
            let level_response = level_result.get(processor_id)
                .unwrap_or_else(|| panic!("LevelByLevel missing {}", processor_id));
            let reactive_response = reactive_result.get(processor_id)
                .unwrap_or_else(|| panic!("Reactive missing {}", processor_id));
            
            // Compare payloads
            if let (Some(Outcome::NextPayload(wq_payload)), 
                    Some(Outcome::NextPayload(level_payload)),
                    Some(Outcome::NextPayload(reactive_payload))) = 
                (&work_queue_response.outcome, &level_response.outcome, &reactive_response.outcome) {
                assert_eq!(wq_payload, level_payload, 
                    "WorkQueue and LevelByLevel payloads differ for {}", processor_id);
                assert_eq!(wq_payload, reactive_payload, 
                    "WorkQueue and Reactive payloads differ for {}", processor_id);
                assert_eq!(level_payload, reactive_payload, 
                    "LevelByLevel and Reactive payloads differ for {}", processor_id);
            } else {
                panic!("One or more executors failed to produce NextPayload for {}", processor_id);
            }
        }
        
        // Verify the expected transformation: "hello world" -> "HELLO WORLD" -> "DLROW OLLEH"
        let final_result = reactive_result.get("reverse").unwrap();
        if let Some(Outcome::NextPayload(final_payload)) = &final_result.outcome {
            let final_text = String::from_utf8_lossy(final_payload);
            assert_eq!(final_text, "DLROW OLLEH", "Final result should be reversed uppercase text");
        }
        
        println!("✅ All three executors (WorkQueue, LevelByLevel, Reactive) produced identical results!");
        println!("   Input: 'hello world' -> Output: 'DLROW OLLEH'");
    }
}
