use wasmtime::component::*;
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};

bindgen!({
    world: "rle-component",
    path: "wasm_components/run_length_encoding/rust/wit",
});

struct MyCtx {
    wasi: WasiCtx,
    table: ResourceTable,
}

impl WasiView for MyCtx {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

fn main() -> wasmtime::Result<()> {
    let engine = Engine::default();
    
    let wasi_ctx = WasiCtxBuilder::new()
        .inherit_stdio()
        .args(&["test"])
        .build();

    let mut store = Store::new(&engine, MyCtx {
        wasi: wasi_ctx,
        table: ResourceTable::new(),
    });

    let component_bytes = std::fs::read("wasm_components/rle_rust.wasm")?;
    let component = Component::new(&engine, &component_bytes)?;

    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker_sync(&mut linker)?;

    let (bindings, _) = RleComponent::instantiate(&mut store, &component, &linker)?;

    let input = b"TACOCAT";
    let result = bindings.dagwood_component_processing_node().call_process(&mut store, input)?;
    
    match result {
        Ok(output) => {
            println!("Success: {}", String::from_utf8_lossy(&output));
        }
        Err(e) => {
            println!("Component returned error: {:?}", e);
        }
    }

    Ok(())
}
