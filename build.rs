extern crate bindgen;

use regex::Regex;
use std::env;
use std::path::{Path, PathBuf};

const D3D12_AGILITY_SDK_INCLUDE_PATH: &str = "D3D12AgilitySDK\\include";
const D3D12_AGILITY_SDK_LIB_PATH: &str = "D3D12AgilitySDK\\bin";

fn find_d3d12_header() -> Option<String> {
    let path = PathBuf::from(D3D12_AGILITY_SDK_INCLUDE_PATH)
        .join("d3d12.h")
        .to_str()
        .expect("Path to Agility SDK is not valid UTF-8")
        .to_owned();

    eprintln!("Trying to find d3d12.h at {}", path);
    Some(path)
}

fn patch_d3d12_header() -> String {
    let d3d12_contents = std::fs::read_to_string(
        find_d3d12_header().expect("Cannot find d3d12.h"),
    )
    .expect("Something went wrong reading d3d12.h");

    let cpu_regex = Regex::new(concat!(
        r"(D3D12_CPU_DESCRIPTOR_HANDLE\s*",
        r"\(\s*STDMETHODCALLTYPE\s*\*GetCPUDescriptorHandleForHeapStart\s*\)",
        r"\(\s*ID3D12DescriptorHeap\s*\*\s*This\);)"
    ))
    .unwrap();

    let patched_contents = cpu_regex.replace_all(
        &d3d12_contents,
        concat!(
            "void(STDMETHODCALLTYPE *GetCPUDescriptorHandleForHeapStart)",
            "(\n\tID3D12DescriptorHeap *This, ",
            "D3D12_CPU_DESCRIPTOR_HANDLE* pHandle);"
        ),
    );

    let gpu_regex = Regex::new(concat!(
        r"(D3D12_GPU_DESCRIPTOR_HANDLE\s*",
        r"\(\s*STDMETHODCALLTYPE\s*\*GetGPUDescriptorHandleForHeapStart\s*\)",
        r"\(\s*ID3D12DescriptorHeap\s*\*\s*This\);)"
    ))
    .unwrap();

    let patched_contents = gpu_regex.replace_all(
        &patched_contents,
        concat!(
            "void(STDMETHODCALLTYPE *GetGPUDescriptorHandleForHeapStart)",
            "(\n\tID3D12DescriptorHeap *This, ",
            "D3D12_GPU_DESCRIPTOR_HANDLE* pHandle);"
        ),
    );

    let resource_desc_regex = Regex::new(concat!(
        r"(D3D12_RESOURCE_DESC\s*",
        r"\(\s*STDMETHODCALLTYPE\s*\*GetDesc\s*\)",
        r"\(\s*ID3D12Resource\s*\*\s*This\);)"
    ))
    .unwrap();

    let patched_contents = resource_desc_regex.replace_all(
        &patched_contents,
        concat!(
            "void(STDMETHODCALLTYPE *GetDesc)",
            "(\n\tID3D12Resource *This, ",
            "D3D12_RESOURCE_DESC* pHandle);"
        ),
    );

    let resource_desc1_regex = Regex::new(concat!(
        r"(D3D12_RESOURCE_DESC1\s*",
        r"\(\s*STDMETHODCALLTYPE\s*\*GetDesc1\s*\)",
        r"\(\s*ID3D12Resource2\s*\*\s*This\);)"
    ))
    .unwrap();

    resource_desc1_regex
        .replace_all(
            &patched_contents,
            concat!(
                "void(STDMETHODCALLTYPE *GetDesc1)",
                "(\n\tID3D12Resource2 *This, ",
                "D3D12_RESOURCE_DESC1* pHandle);"
            ),
        )
        .to_string()
}

fn main() {
    // Tell cargo to tell rustc to link the system d3d12 and dxgi
    // shared libraries.
    println!("cargo:rustc-link-lib=d3d12");
    println!("cargo:rustc-link-lib=dxgi");
    // we have no __uuidof, so let's use oldschool way
    println!("cargo:rustc-link-lib=dxguid");
    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    println!("cargo:rustc-link-arg=/DEF:agility.def");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .clang_arg("-std=c99")
        .header_contents("d3d12_patched.h", &patch_d3d12_header())
        .header("wrapper.h")
        .header(
            PathBuf::from(D3D12_AGILITY_SDK_INCLUDE_PATH)
                .join("d3d12sdklayers.h")
                .to_str()
                .expect("Cannot find vendored d3d12sdklayers.h"),
        )
        .layout_tests(false)
        .derive_debug(true)
        .impl_debug(true)
        .derive_default(true)
        // DXGI and D3D types, vars and functions
        .whitelist_type(".*DXGI.*")
        .whitelist_type(".*D3D12.*")
        .whitelist_var(".*DXGI.*")
        .whitelist_var(".*D3D12.*")
        .whitelist_var(".*IID_.*")
        .whitelist_var(".*WKPDID_.*")
        .whitelist_function(".*DXGI.*")
        .whitelist_function(".*D3D12.*")
        .whitelist_function("Enum.*")
        .whitelist_function("CopyBufferRegion")
        // WinAPI functions from <synchapi.h>
        .whitelist_function("CreateEventW")
        .whitelist_function("WaitForSingleObject")
        .whitelist_function("CloseHandle")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Copy DX12 Agility SDK libs that are needed by examples
    let manifest_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let copy_source_path = manifest_path.join(D3D12_AGILITY_SDK_LIB_PATH);
    let profile = env::var("PROFILE").unwrap();
    let examples_bin_path =
        manifest_path.join("target").join(profile).join("examples");
    let copy_dest_path = examples_bin_path.join("D3D12");
    std::fs::create_dir_all(&copy_dest_path)
        .expect("Cannot create D3D12 dir to copy Agility SDK dlls");

    let files_to_copy = [
        "D3D12Core.dll",
        "D3D12Core.pdb",
        "d3d12SDKLayers.dll",
        "d3d12SDKLayers.pdb",
    ];

    for file in files_to_copy {
        std::fs::copy(copy_source_path.join(file), copy_dest_path.join(file))
            .expect("Cannot copy Agility SDK dlls");
    }

    #[cfg(feature = "pix")]
    setup_pix_wrapper();
}

#[cfg(feature = "pix")]
fn setup_pix_wrapper() {
    let pix_runtime_path = PathBuf::from(env::var("PIX_RUNTIME_PATH").unwrap())
        .to_str()
        .unwrap()
        .to_owned();

    // Build C wrapper over C++ PIX header
    cc::Build::new()
        .cpp(true)
        .include(format!("{}\\include\\", pix_runtime_path))
        .include(
            PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
                .join("D3D12AgilitySDK")
                .join("include"),
        )
        .file("pix_wrapper.cpp")
        .compile("pix_wrapper");

    // Generate Rust bindings to C wrapper
    println!("cargo:rustc-link-search={}\\bin\\x64", pix_runtime_path);
    println!("cargo:rustc-link-lib=WinPixEventRuntime");
    println!("cargo:rerun-if-changed=pix_wrapper.h");
    println!("cargo:rerun-if-changed=pix_wrapper.cpp");
    println!("cargo:rustc-link-lib=static=pix_wrapper");

    let bindings = bindgen::Builder::default()
        .layout_tests(false)
        .header("pix_wrapper.h")
        .whitelist_function("pix_.*")
        .whitelist_type("ID3D12GraphicsCommandList.*")
        .whitelist_type("ID3D12CommandQueue.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("pix_bindings.rs"))
        .expect("Couldn't write bindings!");
}
