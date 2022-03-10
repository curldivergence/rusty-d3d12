extern crate bindgen;

use regex::Regex;
use std::env;
use std::path::PathBuf;

const D3D12_AGILITY_SDK_INCLUDE_PATH: &str = "extern\\D3D12AgilitySDK\\include";
const D3D12_AGILITY_SDK_LIB_PATH: &str = "extern\\D3D12AgilitySDK\\bin";

#[cfg(feature = "pix")]
const PIX_INCLUDE_PATH: &str = "extern\\WinPixEventRuntime\\include";
#[cfg(feature = "pix")]
const PIX_LIB_PATH: &str = "extern\\WinPixEventRuntime\\bin";

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
    let workspace_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    println!(
        "cargo:rustc-link-search={}",
        workspace_dir
            .join(D3D12_AGILITY_SDK_LIB_PATH)
            .to_str()
            .unwrap()
    );
    println!("cargo:rustc-link-lib=d3d12");
    println!("cargo:rustc-link-lib=dxgi");
    // we have no __uuidof, so let's use oldschool way
    println!("cargo:rustc-link-lib=dxguid");
    println!(
        "cargo:rustc-link-arg=/DEF:{}\\agility.def",
        workspace_dir
            .join(D3D12_AGILITY_SDK_LIB_PATH)
            .to_str()
            .unwrap()
    );

    #[cfg(feature = "devel")]
    generate_bindings();

    // PIX support is disabled by default not to introduce a dependency for WinPixEventRuntime.dll
    #[cfg(feature = "pix")]
    {
        // Our PIX wrapper has an extra layer - C wrapper around the original C++ interface
        // Since without `devel` feature we cannot run bindgen and build it, we have to ship
        // the pre-built library
        println!(
            "cargo:rustc-link-search={}",
            workspace_dir.join(PIX_LIB_PATH).to_str().unwrap()
        );
        println!("cargo:rustc-link-lib=static=pix_wrapper");
        println!("cargo:rustc-link-lib=WinPixEventRuntime");
    }

    // Copy DX12 Agility SDK libs that are needed by examples
    let copy_source_path = workspace_dir.join(D3D12_AGILITY_SDK_LIB_PATH);
    let profile = env::var("PROFILE").unwrap();
    let examples_bin_path =
        workspace_dir.join("target").join(profile).join("examples");
    let copy_dest_path = examples_bin_path.join("D3D12");
    std::fs::create_dir_all(&copy_dest_path).unwrap_or_else(|err| {
        eprintln!(
            "Cannot create D3D12 dir to copy Agility SDK dlls: {:?}",
            err
        );
    });

    let files_to_copy = [
        "D3D12Core.dll",
        "D3D12Core.pdb",
        "d3d12SDKLayers.dll",
        "d3d12SDKLayers.pdb",
    ];

    for file in files_to_copy {
        std::fs::copy(copy_source_path.join(file), copy_dest_path.join(file))
            .unwrap_or_else(|err| {
                eprintln!("cannot copy Agility SDK dlls: {:?}", err);
                0
            });
    }

    #[cfg(feature = "pix")]
    {
        // Copy PIX runtime DLL since it's needed by examples
        let pix_dll_name = "WinPixEventRuntime.dll";
        std::fs::copy(
            &format!("{}\\{}", PIX_LIB_PATH, pix_dll_name),
            examples_bin_path.join(pix_dll_name),
        )
        .unwrap_or_else(|err| {
            eprintln!("cannot copy {}: {:?}", pix_dll_name, err);
            0
        });

        // Copy PIX libs to OUT dir
        let pix_wrapper_lib_name = "pix_wrapper.lib";
        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

        // dll
        std::fs::copy(
            &format!("{}\\{}", PIX_LIB_PATH, pix_dll_name),
            out_path.join(pix_dll_name),
        )
        .unwrap_or_else(|err| {
            eprintln!("cannot copy {}: {:?}", pix_dll_name, err);
            0
        });

        // static wrapper lib
        std::fs::copy(
            &format!("{}\\{}", PIX_LIB_PATH, pix_wrapper_lib_name),
            out_path.join(pix_wrapper_lib_name),
        )
        .unwrap_or_else(|err| {
            eprintln!("cannot copy {}: {:?}", pix_wrapper_lib_name, err);
            0
        });
    }
}

#[cfg(feature = "devel")]
fn generate_bindings() {
    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=generation\\d3d12_wrapper.h");

    let workspace_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .to_str()
        .expect("Workspace path is not valid UTF-8")
        .to_owned();
    let bindings = bindgen::Builder::default()
        .clang_arg("-std=c99")
        // .clang_arg(&format!("-Igeneration"))
        .header_contents("d3d12_patched.h", &patch_d3d12_header())
        .header("generation\\d3d12_wrapper.h")
        .layout_tests(false)
        .derive_debug(true)
        .impl_debug(true)
        .derive_default(true)
        .derive_eq(true)
        .derive_partialeq(true)
        .impl_partialeq(true)
        .derive_hash(true)
        .derive_ord(true)
        // DXGI and D3D types, vars and functions
        .allowlist_type(".*DXGI.*")
        .allowlist_type(".*D3D12.*")
        .allowlist_var(".*DXGI.*")
        .allowlist_var(".*D3D12.*")
        .allowlist_var(".*IID_.*")
        .allowlist_var(".*WKPDID_.*")
        .allowlist_function(".*DXGI.*")
        .allowlist_function(".*D3D12.*")
        .allowlist_function("Enum.*")
        .allowlist_function("CopyBufferRegion")
        // WinAPI functions from <synchapi.h>
        .allowlist_function("CreateEventW")
        .allowlist_function("WaitForSingleObject")
        .allowlist_function("CloseHandle")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("d3d12_bindings.rs"))
        .expect("Cannot write bindings");

    generate_pix_bindings();
}

#[cfg(feature = "devel")]
fn generate_pix_bindings() {
    let workspace_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // Build C wrapper over C++ PIX header
    cc::Build::new()
        .cpp(true)
        .include(workspace_dir.join(PIX_INCLUDE_PATH))
        .include(workspace_dir.join(D3D12_AGILITY_SDK_INCLUDE_PATH))
        .include(workspace_dir.join("generation"))
        .file(workspace_dir.join("generation").join("pix_wrapper.cpp"))
        .compile("pix_wrapper");

    // Generate Rust bindings to C wrapper
    println!(
        "cargo:rerun-if-changed={}\\generation\\pix_wrapper.h",
        workspace_dir.to_str().unwrap()
    );
    println!(
        "cargo:rerun-if-changed={}\\generation\\pix_wrapper.cpp",
        workspace_dir.to_str().unwrap()
    );

    let bindings = bindgen::Builder::default()
        .layout_tests(false)
        .clang_arg(&format!(
            "-I{}",
            workspace_dir
                .join(D3D12_AGILITY_SDK_INCLUDE_PATH)
                .to_str()
                .unwrap()
        ))
        .clang_arg(&format!(
            "-I{}",
            workspace_dir.join(PIX_INCLUDE_PATH).to_str().unwrap()
        ))
        .header(&format!(
            "{}\\generation\\pix_wrapper.h",
            workspace_dir.to_str().unwrap()
        ))
        .allowlist_function("pix_.*")
        .allowlist_type("ID3D12GraphicsCommandList.*")
        .allowlist_type("ID3D12CommandQueue.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("pix_bindings.rs"))
        .expect("Couldn't write bindings");
}
