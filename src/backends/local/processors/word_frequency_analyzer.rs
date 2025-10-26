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
        let start_msg = ProcessorExecutionStarted {
            processor_id: self.name(),
            input_size: req.payload.len(),
        };

        let span = start_msg.span("processor_execution");
        let _guard = span.enter();
        start_msg.log();

        let start_time = Instant::now();

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

        // Create pipeline metadata with our processor's results
        let mut pipeline_metadata = PipelineMetadata::new();
        pipeline_metadata.metadata.insert(
            self.name().to_string(),
            ProcessorMetadata {
                metadata: analysis_metadata,
            },
        );

        let duration = start_time.elapsed();

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
        "word_frequency_analyzer"
    }

    fn declared_intent(&self) -> ProcessorIntent {
        ProcessorIntent::Analyze
    }
}
