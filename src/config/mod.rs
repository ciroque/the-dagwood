mod loader;
mod registry;
mod validation;
mod dependency_graph;
mod entry_points;
mod processor_map;

#[cfg(test)]
mod integration_tests;

pub use loader::{Config, ProcessorConfig, Strategy, BackendType, ExecutorOptions, load_config, load_and_validate_config};
pub use registry::{build_registry, build_executor, build_dag_runtime};
pub use validation::validate_dependency_graph;
pub use dependency_graph::DependencyGraph;
pub use entry_points::EntryPoints;
pub use processor_map::ProcessorMap;
