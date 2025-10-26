// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use std::time::Instant;

use crate::observability::messages::{processor::*, StructuredLog};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::proto::processor_v1::{ErrorDetail, ProcessorRequest, ProcessorResponse};
use crate::traits::{processor::ProcessorIntent, Processor};

/// Reverse Text processor - reverses the input string
pub struct ReverseTextProcessor;

impl ReverseTextProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Processor for ReverseTextProcessor {
    async fn process(&self, req: ProcessorRequest) -> ProcessorResponse {
        let start_msg = ProcessorExecutionStarted {
            processor_id: self.name(),
            input_size: req.payload.len(),
        };

        let span = start_msg.span("processor_execution");
        let _guard = span.enter();
        start_msg.log();

        let start_time = Instant::now();

        let input = match String::from_utf8(req.payload) {
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

        let reversed: String = input.chars().rev().collect();
        let output_bytes = reversed.into_bytes();
        let duration = start_time.elapsed();

        ProcessorExecutionCompleted {
            processor_id: self.name(),
            input_size: start_msg.input_size,
            output_size: output_bytes.len(),
            duration,
        }
        .log();

        ProcessorResponse {
            outcome: Some(Outcome::NextPayload(output_bytes)),
            metadata: None,
        }
    }

    fn name(&self) -> &'static str {
        "reverse_text"
    }

    fn declared_intent(&self) -> ProcessorIntent {
        ProcessorIntent::Transform
    }
}
