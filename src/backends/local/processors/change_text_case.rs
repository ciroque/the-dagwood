use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, ErrorDetail};
use crate::proto::processor_v1::processor_response::Outcome;
use crate::traits::Processor;

/// Case transformation types supported by the ChangeTextCase processor
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaseType {
    Upper,
    Lower,
    Proper,
    Title,
    #[serde(untagged)]
    Custom(String),  // Fallback for extensibility
}

impl CaseType {
    /// Create a CaseType from a string, with fallback to Custom variant
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "upper" => CaseType::Upper,
            "lower" => CaseType::Lower,
            "proper" => CaseType::Proper,
            "title" => CaseType::Title,
            _ => CaseType::Custom(s.to_string()),
        }
    }
    
    /// Get the string representation of the case type
    pub fn as_str(&self) -> &str {
        match self {
            CaseType::Upper => "upper",
            CaseType::Lower => "lower",
            CaseType::Proper => "proper",
            CaseType::Title => "title",
            CaseType::Custom(s) => s,
        }
    }
}

/// Configuration for the Change Text Case processor
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangeTextCaseConfig {
    pub case_type: CaseType,
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
            case_type: CaseType::Upper,
        })
    }

    pub fn lower() -> Self {
        Self::new(ChangeTextCaseConfig {
            case_type: CaseType::Lower,
        })
    }

    pub fn proper() -> Self {
        Self::new(ChangeTextCaseConfig {
            case_type: CaseType::Proper,
        })
    }

    pub fn title() -> Self {
        Self::new(ChangeTextCaseConfig {
            case_type: CaseType::Title,
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
                    metadata: HashMap::new(),
                    declared_intent: crate::proto::processor_v1::ProcessorIntent::Transform as i32,
                };
            }
        };

        let result = match &self.config.case_type {
            CaseType::Upper => input.to_uppercase(),
            CaseType::Lower => input.to_lowercase(),
            CaseType::Proper => {
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
            CaseType::Title => {
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
            CaseType::Custom(custom_type) => {
                return ProcessorResponse {
                    outcome: Some(Outcome::Error(ErrorDetail {
                        code: 400,
                        message: format!("Unsupported custom case type: {}", custom_type),
                    })),
                    metadata: HashMap::new(),
                    declared_intent: crate::proto::processor_v1::ProcessorIntent::Transform as i32,
                };
            }
        };

        ProcessorResponse {
            outcome: Some(Outcome::NextPayload(result.into_bytes())),
            metadata: HashMap::new(),
            declared_intent: crate::proto::processor_v1::ProcessorIntent::Transform as i32,
        }
    }

    fn name(&self) -> &'static str {
        "change_text_case"
    }

    fn declared_intent(&self) -> crate::proto::processor_v1::ProcessorIntent {
        crate::proto::processor_v1::ProcessorIntent::Transform
    }
}
