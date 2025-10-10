// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use async_trait::async_trait;

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
        let input = match String::from_utf8(req.payload) {
            Ok(text) => text,
            Err(e) => {
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

        ProcessorResponse {
            outcome: Some(Outcome::NextPayload(reversed.into_bytes())),
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
