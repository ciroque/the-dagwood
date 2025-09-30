use async_trait::async_trait;

use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse};

/// Processor intent declaration for safe parallelism
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessorIntent {
    /// Modifies payload, may modify metadata - must run sequentially
    Transform,
    /// Payload pass-through, may add metadata - can run in parallel
    Analyze,
}

/// Simple Processor trait - clean and focused
#[async_trait]
pub trait Processor: Send + Sync {
    /// Process the input request and return a response
    async fn process(&self, req: ProcessorRequest) -> ProcessorResponse;

    /// Return the processor's name/identifier
    fn name(&self) -> &'static str;

    /// Declare the processor's intent (Transform or Analyze)
    /// 
    /// Transform processors can modify payload and metadata.
    /// Analyze processors should only add metadata (executor enforces payload pass-through).
    /// 
    /// Default implementation returns Transform for backward compatibility.
    fn declared_intent(&self) -> ProcessorIntent {
        ProcessorIntent::Transform
    }
}
