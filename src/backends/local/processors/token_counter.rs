use async_trait::async_trait;
use std::collections::HashMap;

use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, ErrorDetail, Metadata};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::traits::{Processor, processor::ProcessorIntent};

/// Token Counter processor - counts characters and words
pub struct TokenCounterProcessor;

impl TokenCounterProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Processor for TokenCounterProcessor {
    async fn process(&self, req: ProcessorRequest) -> ProcessorResponse {
        let input = match String::from_utf8(req.payload) {
            Ok(text) => text,
            Err(e) => {
                return ProcessorResponse {
                    outcome: Some(Outcome::Error(ErrorDetail {
                        code: 400,
                        message: format!("Invalid UTF-8 input: {}", e),
                    })),
                    metadata: HashMap::new(),
                };
            }
        };

        let char_count = input.chars().count();
        let word_count = input.split_whitespace().count();
        let line_count = input.lines().count().max(1); // At least 1 line even if empty

        // Simple metadata: add our analysis results under our processor name
        let mut own_metadata = HashMap::new();
        own_metadata.insert("char_count".to_string(), char_count.to_string());
        own_metadata.insert("word_count".to_string(), word_count.to_string());
        own_metadata.insert("line_count".to_string(), line_count.to_string());
        
        // Access dependency metadata if needed (simple protobuf access)
        for (processor_name, metadata) in &req.metadata {
            if processor_name != "token_counter" { // Don't process our own metadata
                if let Some(transform_type) = metadata.metadata.get("transform_type") {
                    own_metadata.insert("input_transform".to_string(), transform_type.clone());
                }
            }
        }
        
        // Return metadata under our processor name
        let mut response_metadata = HashMap::new();
        response_metadata.insert(self.name().to_string(), Metadata {
            metadata: own_metadata,
        });

        ProcessorResponse {
            outcome: Some(Outcome::NextPayload(Vec::new())), // Analyze processors: empty payload, executor uses canonical payload
            metadata: response_metadata,
        }
    }

    fn name(&self) -> &'static str {
        "token_counter"
    }

    fn declared_intent(&self) -> ProcessorIntent {
        ProcessorIntent::Analyze
    }
}
