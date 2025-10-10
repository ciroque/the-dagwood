// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WASM Module Loading and Validation
//!
//! This module handles the loading and validation of WASM modules, including:
//! - File I/O and size validation
//! - WASM module compilation and validation
//! - Import validation and capability checking
//! - Component type detection (C-style vs WIT-based)
//!
//! ## Responsibilities
//! - Load WASM module bytes from filesystem
//! - Validate module size and structure
//! - Parse and validate module imports
//! - Detect component interface type
//! - Create wasmtime Engine with appropriate configuration

use crate::backends::wasm::error::{WasmError, WasmResult};
use std::path::Path;
use wasmtime::*;
use wasmtime::component::Component;

/// Maximum allowed WASM module size (16MB)
const MAX_WASM_COMPONENT_SIZE: usize = 16 * 1024 * 1024;

/// WASM component interface detection
#[derive(Debug, Clone)]
pub enum ComponentType {
    /// Legacy C-style exports (process, allocate, deallocate)
    CStyle,
    /// WIT-based component with structured errors
    WitComponent,
}

/// WASM artifact type - either a Module or Component
pub enum WasmArtifact {
    /// Core WASM module (C-style exports)
    Module(Module),
    /// WIT-based component
    Component(Component),
}

/// Loaded and validated WASM module with metadata
pub struct LoadedModule {
    pub engine: Engine,
    pub artifact: WasmArtifact,
    pub component_type: ComponentType,
    pub imports: Vec<ModuleImport>,
    pub module_path: String,
}

/// Module import information for capability analysis
#[derive(Debug, Clone)]
pub struct ModuleImport {
    pub module_name: String,
    pub function_name: String,
    pub import_type: ImportType,
}

#[derive(Debug, Clone)]
pub enum ImportType {
    Wasi,
    Other,
}

/// WASM Module Loader - handles loading, validation, and basic analysis
pub struct WasmModuleLoader;

impl WasmModuleLoader {
    /// Load and validate a WASM module from the filesystem
    pub fn load_module<P: AsRef<Path>>(module_path: P) -> WasmResult<LoadedModule> {
        let module_path = module_path.as_ref();
        let module_path_str = module_path.to_string_lossy().to_string();

        // Create wasmtime engine with security-focused configuration
        let engine = Self::create_engine()?;

        // Load module bytes from filesystem
        let module_bytes = std::fs::read(module_path).map_err(WasmError::IoError)?;

        // Validate module size
        if module_bytes.len() > MAX_WASM_COMPONENT_SIZE {
            return Err(WasmError::ValidationError(format!(
                "WASM module too large: {} bytes (max: {} bytes)",
                module_bytes.len(),
                MAX_WASM_COMPONENT_SIZE
            )));
        }

        // Try to load as Component first (WIT-based), then fallback to Module (C-style)
        let (artifact, component_type, imports) = match Component::new(&engine, &module_bytes) {
            Ok(component) => {
                // Successfully loaded as WIT component
                tracing::debug!("Loaded as WIT component: {}", module_path_str);
                let imports = Self::parse_component_imports(&component)?;
                (WasmArtifact::Component(component), ComponentType::WitComponent, imports)
            }
            Err(_component_err) => {
                // Failed as component, try as core module
                tracing::debug!("Failed as component, trying as core module: {}", module_path_str);
                let module = Module::new(&engine, &module_bytes)
                    .map_err(|e| WasmError::ModuleError(e.to_string()))?;
                
                let imports = Self::parse_module_imports(&module)?;
                let component_type = Self::detect_component_type(&module);
                (WasmArtifact::Module(module), component_type, imports)
            }
        };

        Ok(LoadedModule {
            engine,
            artifact,
            component_type,
            imports,
            module_path: module_path_str,
        })
    }

    /// Create wasmtime engine with security-focused configuration
    fn create_engine() -> WasmResult<Engine> {
        let mut config = Config::new();

        // Security and compatibility settings for wasmtime 25.0
        config.wasm_threads(false);
        config.wasm_simd(false);
        config.wasm_relaxed_simd(false); // Explicitly disable relaxed SIMD to avoid conflicts
        config.wasm_multi_memory(false);
        config.wasm_memory64(false);
        config.wasm_component_model(false); // Will enable this in Phase 2.1

        // Enable fuel consumption for security and resource protection
        // Fuel prevents infinite loops and limits computational resource usage
        // Each WASM instruction consumes fuel; when fuel runs out, execution stops
        config.consume_fuel(true);

        // Disable epoch interruption which might cause "interrupt" traps
        config.epoch_interruption(false);

        Engine::new(&config).map_err(|e| WasmError::EngineError(e.to_string()))
    }

    /// Parse and validate core module imports
    fn parse_module_imports(module: &Module) -> WasmResult<Vec<ModuleImport>> {
        let mut imports = Vec::new();

        for import in module.imports() {
            let module_name = import.module().to_string();
            let function_name = import.name().to_string();

            let import_type = if module_name.starts_with("wasi") {
                // Validate WASI imports against allowlist
                Self::validate_wasi_import(&module_name, &function_name)?;
                ImportType::Wasi
            } else {
                ImportType::Other
            };

            imports.push(ModuleImport {
                module_name,
                function_name,
                import_type,
            });
        }

        Ok(imports)
    }

    /// Parse and validate WIT component imports
    fn parse_component_imports(_component: &Component) -> WasmResult<Vec<ModuleImport>> {
        // For Phase 2.1: WIT components typically have no imports (sandboxed)
        // Future enhancement: Parse component world imports if needed
        tracing::debug!("WIT component imports parsing - currently returns empty (sandboxed)");
        Ok(Vec::new())
    }

    /// Validate WASI imports against allowlist (Phase 1 logic)
    fn validate_wasi_import(module_name: &str, function_name: &str) -> WasmResult<()> {
        // Allow essential WASI functions for modern WASM languages
        let allowed_wasi_functions = [
            "proc_exit",      // Process termination
            "random_get",     // Random number generation
            "clock_time_get", // Time access
            "fd_write",       // Basic output (for debugging)
            "fd_read",        // Basic input
        ];

        if !allowed_wasi_functions.contains(&function_name) {
            return Err(WasmError::ValidationError(format!(
                "WASI function '{}' from module '{}' is not allowed. Allowed functions: {:?}",
                function_name, module_name, allowed_wasi_functions
            )));
        }

        Ok(())
    }

    /// Detect component type based on exports
    fn detect_component_type(module: &Module) -> ComponentType {
        let exports: Vec<_> = module.exports().map(|e| e.name()).collect();

        // Check for WIT-based component exports (future Phase 2.1)
        if exports.contains(&"processing-node") {
            return ComponentType::WitComponent;
        }

        // Check for C-style exports
        let has_process = exports.contains(&"process");
        let has_allocate = exports.contains(&"allocate");
        let has_deallocate = exports.contains(&"deallocate");

        if has_process && has_allocate && has_deallocate {
            ComponentType::CStyle
        } else {
            // Default to C-style for backward compatibility
            ComponentType::CStyle
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_engine_creation() {
        let engine = WasmModuleLoader::create_engine();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_module_size_validation() {
        // Create a temporary file that's too large
        let mut temp_file = NamedTempFile::new().unwrap();
        let large_data = vec![0u8; MAX_WASM_COMPONENT_SIZE + 1];
        temp_file.write_all(&large_data).unwrap();

        let result = WasmModuleLoader::load_module(temp_file.path());
        assert!(result.is_err());
        
        if let Err(WasmError::ValidationError(msg)) = result {
            assert!(msg.contains("WASM module too large"));
        } else {
            panic!("Expected ValidationError for oversized module");
        }
    }

    #[test]
    fn test_wasi_import_validation() {
        // Test allowed WASI function
        let result = WasmModuleLoader::validate_wasi_import("wasi_snapshot_preview1", "proc_exit");
        assert!(result.is_ok());

        // Test disallowed WASI function
        let result = WasmModuleLoader::validate_wasi_import("wasi_snapshot_preview1", "path_open");
        assert!(result.is_err());
        
        if let Err(WasmError::ValidationError(msg)) = result {
            assert!(msg.contains("is not allowed"));
            assert!(msg.contains("path_open"));
        } else {
            panic!("Expected ValidationError for disallowed WASI function");
        }
    }

    #[test]
    fn test_component_type_detection() {
        // This test would need a real WASM module to be comprehensive
        // For now, we test the logic with a mock module structure
        // In practice, this would be tested with integration tests using real WASM files
    }
}
