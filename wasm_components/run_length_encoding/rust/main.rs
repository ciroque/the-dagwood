use wasmtime::component::*;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView, WasiCtxView};

wasmtime::component::bindgen!({
    world: "rle-component",
    path: "wit",
});

struct Ctx {
    wasi: WasiCtx,
    table: wasmtime::component::ResourceTable,
}

impl WasiView for Ctx {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

fn main() -> Result<(), String> {
    // Configure Wasmtime engine
    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config).map_err(|e| format!("Engine creation failed: {}", e))?;

    // Load the component
    let component = Component::from_file(&engine, "../../rle_rust.wasm")
        .map_err(|e| format!("Component loading failed: {}", e))?;

    // Create WASI context
    let wasi_ctx = WasiCtxBuilder::new()
        .inherit_stdio()
        .args(&["rle-runner"])
        .build();

    let store_data = Ctx {
        wasi: wasi_ctx,
        table: wasmtime::component::ResourceTable::new(),
    };
    let mut store = Store::new(&engine, store_data);

    let mut linker = Linker::<Ctx>::new(&engine);

    // Add WASI Preview 2 interfaces
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
        .map_err(|e| format!("Failed to add WASI to linker: {}", e))?;

    // Instantiate the component
    let bindings = RleComponent::instantiate(&mut store, &component, &linker)
        .map_err(|e| format!("Failed to instantiate component: {}", e))?;

    // Test input
    let input = b"aaaaaaaaaaaaaaabbczzzzzzzzzzzzzzzzzzzz";
    let result = bindings
        .dagwood_component_processing_node()
        .call_process(&mut store, input)
        .map_err(|e| format!("Component process call failed: {}", e))?;

    let output = result.map_err(|e| format!("Component returned error: {:?}", e))?;
    println!("Output: {:?}", std::str::from_utf8(&output).unwrap_or("<invalid UTF-8>"));

    Ok(())
}