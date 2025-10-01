mod loader;
mod validation;
mod dependency_graph;
mod entry_points;
mod processor_map;
mod runtime;

#[cfg(test)]
mod integration_tests;

pub use loader::{Config, ProcessorConfig, Strategy, BackendType, ExecutorOptions, load_config, load_and_validate_config};
pub use validation::validate_dependency_graph;
pub use dependency_graph::DependencyGraph;
pub use entry_points::EntryPoints;
pub use processor_map::ProcessorMap;
pub use runtime::RuntimeBuilder;
