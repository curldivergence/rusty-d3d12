# rusty-d3d12
This project provides low-level bindings for D3D12 API. It utilizes rust-bindgen for generating raw bindings (unlike `d3d12-rs` crate), but aims for providing idiomatic APIs (unlike the raw D3D12 wrappers from `winapi` crate).

### Features:
- type-safe wrappers for D3D12 enumerations and bit flags
- wrappers for ID3D12 interfaces and POD structs (the latter are marked as `#[repr(transparent)]` so that they can be used as a drop-in replacement for the native types, but expose type-safe getters and setters)
- D3D12 Agility SDK (see `heterogeneous_multiadapter.rs` for an example of exporting required symbols)
- PIX marker support
- automatic COM object reference counting via `Clone` and `Drop` traits implementations with optional logging possibilities (e.g. see `impl_com_object_refcount_named` macro)
- D3D12 debug callback support, object autonaming and GPU validation
- convenience macros for wrapping API calls (`dx_call` and `dx_try`)
- not yet covered APIs can be accessed through raw bindings exports, and new APIs can be wrapped in semi-automatic mode with the help of `conversion_assist.py` script

### List of currently implemented examples 
Please note their code can be dirty and contains some (non-critical) bugs, so it should not be treated as sane D3D12 tutorials or high-quality Rust code examples since their purpose is just to showcase the API.

- hello_triangle
- hello_texture (based on Microsoft sample)
- dynamic_indexing (based on Microsoft sample)
- dynamic_indexing_sm66 (basically a clone of the previous example with changes related to using SM6.6 dynamic resources and Agility SDK exports)
- hello_mesh_shaders (loosely based on Microsoft sample)
- heterogeneous_multiadapter (closely follows Microsoft sample, so currently it is the most recommended sample to start exploring these bindings if you want to compare them to C++ code line-by-line)
- interprocess_communication (demonstrates usage of a shared heap by two processes - producer and consumer)
- n_body_gravity (based on Microsoft sample, but uses a different threading model).

The next planned goal for this project is to cover DXR APIs and provide the corresponding samples.

### Contribution
This library is still a work-in-progress and is not ready yet to be used in production, so all contributions, including code reviews, are welcome :)

When used as a cargo dependency, `rusty-d3d12` does not generate bindings during build process since running `rust-bindgen` requires `libclang.dll`, which can be absent on some systems, and cannot be vendored via `crates.io` due to its large size. So as a prerequisite, Cargo should be able to found this dll under the path set in `LIBCLANG_PATH` environment variable. After this requirement is met, Cargo feature `devel` can be activated, and `bindings.rs` file will be generated from scratch.
Also, to use `pix` Cargo feature you need to set `PIX_RUNTIME_PATH` environment variable that would point to your directory with PIX runtime headers/libraries. If this variable is missing, the library will fail to compile, so this feature is disabled by default.