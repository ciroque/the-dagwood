use wasmtime::component::*;
use wasmtime::{Engine, Store};
use std::path::PathBuf;

// Test infrastructure for Component Model
struct ComponentTestRunner {
    engine: Engine,
    component: Component,
}

impl ComponentTestRunner {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map_err(|_| "CARGO_MANIFEST_DIR not set")?;
        let wasm_path = PathBuf::from(manifest_dir)
            .join("../..")
            .join("rle_rust.wasm");
        
        if !wasm_path.exists() {
            return Err(format!(
                "WASM component not found at {}. Run 'make build' to compile the component.",
                wasm_path.display()
            ).into());
        }
        
        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);
        let engine = Engine::new(&config)?;
        let component = Component::from_file(&engine, &wasm_path)?;
        
        Ok(Self { engine, component })
    }
    
    fn run_test<F>(&self, test_fn: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnOnce(&mut Store<()>, &Linker<()>) -> Result<(), Box<dyn std::error::Error>>,
    {
        let mut store = Store::new(&self.engine, ());
        let linker = Linker::new(&self.engine);
        test_fn(&mut store, &linker)
    }
}

// Generate bindings for tests
wasmtime::component::bindgen!({
    world: "dagwood-component",
    path: "../../../wit/versions/v1.0.0",
    async: false
});

#[test]
fn test_component_loads() {
    let runner = ComponentTestRunner::new().expect("Failed to create component test runner");
    
    runner.run_test(|mut store, linker| {
        let instance = linker.instantiate(&mut store, &runner.component)?;
        let _bindings = DagwoodComponent::new(&mut store, &instance)?;
        
        println!("✅ Component loaded successfully with proper interface");
        Ok(())
    }).expect("Component should load with dagwood-component interface");
}

#[test]
fn test_component_rle_basic() {
    let runner = ComponentTestRunner::new().expect("Should be able to create component test runner");
    
    runner.run_test(|mut store, linker| {
        let instance = linker.instantiate(&mut store, &runner.component)?;
        let bindings = DagwoodComponent::new(&mut store, &instance)?;
        
        let input = b"aaabbc";
        let expected_output = "3a2b1c";
        
        let result = bindings.dagwood_component_v1_0_0_processing_node()
            .call_process(&mut store, input)?;
        
        let output_str = std::str::from_utf8(&result)?;
        assert_eq!(output_str, expected_output, "Output should match expected RLE result");
        
        println!("✅ Component RLE works: '{}' -> '{}'", std::str::from_utf8(input)?, output_str);
        Ok(())
    }).expect("Component RLE should work correctly");
}

#[test]
fn test_component_rle_single_chars() {
    let runner = ComponentTestRunner::new().expect("Failed to create component test runner");
    
    runner.run_test(|mut store, linker| {
        let instance = linker.instantiate(&mut store, &runner.component)?;
        let bindings = DagwoodComponent::new(&mut store, &instance)?;
        
        let input = b"abc";
        let expected_output = "1a1b1c";
        
        let result = bindings.dagwood_component_v1_0_0_processing_node()
            .call_process(&mut store, input)?;
        
        let output_str = std::str::from_utf8(&result)?;
        assert_eq!(output_str, expected_output);
        
        println!("✅ RLE handles single characters: '{}' -> '{}'", std::str::from_utf8(input)?, output_str);
        Ok(())
    }).expect("Component should handle single characters");
}

#[test]
fn test_component_rle_long_sequence() {
    let runner = ComponentTestRunner::new().expect("Failed to create component test runner");
    
    runner.run_test(|mut store, linker| {
        let instance = linker.instantiate(&mut store, &runner.component)?;
        let bindings = DagwoodComponent::new(&mut store, &instance)?;
        
        let input = b"aaaaaaaaaa";
        let expected_output = "10a";
        
        let result = bindings.dagwood_component_v1_0_0_processing_node()
            .call_process(&mut store, input)?;
        
        let output_str = std::str::from_utf8(&result)?;
        assert_eq!(output_str, expected_output);
        
        println!("✅ RLE handles long sequences: '{}' -> '{}'", std::str::from_utf8(input)?, output_str);
        Ok(())
    }).expect("Component should handle long sequences");
}

#[test]
fn test_component_rle_mixed_patterns() {
    let runner = ComponentTestRunner::new().expect("Failed to create component test runner");
    
    runner.run_test(|mut store, linker| {
        let instance = linker.instantiate(&mut store, &runner.component)?;
        let bindings = DagwoodComponent::new(&mut store, &instance)?;
        
        let input = b"aabbbaaccc";
        let expected_output = "2a3b2a3c";
        
        let result = bindings.dagwood_component_v1_0_0_processing_node()
            .call_process(&mut store, input)?;
        
        let output_str = std::str::from_utf8(&result)?;
        assert_eq!(output_str, expected_output);
        
        println!("✅ RLE handles mixed patterns: '{}' -> '{}'", std::str::from_utf8(input)?, output_str);
        Ok(())
    }).expect("Component should handle mixed patterns");
}

#[test]
fn test_component_process_empty_string() {
    let runner = ComponentTestRunner::new().expect("Should be able to create component test runner");
    
    runner.run_test(|mut store, linker| {
        let instance = linker.instantiate(&mut store, &runner.component)?;
        let bindings = DagwoodComponent::new(&mut store, &instance)?;
        
        let input = b"";
        let result = bindings.dagwood_component_v1_0_0_processing_node()
            .call_process(&mut store, input)?;
        
        assert_eq!(result.len(), 0, "Empty input should return empty output");
        
        println!("✅ Empty string processing works correctly");
        Ok(())
    }).expect("Component should handle empty strings");
}
