fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = "proto";

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/proto") // generated Rust goes here
        .compile(
            &[format!("{}/processor.proto", proto_root)],
            &[proto_root],
        )?;

    println!("cargo:rerun-if-changed={}/processor.proto", proto_root);
    Ok(())
}
