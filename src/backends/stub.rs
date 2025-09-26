use crate::traits::{Processor, processor::ProcessorIntent};

/// A stub processor implementation for testing and placeholder purposes
pub struct StubProcessor {
    pub id: String,
}

impl StubProcessor {
    pub fn new(id: String) -> Self {
        Self { id }
    }
}

#[async_trait::async_trait]
impl Processor for StubProcessor {
    async fn process(
        &self,
        _req: crate::proto::processor_v1::ProcessorRequest,
    ) -> crate::proto::processor_v1::ProcessorResponse {
        // For now, just return an empty success response
        crate::proto::processor_v1::ProcessorResponse {
            outcome: Some(
                crate::proto::processor_v1::processor_response::Outcome::NextPayload(vec![])
            ),
            metadata: std::collections::HashMap::new(),
        }
    }

    fn name(&self) -> &'static str {
        "stub"
    }

    fn declared_intent(&self) -> ProcessorIntent {
        ProcessorIntent::Transform
    }
}