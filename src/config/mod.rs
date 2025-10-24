// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

mod dependency_graph;
mod entry_points;
mod loader;
mod processor_map;
mod runtime;
mod validation;

#[cfg(test)]
mod integration_tests;
pub mod consts;

pub use dependency_graph::DependencyGraph;
pub use entry_points::EntryPoints;
pub use loader::{
    load_and_validate_config, load_config, BackendType, Config, ExecutorOptions, FuelConfig,
    ProcessorConfig, Strategy, WasmConfig
};
pub use processor_map::ProcessorMap;
pub use runtime::RuntimeBuilder;
pub use validation::validate_dependency_graph;
