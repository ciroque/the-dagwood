// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WASM Executor Implementations
//!
//! This module contains the concrete implementations of the `ProcessingNodeExecutor`
//! trait for different types of WASM artifacts.

mod component_executor;
mod cstyle_executor;
mod wasi_executor;

pub use component_executor::ComponentNodeExecutor;
pub use cstyle_executor::CStyleNodeExecutor;
pub use wasi_executor::WasiNodeExecutor;
