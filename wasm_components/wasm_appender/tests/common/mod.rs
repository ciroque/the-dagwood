use wasmtime::*;
use std::path::Path;

/// Shared WASM test runner that handles module loading and test execution
/// This eliminates duplication between test files and provides a single place
/// for WASM runtime configuration.
pub struct WasmTestRunner {
    engine: Engine,
    module: Module,
}

impl WasmTestRunner {
    /// Create a new WASM test runner, loading the wasm_appender module
    /// 
    /// This function will:
    /// 1. Assert that the WASM module exists (fail fast if not built)
    /// 2. Load the module with a default engine configuration
    /// 3. Return a runner ready for test execution
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let wasm_path = "../wasm_appender.wasm";
        
        // Assert the WASM file exists to provide clear error messages
        if !Path::new(wasm_path).exists() {
            return Err(format!(
                "WASM module not found at {}. Run 'make build' first to compile the module.",
                wasm_path
            ).into());
        }
        
        let engine = Engine::default();
        let module = Module::from_file(&engine, wasm_path)
            .map_err(|e| format!("Failed to load WASM module from {}: {}", wasm_path, e))?;
        
        Ok(Self { engine, module })
    }
    
    /// Run a test function with a fresh WASM store and instance
    /// 
    /// This method handles:
    /// - Store creation and management
    /// - Instance instantiation
    /// - Error propagation from test functions
    /// - Proper cleanup (automatic via Drop)
    pub fn run_test<F>(&self, test_fn: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnOnce(&mut Store<()>, &Instance) -> Result<(), Box<dyn std::error::Error>>,
    {
        let mut store = Store::new(&self.engine, ());
        let instance = Instance::new(&mut store, &self.module, &[])?;
        test_fn(&mut store, &instance)
    }
}
