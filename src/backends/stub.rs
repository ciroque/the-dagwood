// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use crate::traits::{processor::ProcessorIntent, Processor};

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
                crate::proto::processor_v1::processor_response::Outcome::NextPayload(vec![]),
            ),
            metadata: None,
        }
    }

    fn name(&self) -> &'static str {
        "stub"
    }

    fn declared_intent(&self) -> ProcessorIntent {
        ProcessorIntent::Transform
    }
}

/// A processor that always fails for testing failure scenarios
pub struct FailingProcessor {
    pub id: String,
}

impl FailingProcessor {
    pub fn new(id: String) -> Self {
        Self { id }
    }
}

#[async_trait::async_trait]
impl Processor for FailingProcessor {
    async fn process(
        &self,
        _req: crate::proto::processor_v1::ProcessorRequest,
    ) -> crate::proto::processor_v1::ProcessorResponse {
        // Always return an error
        crate::proto::processor_v1::ProcessorResponse {
            outcome: Some(
                crate::proto::processor_v1::processor_response::Outcome::Error(
                    crate::proto::processor_v1::ErrorDetail {
                        code: 500,
                        message: "Simulated processor failure".to_string(),
                    },
                ),
            ),
            metadata: None,
        }
    }

    fn name(&self) -> &'static str {
        "failing"
    }

    fn declared_intent(&self) -> ProcessorIntent {
        ProcessorIntent::Transform
    }
}

/// A processor that returns no outcome for testing invalid response scenarios
pub struct NoOutcomeProcessor {
    pub id: String,
}

impl NoOutcomeProcessor {
    pub fn new(id: String) -> Self {
        Self { id }
    }
}

#[async_trait::async_trait]
impl Processor for NoOutcomeProcessor {
    async fn process(
        &self,
        _req: crate::proto::processor_v1::ProcessorRequest,
    ) -> crate::proto::processor_v1::ProcessorResponse {
        // Return no outcome (None)
        crate::proto::processor_v1::ProcessorResponse {
            outcome: None,
            metadata: None,
        }
    }

    fn name(&self) -> &'static str {
        "no_outcome"
    }

    fn declared_intent(&self) -> ProcessorIntent {
        ProcessorIntent::Transform
    }
}
