# `egui-directx11`: a Direct3D11 renderer for [`egui`](https://crates.io/crates/egui)

This crate aims to provide a *minimal* set of features and APIs to render
outputs from `egui` using Direct3D11.

## NOTICE: Breaking Change in Version 0.10.0

Due to `egui` requiring **all** color blending performed in gamma space to
produce correct results, **render target passed to `Render::render`
MUST be in the gamma color space and viewed as non-sRGB-aware** since version 0.10.0.
**This is a breaking change when upgrading to version 0.10.0 from a previous version**.

**Support for rendering to linear render targets have been discontinued**.
If you have to render to a render target in linear color space, you must create an
intermediate render target in gamma color space and perform a blit operation afterwards.

## Examples

For a quick start, `examples/main.rs` demonstrates how to set up a minimal application
with Direct3D11 and `egui`.

+ Run `cargo run --example main` for the `egui` demo;
+ Run `cargo run --example main -- color-test` for the `egui` color test;

Provided examples use `winit` for window management and event handling,
while native Win32 APIs also works well.

## Considerations

This crate is a successor to [`egui-d3d11`](https://crates.io/crates/egui-d3d11),
which is no longer maintained and has certain issues or inconvenience in some cases.

We assume you to be familiar with developing
graphics applications using Direct3D11, and if not, this crate is not likely
useful for you. Besides, this crate cares only about rendering outputs
from `egui`, so it is all *your* responsibility to handle things like
setting up the window and event loop, creating the device and swap chain, etc.

This crate is built upon the *official* Rust bindings of Direct3D11 and DXGI APIs
from the [`windows`](https://crates.io/crates/windows) crate [maintained by
Microsoft](https://github.com/microsoft/windows-rs). Using this crate with
other Direct3D11 bindings is not recommended and may result in unexpected behavior.

## Stability and Versioning

This crate has been considered as general available without known issues since
version `0.10.0`. Though, it keeps bumping major version to follow major version
bumps on its direct dependencies, namely `windows` and  `egui`. Releases of this
crate before `0.10.0` are considered premature and are not recommended to use.

Breaking changes across *major* versions are generally avoided, but is not guaranteed.
Minor and patch version bumps are guaranteed to be backward compatible without
behavior or visual changes.

## License and Contribution

This repo and the `egui-directx11` crate is licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
