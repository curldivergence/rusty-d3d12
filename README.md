# rusty-d3d12
This project provides low-level bindings for D3D12 API. It utilizes rust-bindgen for generating raw bindings (unlike `d3d12-rs` crate), but aims for providing idiomatic APIs (unlike the raw D3D12 wrappers from `winapi` crate).

Features:
- type-safe wrappers for D3D12 enumerations and bit flags
- wrappers for ID3D12 interfaces and POD structs (the latter are marked as `#[repr(transparent)]` so that they can be used as a drop-in replacement for the native types, but expose type-safe getters and setters)
- D3D12 Agility SDK (see heterogeneous_multiadapter.rs for an example of exporting required symbols)
- PIX marker support (to use this feature you need to set PIX_RUNTIME_PATH environment variable that would point to your directory with PIX runtime headers/libraries. If this variable is missing, the library will fail to compile, so this feature is disabled by default)
- automatic COM object reference counting via `Clone` and `Drop` traits implementations with optional logging possibilities (e.g. see `impl_com_object_refcount_named` macro)
- D3D12 debug callback support, object autonaming and GPU validation
- convenience macros for wrapping API calls (`dx_call` and `dx_try`)
- not yet covered APIs can be accessed through raw bindings exports, and new APIs can be wrapped in semi-automatic mode with the help of `conversion_assist.py` script

A list of currently implemented examples (note their code can be dirty and should not be treated as sane D3D12 tutorials or high-quality Rust code examples since their purpose is just to showcase the API):
- hello triangle
- hello texture (based on Microsoft sample)
- dynamic indexing (based on Microsoft sample)
- dynamic indexing using SM6.6 dynamic resources (basically a clone of the previous example with changes to indexing method and Agility SDK exports)
- mesh shaders (loosely based on Microsoft sample)
- heterogeneous multiadapter (closely follows Microsoft sample, so currently it is the most recommended sample to start exploring these bindings)

Examples that are planned to be added in the near future include port of Microsoft's D3D12nBodyGravity sample and DXR showcase.

This library is still a work-in-progress and is not ready yet to be used in production, so all contributions, including code reviews, are welcome :)