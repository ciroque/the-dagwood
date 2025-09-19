use std::sync::Arc;

use crate::traits::Processor;
use crate::config::{ProcessorConfig, CollectionStrategy};
use super::processors::*;

/// Factory for creating local (in-process) processor instances
pub struct LocalProcessorFactory;

impl LocalProcessorFactory {
    /// Create a processor instance from configuration
    /// 
    /// The `impl_` field in the config determines which processor to create:
    /// - "change_text_case_upper" -> ChangeTextCaseProcessor (uppercase)
    /// - "change_text_case_lower" -> ChangeTextCaseProcessor (lowercase)
    /// - "change_text_case_proper" -> ChangeTextCaseProcessor (proper case)
    /// - "change_text_case_title" -> ChangeTextCaseProcessor (title case)
    /// - "reverse_text" -> ReverseTextProcessor
    /// - "token_counter" -> TokenCounterProcessor
    /// - "word_frequency_analyzer" -> WordFrequencyAnalyzerProcessor
    /// - "prefix_suffix_adder" -> PrefixSuffixAdderProcessor (requires additional config)
    pub fn create_processor(config: &ProcessorConfig) -> Result<Arc<dyn Processor>, String> {
        let impl_name = config.impl_.as_ref()
            .ok_or_else(|| format!("Local processor '{}' missing 'impl_' field", config.id))?;

        match impl_name.as_str() {
            // Text case processors
            "change_text_case_upper" => Ok(Arc::new(ChangeTextCaseProcessor::upper())),
            "change_text_case_lower" => Ok(Arc::new(ChangeTextCaseProcessor::lower())),
            "change_text_case_proper" => Ok(Arc::new(ChangeTextCaseProcessor::proper())),
            "change_text_case_title" => Ok(Arc::new(ChangeTextCaseProcessor::title())),
            
            // Text manipulation processors
            "reverse_text" => Ok(Arc::new(ReverseTextProcessor::new())),
            
            // Analysis processors
            "token_counter" => Ok(Arc::new(TokenCounterProcessor::new())),
            "word_frequency_analyzer" => Ok(Arc::new(WordFrequencyAnalyzerProcessor::new())),
            
            // Configurable processors - these would need additional config parsing
            "prefix_suffix_adder" => {
                // For now, create with default brackets
                Ok(Arc::new(PrefixSuffixAdderProcessor::with_prefix_and_suffix(
                    "[".to_string(), 
                    "]".to_string()
                )))
            },
            
            // Result collection processor
            "result_collector" => {
                let strategy = config.collection_strategy.clone()
                    .unwrap_or(CollectionStrategy::FirstAvailable);
                Ok(Arc::new(ResultCollectorProcessor::new(strategy)))
            },
            
            // Add more processors here as they're implemented
            _ => Err(format!("Unknown local processor implementation: '{}'", impl_name)),
        }
    }

    /// List all available local processor implementations
    pub fn list_available_implementations() -> Vec<&'static str> {
        vec![
            "change_text_case_upper",
            "change_text_case_lower", 
            "change_text_case_proper",
            "change_text_case_title",
            "reverse_text",
            "token_counter",
            "word_frequency_analyzer",
            "prefix_suffix_adder",
            "result_collector",
        ]
    }

    /// Check if an implementation is available
    pub fn is_implementation_available(impl_name: &str) -> bool {
        Self::list_available_implementations().contains(&impl_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProcessorConfig, BackendType};
    use crate::proto::processor_v1::ProcessorRequest;
    use std::collections::HashMap;

    fn create_test_config(id: &str, impl_name: &str) -> ProcessorConfig {
        ProcessorConfig {
            id: id.to_string(),
            backend: BackendType::Local,
            impl_: Some(impl_name.to_string()),
            endpoint: None,
            module: None,
            depends_on: vec![],
            collection_strategy: None,
            options: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_create_change_text_case_processors() {
        let test_cases = vec![
            ("change_text_case_upper", "hello", "HELLO"),
            ("change_text_case_lower", "HELLO", "hello"),
            ("change_text_case_proper", "hello world", "Hello World"),
            ("change_text_case_title", "the quick brown fox", "The Quick Brown Fox"),
        ];

        for (impl_name, input, expected) in test_cases {
            let config = create_test_config("test", impl_name);
            let processor = LocalProcessorFactory::create_processor(&config)
                .expect(&format!("Failed to create processor: {}", impl_name));

            let request = ProcessorRequest {
                payload: input.as_bytes().to_vec(),
                metadata: HashMap::new(),
            };

            let response = processor.process(request).await;
            
            if let Some(crate::proto::processor_v1::processor_response::Outcome::NextPayload(payload)) = response.outcome {
                let result = String::from_utf8(payload).unwrap();
                assert_eq!(result, expected, "Failed for implementation: {}", impl_name);
            } else {
                panic!("Expected NextPayload outcome for {}", impl_name);
            }
        }
    }

    #[tokio::test]
    async fn test_create_reverse_text_processor() {
        let config = create_test_config("test", "reverse_text");
        let processor = LocalProcessorFactory::create_processor(&config).unwrap();

        let request = ProcessorRequest {
            payload: "hello".as_bytes().to_vec(),
            metadata: HashMap::new(),
        };

        let response = processor.process(request).await;
        
        if let Some(crate::proto::processor_v1::processor_response::Outcome::NextPayload(payload)) = response.outcome {
            let result = String::from_utf8(payload).unwrap();
            assert_eq!(result, "olleh");
        } else {
            panic!("Expected NextPayload outcome");
        }
    }

    #[tokio::test]
    async fn test_create_token_counter_processor() {
        let config = create_test_config("test", "token_counter");
        let processor = LocalProcessorFactory::create_processor(&config).unwrap();

        let request = ProcessorRequest {
            payload: "hello world test".as_bytes().to_vec(),
            metadata: HashMap::new(),
        };

        let response = processor.process(request).await;
        
        if let Some(crate::proto::processor_v1::processor_response::Outcome::NextPayload(payload)) = response.outcome {
            let result = String::from_utf8(payload).unwrap();
            // Should be JSON with char_count, word_count, line_count
            assert!(result.contains("word_count"));
            assert!(result.contains("char_count"));
        } else {
            panic!("Expected NextPayload outcome");
        }
    }

    #[test]
    fn test_create_processor_missing_impl() {
        let mut config = create_test_config("test", "");
        config.impl_ = None;
        
        let result = LocalProcessorFactory::create_processor(&config);
        assert!(result.is_err());
        let error_msg = result.err().unwrap();
        assert!(error_msg.contains("missing 'impl_' field"));
    }

    #[test]
    fn test_create_processor_unknown_impl() {
        let config = create_test_config("test", "unknown_processor");
        
        let result = LocalProcessorFactory::create_processor(&config);
        assert!(result.is_err());
        let error_msg = result.err().unwrap();
        assert!(error_msg.contains("Unknown local processor implementation"));
    }

    #[test]
    fn test_list_available_implementations() {
        let implementations = LocalProcessorFactory::list_available_implementations();
        assert!(!implementations.is_empty());
        assert!(implementations.contains(&"change_text_case_upper"));
        assert!(implementations.contains(&"reverse_text"));
        assert!(implementations.contains(&"token_counter"));
    }

    #[test]
    fn test_is_implementation_available() {
        assert!(LocalProcessorFactory::is_implementation_available("change_text_case_upper"));
        assert!(LocalProcessorFactory::is_implementation_available("reverse_text"));
        assert!(!LocalProcessorFactory::is_implementation_available("nonexistent_processor"));
    }
}
