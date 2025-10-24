// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

mod config;
mod execution;
mod processor_map;

pub use config::ValidationError;
pub use execution::{ExecutionError, FailureStrategy};
pub use processor_map::ProcessorMapError;
