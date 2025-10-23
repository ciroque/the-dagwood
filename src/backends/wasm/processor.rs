// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use crate::backends::wasm::error::WasmResult;
use crate::backends::wasm::factory::create_executor;
use crate::backends::wasm::detector::wasm_encoding;
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
    pub fn new(processor_id: String, module_path: String) -> WasmResult<Self> {
        let bytes = load_wasm_bytes(&module_path)?;
        let encoding = wasm_encoding(&bytes)
            .map_err(|e| crate::backends::wasm::WasmError::ValidationError(e.to_string()))?;
        let executor = create_executor(&bytes, encoding)?.into();

        Ok(Self {
            processor_id,
            module_path,
            executor,
            intent: ProcessorIntent::Transform,
        })
    }

    pub fn from_config(config: &crate::config::ProcessorConfig) -> WasmResult<Self> {
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

        let bytes = load_wasm_bytes(module_path)?;
        let encoding = wasm_encoding(&bytes)
            .map_err(|e| crate::backends::wasm::WasmError::ValidationError(e.to_string()))?;
        let executor = create_executor(&bytes, encoding)?.into();

        Ok(Self {
            processor_id: config.id.clone(),
            module_path: module_path.clone(),
            executor,
            intent,
        })
    }

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
