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

pub mod module_loader;

pub use error::{WasmError, WasmResult};

pub use detector::{wasm_encoding, WasmEncoding};
pub use factory::create_executor;
pub use loader::load_wasm_bytes;

pub use executors::{CStyleNodeExecutor, WitNodeExecutor};

pub use processing_node::{ExecutionMetadata, ProcessingNodeError, ProcessingNodeExecutor};

pub use processor::WasmProcessor;

pub use module_loader::{ComponentType, ImportType, LoadedModule, ModuleImport, WasmArtifact};
