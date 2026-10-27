fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let proto_dir = manifest.join("../../../protocol");
    let proto = proto_dir.join("messages.proto");
    println!("cargo:rerun-if-changed={}", proto.display());
    prost_build::compile_protos(&[proto.as_path()], &[proto_dir.as_path()])?;
    Ok(())
}
