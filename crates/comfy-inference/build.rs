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

    let possible_dirs = [
        "cpp/stable-diffusion-cpp",
        "cpp/stable-diffusion.cpp",
    ];
    let sd_cpp_dir = possible_dirs
        .iter()
        .map(|d| workspace_root.join(d))
        .find(|d| d.exists())
        .unwrap_or_else(|| workspace_root.join("cpp/stable-diffusion-cpp"));

    let sd_lib_dir_env = std::env::var("SD_LIB_DIR").ok();

    let has_prebuilt = sd_lib_dir_env.is_some()
        || sd_cpp_dir.join("build/libstable-diffusion.a").exists()
        || sd_cpp_dir.join("build/lib/libstable-diffusion.a").exists();

    let search_dirs = if has_prebuilt {
        link_prebuilt_library(&sd_cpp_dir, sd_lib_dir_env.as_deref())
    } else if sd_cpp_dir.exists() && local_build_enabled {
        build_and_link_cpp_library(&sd_cpp_dir)
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
    };

    emit_link_libs(&search_dirs);

    println!("cargo:rerun-if-env-changed=SD_LIB_DIR");
}

fn link_prebuilt_library(sd_cpp_dir: &std::path::Path, sd_lib_dir_env: Option<&str>) -> Vec<std::path::PathBuf> {
    let mut search_dirs: Vec<std::path::PathBuf> = Vec::new();

    if let Some(sd_lib_dir) = sd_lib_dir_env {
        let p = std::path::PathBuf::from(sd_lib_dir);
        println!("cargo:rustc-link-search=native={}", sd_lib_dir);
        search_dirs.push(p);
    } else {
        let dirs_to_check = [
            sd_cpp_dir.join("build"),
            sd_cpp_dir.join("build/lib"),
            sd_cpp_dir.join("build/ggml/src"),
            sd_cpp_dir.join("build/ggml/src/ggml-metal"),
            sd_cpp_dir.join("build/ggml/src/ggml-blas"),
            sd_cpp_dir.join("build/ggml/src/ggml-cuda"),
        ];
        for dir in &dirs_to_check {
            if dir.exists() {
                println!("cargo:rustc-link-search=native={}", dir.display());
                search_dirs.push(dir.clone());
            }
        }
    }

    search_dirs
}

fn build_and_link_cpp_library(sd_cpp_dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut search_dirs: Vec<std::path::PathBuf> = Vec::new();

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

    let dirs_to_check = [
        dst.join("lib"),
        dst.join("build/lib"),
        dst.join("build/ggml/src"),
        dst.join("build/ggml/src/ggml-metal"),
        dst.join("build/ggml/src/ggml-blas"),
        dst.join("build/ggml/src/ggml-cuda"),
    ];
    for dir in &dirs_to_check {
        if dir.exists() {
            println!("cargo:rustc-link-search=native={}", dir.display());
            search_dirs.push(dir.clone());
        }
    }

    println!("cargo:rustc-link-search=native={}", out_path.display());
    search_dirs.push(out_path.to_path_buf());

    println!("cargo:rerun-if-changed={}", sd_cpp_dir.join("CMakeLists.txt").display());

    search_dirs
}

fn lib_exists(search_dirs: &[std::path::PathBuf], lib_name: &str) -> bool {
    let lib_file = format!("lib{}.a", lib_name);
    search_dirs.iter().any(|d| d.join(&lib_file).exists())
}

fn emit_link_libs(search_dirs: &[std::path::PathBuf]) {
    let required_libs = ["stable-diffusion", "ggml", "ggml-base", "ggml-cpu"];
    let optional_libs = ["ggml-blas", "ggml-cuda"];

    for lib in &required_libs {
        if lib_exists(search_dirs, lib) {
            println!("cargo:rustc-link-lib=static={}", lib);
        } else {
            println!("cargo:warning=Required library lib{}.a not found in search paths", lib);
        }
    }

    for lib in &optional_libs {
        if lib_exists(search_dirs, lib) {
            println!("cargo:rustc-link-lib=static={}", lib);
        }
    }

    if cfg!(target_os = "macos") {
        if lib_exists(search_dirs, "ggml-metal") {
            println!("cargo:rustc-link-lib=static=ggml-metal");
        }
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
