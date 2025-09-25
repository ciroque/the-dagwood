use async_trait::async_trait;
use std::collections::HashMap;
use crate::traits::Processor;
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, ErrorDetail};
use crate::proto::processor_v1::processor_response::Outcome;
use super::{ResultCollector, CollectableResult};

/// CustomCollector provides extensible custom collection logic
/// 
/// This collector serves as a placeholder for custom collection strategies
/// that can be implemented by users. It currently returns an error but
/// can be extended to support pluggable custom combiners.
pub struct CustomCollector {
    combiner_impl: String,
}

impl CustomCollector {
    pub fn new(combiner_impl: String) -> Self {
        Self { combiner_impl }
    }

    /// Helper to create error responses
    fn error_response(&self, message: &str) -> ProcessorResponse {
        ProcessorResponse {
            outcome: Some(Outcome::Error(ErrorDetail {
                code: 500,
                message: message.to_string(),
            })),
            metadata: HashMap::new(),
            declared_intent: crate::proto::processor_v1::ProcessorIntent::Transform as i32,
        }
    }
}

#[async_trait]
impl ResultCollector for CustomCollector {
    async fn collect_results(
        &self,
        _dependency_results: &HashMap<String, CollectableResult>,
        _request: &ProcessorRequest,
    ) -> ProcessorResponse {
        // TODO: Implement custom combiner loading/execution
        // This could be extended to support:
        // - Dynamic loading of custom combiners
        // - Scripting language integration (Lua, Python, etc.)
        // - Plugin system for user-defined collection strategies
        self.error_response(&format!("Custom combiner '{}' not implemented yet", self.combiner_impl))
    }
}

#[async_trait]
impl Processor for CustomCollector {
    async fn process(&self, request: ProcessorRequest) -> ProcessorResponse {
        // The input payload should contain serialized dependency results
        match serde_json::from_slice::<HashMap<String, CollectableResult>>(&request.payload) {
            Ok(dependency_results) => self.collect_results(&dependency_results, &request).await,
            Err(e) => self.error_response(&format!("Failed to deserialize dependency results: {}", e)),
        }
    }

    fn name(&self) -> &'static str {
        "custom_collector"
    }
}
