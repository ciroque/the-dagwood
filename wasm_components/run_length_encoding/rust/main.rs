use wasmtime::component::*;
use wasmtime::{Config, Engine, Store, Linker};
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi_http::WasiHttpCtx;

wit_bindgen::generate!({
    world: "rle-component",
    path: "wit/world.wit",
});

#[derive(Clone)]
struct Ctx {
    wasi: wasmtime_wasi::WasiCtx,
    table: ResourceTable,
    http: WasiHttpCtx,
}

fn main() -> Result<(), String> {
    // Configure Wasmtime engine
    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config).map_err(|e| format!("Engine creation failed: {}", e))?;

    // Load the component
    let component = Component::from_file(&engine, "target/wasm32-wasip2/release/rle_rust.wasm")
        .map_err(|e| format!("Component loading failed: {}", e))?;

    // Create WASI context
    let wasi_ctx = WasiCtxBuilder::new()
        .inherit_stdio()
        .args(&["dagwood-component"])
        .build();

    let store_data = Ctx {
        wasi: wasi_ctx,
        table: ResourceTable::new(),
        http: WasiHttpCtx::new(),
    };
    let mut store = Store::new(&engine, store_data);

    let mut linker = Linker::new(&engine);

    // Add WASI Preview 2 interfaces
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
        .map_err(|e| format!("Failed to add WASI to linker: {}", e))?;

    // Add HTTP interfaces
    wasmtime_wasi_http::add_only_http_to_linker_sync(&mut linker)
        .map_err(|e| format!("Failed to add WASI HTTP to linker: {}", e))?;

    // Add memory-allocator interface with cabi_realloc
    linker
        .root()
        .func_wrap(
            "memory-allocator:cabi-realloc",
            |mut caller: wasmtime::StoreContextMut<Ctx>, (old_ptr, old_size, new_size): (u32, u32, u32)| {
                // Access memory via instance
                let instance = caller.instance();
                let memory = instance
                    .get_memory(&mut caller, "memory")
                    .ok_or_else(|| format!("Failed to get memory"))?;
                let new_ptr = memory
                    .data_mut(&mut caller)
                    .realloc(old_ptr, old_size, new_size, 4)
                    .map_err(|e| format!("Realloc failed: {}", e))?;
                Ok(new_ptr)
            },
        )
        .map_err(|e| format!("Failed to add cabi-realloc: {}", e))?;

    // Instantiate the component
    let (bindings, _) = DagwoodComponent::instantiate(&mut store, &component, &linker)
        .map_err(|e| format!("Failed to instantiate component: {}", e))?;

    // Test input
    let input = b"aaabbc";
    let result = bindings
        .dagwood_component_processing_node()
        .call_process(&mut store, input)
        .map_err(|e| format!("Component process call failed: {}", e))?;

    let output = result.map_err(|e| format!("Component returned error: {:?}", e))?;
    println!("Output: {:?}", std::str::from_utf8(&output).unwrap_or("<invalid UTF-8>"));

    Ok(())
}