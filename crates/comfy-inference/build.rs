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

    println!("cargo:rerun-if-env-changed=SD_LIB_DIR");
    println!("cargo:rerun-if-changed={}", sd_cpp_dir.join("CMakeLists.txt").display());
}

#[cfg(all(feature = "local", not(feature = "local-build")))]
fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let sd_cpp_dir = workspace_root.join("cpp/stable-diffusion-cpp");

    if let Ok(sd_lib_dir) = std::env::var("SD_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", sd_lib_dir);
    } else if sd_cpp_dir.exists() {
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
        let ggml_cpu_dir = sd_cpp_dir.join("build/ggml/src");
        if ggml_cpu_dir.exists() {
            println!("cargo:rustc-link-search=native={}", ggml_cpu_dir.display());
        }
        let ggml_metal_dir = sd_cpp_dir.join("build/ggml/src/ggml-metal");
        if ggml_metal_dir.exists() {
            println!("cargo:rustc-link-search=native={}", ggml_metal_dir.display());
        }
        let ggml_blas_dir = sd_cpp_dir.join("build/ggml/src/ggml-blas");
        if ggml_blas_dir.exists() {
            println!("cargo:rustc-link-search=native={}", ggml_blas_dir.display());
        }
    } else {
        println!("cargo:warning=stable-diffusion-cpp not found at {:?}", sd_cpp_dir);
        println!("cargo:warning=Set SD_LIB_DIR to link pre-built library.");
        return;
    }

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

    println!("cargo:rerun-if-env-changed=SD_LIB_DIR");
}

#[cfg(not(feature = "local"))]
fn main() {}
