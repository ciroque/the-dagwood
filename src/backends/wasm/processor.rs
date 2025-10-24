// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! High-level WASM processor implementation with automatic strategy selection.
//!
//! This module provides `WasmProcessor`, the main entry point for WASM-based processors
//! in The DAGwood system. It implements the `Processor` trait and automatically selects
//! the appropriate execution strategy based on WASM binary encoding.
//!
//! # Architecture
//!
//! `WasmProcessor` orchestrates the ADR-17 three-step flow:
//! ```text
//! Configuration → WasmProcessor::new() → Processor Instance
//!                          ↓
//!                  load_wasm_bytes()
//!                          ↓
//!                  detect_component_type()
//!                          ↓
//!                  create_executor()
//!                          ↓
//!                  ProcessingNodeExecutor
//! ```
//!
//! ## Key Features
//! - **Automatic Strategy Selection**: Detects Component Model vs Classic WASM
//! - **Intent Support**: Configurable Transform vs Analyze processor intent
//! - **Metadata Collection**: Comprehensive execution metadata
//! - **Error Handling**: Converts WASM errors to processor responses
//!
//! # Processor Intent
//!
//! WASM processors support both Transform and Analyze intents:
//! - **Transform** (default): Can modify payloads
//! - **Analyze**: Only contributes metadata
//!
//! Intent is configured via the `intent` option in processor configuration:
//! ```yaml
//! processors:
//!   - id: analyzer
//!     backend: wasm
//!     module: analyzer.wasm
//!     options:
//!       intent: analyze  # or "transform"
//! ```
//!
//! # Examples
//!
//! ## Creating from Path
//! ```rust,no_run
//! use the_dagwood::backends::wasm::WasmProcessor;
//!
//! let processor = WasmProcessor::new(
//!     "text_processor".to_string(),
//!     "processors/transform.wasm".to_string(),
//! )?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Creating from Configuration
//! ```rust,no_run
//! use the_dagwood::backends::wasm::WasmProcessor;
//! use the_dagwood::config::{ProcessorConfig, BackendType};
//! use std::collections::HashMap;
//! use serde_yaml::Value;
//!
//! let mut options = HashMap::new();
//! options.insert("intent".to_string(), Value::String("analyze".to_string()));
//!
//! let config = ProcessorConfig {
//!     id: "analyzer".to_string(),
//!     backend: BackendType::Wasm,
//!     processor: None,
//!     endpoint: None,
//!     module: Some("analyzer.wasm".to_string()),
//!     depends_on: vec![],
//!     options,
//! };
//!
//! let processor = WasmProcessor::from_config(&config)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Using with Executor
//! ```rust,no_run
//! use the_dagwood::backends::wasm::WasmProcessor;
//! use the_dagwood::traits::Processor;
//! use the_dagwood::proto::processor_v1::ProcessorRequest;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let processor = WasmProcessor::new(
//!     "processor".to_string(),
//!     "module.wasm".to_string(),
//! )?;
//!
//! let request = ProcessorRequest {
//!     payload: b"input data".to_vec(),
//! };
//!
//! let response = processor.process(request).await;
//! # Ok(())
//! # }
//! ```

use crate::backends::wasm::detector::detect_component_type;
use crate::backends::wasm::error::WasmResult;
use crate::backends::wasm::factory::create_executor;
use crate::backends::wasm::loader::load_wasm_bytes;
use crate::backends::wasm::processing_node::ProcessingNodeExecutor;
use crate::proto::processor_v1::{
    processor_response::Outcome, ErrorDetail, PipelineMetadata, ProcessorMetadata,
    ProcessorRequest, ProcessorResponse,
};
use crate::traits::processor::{Processor, ProcessorIntent};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// High-level WASM processor with automatic strategy selection.
///
/// This is the main entry point for WASM-based processors in The DAGwood system.
/// It implements the `Processor` trait and automatically selects the appropriate
/// execution strategy based on WASM binary encoding.
///
/// # Fields
/// - **processor_id**: Unique identifier for this processor instance
/// - **module_path**: Path to the WASM module file
/// - **executor**: Strategy-specific executor (WIT or C-Style)
/// - **intent**: Processor intent (Transform or Analyze)
///
/// # Thread Safety
/// - Uses `Arc<dyn ProcessingNodeExecutor>` for shared executor access
/// - Safe for concurrent use across async tasks
pub struct WasmProcessor {
    /// Unique identifier for this processor instance
    processor_id: String,
    /// Path to the WASM module file
    module_path: String,
    /// The appropriate executor for this WASM artifact type
    executor: Arc<dyn ProcessingNodeExecutor>,
    /// Processor intent (Transform or Analyze)
    intent: ProcessorIntent,
}

impl WasmProcessor {
    /// Create a new WASM processor from a module path.
    ///
    /// This method performs the complete ADR-17 flow:
    /// 1. Load WASM bytes from disk
    /// 2. Detect encoding type
    /// 3. Create appropriate executor
    ///
    /// The processor defaults to `Transform` intent and uses the default fuel level (100M).
    ///
    /// # Arguments
    /// * `processor_id` - Unique identifier for this processor
    /// * `module_path` - Path to the WASM file
    ///
    /// # Returns
    /// * `Ok(WasmProcessor)` - Ready-to-use processor
    /// * `Err(WasmError)` - If loading, detection, or executor creation fails
    ///
    /// # Examples
    /// ```rust,no_run
    /// use the_dagwood::backends::wasm::WasmProcessor;
    ///
    /// let processor = WasmProcessor::new(
    ///     "text_processor".to_string(),
    ///     "processors/transform.wasm".to_string(),
    /// )?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(processor_id: String, module_path: String) -> WasmResult<Self> {
        let bytes = load_wasm_bytes(&module_path)?;
        let component_type = detect_component_type(&bytes)
            .map_err(|e| crate::backends::wasm::WasmError::ValidationError(e.to_string()))?;
        // Use default fuel level (100M) for direct instantiation
        let executor = create_executor(&bytes, component_type, 100_000_000)?.into();

        Ok(Self {
            processor_id,
            module_path,
            executor,
            intent: ProcessorIntent::Transform,
        })
    }

    /// Create a new WASM processor from configuration.
    ///
    /// This method creates a processor from a `ProcessorConfig` struct, supporting
    /// configuration-driven processor instantiation. It extracts the module path,
    /// optional intent, and optional fuel level from the configuration.
    ///
    /// # Configuration Options
    /// - **module** (required): Path to the WASM file
    /// - **intent** (optional): "transform" or "analyze" (defaults to "transform")
    /// - **fuel_level** (optional): Fuel limit for execution (defaults to global config)
    ///
    /// # Arguments
    /// * `config` - Processor configuration
    /// * `global_fuel_config` - Global fuel configuration for defaults and validation
    ///
    /// # Returns
    /// * `Ok(WasmProcessor)` - Ready-to-use processor
    /// * `Err(WasmError)` - If configuration is invalid or loading fails
    ///
    /// # Examples
    /// ```rust,no_run
    /// use the_dagwood::backends::wasm::WasmProcessor;
    /// use the_dagwood::config::{ProcessorConfig, BackendType, FuelConfig};
    /// use std::collections::HashMap;
    /// use serde_yaml::Value;
    ///
    /// let mut options = HashMap::new();
    /// options.insert("intent".to_string(), Value::String("analyze".to_string()));
    /// options.insert("fuel_level".to_string(), Value::Number(50_000_000.into()));
    ///
    /// let config = ProcessorConfig {
    ///     id: "analyzer".to_string(),
    ///     backend: BackendType::Wasm,
    ///     processor: None,
    ///     endpoint: None,
    ///     module: Some("analyzer.wasm".to_string()),
    ///     depends_on: vec![],
    ///     options,
    /// };
    ///
    /// let fuel_config = FuelConfig::default();
    /// let processor = WasmProcessor::from_config(&config, &fuel_config)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn from_config(
        config: &crate::config::ProcessorConfig,
        global_fuel_config: &crate::config::FuelConfig,
    ) -> WasmResult<Self> {
        let module_path = config.module.as_ref().ok_or_else(|| {
            crate::backends::wasm::WasmError::ValidationError(
                "Missing required 'module' field in WASM processor configuration".to_string(),
            )
        })?;

        let intent = if let Some(intent_value) = config.options.get("intent") {
            if let Some(intent_str) = intent_value.as_str() {
                match intent_str.to_lowercase().as_str() {
                    "transform" => ProcessorIntent::Transform,
                    "analyze" => ProcessorIntent::Analyze,
                    invalid => {
                        return Err(crate::backends::wasm::WasmError::ValidationError(format!(
                            "Invalid intent '{}'. Must be 'transform' or 'analyze'.",
                            invalid
                        )))
                    }
                }
            } else {
                return Err(crate::backends::wasm::WasmError::ValidationError(
                    "Intent option must be a string".to_string(),
                ));
            }
        } else {
            ProcessorIntent::Transform
        };

        // Extract and validate fuel_level from options
        let fuel_level = if let Some(fuel_value) = config.options.get("fuel_level") {
            // Try to parse as u64 from various YAML number types
            let requested_fuel = if let Some(num) = fuel_value.as_u64() {
                num
            } else if let Some(num) = fuel_value.as_i64() {
                if num < 0 {
                    return Err(crate::backends::wasm::WasmError::ValidationError(
                        "fuel_level must be a positive number".to_string(),
                    ));
                }
                num as u64
            } else {
                return Err(crate::backends::wasm::WasmError::ValidationError(
                    "fuel_level must be a number".to_string(),
                ));
            };

            // Validate and clamp to configured bounds
            global_fuel_config.validate_and_clamp(requested_fuel)
        } else {
            // Use global default if not specified
            global_fuel_config.get_default()
        };

        let bytes = load_wasm_bytes(module_path)?;
        let component_type = detect_component_type(&bytes)
            .map_err(|e| crate::backends::wasm::WasmError::ValidationError(e.to_string()))?;
        let executor = create_executor(&bytes, component_type, fuel_level)?.into();

        Ok(Self {
            processor_id: config.id.clone(),
            module_path: module_path.clone(),
            executor,
            intent,
        })
    }

    /// Execute WASM module synchronously.
    ///
    /// Internal method that calls the executor and handles error conversion.
    /// This bridges the synchronous WASM execution with the async processor interface.
    ///
    /// # Arguments
    /// * `input` - Input data bytes
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - Processed output data
    /// * `Err(Box<dyn Error>)` - If execution fails
    fn execute_wasm(
        &self,
        input: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!(
            "Executing WASM module '{}' using {} executor",
            self.module_path,
            self.executor.artifact_type()
        );

        match self.executor.execute(input) {
            Ok(output) => {
                tracing::debug!(
                    "WASM execution successful: input_size={}, output_size={}, artifact_type={}",
                    input.len(),
                    output.len(),
                    self.executor.artifact_type()
                );
                Ok(output)
            }
            Err(error) => {
                tracing::error!(
                    "WASM execution failed for {}: {}",
                    self.executor.artifact_type(),
                    error
                );
                Err(Box::new(error))
            }
        }
    }
}

#[async_trait]
impl Processor for WasmProcessor {
    fn name(&self) -> &'static str {
        "WasmProcessor"
    }

    fn declared_intent(&self) -> ProcessorIntent {
        self.intent
    }

    async fn process(&self, request: ProcessorRequest) -> ProcessorResponse {
        let input = request.payload;

        match self.execute_wasm(&input) {
            Ok(output) => {
                let mut processor_metadata_map = HashMap::new();
                processor_metadata_map
                    .insert("processor_id".to_string(), self.processor_id.clone());
                processor_metadata_map.insert("module_path".to_string(), self.module_path.clone());
                processor_metadata_map.insert(
                    "artifact_type".to_string(),
                    self.executor.artifact_type().to_string(),
                );
                processor_metadata_map.insert(
                    "capabilities".to_string(),
                    format!("{:?}", self.executor.capabilities()),
                );
                processor_metadata_map.insert("input_length".to_string(), input.len().to_string());
                processor_metadata_map
                    .insert("output_length".to_string(), output.len().to_string());

                let processor_metadata = ProcessorMetadata {
                    metadata: processor_metadata_map,
                };

                let mut pipeline_metadata_map = HashMap::new();
                pipeline_metadata_map.insert(self.processor_id.clone(), processor_metadata);

                let pipeline_metadata = PipelineMetadata {
                    metadata: pipeline_metadata_map,
                };

                ProcessorResponse {
                    outcome: Some(Outcome::NextPayload(output)),
                    metadata: Some(pipeline_metadata),
                }
            }
            Err(error) => {
                let error_detail = ErrorDetail {
                    code: 500,
                    message: format!("WASM execution failed: {}", error),
                };

                ProcessorResponse {
                    outcome: Some(Outcome::Error(error_detail)),
                    metadata: None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BackendType, FuelConfig, ProcessorConfig};
    use std::collections::HashMap;

    #[test]
    fn test_fuel_level_from_config_default() {
        let mut options = HashMap::new();
        options.insert(
            "intent".to_string(),
            serde_yaml::Value::String("transform".to_string()),
        );

        let config = ProcessorConfig {
            id: "test_processor".to_string(),
            backend: BackendType::Wasm,
            processor: None,
            endpoint: None,
            module: Some("wasm_components/wasm_appender.wasm".to_string()),
            depends_on: vec![],
            options,
        };

        let fuel_config = FuelConfig::default();

        // This test validates that the processor can be created with default fuel
        // The actual execution would require the WASM file to exist
        let result = WasmProcessor::from_config(&config, &fuel_config);

        // We expect this to fail with module loading error since we're in test,
        // but it should NOT fail with fuel configuration errors
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                !error_msg.contains("fuel"),
                "Should not have fuel-related errors, got: {}",
                error_msg
            );
        }
    }

    #[test]
    fn test_fuel_level_from_config_custom() {
        let mut options = HashMap::new();
        options.insert(
            "intent".to_string(),
            serde_yaml::Value::String("transform".to_string()),
        );
        options.insert(
            "fuel_level".to_string(),
            serde_yaml::Value::Number(50_000_000.into()),
        );

        let config = ProcessorConfig {
            id: "test_processor".to_string(),
            backend: BackendType::Wasm,
            processor: None,
            endpoint: None,
            module: Some("wasm_components/wasm_appender.wasm".to_string()),
            depends_on: vec![],
            options,
        };

        let fuel_config = FuelConfig::default();

        let result = WasmProcessor::from_config(&config, &fuel_config);

        // Should not fail with fuel configuration errors
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                !error_msg.contains("fuel") || error_msg.contains("Failed to load"),
                "Should not have fuel validation errors, got: {}",
                error_msg
            );
        }
    }

    #[test]
    fn test_fuel_level_clamped_to_maximum() {
        let mut options = HashMap::new();
        options.insert(
            "fuel_level".to_string(),
            serde_yaml::Value::Number(1_000_000_000.into()),
        ); // 1B - above max

        let config = ProcessorConfig {
            id: "test_processor".to_string(),
            backend: BackendType::Wasm,
            processor: None,
            endpoint: None,
            module: Some("wasm_components/wasm_appender.wasm".to_string()),
            depends_on: vec![],
            options,
        };

        let fuel_config = FuelConfig {
            default: Some(100_000_000),
            minimum: Some(1_000_000),
            maximum: Some(500_000_000),
        };

        // Should clamp to maximum without error
        let result = WasmProcessor::from_config(&config, &fuel_config);

        if let Err(e) = result {
            let error_msg = e.to_string();
            // Should fail with module loading, not fuel validation
            assert!(
                error_msg.contains("Failed to load") || error_msg.contains("No such file"),
                "Should fail with file error, not fuel error, got: {}",
                error_msg
            );
        }
    }

    #[test]
    fn test_fuel_level_clamped_to_minimum() {
        let mut options = HashMap::new();
        options.insert(
            "fuel_level".to_string(),
            serde_yaml::Value::Number(100.into()),
        ); // Below minimum

        let config = ProcessorConfig {
            id: "test_processor".to_string(),
            backend: BackendType::Wasm,
            processor: None,
            endpoint: None,
            module: Some("wasm_components/wasm_appender.wasm".to_string()),
            depends_on: vec![],
            options,
        };

        let fuel_config = FuelConfig {
            default: Some(100_000_000),
            minimum: Some(1_000_000),
            maximum: Some(500_000_000),
        };

        // Should clamp to minimum without error
        let result = WasmProcessor::from_config(&config, &fuel_config);

        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("Failed to load") || error_msg.contains("No such file"),
                "Should fail with file error, not fuel error, got: {}",
                error_msg
            );
        }
    }

    #[test]
    fn test_fuel_level_negative_number_error() {
        let mut options = HashMap::new();
        options.insert(
            "fuel_level".to_string(),
            serde_yaml::Value::Number((-100).into()),
        );

        let config = ProcessorConfig {
            id: "test_processor".to_string(),
            backend: BackendType::Wasm,
            processor: None,
            endpoint: None,
            module: Some("wasm_components/wasm_appender.wasm".to_string()),
            depends_on: vec![],
            options,
        };

        let fuel_config = FuelConfig::default();

        let result = WasmProcessor::from_config(&config, &fuel_config);

        assert!(result.is_err(), "Should fail with negative fuel_level");
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("fuel_level must be a positive number"),
                "Should have positive number error, got: {}",
                error_msg
            );
        }
    }

    #[test]
    fn test_fuel_level_invalid_type_error() {
        let mut options = HashMap::new();
        options.insert(
            "fuel_level".to_string(),
            serde_yaml::Value::String("not_a_number".to_string()),
        );

        let config = ProcessorConfig {
            id: "test_processor".to_string(),
            backend: BackendType::Wasm,
            processor: None,
            endpoint: None,
            module: Some("wasm_components/wasm_appender.wasm".to_string()),
            depends_on: vec![],
            options,
        };

        let fuel_config = FuelConfig::default();

        let result = WasmProcessor::from_config(&config, &fuel_config);

        assert!(result.is_err(), "Should fail with invalid fuel_level type");
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("fuel_level must be a number"),
                "Should have number type error, got: {}",
                error_msg
            );
        }
    }
}
