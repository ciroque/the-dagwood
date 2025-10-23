// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

pub mod bindings;
pub mod capability_manager;
pub mod detector;
pub mod factory;
pub mod loader;
mod error;
pub mod executors;
pub mod processing_node;
pub mod processing_node_factory;
pub mod processor;

// Legacy modules (will be deprecated)
pub mod module_loader;

pub use error::{WasmError, WasmResult};

// New clean API (ADR-17)
pub use detector::{wasm_encoding, WasmEncoding};
pub use factory::create_executor;
pub use loader::load_wasm_bytes;

// Executors
pub use executors::{CStyleNodeExecutor, ComponentNodeExecutor, WasiNodeExecutor, WitNodeExecutor};

// Processing node traits and types
pub use processing_node::{ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor};

// Processor
pub use processor::WasmProcessor;

// Legacy exports (for backwards compatibility during transition)
// pub use component_detector::WasmComponentDetector;
pub use module_loader::{ComponentType, ImportType, LoadedModule, ModuleImport, WasmArtifact};
