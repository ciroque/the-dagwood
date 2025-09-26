use async_trait::async_trait;
use serde::Serialize;

use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, ErrorDetail};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::traits::Processor;

/// Token Counter processor - counts characters and words
pub struct TokenCounterProcessor;

impl TokenCounterProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Serialize)]
struct TokenCountResult {
    char_count: usize,
    word_count: usize,
    line_count: usize,
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
                    metadata: std::collections::HashMap::new(),
                    declared_intent: crate::proto::processor_v1::ProcessorIntent::Analyze as i32,
                };
            }
        };

        let char_count = input.chars().count();
        let word_count = input.split_whitespace().count();
        let line_count = input.lines().count().max(1); // At least 1 line even if empty

        let result = TokenCountResult {
            char_count,
            word_count,
            line_count,
        };

        let json_result = match serde_json::to_string(&result) {
            Ok(json) => json,
            Err(e) => {
                return ProcessorResponse {
                    outcome: Some(Outcome::Error(ErrorDetail {
                        code: 500,
                        message: format!("Failed to serialize result: {}", e),
                    })),
                    metadata: std::collections::HashMap::new(),
                    declared_intent: crate::proto::processor_v1::ProcessorIntent::Analyze as i32,
                };
            }
        };

        ProcessorResponse {
            outcome: Some(Outcome::NextPayload(json_result.into_bytes())),
            metadata: std::collections::HashMap::new(),
            declared_intent: crate::proto::processor_v1::ProcessorIntent::Analyze as i32,
        }
    }

    fn name(&self) -> &'static str {
        "token_counter"
    }

    fn declared_intent(&self) -> crate::proto::processor_v1::ProcessorIntent {
        crate::proto::processor_v1::ProcessorIntent::Analyze
    }
}
