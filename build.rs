// Copyright (c) 2025 Steve Wagner (ciroque@live.com)
// SPDX-License-Identifier: MIT

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = "proto";

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/proto") // generated Rust goes here
        .compile(&[format!("{proto_root}/processor.proto")], &[proto_root])?;

    println!("cargo:rerun-if-changed={proto_root}/processor.proto");
    Ok(())
}
