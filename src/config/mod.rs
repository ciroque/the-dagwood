mod loader;
mod registry;

pub use loader::{Config, ProcessorConfig, Strategy, BackendType, load_config};
pub use registry::build_registry;