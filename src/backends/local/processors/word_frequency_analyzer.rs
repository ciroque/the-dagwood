use async_trait::async_trait;
use std::collections::HashMap;

use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, ErrorDetail, Metadata};
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
        let input = match String::from_utf8(req.payload.clone()) {
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

        // Analyze processors MUST NOT modify payload - put analysis results in metadata
        let mut analysis_metadata = HashMap::new();
        
        // Add word frequency analysis to metadata
        for (word, count) in word_counts {
            analysis_metadata.insert(format!("word_freq_{}", word), count.to_string());
        }
        
        // Add summary statistics to metadata
        let total_words: usize = analysis_metadata.len();
        analysis_metadata.insert("total_unique_words".to_string(), total_words.to_string());
        
        // Create proper protobuf metadata structure
        let mut metadata = HashMap::new();
        metadata.insert("analysis".to_string(), Metadata {
            metadata: analysis_metadata,
        });

        ProcessorResponse {
            outcome: Some(Outcome::NextPayload(req.payload)), // Pass through original payload unchanged
            metadata,
        }
    }

    fn name(&self) -> &'static str {
        "word_frequency_analyzer"
    }

    fn declared_intent(&self) -> ProcessorIntent {
        ProcessorIntent::Analyze
    }
}
