// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Stub processor implementations for testing DAG executors and failure scenarios.
//!
//! This module provides lightweight processor implementations designed specifically for
//! testing executor logic, error handling, and edge cases without the overhead of real
//! processor implementations.
//!
//! # Available Stubs
//!
//! ## StubProcessor
//! A minimal no-op processor that always succeeds with empty output:
//! - **Use Case**: Testing DAG structure and dependency resolution
//! - **Behavior**: Returns `NextPayload(vec![])` immediately
//! - **Intent**: Transform processor
//! - **Performance**: Zero overhead, instant execution
//!
//! ## FailingProcessor
//! A processor that always fails with a simulated error:
//! - **Use Case**: Testing error handling and failure strategies
//! - **Behavior**: Returns `Error` outcome with code 500
//! - **Intent**: Transform processor
//! - **Testing**: Validates FailFast, ContinueOnError, BestEffort strategies
//!
//! ## NoOutcomeProcessor
//! A processor that returns invalid responses (no outcome):
//! - **Use Case**: Testing executor robustness against malformed responses
//! - **Behavior**: Returns `ProcessorResponse { outcome: None, metadata: None }`
//! - **Intent**: Transform processor
//! - **Testing**: Validates executor error handling for protocol violations
//!
//! # Examples
//!
//! ## Testing DAG Structure
//! ```rust
//! use std::sync::Arc;
//! use the_dagwood::backends::stub::StubProcessor;
//! use the_dagwood::traits::Processor;
//! use the_dagwood::proto::processor_v1::ProcessorRequest;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let processor = Arc::new(StubProcessor::new("test".to_string()));
//! let request = ProcessorRequest { payload: vec![] };
//! let response = processor.process(request).await;
//!
//! assert!(response.outcome.is_some());
//! # }
//! ```
//!
//! ## Testing Failure Handling
//! ```rust
//! use std::sync::Arc;
//! use the_dagwood::backends::stub::FailingProcessor;
//! use the_dagwood::traits::Processor;
//! use the_dagwood::proto::processor_v1::{ProcessorRequest, processor_response::Outcome};
//!
//! # #[tokio::main]
//! # async fn main() {
//! let processor = Arc::new(FailingProcessor::new("failing".to_string()));
//! let request = ProcessorRequest { payload: vec![] };
//! let response = processor.process(request).await;
//!
//! match response.outcome {
//!     Some(Outcome::Error(err)) => {
//!         assert_eq!(err.code, 500);
//!         assert_eq!(err.message, "Simulated processor failure");
//!     }
//!     _ => panic!("Expected error outcome"),
//! }
//! # }
//! ```

use crate::traits::{processor::ProcessorIntent, Processor};

/// A stub processor implementation for testing DAG structure and dependency resolution.
///
/// This processor always succeeds immediately with an empty payload, making it ideal for
/// testing executor logic without the complexity of real processor implementations.
///
/// ## Behavior
/// - **Execution**: Instant, no actual processing
/// - **Output**: Empty payload (`vec![]`)
/// - **Metadata**: None
/// - **Intent**: Transform processor
///
/// ## Use Cases
/// - Testing DAG topology and dependency resolution
/// - Benchmarking executor overhead
/// - Integration testing without real processors
/// - Validating concurrency and parallelism logic
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

/// A processor that always fails for testing error handling and failure strategies.
///
/// This processor simulates processor failures to validate executor error handling,
/// failure strategies (FailFast, ContinueOnError, BestEffort), and dependent notification.
///
/// ## Behavior
/// - **Execution**: Instant failure
/// - **Output**: Error outcome with code 500
/// - **Message**: "Simulated processor failure"
/// - **Intent**: Transform processor
///
/// ## Use Cases
/// - Testing FailFast strategy (executor stops on first failure)
/// - Testing ContinueOnError strategy (executor continues despite failures)
/// - Testing BestEffort strategy (executor collects partial results)
/// - Validating dependent processor notification on failure
/// - Testing panic recovery and deadlock prevention
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

/// A processor that returns invalid responses for testing executor robustness.
///
/// This processor violates the processor protocol by returning responses with no outcome,
/// allowing validation of executor error handling for malformed responses.
///
/// ## Behavior
/// - **Execution**: Instant
/// - **Output**: `ProcessorResponse { outcome: None, metadata: None }`
/// - **Protocol Violation**: Missing required outcome field
/// - **Intent**: Transform processor
///
/// ## Use Cases
/// - Testing executor handling of protocol violations
/// - Validating error messages for invalid responses
/// - Testing executor robustness against malformed data
/// - Ensuring graceful degradation on invalid processor behavior
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
