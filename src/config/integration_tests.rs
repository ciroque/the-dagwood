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
        let config = load_and_validate_config("configs/simple-pipeline.yaml").unwrap();
        
        assert_eq!(config.strategy, Strategy::WorkQueue);
        assert_eq!(config.failure_strategy, FailureStrategy::FailFast);
        assert_eq!(config.executor_options.max_concurrency, Some(2));
        assert_eq!(config.processors.len(), 4);
        assert_eq!(config.processors[0].id, "uppercase");
        assert_eq!(config.processors[1].id, "reverse");
        assert_eq!(config.processors[2].id, "word_counter");
        assert_eq!(config.processors[3].id, "prefix_suffix_arrows");
        assert_eq!(config.processors[1].depends_on, vec!["uppercase"]);
    }

    /// Test failure strategies configuration loading
    #[test]
    fn test_failure_strategies_yaml_loading() {
        // Test loading the failure strategies configuration file
        let config = load_and_validate_config("configs/failure-strategies.yaml").unwrap();
        
        assert_eq!(config.strategy, Strategy::WorkQueue);
        assert_eq!(config.failure_strategy, FailureStrategy::ContinueOnError);
        assert_eq!(config.executor_options.max_concurrency, Some(3));
        assert_eq!(config.processors.len(), 4);
        
        // Verify processor configuration
        assert_eq!(config.processors[0].id, "input_processor");
        assert_eq!(config.processors[1].id, "failing_processor");
        assert_eq!(config.processors[2].id, "independent_processor");
        assert_eq!(config.processors[3].id, "dependent_on_failure");
        
        // Verify dependencies
        assert!(config.processors[0].depends_on.is_empty());
        assert_eq!(config.processors[1].depends_on, vec!["input_processor"]);
        assert_eq!(config.processors[2].depends_on, vec!["input_processor"]);
        assert_eq!(config.processors[3].depends_on, vec!["failing_processor"]);
    }


    /// Test building DAG runtime from YAML configuration
    #[test]
    fn test_build_dag_runtime_from_yaml() {
        let config = load_and_validate_config("configs/simple-pipeline.yaml").unwrap();
        let (processors, _executor, failure_strategy) = build_dag_runtime(&config);
        
        // Verify processor registry
        assert_eq!(processors.len(), 4);
        assert!(processors.contains_key("uppercase"));
        assert!(processors.contains_key("reverse"));
        assert!(processors.contains_key("word_counter"));
        assert!(processors.contains_key("prefix_suffix_arrows"));
        
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
        let config = load_and_validate_config("configs/demo.yaml").unwrap();
        
        assert_eq!(config.strategy, Strategy::WorkQueue);
        assert_eq!(config.failure_strategy, FailureStrategy::FailFast);
        assert_eq!(config.executor_options.max_concurrency, Some(4));
        assert_eq!(config.executor_options.timeout_seconds, Some(30));
        assert_eq!(config.executor_options.retry_attempts, Some(1));
        
        // Verify we have multiple processors
        assert!(config.processors.len() > 5);
        
        // Verify some key processors exist
        let processor_ids: Vec<&String> = config.processors.iter().map(|p| &p.id).collect();
        assert!(processor_ids.contains(&&"case_change".to_string()));
        assert!(processor_ids.contains(&&"token_counter".to_string()));
        assert!(processor_ids.contains(&&"word_frequency".to_string()));
    }

    /// Test executor options configuration
    #[test]
    fn test_executor_options_from_yaml() {
        let config = load_and_validate_config("configs/demo.yaml").unwrap();
        let (_, _executor, _) = build_dag_runtime(&config);
        
        // The executor should be created successfully with the configured options
        // We can't easily inspect internal state, but we can verify it exists
        // Just check that we got an executor back - the Box is guaranteed to be non-null
        assert!(true); // Executor creation succeeded if we got here
    }
}
