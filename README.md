[![Documentation](https://docs.rs/rusty-d3d12/badge.svg)](https://docs.rs/rusty-d3d12)
[![Crates.io](https://img.shields.io/crates/v/rusty-d3d12.svg)](https://crates.io/crates/rusty-d3d12)

# rusty-d3d12
This project provides low-level bindings for D3D12 API. It utilizes `rust-bindgen` for generating raw bindings (unlike `d3d12-rs` crate), but aims for providing idiomatic APIs (unlike the raw D3D12 wrappers from `winapi` or `windows-rs` crates).

## Features
- wrappers for `ID3D12*` interfaces and POD structs. The latter are marked as `#[repr(transparent)]` so that they can be used as a drop-in replacement for the native types, but expose type-safe getters and setters. The setters have two forms: `with_*(mut self, ...) -> Self` and `set_*(&mut self, ...) -> &mut Self` and are intended for building new structures and modifying the existing ones, respectively
- type-safe wrappers for D3D12 enumerations and bit flags (see [enum_wrappers.rs](src/enum_wrappers.rs) for details)
- `D3D12` and `DXGI` prefixes have been stripped from all types, functions and enum variants (e.g. this library exposes `CommandListType::Direct` instead of `D3D12_COMMAND_LIST_TYPE_DIRECT`) since it's very likely that people who use it already know the name of the API it wraps (it's mentioned in the crate name after all), and do not need to be constantly reminded about it :) Also all type and function names have been reshaped with respect to the official Rust code style (e.g. `get_gpu_descriptor_handle_for_heap_start` instead of `GetGPUDescriptorHandleForHeapStart`). Note that most, but *not* all the enum variant names have been converted yet, so some of them will be changed in future versions
- D3D12 Agility SDK is integrated into the library and shipped along with it (see `heterogeneous_multiadapter.rs` for an example of exporting required symbols). Current SDK version is `1.600.10`
- PIX markers (they require enabling `pix` feature which is off by default not to introduce a dependency on `WinPixEventRuntime.dll` for people who don't need it)
- automatic COM object reference counting via `Clone` and `Drop` traits implementations with optional logging possibilities (e.g. see `impl_com_object_refcount_named` macro)
- D3D12 debug callback support (please note that `debug_callback` feature needs to be activated explicitly since `ID3D12InfoQueue1` interface is only supported on Windows 11), object autonaming and GPU validation
- convenience macros for wrapping API calls (`dx_call!` and `dx_try!`)
- not yet covered APIs can be accessed through raw bindings exports, and new APIs can be wrapped in semi-automatic mode with the help of `conversion_assist.py` script
- most of the APIs provided by `rusty-d3d12` are *not* marked as `unsafe` since it pollutes client code while giving little in return: obviously, a lot of bad things can happen due to misusing D3D12, but guarding against something like that is a task for a *high*-level graphics library or engine. So `unsafe` is reserved for something unsafe that happens on Rust side, e.g. accessing unions (see `ClearValue::color()`)

## Examples

- create debug controller and enable validations:
```rust
let debug_controller = Debug::new().expect("cannot create debug controller");
debug_controller.enable_debug_layer();
debug_controller.enable_gpu_based_validation();
debug_controller.enable_object_auto_name();
```
- create a descriptor heap:
```rust
let rtv_heap = device
    .create_descriptor_heap(
        &DescriptorHeapDesc::default()
            .with_heap_type(DescriptorHeapType::Rtv)
            .with_num_descriptors(FRAMES_IN_FLIGHT),
    )
    .expect("Cannot create RTV heap");
rtv_heap
    .set_name("RTV heap")
    .expect("Cannot set RTV heap name");
```
- check if cross-adapter textures are supported:
```rust
let mut feature_data = FeatureDataOptions::default();
device
    .check_feature_support(Feature::D3D12Options, &mut feature_data)
    .expect("Cannot check feature support");

let cross_adapter_textures_supported = feature_data.cross_adapter_row_major_texture_supported();
```
- create mesh shader PSO:
```rust
let ms_bytecode = ShaderBytecode::new(&mesh_shader);
let ps_bytecode = ShaderBytecode::new(&pixel_shader);

let pso_subobjects_desc = MeshShaderPipelineStateDesc::default()
    .with_root_signature(root_signature)
    .with_ms_bytecode(&ms_bytecode)
    .with_ps_bytecode(&ps_bytecode)
    .with_rasterizer_state(
        RasterizerDesc::default().with_depth_clip_enable(false),
    )
    .with_blend_state(BlendDesc::default())
    .with_depth_stencil_state(
        DepthStencilDesc::default().with_depth_enable(false),
    )
    .with_primitive_topology_type(PrimitiveTopologyType::Triangle)
    .with_rtv_formats(&[Format::R8G8B8A8Unorm]);

let pso_desc = PipelineStateStreamDesc::default()
    .with_pipeline_state_subobject_stream(
        pso_subobjects_desc.as_byte_stream(),
    );

let pso = device
    .create_pipeline_state(&pso_desc)
    .expect("Cannot create PSO");
```

Several runnable samples can be found in [examples](examples/) directory. Please note their code can be dirty and contains some (non-critical) bugs, so they should not be treated as sane D3D12 tutorials or high-quality Rust code examples since their purpose is just to showcase the API.

Currently implemented examples include:

- [hello_triangle](examples/hello_triangle.rs)
- [hello_texture](examples/hello_texture.rs) (based on Microsoft sample)
- [dynamic_indexing_sm66](examples/dynamic_indexing_sm66.rs) (based on Microsoft sample with changes related to using SM6.6 dynamic resources and Agility SDK exports)
- [hello_mesh_shaders](examples/hello_mesh_shaders.rs) (loosely based on Microsoft sample)
- [heterogeneous_multiadapter](examples/heterogeneous_multiadapter.rs) (closely follows Microsoft sample, so currently it is the most recommended sample to start exploring these bindings if you want to compare them to C++ code line-by-line)
- [interprocess_communication](examples/interprocess_communication.rs) (demonstrates usage of a shared heap by two processes - producer and consumer)
- [n_body_gravity](examples/n_body_gravity.rs) (based on Microsoft sample, but uses a different threading model).

The next planned goal for this project is to cover DXR APIs and provide the corresponding samples.

## API stability
Currently the library is under active development, so breaking changes can happen between minor releases (but *should* not happen between patch releases). After publishing version `1.0` standard semantic versioning will be applied.

## Making changes
As mentioned above, the library is still a work-in-progress, so all contributions are welcome :)

### How to add a struct or enum that is missing
If the type in question is already present in the pre-generated `d3d12.rs`, you can use [conversion_assist.py](tools/conversion_assist.py) script to generate most (or sometimes all) of the code for you.

- to generate a struct wrapper:
  1. run `python tools/conversion_assist.py struct`
  2. paste the raw definition of the struct without attributes and derives (i.e. starting from `pub struct`) and without impl blocks, e.g.:
    ```rust
    pub struct D3D12_ROOT_DESCRIPTOR1 {
        pub ShaderRegister: UINT,
        pub RegisterSpace: UINT,
        pub Flags: D3D12_ROOT_DESCRIPTOR_FLAGS,
    }
    ```
  3. Press `Enter`.
  4. The script will provide you with the boilerplate wrapper struct definition, e.g.
  ```rust
  /// Wrapper around D3D12_ROOT_DESCRIPTOR1 structure
  #[derive(Default, Debug, Hash, PartialOrd, Ord, PartialEq, Eq, Clone)]
  #[repr(transparent)]
  pub struct RootDescriptor(pub(crate) D3D12_ROOT_DESCRIPTOR1);

  impl RootDescriptor {
      pub fn set_shader_register(&mut self, shader_register: u32) -> &mut Self {
          self.0.ShaderRegister = shader_register;
          self
      }

      pub fn with_shader_register(mut self, shader_register: u32) -> Self {
          self.set_shader_register(shader_register);
          self
      }

      pub fn shader_register(&self) -> u32 {
          self.0.ShaderRegister
      }

      pub fn set_register_space(&mut self, register_space: u32) -> &mut Self {
          self.0.RegisterSpace = register_space;
          self
      }

      pub fn with_register_space(mut self, register_space: u32) -> Self {
          self.set_register_space(register_space);
          self
      }

      pub fn register_space(&self) -> u32 {
          self.0.RegisterSpace
      }

      pub fn set_flags(&mut self, flags: RootDescriptorFlags) -> &mut Self {
          self.0.Flags = flags.bits();
          self
      }

      pub fn with_flags(mut self, flags: RootDescriptorFlags) -> Self {
          self.set_flags(flags);
          self
      }

      pub fn flags(&self) -> RootDescriptorFlags {
          unsafe { RootDescriptorFlags::from_bits_unchecked(self.0.Flags) }
      }
  }
  ```
  5. Note that the raw untyped enumeration `D3D12_ROOT_DESCRIPTOR_FLAGS` was automatically changed to the correspondent wrapper `RootDescriptorFlags` in the signatures of the getter and setters: this is possible since the script parses `enum_wrappers.rs` for the already known types and tries to recognize them.
  6. If needed (i.e. if the original struct contains raw pointers), add the `PhantomData`'s with lifetime specifiers  (please see [src/struct_wrappers.rs](src/struct_wrappers.rs) for examples).
  7. Add the final type definition to `struct_wrappers.rs` and open a PR :)

- to generate enum wrapper:
  1. run `python tools/conversion_assist.py enum`
  2. paste enum variants and the type alias from `d3d12.rs`:
  ```rust
  pub const D3D12_DESCRIPTOR_RANGE_TYPE_D3D12_DESCRIPTOR_RANGE_TYPE_SRV:
      D3D12_DESCRIPTOR_RANGE_TYPE = 0;
  pub const D3D12_DESCRIPTOR_RANGE_TYPE_D3D12_DESCRIPTOR_RANGE_TYPE_UAV:
      D3D12_DESCRIPTOR_RANGE_TYPE = 1;
  pub const D3D12_DESCRIPTOR_RANGE_TYPE_D3D12_DESCRIPTOR_RANGE_TYPE_CBV:
      D3D12_DESCRIPTOR_RANGE_TYPE = 2;
  pub const D3D12_DESCRIPTOR_RANGE_TYPE_D3D12_DESCRIPTOR_RANGE_TYPE_SAMPLER:
      D3D12_DESCRIPTOR_RANGE_TYPE = 3;
  pub type D3D12_DESCRIPTOR_RANGE_TYPE = ::std::os::raw::c_int;
  ```
  3. As you could notice, `rust-bindgen` duplicates enum name in each of the variants, so the script will ask you about the part you'd like to strip from the variants; in this case it's `D3D12_DESCRIPTOR_RANGE_TYPE_D3D12_DESCRIPTOR_RANGE_TYPE_`
  4. Paste the autogenerated enum definition to [src/enum_wrappers.rs](src/enum_wrappers.rs).
- if your enum variants are not exclusive (i.e. can be OR'ed together etc.), then follow the same procedure, but use `python tools/conversion_assist.py flags`: the script will generate [bitflags](https://crates.io/crates/bitflags) definition.

### How to add a missing function
Unfortunately `conversion_assist.py` doesn't support generating function definitions yet, so it should be done manually :( Please refer to [lib.rs](src/lib.rs) for examples.

### Running rust-bindgen
If the required function or type is not yet present in the shipped `d3d12.rs` (i.e. the new Agility SDK has come out but has not been integrated into `rusty-d3d12` yet), then running `rust-bindgen` on the workspace is required after updating Agility SDK.

When used as a Cargo dependency, `rusty-d3d12` does not generate bindings during build process by default (besides increasing build times, running `rust-bindgen` requires `libclang.dll`, which can be absent on some systems, and cannot be vendored via `crates.io` due to its large size). So as a prerequisite, Cargo should be able to find this DLL under the path set in `LIBCLANG_PATH` environment variable. After this requirement is met, Cargo feature `devel` can be activated, and `d3d12_bindings.rs` and `pix_bindings.rs` files will be generated from scratch, and included into `src/raw_bindings/mod.rs` instead of the shipped ones. Of course, enabling this feature and copying `libclang.dll` is *not* required if one doesn't need to update Agility SDK headers and just wants to wrap some APIs that are already present in the shipped `d3d12.rs` but not yet covered by this library.

After generating the new raw bindings file (`d3d12_bindings.rs`, please see the build script for details) using `rust-bindgen` it should be copied from `$OUT_DIR` to [src/raw_bindings](src/raw_bindings/) directory and renamed into `d3d12.rs`.