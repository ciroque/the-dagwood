use async_trait::async_trait;
use std::collections::HashMap;
use crate::traits::Processor;
use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse, ErrorDetail};
use crate::proto::processor_v1::processor_response::Outcome;
use super::{ResultCollector, CollectableResult};

/// FirstAvailableCollector uses the first successful dependency result
/// 
/// This collector implements a simple fallback strategy where it returns
/// the payload from the first dependency that completed successfully.
/// This is the default behavior that matches the original work queue executor.
pub struct FirstAvailableCollector;

impl FirstAvailableCollector {
    pub fn new() -> Self {
        Self
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
impl ResultCollector for FirstAvailableCollector {
    async fn collect_results(
        &self,
        dependency_results: &HashMap<String, CollectableResult>,
        _request: &ProcessorRequest,
    ) -> ProcessorResponse {

        // Iterate in deterministic order by sorting dependency IDs
        let mut keys: Vec<_> = dependency_results.keys().collect();
        keys.sort();
        for key in keys {
            if let Some(result) = dependency_results.get(key) {
                if result.success {
                    if let Some(payload) = &result.payload {
                        return ProcessorResponse {
                            outcome: Some(Outcome::NextPayload(payload.clone())),
                            metadata: HashMap::new(),
                            declared_intent: crate::proto::processor_v1::ProcessorIntent::Transform as i32,
                        }
                    }
                }
            }
        }

        self.error_response("No successful dependency results found")
    }
}

#[async_trait]
impl Processor for FirstAvailableCollector {
    async fn process(&self, request: ProcessorRequest) -> ProcessorResponse {
        // The input payload should contain serialized dependency results
        match serde_json::from_slice::<HashMap<String, CollectableResult>>(&request.payload) {
            Ok(dependency_results) => self.collect_results(&dependency_results, &request).await,
            Err(e) => self.error_response(&format!("Failed to deserialize dependency results: {}", e)),
        }
    }

    fn name(&self) -> &'static str {
        "first_available_collector"
    }
}
