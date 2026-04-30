#[cfg(feature = "local-build")]
fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let sd_cpp_dir = workspace_root.join("cpp/stable-diffusion-cpp");

    if !sd_cpp_dir.exists() {
        println!("cargo:warning=stable-diffusion-cpp not found at {:?}", sd_cpp_dir);
        println!("cargo:warning=Skipping C++ library build. Set SD_LIB_DIR to link pre-built library.");
        return;
    }

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = std::path::Path::new(&out_dir);

    let sd_metal = cfg!(target_os = "macos");
    let sd_cuda = cfg!(target_feature = "cuda");

    let mut config = cmake::Config::new(&sd_cpp_dir);

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

    println!("cargo:rustc-link-search=native={}", out_path.display());

    println!("cargo:rustc-link-lib=static=stable-diffusion");
    println!("cargo:rustc-link-lib=static=ggml");

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=framework=Metal");
        println!("cargo:rustc-link-lib=framework=MetalKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=Accelerate");
    }

    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=stdc++");
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=dl");
    }

    println!("cargo:rerun-if-env-changed=SD_LIB_DIR");
    println!("cargo:rerun-if-changed={}", sd_cpp_dir.join("CMakeLists.txt").display());
}

#[cfg(not(feature = "local-build"))]
fn main() {}
