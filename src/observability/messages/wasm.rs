// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! Message types for WASM backend loading and execution events.
//!
//! This module contains message types for logging events related to:
//! * WASM module loading and validation
//! * WASM component type detection
//! * WASM executor creation and configuration
//! * WASM execution lifecycle and performance

use std::fmt::{Display, Formatter};

/// WASM module loaded successfully.
///
/// # Log Level
/// `info!` - Important operational event
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::wasm::ModuleLoaded;
///
/// let msg = ModuleLoaded {
///     module_path: "wasm_modules/hello_world.wasm",
///     size_bytes: 4096,
/// };
///
/// tracing::info!("{}", msg);
/// ```
pub struct ModuleLoaded<'a> {
    pub module_path: &'a str,
    pub size_bytes: usize,
}

impl Display for ModuleLoaded<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Loaded WASM module: {} ({} bytes)",
            self.module_path, self.size_bytes
        )
    }
}

/// WASM module loading failed.
///
/// # Log Level
/// `error!` - Failure requiring attention
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::wasm::ModuleLoadFailed;
///
/// let error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
/// let msg = ModuleLoadFailed {
///     module_path: "wasm_modules/missing.wasm",
///     error: &error,
/// };
///
/// tracing::error!("{}", msg);
/// ```
pub struct ModuleLoadFailed<'a> {
    pub module_path: &'a str,
    pub error: &'a dyn std::error::Error,
}

impl Display for ModuleLoadFailed<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Failed to load WASM module '{}': {}",
            self.module_path, self.error
        )
    }
}

/// WASM component type detected.
///
/// # Log Level
/// `info!` - Important operational event
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::wasm::ComponentTypeDetected;
///
/// let msg = ComponentTypeDetected {
///     module_path: "wasm_modules/hello_world.wasm",
///     component_type: "CStyle",
/// };
///
/// tracing::info!("{}", msg);
/// ```
pub struct ComponentTypeDetected<'a> {
    pub module_path: &'a str,
    pub component_type: &'a str,
}

impl Display for ComponentTypeDetected<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Detected {} component type for module: {}",
            self.component_type, self.module_path
        )
    }
}

/// WASM executor created.
///
/// # Log Level
/// `info!` - Important operational event
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::wasm::ExecutorCreated;
///
/// let msg = ExecutorCreated {
///     executor_type: "CStyleNodeExecutor",
///     fuel_level: 1_000_000,
/// };
///
/// tracing::info!("{}", msg);
/// ```
pub struct ExecutorCreated<'a> {
    pub executor_type: &'a str,
    pub fuel_level: u64,
}

impl Display for ExecutorCreated<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Created {} with fuel_level={}",
            self.executor_type, self.fuel_level
        )
    }
}

/// WASM execution started.
///
/// # Log Level
/// `info!` - Important operational event
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::wasm::ExecutionStarted;
///
/// let msg = ExecutionStarted {
///     module_path: "wasm_modules/hello_world.wasm",
///     executor_type: "CStyleNodeExecutor",
///     input_size: 1024,
/// };
///
/// tracing::info!("{}", msg);
/// ```
pub struct ExecutionStarted<'a> {
    pub module_path: &'a str,
    pub executor_type: &'a str,
    pub input_size: usize,
}

impl Display for ExecutionStarted<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Executing WASM module '{}' using {} executor: input_size={} bytes",
            self.module_path, self.executor_type, self.input_size
        )
    }
}

/// WASM execution completed successfully.
///
/// # Log Level
/// `info!` - Important operational event
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::wasm::ExecutionCompleted;
///
/// let msg = ExecutionCompleted {
///     module_path: "wasm_modules/hello_world.wasm",
///     executor_type: "CStyleNodeExecutor",
///     input_size: 1024,
///     output_size: 1050,
/// };
///
/// tracing::info!("{}", msg);
/// ```
pub struct ExecutionCompleted<'a> {
    pub module_path: &'a str,
    pub executor_type: &'a str,
    pub input_size: usize,
    pub output_size: usize,
}

impl Display for ExecutionCompleted<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "WASM execution successful for '{}' using {}: input={} bytes, output={} bytes",
            self.module_path, self.executor_type, self.input_size, self.output_size
        )
    }
}

/// WASM execution failed.
///
/// # Log Level
/// `error!` - Failure requiring attention
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::wasm::ExecutionFailed;
///
/// let error = std::io::Error::new(std::io::ErrorKind::Other, "trap occurred");
/// let msg = ExecutionFailed {
///     module_path: "wasm_modules/hello_world.wasm",
///     executor_type: "CStyleNodeExecutor",
///     error: &error,
/// };
///
/// tracing::error!("{}", msg);
/// ```
pub struct ExecutionFailed<'a> {
    pub module_path: &'a str,
    pub executor_type: &'a str,
    pub error: &'a dyn std::error::Error,
}

impl Display for ExecutionFailed<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "WASM execution failed for '{}' using {}: {}",
            self.module_path, self.executor_type, self.error
        )
    }
}

/// WASM engine creation started.
///
/// # Log Level
/// `info!` - Important operational event
///
/// # Example
/// ```
/// use the_dagwood::observability::messages::wasm::EngineCreationStarted;
///
/// let msg = EngineCreationStarted {
///     component_type: "Wit",
/// };
///
/// tracing::info!("{}", msg);
/// ```
pub struct EngineCreationStarted<'a> {
    pub component_type: &'a str,
}

impl Display for EngineCreationStarted<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Creating WASM engine for {} component type",
            self.component_type
        )
    }
}
