// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

use crate::backends::wasm::{ComponentType, LoadedModule};

pub struct WasmComponentDetector;

impl WasmComponentDetector {
    pub fn determine_type(_loaded_module: &LoadedModule) -> ComponentType {
        ComponentType::CStyle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::wasm::module_loader::WasmArtifact;
    use wasmtime::{Engine, Module};

    fn create_test_loaded_module() -> LoadedModule {
        let engine = Engine::default();
        let wasm_bytes = wat::parse_str("(module)").unwrap();
        let module = Module::new(&engine, &wasm_bytes).unwrap();

        LoadedModule {
            engine,
            artifact: WasmArtifact::Module(module),
            imports: vec![],
            module_path: "test.wasm".to_string(),
        }
    }

    #[test]
    fn test_determine_type_returns_cstyle() {
        let loaded_module = create_test_loaded_module();
        let component_type = WasmComponentDetector::determine_type(&loaded_module);
        assert!(matches!(component_type, ComponentType::CStyle));
    }
}
