mod loader;
mod registry;
mod validation;

pub use loader::{Config, ProcessorConfig, Strategy, BackendType, CollectionStrategy, ConflictResolution, load_config, load_and_validate_config};
pub use registry::build_registry;
pub use validation::validate_dependency_graph;