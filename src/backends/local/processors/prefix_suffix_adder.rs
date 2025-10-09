// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, ErrorDetail};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::traits::{Processor, processor::ProcessorIntent};

/// Configuration for the Prefix/Suffix Adder processor
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PrefixSuffixConfig {
    pub prefix: Option<String>,
    pub suffix: Option<String>,
}

/// Prefix/Suffix Adder processor - adds prefix and/or suffix to text
pub struct PrefixSuffixAdderProcessor {
    config: PrefixSuffixConfig,
}

impl PrefixSuffixAdderProcessor {
    pub fn new(config: PrefixSuffixConfig) -> Self {
        Self { config }
    }

    pub fn with_prefix(prefix: String) -> Self {
        Self::new(PrefixSuffixConfig {
            prefix: Some(prefix),
            suffix: None,
        })
    }

    pub fn with_suffix(suffix: String) -> Self {
        Self::new(PrefixSuffixConfig {
            prefix: None,
            suffix: Some(suffix),
        })
    }

    pub fn with_prefix_and_suffix(prefix: String, suffix: String) -> Self {
        Self::new(PrefixSuffixConfig {
            prefix: Some(prefix),
            suffix: Some(suffix),
        })
    }
}

#[async_trait]
impl Processor for PrefixSuffixAdderProcessor {
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

        let mut result = String::new();
        
        if let Some(prefix) = &self.config.prefix {
            result.push_str(prefix);
        }
        
        result.push_str(&input);
        
        if let Some(suffix) = &self.config.suffix {
            result.push_str(suffix);
        }

        ProcessorResponse {
            outcome: Some(Outcome::NextPayload(result.into_bytes())),
            metadata: None,
        }
    }

    fn name(&self) -> &'static str {
        "prefix_suffix_adder"
    }

    fn declared_intent(&self) -> ProcessorIntent {
        ProcessorIntent::Transform
    }
}
