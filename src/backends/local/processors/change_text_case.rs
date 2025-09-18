use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, ErrorDetail};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::traits::Processor;

/// Configuration for the Change Text Case processor
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangeTextCaseConfig {
    pub case_type: String, // "upper", "lower", "proper", "title"
}

/// Change Text Case processor - converts text to different cases
pub struct ChangeTextCaseProcessor {
    config: ChangeTextCaseConfig,
}

impl ChangeTextCaseProcessor {
    pub fn new(config: ChangeTextCaseConfig) -> Self {
        Self { config }
    }

    pub fn upper() -> Self {
        Self::new(ChangeTextCaseConfig {
            case_type: "upper".to_string(),
        })
    }

    pub fn lower() -> Self {
        Self::new(ChangeTextCaseConfig {
            case_type: "lower".to_string(),
        })
    }

    pub fn proper() -> Self {
        Self::new(ChangeTextCaseConfig {
            case_type: "proper".to_string(),
        })
    }

    pub fn title() -> Self {
        Self::new(ChangeTextCaseConfig {
            case_type: "title".to_string(),
        })
    }
}

#[async_trait]
impl Processor for ChangeTextCaseProcessor {
    async fn process(&self, req: ProcessorRequest) -> ProcessorResponse {
        let input = match String::from_utf8(req.payload) {
            Ok(text) => text,
            Err(e) => {
                return ProcessorResponse {
                    outcome: Some(Outcome::Error(ErrorDetail {
                        code: 400,
                        message: format!("Invalid UTF-8 input: {}", e),
                    })),
                };
            }
        };

        let result = match self.config.case_type.as_str() {
            "upper" => input.to_uppercase(),
            "lower" => input.to_lowercase(),
            "proper" => {
                // Proper case: first letter of each word capitalized
                input
                    .split_whitespace()
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            }
            "title" => {
                // Title case: similar to proper but with some exceptions for articles, prepositions
                input
                    .split_whitespace()
                    .enumerate()
                    .map(|(i, word)| {
                        let lower_word = word.to_lowercase();
                        // Always capitalize first word, otherwise check if it's a small word
                        if i == 0 || !matches!(lower_word.as_str(), "a" | "an" | "the" | "and" | "or" | "but" | "in" | "on" | "at" | "to" | "for" | "of" | "with" | "by") {
                            let mut chars = word.chars();
                            match chars.next() {
                                None => String::new(),
                                Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                            }
                        } else {
                            lower_word
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            }
            _ => {
                return ProcessorResponse {
                    outcome: Some(Outcome::Error(ErrorDetail {
                        code: 400,
                        message: format!("Unknown case type: {}", self.config.case_type),
                    })),
                };
            }
        };

        ProcessorResponse {
            outcome: Some(Outcome::NextPayload(result.into_bytes())),
        }
    }

    fn name(&self) -> &'static str {
        "change_text_case"
    }
}
