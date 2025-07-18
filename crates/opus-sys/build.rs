use std::path::{Path, PathBuf};

use cmake::Config;

fn main() {
    let opus_dir = Path::new("../../lib/opus");
    println!(
        "cargo:rerun-if-env-changed={}",
        opus_dir.join("src").display()
    );
    println!("cargo:rerun-if-changed=src/wrapper.h");
    println!("cargo:rerun-if-changed=build.rs");

    let dst = Config::new(opus_dir)
        .define("OPUS_BUILD", "")
        .define("USE_ALLOCA", "")
        // .very_verbose(true)
        .build();

    if cfg!(target_os = "ios") || cfg!(target_os = "macos") {
        println!("cargo:rustc-link-search=native={}/lib", dst.display());
    } else {
        println!("cargo:rustc-link-search=native={}/lib64", dst.display());
    }
    println!("cargo:rustc-link-lib=static=opus");

    let allows: &'static str = r#"#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(deref_nullptr)]
    "#;

    let cb = bindgen::CargoCallbacks::new();
    let bindings = bindgen::Builder::default()
        .raw_line(allows)
        .header("src/wrapper.h")
        .clang_arg(format!("-I{}", dst.join("include").join("opus").display()))
        .parse_callbacks(Box::new(cb))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from("src/lib.rs");
    bindings
        .write_to_file(out_path)
        .expect("failed to write bindings");
}
