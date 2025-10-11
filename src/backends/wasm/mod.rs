// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

mod error;
pub mod capability_manager;
pub mod executor;
pub mod executors;
pub mod factory;
pub mod module_loader;
pub mod processor;
pub mod processing_node;
pub mod processing_node_factory;

// Re-export the error types for public use
pub use error::{WasmError, WasmResult};

pub use capability_manager::{CapabilityManager, CapabilityRequirements, WasiSetup};
pub use executor::{WasmExecutor, ExecutionResult};
pub use executors::{ComponentNodeExecutor, CStyleNodeExecutor, WasiNodeExecutor};
pub use factory::WasmProcessorFactory;
pub use module_loader::{WasmModuleLoader, LoadedModule, ComponentType, ModuleImport, ImportType};
pub use processor::WasmProcessor;
pub use processing_node::{ProcessingNodeExecutor, ProcessingNodeError, ExecutionMetadata};
pub use processing_node_factory::ProcessingNodeFactory;
