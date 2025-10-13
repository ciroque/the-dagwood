// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use crate::backends::wasm::{
    ProcessingNodeFactory, WasmError, WasmModuleLoader, WasmProcessor, WasmResult,
};
use crate::config::ProcessorConfig;
use crate::traits::processor::{Processor, ProcessorIntent};
use std::sync::Arc;

/// Factory for creating WASM-based processors from configuration.
///
/// The WasmProcessorFactory handles the creation of WasmProcessor instances
/// by loading WASM modules from the filesystem and configuring them based
/// on the processor configuration.
///
/// # Configuration Requirements
///
/// WASM processors require the following configuration fields:
/// - `module`: Path to the WASM module file (required)
/// - `intent`: Processor intent - "transform" or "analyze" (optional, defaults to "transform")
///
/// # Example Configuration
///
/// ```yaml
/// processors:
///   - id: hello_world_wasm
///     type: wasm
///     module: "modules/hello.wasm"
///     options:
///       intent: "transform"
/// ```
pub struct WasmProcessorFactory;

impl WasmProcessorFactory {
    /// Creates a new WASM processor from the given configuration.
    ///
    /// This method implements a three-way detection strategy:
    /// 1. **Preview 2 WIT Component** (The New Hotness) - Proper WIT components
    /// 2. **Preview 1 WASI Module** (Legacy but Common) - Modules with WASI imports (like Grain)
    /// 3. **C-Style Module** (Old Reliable) - Modules with C-style exports
    ///
    /// # Arguments
    ///
    /// * `config` - The processor configuration containing module path and options
    ///
    /// # Returns
    ///
    /// Returns a Result containing an Arc-wrapped Processor or an error if the
    /// WASM module cannot be loaded or the configuration is invalid.
    ///
    /// # Configuration Fields
    ///
    /// - `module`: Required. Path to the WASM module file
    /// - `options.intent`: Optional. Either "transform" or "analyze" (defaults to "transform")
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The `module` field is missing from the configuration
    /// - The WASM module cannot be loaded or compiled
    /// - The intent is invalid
    pub fn create_processor(config: &ProcessorConfig) -> WasmResult<Arc<dyn Processor>> {
        // Extract the module path from the configuration
        let module_path = config.module.as_ref().ok_or_else(|| {
            WasmError::ValidationError(
                "Missing required 'module' field in WASM processor configuration".to_string(),
            )
        })?;

        // Parse the intent with a default of Transform
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
            ProcessorIntent::Transform // Default
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
        // Will fail due to missing file, but should pass intent validation
        assert!(result.is_err());
        // Should fail on file loading, not intent parsing
        let error_msg = result.err().unwrap().to_string();
        assert!(!error_msg.contains("Invalid intent"));
    }

    #[test]
    fn test_valid_analyze_intent() {
        let config = create_config_with_intent("test_wasm", "/tmp/nonexistent.wasm", "analyze");

        let result = WasmProcessorFactory::create_processor(&config);
        // Will fail due to missing file, but should pass intent validation
        assert!(result.is_err());
        // Should fail on file loading, not intent parsing
        let error_msg = result.err().unwrap().to_string();
        assert!(!error_msg.contains("Invalid intent"));
    }

    #[test]
    fn test_default_intent() {
        let config = create_test_config("test_wasm", "/tmp/nonexistent.wasm");

        let result = WasmProcessorFactory::create_processor(&config);
        // Will fail due to missing file, but should use default intent
        assert!(result.is_err());
        // Should fail on file loading, not intent parsing
        let error_msg = result.err().unwrap().to_string();
        assert!(!error_msg.contains("Invalid intent"));
    }

    #[test]
    fn test_non_string_intent() {
        let mut options = HashMap::new();
        options.insert(
            "intent".to_string(),
            serde_yaml::Value::Number(serde_yaml::Number::from(123)),
        ); // Non-string value

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
