fn main() {
    let local_enabled = std::env::var("CARGO_FEATURE_LOCAL").is_ok();
    let local_ffi_enabled = std::env::var("CARGO_FEATURE_LOCAL_FFI").is_ok();
    let local_build_enabled = std::env::var("CARGO_FEATURE_LOCAL_BUILD").is_ok();

    if !local_enabled || !local_ffi_enabled {
        return;
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let sd_cpp_dir = workspace_root.join("cpp/stable-diffusion-cpp");

    let sd_lib_dir_env = std::env::var("SD_LIB_DIR").ok();

    let has_prebuilt = sd_lib_dir_env.is_some()
        || sd_cpp_dir.join("build/libstable-diffusion.a").exists()
        || sd_cpp_dir.join("build/lib/libstable-diffusion.a").exists();

    if has_prebuilt {
        link_prebuilt_library(&sd_cpp_dir, sd_lib_dir_env.as_deref());
    } else if sd_cpp_dir.exists() && local_build_enabled {
        build_and_link_cpp_library(&sd_cpp_dir);
    } else if sd_cpp_dir.exists() {
        println!("cargo:error=Pre-built stable-diffusion library not found.");
        println!("cargo:error=Either:");
        println!("cargo:error=  1. Build the C++ library first: cd cpp/stable-diffusion-cpp && mkdir build && cd build && cmake .. && make -j$(nproc)");
        println!("cargo:error=  2. Use --features local-build to auto-build: cargo build --features local-build");
        println!("cargo:error=  3. Set SD_LIB_DIR env var to the directory containing libstable-diffusion.a");
        println!("cargo:error=  4. Use CLI backend only: cargo build --features local (no FFI, uses sd-cli subprocess)");
        panic!("Pre-built stable-diffusion library not found. Use --features local-build to build from source, or --features local for CLI-only mode.");
    } else {
        println!("cargo:warning=stable-diffusion-cpp not found at {:?}", sd_cpp_dir);
        println!("cargo:warning=Clone the submodule first: git submodule update --init --recursive");
        println!("cargo:warning=Or set SD_LIB_DIR to link a pre-built library.");
        println!("cargo:warning=Or use --features local for CLI-only mode (no FFI).");
        panic!(
            "stable-diffusion-cpp not found at {:?}. \
             Clone the submodule, set SD_LIB_DIR, or use --features local for CLI-only mode.",
            sd_cpp_dir
        );
    }

    println!("cargo:rerun-if-env-changed=SD_LIB_DIR");
}

fn link_prebuilt_library(sd_cpp_dir: &std::path::Path, sd_lib_dir_env: Option<&str>) {
    if let Some(sd_lib_dir) = sd_lib_dir_env {
        println!("cargo:rustc-link-search=native={}", sd_lib_dir);
    } else {
        let build_dir = sd_cpp_dir.join("build");
        if build_dir.exists() {
            println!("cargo:rustc-link-search=native={}", build_dir.display());
        }
        let build_lib_dir = sd_cpp_dir.join("build/lib");
        if build_lib_dir.exists() {
            println!("cargo:rustc-link-search=native={}", build_lib_dir.display());
        }
        let ggml_dir = sd_cpp_dir.join("build/ggml/src");
        if ggml_dir.exists() {
            println!("cargo:rustc-link-search=native={}", ggml_dir.display());
        }
        let ggml_metal_dir = sd_cpp_dir.join("build/ggml/src/ggml-metal");
        if ggml_metal_dir.exists() {
            println!("cargo:rustc-link-search=native={}", ggml_metal_dir.display());
        }
        let ggml_blas_dir = sd_cpp_dir.join("build/ggml/src/ggml-blas");
        if ggml_blas_dir.exists() {
            println!("cargo:rustc-link-search=native={}", ggml_blas_dir.display());
        }
    }

    emit_link_libs();
}

fn build_and_link_cpp_library(sd_cpp_dir: &std::path::Path) {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = std::path::Path::new(&out_dir);

    let sd_metal = cfg!(target_os = "macos");
    let sd_cuda = std::env::var("CARGO_CFG_SD_CUDA").is_ok();

    let mut config = cmake::Config::new(sd_cpp_dir);

    config
        .define("SD_BUILD_EXAMPLES", "OFF")
        .define("SD_BUILD_SHARED_LIBS", "OFF")
        .define("SD_WEBP", "OFF")
        .define("SD_WEBM", "OFF");

    if sd_metal {
        config.define("SD_METAL", "ON");
    }
    if sd_cuda {
        config.define("SD_CUDA", "ON");
    }

    let dst = config.build();

    let lib_dir = dst.join("lib");
    if lib_dir.exists() {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
    }

    let build_lib_dir = dst.join("build/lib");
    if build_lib_dir.exists() {
        println!("cargo:rustc-link-search=native={}", build_lib_dir.display());
    }

    let ggml_dir = dst.join("build/ggml/src");
    if ggml_dir.exists() {
        println!("cargo:rustc-link-search=native={}", ggml_dir.display());
    }

    let ggml_metal_dir = dst.join("build/ggml/src/ggml-metal");
    if ggml_metal_dir.exists() {
        println!("cargo:rustc-link-search=native={}", ggml_metal_dir.display());
    }

    let ggml_blas_dir = dst.join("build/ggml/src/ggml-blas");
    if ggml_blas_dir.exists() {
        println!("cargo:rustc-link-search=native={}", ggml_blas_dir.display());
    }

    println!("cargo:rustc-link-search=native={}", out_path.display());

    emit_link_libs();

    println!("cargo:rerun-if-changed={}", sd_cpp_dir.join("CMakeLists.txt").display());
}

fn emit_link_libs() {
    println!("cargo:rustc-link-lib=static=stable-diffusion");
    println!("cargo:rustc-link-lib=static=ggml");
    println!("cargo:rustc-link-lib=static=ggml-base");
    println!("cargo:rustc-link-lib=static=ggml-cpu");
    println!("cargo:rustc-link-lib=static=ggml-blas");

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=static=ggml-metal");
        println!("cargo:rustc-link-lib=framework=Metal");
        println!("cargo:rustc-link-lib=framework=MetalKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=Accelerate");
        println!("cargo:rustc-link-lib=c++");
    }

    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=stdc++");
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=dl");
    }
}
