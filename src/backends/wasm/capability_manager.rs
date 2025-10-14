// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

//
// use crate::backends::wasm::error::WasmResult;
// use crate::backends::wasm::module_loader::LoadedModule;
// use std::collections::HashSet;
// use std::sync::Arc;
// use wasmtime::*;
// use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};
// use wasmtime_wasi::p1::{wasi_snapshot_preview1::add_to_linker, WasiP1Ctx};
//
// #[cfg(test)]
// use crate::backends::wasm::module_loader::WasmArtifact;
//
// #[derive(Debug, Clone)]
// pub struct CapabilityRequirements {
//     pub needs_stdio: bool,
//     pub needs_clocks: bool,
//     pub needs_filesystem: bool,
//     pub needs_network: bool,
//     pub needs_random: bool,
//     pub wasi_functions: HashSet<String>,
// }
//
// #[derive(Clone)]
// pub struct WasiCtxWrapper(Arc<WasiCtx>);
//
// impl std::fmt::Debug for WasiCtxWrapper {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("WasiCtx").finish()
//     }
// }
//
// #[derive(Debug)]
// pub struct WasiSetup {
//     pub linker: Linker<WasiP1Ctx>,
//     pub wasi_ctx: WasiCtxWrapper,
// }
//
// pub struct CapabilityManager;
//
// impl CapabilityManager {
//     pub fn analyze_capabilities(module: &LoadedModule) -> CapabilityRequirements {
//         let mut requirements = CapabilityRequirements {
//             needs_stdio: false,
//             needs_clocks: false,
//             needs_filesystem: false,
//             needs_network: false,
//             needs_random: false,
//             wasi_functions: HashSet::new(),
//         };
//
//         for import in &module.imports {
//             if import.module_name.starts_with("wasi") {
//                 requirements.wasi_functions.insert(import.function_name.clone());
//                 if import.module_name == "wasi:cli_base/stdio" ||
//                    import.function_name == "stdin_get_stdin" ||
//                    import.function_name == "stdout_get_stdout" ||
//                    import.function_name == "stderr_get_stderr" {
//                     requirements.needs_stdio = true;
//                 } else if import.module_name == "wasi:clocks/wall_clock" ||
//                           import.function_name == "now" ||
//                           import.function_name == "resolution" {
//                     requirements.needs_clocks = true;
//                 } else if import.module_name.starts_with("wasi:filesystem") ||
//                           import.module_name.starts_with("wasi:cli_base/filesystem") {
//                     requirements.needs_filesystem = true;
//                 } else if import.module_name.starts_with("wasi:sockets") ||
//                           import.module_name.starts_with("wasi:http") ||
//                           import.module_name.starts_with("wasi:network") {
//                     requirements.needs_network = true;
//                 }
//             }
//         }
//
//         requirements
//     }
//     //
//     // pub fn create_store(engine: &Engine, setup: &WasiSetup) -> WasmResult<Store<WasiCtx>> {
//     //     let wtf = setup.wasi_ctx.0.clone();
//     //     Ok(Store::new(engine, wtf))
//     // }
//
//     pub fn create_wasi_setup(
//         engine: &Engine,
//         requirements: &CapabilityRequirements,
//     ) -> WasmResult<WasiSetup> {
//         let wasi_ctx = WasiCtxWrapper(Arc::new(
//             WasiCtxBuilder::new()
//                 .inherit_stdin()
//                 .inherit_stdout()
//                 .inherit_stderr()
//                 .build()
//         ));
//
//         if requirements.needs_stdio {
//             tracing::debug!("Enabling stdio capabilities");
//         }
//         if requirements.needs_clocks {
//             tracing::debug!("Enabling clock capabilities");
//         }
//         if requirements.wasi_functions.contains("random_get") {
//             tracing::debug!("Enabling random number generation");
//         }
//
//         let mut linker = Linker::new(engine);
//
//         add_to_linker(&mut linker, |wasi_ctx| wasi_ctx)
//             .map_err(|e| crate::backends::wasm::error::WasmError::ExecutionError(e))?;
//
//         Ok(WasiSetup {
//             linker,
//             wasi_ctx,
//         })
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::backends::wasm::module_loader::{ComponentType, ImportType, ModuleImport};
//     use wasmtime::{Engine, Module};
//
//     fn create_test_loaded_module(imports: Vec<ModuleImport>) -> LoadedModule {
//         let engine = Engine::default();
//         let wasm_bytes = wat::parse_str(
//             r#"
//             (module
//                 (func (export "process") (param i32 i32) (result i32)
//                     i32.const 0
//                 )
//                 (memory (export "memory") 1)
//             )
//         "#,
//         )
//         .unwrap();
//         let module = Module::new(&engine, &wasm_bytes).unwrap();
//
//         LoadedModule {
//             engine,
//             artifact: WasmArtifact::Module(module),
//             component_type: ComponentType::CStyle,
//             imports,
//             module_path: "test.wasm".to_string(),
//         }
//     }
//
//     #[test]
//     fn test_analyze_capabilities_no_wasi() {
//         let loaded_module = create_test_loaded_module(vec![]);
//         let requirements = CapabilityManager::analyze_capabilities(&loaded_module);
//
//         assert!(!requirements.needs_stdio);
//         assert!(!requirements.needs_clocks);
//         assert!(!requirements.needs_random);
//         assert!(!requirements.needs_filesystem);
//         assert!(!requirements.needs_network);
//         assert!(requirements.wasi_functions.is_empty());
//     }
//
//     #[test]
//     fn test_analyze_capabilities_with_wasi() {
//         let imports = vec![
//             ModuleImport {
//                 module_name: "wasi_snapshot_preview1".to_string(),
//                 function_name: "fd_write".to_string(),
//                 import_type: ImportType::Wasi,
//             },
//             ModuleImport {
//                 module_name: "wasi_snapshot_preview1".to_string(),
//                 function_name: "clock_time_get".to_string(),
//                 import_type: ImportType::Wasi,
//             },
//             ModuleImport {
//                 module_name: "wasi_snapshot_preview1".to_string(),
//                 function_name: "random_get".to_string(),
//                 import_type: ImportType::Wasi,
//             },
//         ];
//
//         let loaded_module = create_test_loaded_module(imports);
//         let requirements = CapabilityManager::analyze_capabilities(&loaded_module);
//
//         assert!(requirements.needs_stdio);
//         assert!(requirements.needs_clocks);
//         assert!(requirements.needs_random);
//         assert!(!requirements.needs_filesystem);
//         assert!(!requirements.needs_network);
//         assert_eq!(requirements.wasi_functions.len(), 3);
//         assert!(requirements.wasi_functions.contains("fd_write"));
//         assert!(requirements.wasi_functions.contains("clock_time_get"));
//         assert!(requirements.wasi_functions.contains("random_get"));
//     }
//
//     // #[test]
//     // fn test_validate_capabilities_allows_all() {
//     //     let requirements = CapabilityRequirements {
//     //         needs_stdio: true,
//     //         needs_clocks: true,
//     //         needs_random: true,
//     //         needs_filesystem: true,
//     //         needs_network: true,
//     //         wasi_functions: HashSet::new(),
//     //     };
//     //
//     //     let result = CapabilityManager::validate_capabilities(&requirements);
//     //     assert!(result.is_ok());
//     // }
//
//     // #[test]
//     // fn test_create_wasi_setup() {
//     //     let engine = Engine::default();
//     //     let requirements = CapabilityRequirements {
//     //         needs_stdio: true,
//     //         needs_clocks: false,
//     //         needs_random: false,
//     //         needs_filesystem: false,
//     //         needs_network: false,
//     //         wasi_functions: HashSet::new(),
//     //     };
//     //
//     //     let result = CapabilityManager::create_wasi_setup(&engine, &requirements);
//     //     assert!(result.is_ok());
//     // }
//
//     // #[test]
//     // fn test_capability_summary() {
//     //     let requirements = CapabilityRequirements {
//     //         needs_stdio: true,
//     //         needs_clocks: true,
//     //         needs_random: false,
//     //         needs_filesystem: false,
//     //         needs_network: false,
//     //         wasi_functions: HashSet::new(),
//     //     };
//     //
//     //     let summary = CapabilityManager::capability_summary(&requirements);
//     //     assert_eq!(summary, "stdio, clocks");
//     //
//     //     let empty_requirements = CapabilityRequirements {
//     //         needs_stdio: false,
//     //         needs_clocks: false,
//     //         needs_random: false,
//     //         needs_filesystem: false,
//     //         needs_network: false,
//     //         wasi_functions: HashSet::new(),
//     //     };
//     //
//     //     let empty_summary = CapabilityManager::capability_summary(&empty_requirements);
//     //     assert_eq!(empty_summary, "none (complete isolation)");
//     // }
// }
