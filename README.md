# rusty-d3d12
This project provides low-level bindings for D3D12 API. It utilizes rust-bindgen for generating raw bindings (unlike d3d12-rs crate), but aims for providing idiomatic APIs (unlike the raw D3D12 wrappers from winapi crate).

A list of currently implemented examples (note their code can be dirty and should not be treated as sane D3D12 tutorials or high-quality Rust code examples since their purpose is just to showcase the API):
- hello triangle
- hello texture (based on Microsoft sample)
- dynamic indexing (based on Microsoft sample)
- dynamic indexing using SM6.6 dynamic resources (basically a clone of the previous example with changes to indexing method and Agility SDK exports)
- mesh shaders (loosely based on Microsoft sample)

Examples that are planned to be added in the near future include port of Microsoft's D3D12nBodyGravity sample and DXR showcase.

Also, please check out a satellite [pixwrapper](https://crates.io/crates/pixwrapper) crate that can also be used as a standalone helper.

This library is still work-in-progress and is not ready yet to be used in production, so all contributions, including code reviews, are welcome :)