use std::env;
use std::io::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=src/mumble/mumble.proto");
    println!("cargo:rerun-if-changed=src/mumble/mumbleUDP.proto");

    let inputs = &["src/mumble/mumble.proto", "src/mumble/mumbleUDP.proto"];

    let out_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("src/mumble/proto");
    std::fs::create_dir_all(&out_dir).unwrap();
    let mut cfg = prost_build::Config::new();

    cfg.out_dir(out_dir.clone());
    cfg.compile_protos(inputs, &["src/mumble/"])?;

    Ok(())
}
