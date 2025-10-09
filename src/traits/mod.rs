// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

pub mod executor;
pub mod processor;

pub use executor::DagExecutor;
pub use crate::config::{ProcessorMap, DependencyGraph, EntryPoints};
pub use processor::Processor;
