// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WASM Executor Implementations
//!
//! This module contains the concrete implementations of the `ProcessingNodeExecutor`
//! trait for different types of WASM artifacts.

mod cstyle_executor;
mod wit_executor;

pub use cstyle_executor::CStyleNodeExecutor;
pub use wit_executor::WitNodeExecutor;
