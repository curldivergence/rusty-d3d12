# rusty-d3d12
This project provides low-level bindings for D3D12 API. It utilizes `rust-bindgen` for generating raw bindings (unlike `d3d12-rs` crate), but aims for providing idiomatic APIs (unlike the raw D3D12 wrappers from `winapi` or `windows-rs` crates).

### Features:
- type-safe wrappers for D3D12 enumerations and bit flags
- wrappers for ID3D12 interfaces and POD structs (the latter are marked as `#[repr(transparent)]` so that they can be used as a drop-in replacement for the native types, but expose type-safe getters and setters)
- `D3D12` and `DXGI` prefixes have been stripped from all types, functions and enum variants (e.g. this library exposes `CommandListType::Direct` instead of `D3D12_COMMAND_LIST_TYPE_DIRECT`) since it's very likely that people who use it already know the name of the API it wraps (it's mentioned in the crate name after all), and do not need to be constantly reminded about it :) Also all type and function names have been reshaped with respect to the official Rust code style (e.g. `get_gpu_descriptor_handle_for_heap_start` instead of `GetGPUDescriptorHandleForHeapStart`). Note that *not* all enum variant names have been converted yet, so some of them will be changed in future versions
- D3D12 Agility SDK is integrated into the library and shipped along with it (see `heterogeneous_multiadapter.rs` for an example of exporting required symbols)
- PIX marker support (enabled by a feature `pix` that is off by default not to introduce a dependency on `WinPixEventRuntime.dll` for people who don't need it)
- automatic COM object reference counting via `Clone` and `Drop` traits implementations with optional logging possibilities (e.g. see `impl_com_object_refcount_named` macro)
- D3D12 debug callback support, object autonaming and GPU validation
- convenience macros for wrapping API calls (`dx_call` and `dx_try`)
- not yet covered APIs can be accessed through raw bindings exports, and new APIs can be wrapped in semi-automatic mode with the help of `conversion_assist.py` script

### List of currently implemented examples
Please note their code can be dirty and contains some (non-critical) bugs, so they should not be treated as sane D3D12 tutorials or high-quality Rust code examples since their purpose is just to showcase the API.

- hello_triangle
- hello_texture (based on Microsoft sample)
- dynamic_indexing_sm66 (based on Microsoft sample with changes related to using SM6.6 dynamic resources and Agility SDK exports)
- hello_mesh_shaders (loosely based on Microsoft sample)
- heterogeneous_multiadapter (closely follows Microsoft sample, so currently it is the most recommended sample to start exploring these bindings if you want to compare them to C++ code line-by-line)
- interprocess_communication (demonstrates usage of a shared heap by two processes - producer and consumer)
- n_body_gravity (based on Microsoft sample, but uses a different threading model).

The next planned goal for this project is to cover DXR APIs and provide the corresponding samples.

### Making changes
This library is still a work-in-progress, so all contributions are welcome :)

When used as a Cargo dependency, `rusty-d3d12` does not generate bindings during build process by default, since running `rust-bindgen` requires `libclang.dll`, which can be absent on some systems, and cannot be vendored via `crates.io` due to its large size. So as a prerequisite, Cargo should be able to find this DLL under the path set in `LIBCLANG_PATH` environment variable. After this requirement is met, Cargo feature `devel` can be activated, and `d3d12_bindings.rs` and `pix_bindings.rs` files will be generated from scratch, and included into `src/raw_bindings/mod.rs` instead of the shipped ones. Of course, enabling this feature and copying `libclang.dll` is *not* required if one doesn't need to update Agility SDK headers and just wants to wrap some APIs that are already present in the shipped `d3d12.rs` but not yet covered by this library.