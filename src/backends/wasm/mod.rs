// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

pub mod capability_manager;
pub mod component_detector;
mod error;
pub mod executors;
pub mod module_loader;
pub mod processing_node;
pub mod processing_node_factory;
pub mod processor;

pub use error::{WasmError, WasmResult};

pub use component_detector::WasmComponentDetector;
pub use executors::{CStyleNodeExecutor, ComponentNodeExecutor, WasiNodeExecutor, WitNodeExecutor};
pub use module_loader::{ComponentType, ImportType, LoadedModule, ModuleImport, WasmArtifact, WasmModuleLoader};
pub use processing_node::{ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor};
pub use processing_node_factory::ProcessingNodeFactory;
pub use processor::WasmProcessor;
