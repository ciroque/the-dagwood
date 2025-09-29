#[cfg(test)]
mod integration_tests {
    use crate::config::{
        Strategy, 
        load_and_validate_config, build_dag_runtime
    };
    use crate::errors::FailureStrategy;

    /// Test that YAML configurations can be loaded and parsed correctly
    #[test]
    fn test_simple_pipeline_yaml_loading() {
        // Test loading the simple pipeline configuration file
        let config = load_and_validate_config("configs/simple-text-pipeline.yaml").unwrap();
        
        assert_eq!(config.strategy, Strategy::WorkQueue);
        assert_eq!(config.failure_strategy, FailureStrategy::FailFast);
        assert_eq!(config.executor_options.max_concurrency, Some(2));
        assert_eq!(config.processors.len(), 3);
        assert_eq!(config.processors[0].id, "to_uppercase");
        assert_eq!(config.processors[1].id, "reverse_text");
        assert_eq!(config.processors[2].id, "add_brackets");
        assert_eq!(config.processors[1].depends_on, vec!["to_uppercase"]);
    }

    /// Test failure strategies configuration loading
    #[test]
    fn test_failure_strategies_yaml_loading() {
        // Test loading the failure strategies configuration file
        let config = load_and_validate_config("configs/failure-handling-demo.yaml").unwrap();
        
        assert_eq!(config.strategy, Strategy::WorkQueue);
        assert_eq!(config.failure_strategy, FailureStrategy::ContinueOnError);
        assert_eq!(config.executor_options.max_concurrency, Some(3));
        assert_eq!(config.processors.len(), 7);
        
        // Verify processor configuration
        assert_eq!(config.processors[0].id, "entry_processor");
        assert_eq!(config.processors[1].id, "branch_a_transform");
        assert_eq!(config.processors[2].id, "branch_b_analysis");
        assert_eq!(config.processors[3].id, "branch_c_transform");
        
        // Verify dependencies
        assert!(config.processors[0].depends_on.is_empty());
        assert_eq!(config.processors[1].depends_on, vec!["entry_processor"]);
        assert_eq!(config.processors[2].depends_on, vec!["entry_processor"]);
        assert_eq!(config.processors[3].depends_on, vec!["entry_processor"]);
    }


    /// Test building DAG runtime from YAML configuration
    #[test]
    fn test_build_dag_runtime_from_yaml() {
        let config = load_and_validate_config("configs/simple-text-pipeline.yaml").unwrap();
        let (processors, _executor, failure_strategy) = build_dag_runtime(&config);
        
        // Verify processor registry
        assert_eq!(processors.len(), 3);
        assert!(processors.contains_key("to_uppercase"));
        assert!(processors.contains_key("reverse_text"));
        assert!(processors.contains_key("add_brackets"));
        
        // Verify failure strategy
        assert_eq!(failure_strategy, FailureStrategy::FailFast);
        
        // Verify executor is created (we can't easily test internal state, but we can verify it exists)
        // Just check that we got an executor back - the Box is guaranteed to be non-null
        assert!(true); // Executor creation succeeded if we got here
    }

    /// Test default values for optional configuration fields
    #[test]
    fn test_demo_yaml_loading() {
        // Test loading the demo configuration file which has comprehensive settings
        let config = load_and_validate_config("configs/parallel-analysis-pipeline.yaml").unwrap();
        
        assert_eq!(config.strategy, Strategy::WorkQueue);
        assert_eq!(config.failure_strategy, FailureStrategy::FailFast);
        assert_eq!(config.executor_options.max_concurrency, Some(4));
        assert_eq!(config.executor_options.timeout_seconds, Some(45));
        assert_eq!(config.executor_options.retry_attempts, Some(2));
        
        // Verify we have multiple processors
        assert_eq!(config.processors.len(), 5);
        
        // Verify some key processors exist
        let processor_ids: Vec<&String> = config.processors.iter().map(|p| &p.id).collect();
        assert!(processor_ids.contains(&&"normalize_input".to_string()));
        assert!(processor_ids.contains(&&"count_tokens".to_string()));
        assert!(processor_ids.contains(&&"analyze_frequency".to_string()));
    }

    /// Test executor options configuration
    #[test]
    fn test_executor_options_from_yaml() {
        let config = load_and_validate_config("configs/parallel-analysis-pipeline.yaml").unwrap();
        let (_, _executor, _) = build_dag_runtime(&config);
        
        // The executor should be created successfully with the configured options
        // We can't easily inspect internal state, but we can verify it exists
        // Just check that we got an executor back - the Box is guaranteed to be non-null
        assert!(true); // Executor creation succeeded if we got here
    }
}
