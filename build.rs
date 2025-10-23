// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build protobuf definitions
    let proto_root = "proto";
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/proto") // generated Rust goes here
        .compile(&[format!("{proto_root}/processor.proto")], &[proto_root])?;

    println!("cargo:rerun-if-changed={proto_root}/processor.proto");

    // Generate WIT bindings for Component Model
    // wit-bindgen generates bindings at compile time
    // The generated code will be available via the wasmtime::component API
    // See: https://docs.rs/wit-bindgen/latest/wit_bindgen/
    
    println!("cargo:rerun-if-changed=wit/versions/v1.0.0/dagwood-processor.wit");
    
    Ok(())
}
