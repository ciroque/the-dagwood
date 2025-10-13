use crate::backends::wasm::{
    ProcessingNodeFactory, WasmError, WasmModuleLoader, WasmProcessor, WasmResult,
};
use crate::config::ProcessorConfig;
use crate::traits::processor::{Processor, ProcessorIntent};
use std::sync::Arc;

pub struct WasmProcessorFactory;

impl WasmProcessorFactory {
    pub fn create_processor(config: &ProcessorConfig) -> WasmResult<Arc<dyn Processor>> {
        let module_path = config.module.as_ref().ok_or_else(|| {
            WasmError::ValidationError(
                "Missing required 'module' field in WASM processor configuration".to_string(),
            )
        })?;

        let intent = if let Some(intent_value) = config.options.get("intent") {
            if let Some(intent_str) = intent_value.as_str() {
                match intent_str.to_lowercase().as_str() {
                    "transform" => ProcessorIntent::Transform,
                    "analyze" => ProcessorIntent::Analyze,
                    invalid => {
                        return Err(WasmError::ValidationError(format!(
                            "Invalid intent '{}'. Must be 'transform' or 'analyze'.",
                            invalid
                        )))
                    }
                }
            } else {
                return Err(WasmError::ValidationError(
                    "Intent option must be a string".to_string(),
                ));
            }
        } else {
            ProcessorIntent::Transform
        };

        let loaded_module = WasmModuleLoader::load_module(module_path)?;
        
        let executor = ProcessingNodeFactory::create_executor(loaded_module)
            .map_err(|e| WasmError::ProcessorError(e.to_string()))?;
        
        let processor = WasmProcessor::new_with_executor(config.id.clone(), executor, intent)?;
        
        Ok(Arc::new(processor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BackendType, ProcessorConfig};
    use std::collections::HashMap;

    fn create_test_config(id: &str, module_path: &str) -> ProcessorConfig {
        ProcessorConfig {
            id: id.to_string(),
            backend: BackendType::Wasm,
            processor: None,
            endpoint: None,
            module: Some(module_path.to_string()),
            depends_on: vec![],
            options: HashMap::new(),
        }
    }

    fn create_config_with_intent(id: &str, module_path: &str, intent: &str) -> ProcessorConfig {
        let mut options = HashMap::new();
        options.insert(
            "intent".to_string(),
            serde_yaml::Value::String(intent.to_string()),
        );

        ProcessorConfig {
            id: id.to_string(),
            backend: BackendType::Wasm,
            processor: None,
            endpoint: None,
            module: Some(module_path.to_string()),
            depends_on: vec![],
            options,
        }
    }

    #[test]
    fn test_missing_module_path() {
        let config = ProcessorConfig {
            id: "test_wasm".to_string(),
            backend: BackendType::Wasm,
            processor: None,
            endpoint: None,
            module: None, // Missing module path
            depends_on: vec![],
            options: HashMap::new(),
        };

        let result = WasmProcessorFactory::create_processor(&config);
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("module"));
    }

    #[test]
    fn test_invalid_intent_option() {
        let config =
            create_config_with_intent("test_wasm", "/tmp/nonexistent.wasm", "invalid_intent");

        let result = WasmProcessorFactory::create_processor(&config);
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Invalid intent"));
    }

    #[test]
    fn test_valid_transform_intent() {
        let config = create_config_with_intent("test_wasm", "/tmp/nonexistent.wasm", "transform");

        let result = WasmProcessorFactory::create_processor(&config);
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(!error_msg.contains("Invalid intent"));
    }

    #[test]
    fn test_valid_analyze_intent() {
        let config = create_config_with_intent("test_wasm", "/tmp/nonexistent.wasm", "analyze");

        let result = WasmProcessorFactory::create_processor(&config);
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(!error_msg.contains("Invalid intent"));
    }

    #[test]
    fn test_default_intent() {
        let config = create_test_config("test_wasm", "/tmp/nonexistent.wasm");

        let result = WasmProcessorFactory::create_processor(&config);
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(!error_msg.contains("Invalid intent"));
    }

    #[test]
    fn test_non_string_intent() {
        let mut options = HashMap::new();
        options.insert(
            "intent".to_string(),
            serde_yaml::Value::Number(serde_yaml::Number::from(123)),
        );

        let config = ProcessorConfig {
            id: "test_wasm".to_string(),
            backend: BackendType::Wasm,
            processor: None,
            endpoint: None,
            module: Some("/tmp/nonexistent.wasm".to_string()),
            depends_on: vec![],
            options,
        };

        let result = WasmProcessorFactory::create_processor(&config);
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("must be a string"));
    }
}
