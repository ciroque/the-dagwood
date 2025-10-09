// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

mod error;
pub mod factory;
pub mod processor;

// Re-export the error types for public use
pub use error::{WasmError, WasmResult};

pub use factory::WasmProcessorFactory;
pub use processor::WasmProcessor;
