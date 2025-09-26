use async_trait::async_trait;
use std::collections::HashMap;

use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, ErrorDetail};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::traits::{Processor, processor::ProcessorIntent};

/// Word Frequency Analyzer processor - creates a histogram of words
pub struct WordFrequencyAnalyzerProcessor;

impl WordFrequencyAnalyzerProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Processor for WordFrequencyAnalyzerProcessor {
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

        let mut word_counts: HashMap<String, usize> = HashMap::new();
        
        // Normalize and count words
        for word in input.split_whitespace() {
            let normalized_word = word
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase();
            
            if !normalized_word.is_empty() {
                *word_counts.entry(normalized_word).or_insert(0) += 1;
            }
        }

        let json_result = match serde_json::to_string(&word_counts) {
            Ok(json) => json,
            Err(e) => {
                return ProcessorResponse {
                    outcome: Some(Outcome::Error(ErrorDetail {
                        code: 500,
                        message: format!("Failed to serialize result: {}", e),
                    })),
                    metadata: std::collections::HashMap::new(),
                };
            }
        };

        ProcessorResponse {
            outcome: Some(Outcome::NextPayload(json_result.into_bytes())),
            metadata: std::collections::HashMap::new(),
        }
    }

    fn name(&self) -> &'static str {
        "word_frequency_analyzer"
    }

    fn declared_intent(&self) -> ProcessorIntent {
        ProcessorIntent::Analyze
    }
}
