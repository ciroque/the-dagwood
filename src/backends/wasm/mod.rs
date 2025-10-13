pub mod capability_manager;
pub mod component_detector;
mod error;
pub mod executors;
pub mod factory;
pub mod module_loader;
pub mod processing_node;
pub mod processing_node_factory;
pub mod processor;

pub use error::{WasmError, WasmResult};

pub use component_detector::WasmComponentDetector;
pub use executors::{CStyleNodeExecutor, ComponentNodeExecutor, WasiNodeExecutor};
pub use factory::WasmProcessorFactory;
pub use module_loader::{ComponentType, ImportType, LoadedModule, ModuleImport, WasmModuleLoader};
pub use processing_node::{ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor};
pub use processing_node_factory::ProcessingNodeFactory;
pub use processor::WasmProcessor;
