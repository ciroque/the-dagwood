mod loader;
mod registry;
mod validation;

#[cfg(test)]
mod integration_tests;

pub use loader::{Config, ProcessorConfig, Strategy, BackendType, CollectionStrategy, ConflictResolution, ExecutorOptions, load_config, load_and_validate_config};
pub use registry::{build_registry, build_executor, build_dag_runtime};
pub use validation::validate_dependency_graph;