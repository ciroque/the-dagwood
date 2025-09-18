use async_trait::async_trait;

use crate::proto::processor_v1::{ProcessorRequest, ProcessorResponse};

#[async_trait]
pub trait Processor: Send + Sync {
    async fn process(&self, req: ProcessorRequest) -> ProcessorResponse;

    fn name(&self) -> &'static str;
}
