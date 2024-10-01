# `egui-directx11`: a Direct3D11 renderer for [`egui`](https://crates.io/crates/egui)

This crate aims to provide a *minimal* set of features and APIs to render
outputs from `egui` using Direct3D11.

## Quick Start

There is an [`egui-demo`](examples/egui-demo.rs) example, which demonstrates all you need to do to set up a minimal application
with Direct3D11 and `egui`. This example uses `winit` for window management and
event handling, while native Win32 APIs should also work well.

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

## Project Status and Versioning Compatibility on `windows` and `egui`

This crate has been considered as general available without known issues since
version `0.3.1`. Though, it keeps bumping major version to follow major version
bumps on its direct dependencies, namely `windows` and  `egui`.

To select a version for this crate according to the `windows` and `egui` version
your project uses, please check the table below:

|`egui-directx11`|`windows`|`egui`|
|-|-|-|
|0.6.0|0.58.0|0.29.x|
|0.5.0|0.58.0|0.28.x|
|0.4.0|0.56.0|0.27.x|
|0.3.1|0.54.0|0.27.x|

Releases of this crate before `0.3.1` are considered premature and are not recommended to use.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT).
