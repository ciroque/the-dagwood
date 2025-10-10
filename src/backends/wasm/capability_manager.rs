// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//! WASM Capability Management and WASI Context Creation
//!
//! This module handles capability analysis and WASI context setup for WASM components:
//! - WIT introspection and capability discovery
//! - WASI context creation with minimal privileges
//! - Linker setup with requested capabilities
//! - Future: Policy-based capability restrictions
//!
//! ## Responsibilities
//! - Analyze component imports to determine capability requirements
//! - Create WASI context with only requested capabilities
//! - Configure wasmtime linker with appropriate WASI functions
//! - Validate capability requests against security policies (future)

use crate::backends::wasm::error::WasmResult;
use crate::backends::wasm::module_loader::{LoadedModule, ImportType};
use std::collections::HashSet;
use wasmtime::*;

/// Capability requirements discovered from component analysis
#[derive(Debug, Clone)]
pub struct CapabilityRequirements {
    pub needs_stdio: bool,
    pub needs_clocks: bool,
    pub needs_random: bool,
    pub needs_filesystem: bool,
    pub needs_network: bool,
    pub wasi_functions: HashSet<String>,
}

/// Configured WASI context and linker ready for component instantiation
pub struct WasiSetup {
    pub linker: Linker<()>,
    pub store_data: (),
}

/// Capability Manager - analyzes requirements and creates WASI contexts
pub struct CapabilityManager;

impl CapabilityManager {
    /// Analyze loaded module to determine capability requirements
    pub fn analyze_capabilities(loaded_module: &LoadedModule) -> CapabilityRequirements {
        let mut requirements = CapabilityRequirements {
            needs_stdio: false,
            needs_clocks: false,
            needs_random: false,
            needs_filesystem: false,
            needs_network: false,
            wasi_functions: HashSet::new(),
        };

        // TODO: Phase 2.1 - Parse WIT world from component instead of raw WASM imports
        // Should use: Component::from_bytes() -> resolve.worlds -> world.imports
        // Current: Raw WASM imports (wasi_snapshot_preview1.fd_write)
        // Future: WIT imports (wasi:filesystem/types@0.2.0)
        // 
        // TODO: Consider CapabilityAnalyzer trait for strategy pattern:
        // - WitWorldAnalyzer: Component Model WIT parsing
        // - RawImportAnalyzer: Legacy WASM import analysis (fallback)
        // - Chain of responsibility: try WIT first, fallback to raw imports
        
        // Analyze imports to determine capability needs
        for import in &loaded_module.imports {
            if let ImportType::Wasi = import.import_type {
                requirements.wasi_functions.insert(import.function_name.clone());
                
                // Map WASI functions to capability categories
                match import.function_name.as_str() {
                    "fd_write" | "fd_read" => {
                        requirements.needs_stdio = true;
                    }
                    "clock_time_get" => {
                        requirements.needs_clocks = true;
                    }
                    "random_get" => {
                        requirements.needs_random = true;
                    }
                    "path_open" | "fd_readdir" | "path_create_directory" => {
                        requirements.needs_filesystem = true;
                    }
                    "sock_accept" | "sock_connect" => {
                        requirements.needs_network = true;
                    }
                    _ => {
                        // Unknown WASI function - for now, we allow it (Phase 2 policy)
                        // Future: Check against capability policies
                    }
                }
            }
        }

        requirements
    }

    /// Create WASI context and linker based on capability requirements
    pub fn create_wasi_setup(
        engine: &Engine,
        _requirements: &CapabilityRequirements,
    ) -> WasmResult<WasiSetup> {
        // For Phase 1: Create minimal setup without actual WASI functions
        // TODO: Phase 2.1 - Add actual WASI function provisioning
        
        let linker = Linker::new(engine);
        let store_data = ();

        // Future Phase 2.1 implementation will:
        // 1. Create WasiCtxBuilder based on requirements
        // 2. Add only requested WASI functions to linker
        // 3. Apply capability restrictions based on policies
        
        Ok(WasiSetup {
            linker,
            store_data,
        })
    }

    /// Validate capability requirements against security policies
    /// Phase 2: Allow all for now, Phase 3: Policy-based restrictions
    pub fn validate_capabilities(requirements: &CapabilityRequirements) -> WasmResult<()> {
        // Phase 2: Allow all requested capabilities
        // This is where future policy enforcement will happen
        
        // Log capability requests for audit purposes
        tracing::debug!(
            "Component capability requirements: stdio={}, clocks={}, random={}, fs={}, net={}",
            requirements.needs_stdio,
            requirements.needs_clocks,
            requirements.needs_random,
            requirements.needs_filesystem,
            requirements.needs_network
        );

        // Future Phase 3: Check against configurable policies
        // if requirements.needs_network && !policy.allow_network {
        //     return Err(WasmError::ValidationError("Network access denied by policy".to_string()));
        // }

        Ok(())
    }

    /// Create a store with the appropriate WASI context
    pub fn create_store(engine: &Engine, setup: WasiSetup) -> Store<()> {
        Store::new(engine, setup.store_data)
    }

    /// Get human-readable capability summary for logging/debugging
    pub fn capability_summary(requirements: &CapabilityRequirements) -> String {
        let mut capabilities = Vec::new();
        
        if requirements.needs_stdio {
            capabilities.push("stdio");
        }
        if requirements.needs_clocks {
            capabilities.push("clocks");
        }
        if requirements.needs_random {
            capabilities.push("random");
        }
        if requirements.needs_filesystem {
            capabilities.push("filesystem");
        }
        if requirements.needs_network {
            capabilities.push("network");
        }

        if capabilities.is_empty() {
            "none (complete isolation)".to_string()
        } else {
            capabilities.join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::wasm::module_loader::{ComponentType, ModuleImport, ImportType};
    use wasmtime::{Engine, Module};

    fn create_test_loaded_module(imports: Vec<ModuleImport>) -> LoadedModule {
        let engine = Engine::default();
        // Create a minimal valid WASM module for testing
        let wasm_bytes = wat::parse_str(r#"
            (module
                (func (export "process") (param i32 i32) (result i32)
                    i32.const 0
                )
                (memory (export "memory") 1)
            )
        "#).unwrap();
        let module = Module::new(&engine, &wasm_bytes).unwrap();

        LoadedModule {
            engine,
            module,
            component_type: ComponentType::CStyle,
            imports,
            module_path: "test.wasm".to_string(),
        }
    }

    #[test]
    fn test_analyze_capabilities_no_wasi() {
        let loaded_module = create_test_loaded_module(vec![]);
        let requirements = CapabilityManager::analyze_capabilities(&loaded_module);

        assert!(!requirements.needs_stdio);
        assert!(!requirements.needs_clocks);
        assert!(!requirements.needs_random);
        assert!(!requirements.needs_filesystem);
        assert!(!requirements.needs_network);
        assert!(requirements.wasi_functions.is_empty());
    }

    #[test]
    fn test_analyze_capabilities_with_wasi() {
        let imports = vec![
            ModuleImport {
                module_name: "wasi_snapshot_preview1".to_string(),
                function_name: "fd_write".to_string(),
                import_type: ImportType::Wasi,
            },
            ModuleImport {
                module_name: "wasi_snapshot_preview1".to_string(),
                function_name: "clock_time_get".to_string(),
                import_type: ImportType::Wasi,
            },
            ModuleImport {
                module_name: "wasi_snapshot_preview1".to_string(),
                function_name: "random_get".to_string(),
                import_type: ImportType::Wasi,
            },
        ];

        let loaded_module = create_test_loaded_module(imports);
        let requirements = CapabilityManager::analyze_capabilities(&loaded_module);

        assert!(requirements.needs_stdio);
        assert!(requirements.needs_clocks);
        assert!(requirements.needs_random);
        assert!(!requirements.needs_filesystem);
        assert!(!requirements.needs_network);
        assert_eq!(requirements.wasi_functions.len(), 3);
        assert!(requirements.wasi_functions.contains("fd_write"));
        assert!(requirements.wasi_functions.contains("clock_time_get"));
        assert!(requirements.wasi_functions.contains("random_get"));
    }

    #[test]
    fn test_validate_capabilities_allows_all() {
        let requirements = CapabilityRequirements {
            needs_stdio: true,
            needs_clocks: true,
            needs_random: true,
            needs_filesystem: true,
            needs_network: true,
            wasi_functions: HashSet::new(),
        };

        let result = CapabilityManager::validate_capabilities(&requirements);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_wasi_setup() {
        let engine = Engine::default();
        let requirements = CapabilityRequirements {
            needs_stdio: true,
            needs_clocks: false,
            needs_random: false,
            needs_filesystem: false,
            needs_network: false,
            wasi_functions: HashSet::new(),
        };

        let result = CapabilityManager::create_wasi_setup(&engine, &requirements);
        assert!(result.is_ok());
    }

    #[test]
    fn test_capability_summary() {
        let requirements = CapabilityRequirements {
            needs_stdio: true,
            needs_clocks: true,
            needs_random: false,
            needs_filesystem: false,
            needs_network: false,
            wasi_functions: HashSet::new(),
        };

        let summary = CapabilityManager::capability_summary(&requirements);
        assert_eq!(summary, "stdio, clocks");

        let empty_requirements = CapabilityRequirements {
            needs_stdio: false,
            needs_clocks: false,
            needs_random: false,
            needs_filesystem: false,
            needs_network: false,
            wasi_functions: HashSet::new(),
        };

        let empty_summary = CapabilityManager::capability_summary(&empty_requirements);
        assert_eq!(empty_summary, "none (complete isolation)");
    }
}
