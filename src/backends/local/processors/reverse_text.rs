use async_trait::async_trait;

use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, ErrorDetail};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::traits::{Processor, processor::ProcessorIntent};

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
                    metadata: std::collections::HashMap::new(),
                };
            }
        };

        let reversed: String = input.chars().rev().collect();

        ProcessorResponse {
            outcome: Some(Outcome::NextPayload(reversed.into_bytes())),
            metadata: std::collections::HashMap::new(),
        }
    }

    fn name(&self) -> &'static str {
        "reverse_text"
    }

    fn declared_intent(&self) -> ProcessorIntent {
        ProcessorIntent::Transform
    }
}
