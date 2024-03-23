# `egui-directx11`: a minimal D3D11 renderer for [`egui`](https://crates.io/crates/egui).

This crate is a successor to [`egui-d3d11`](https://crates.io/crates/egui-d3d11),
which is no longer maintained and has certain issues or inconvenience in some cases.

This crate aims to provide a *minimal* set of features and APIs to render
outputs from `egui` using D3D11, and is built upon the *official* Rust bindings of D3D11 and DXGI APIs
from the [`windows`](https://crates.io/crates/windows) crate [maintained by
Microsoft](https://github.com/microsoft/windows-rs).

This crate is in early development. It should work in most cases but may lack
certain features or functionalities.

To get started, you can take a look at the [`egui-demo`](https://github.com/Nekomaru-PKU/egui-directx11/blob/main/examples/egui-demo/src/main.rs) example.
