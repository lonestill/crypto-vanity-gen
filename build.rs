use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "macos" {
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

        let status = Command::new("clang")
            .args(&[
                "-O3",
                "-c",
                "src/metal/metal_bridge.m",
                "-o",
            ])
            .arg(out_dir.join("metal_bridge.o"))
            .status();

        if let Ok(s) = status {
            if s.success() {
                let _ = Command::new("ar")
                    .args(&["rcs"])
                    .arg(out_dir.join("libmetal_bridge.a"))
                    .arg(out_dir.join("metal_bridge.o"))
                    .status();

                println!("cargo:rustc-link-search=native={}", out_dir.display());
                println!("cargo:rustc-link-lib=static=metal_bridge");
                println!("cargo:rustc-link-lib=framework=Metal");
                println!("cargo:rustc-link-lib=framework=Foundation");
            }
        }
    }

    println!("cargo:rerun-if-changed=src/metal/metal_bridge.m");
    println!("cargo:rerun-if-changed=src/metal/metal_bridge.h");
    println!("cargo:rerun-if-changed=metal/vanity_engine.metal");
}
