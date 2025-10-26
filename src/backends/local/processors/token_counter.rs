// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;

use crate::observability::messages::{processor::*, StructuredLog};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::proto::processor_v1::{
    ErrorDetail, PipelineMetadata, ProcessorMetadata, ProcessorRequest, ProcessorResponse,
};
use crate::traits::{processor::ProcessorIntent, Processor};

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
        let start_msg = ProcessorExecutionStarted {
            processor_id: self.name(),
            input_size: req.payload.len(),
        };

        let span = start_msg.span("processor_execution");
        let _guard = span.enter();
        start_msg.log();

        let start = Instant::now();

        let input = match String::from_utf8(req.payload.clone()) {
            Ok(text) => text,
            Err(e) => {
                let error = std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Invalid UTF-8 input: {}", e));
                ProcessorExecutionFailed {
                    processor_id: self.name(),
                    error: &error,
                }
                .log();

                return ProcessorResponse {
                    outcome: Some(Outcome::Error(ErrorDetail {
                        code: 400,
                        message: format!("Invalid UTF-8 input: {}", e),
                    })),
                    metadata: None,
                };
            }
        };

        let char_count = input.chars().count();
        let word_count = input.split_whitespace().count();
        let line_count = input.lines().count().max(1); // At least 1 line even if empty

        // Create metadata for our analysis results
        let mut own_metadata = HashMap::new();
        own_metadata.insert("char_count".to_string(), char_count.to_string());
        own_metadata.insert("word_count".to_string(), word_count.to_string());
        own_metadata.insert("line_count".to_string(), line_count.to_string());

        // Create pipeline metadata with our processor's results
        let mut pipeline_metadata = PipelineMetadata::new();
        pipeline_metadata.metadata.insert(
            self.name().to_string(),
            ProcessorMetadata {
                metadata: own_metadata,
            },
        );

        let duration = start.elapsed();

        ProcessorExecutionCompleted {
            processor_id: self.name(),
            input_size: start_msg.input_size,
            output_size: 0, // Analyze processor - no output payload
            duration,
        }
        .log();

        ProcessorResponse {
            outcome: Some(Outcome::NextPayload(vec![])), // Analyze processors: return empty payload (executor ignores it)
            metadata: Some(pipeline_metadata),
        }
    }

    fn name(&self) -> &'static str {
        "token_counter"
    }

    fn declared_intent(&self) -> ProcessorIntent {
        ProcessorIntent::Analyze
    }
}
