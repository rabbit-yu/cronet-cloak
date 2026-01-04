use std::env;
use std::path::PathBuf;

fn main() {
    // 1. Generate Bindings for Cronet C API
    // Determine paths based on OS
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = PathBuf::from(dir).join("cronet-bin");

    let (include_dir, lib_dir) = if env::var("CARGO_CFG_TARGET_OS").unwrap() == "linux" {
        (root.join("linux").join("include"), root.join("linux"))
    } else {
        (root.join("include"), root)
    };

    // 0. Export Cronet Version
    let version_path = lib_dir.join("VERSION");
    let version_content =
        std::fs::read_to_string(&version_path).expect("Failed to read VERSION file");

    // Parse VERSION file (format: MAJOR=x\nMINOR=y\nBUILD=z\nPATCH=w)
    let mut major = String::new();
    let mut minor = String::new();
    let mut build = String::new();
    let mut patch = String::new();
    for line in version_content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "MAJOR" => major = value.trim().to_string(),
                "MINOR" => minor = value.trim().to_string(),
                "BUILD" => build = value.trim().to_string(),
                "PATCH" => patch = value.trim().to_string(),
                _ => {}
            }
        }
    }
    let version = format!("{}.{}.{}.{}", major, minor, build, patch);
    println!("cargo:rustc-env=CRONET_VERSION={}", version);
    println!("cargo:rerun-if-changed={}", version_path.display());

    // 1. Generate Bindings for Cronet C API
    let bindings = bindgen::Builder::default()
        .header_contents(
            "wrapper.h",
            "#include <stdbool.h>\n#include \"cronet.idl_c.h\"",
        )
        .clang_arg(format!("-I{}", include_dir.display()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("cronet_bindings.rs"))
        .expect("Couldn't write bindings!");

    // 2. Compile Protos (Standard Prost)
    let proto_file = "proto/cronet_engine.proto";

    // Check if proto exists
    if std::path::Path::new(proto_file).exists() {
        // We want to generate Serde traits for JSON serialization
        let mut config = prost_build::Config::new();
        config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");
        // Apply default only to specific messages or handle Enums carefully
        // prost-build type_attribute "." applies to everything including enums which causes error.
        // We will apply it to the main request/response types.
        config.type_attribute("cronet.engine.v1.ExecuteRequest", "#[serde(default)]");
        config.type_attribute("cronet.engine.v1.TargetRequest", "#[serde(default)]");
        config.type_attribute("cronet.engine.v1.ExecutionConfig", "#[serde(default)]");
        config.type_attribute("cronet.engine.v1.ExecuteResponse", "#[serde(default)]");

        // Serialize body fields as hex strings instead of byte arrays
        config.field_attribute(
            "cronet.engine.v1.TargetRequest.body",
            "#[serde(with = \"hex::serde\")]",
        );
        config.field_attribute(
            "cronet.engine.v1.TargetResponse.body",
            "#[serde(with = \"hex::serde\")]",
        );
        config
            .compile_protos(&[proto_file], &["proto"])
            .expect("failed to compile protos");
    }

    // 3. Link against the Cronet DLL/SO
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=cronet");

    // 4. Copy DLL to output directory for Windows
    #[cfg(target_os = "windows")]
    {
        // Get the target directory (target/debug or target/release)
        let out_dir = env::var("OUT_DIR").unwrap();
        // OUT_DIR is like: target/debug/build/cronet-cloak-xxx/out
        // We need: target/debug/
        let target_dir = PathBuf::from(&out_dir)
            .ancestors()
            .nth(3)
            .unwrap()
            .to_path_buf();

        // The actual DLL name that the linker expects (from .lib file)
        let dll_name = format!("cronet.{}.dll", version);
        let src_dll = lib_dir.join("cronet.dll");
        let dst_dll = target_dir.join(&dll_name);

        // Copy DLL if source exists and destination doesn't or is older
        if src_dll.exists() {
            let should_copy = if dst_dll.exists() {
                let src_meta = std::fs::metadata(&src_dll).ok();
                let dst_meta = std::fs::metadata(&dst_dll).ok();
                match (src_meta, dst_meta) {
                    (Some(s), Some(d)) => s.modified().ok() > d.modified().ok(),
                    _ => true,
                }
            } else {
                true
            };

            if should_copy {
                std::fs::copy(&src_dll, &dst_dll).expect("Failed to copy DLL to target dir");
                println!(
                    "cargo:warning=Copied {} to {}",
                    src_dll.display(),
                    dst_dll.display()
                );
            }
        }

        println!("cargo:rerun-if-changed={}", src_dll.display());
    }

    println!("cargo:rerun-if-changed=build.rs");
}
