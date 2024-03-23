#![warn(missing_docs)]

//! `egui-directx11`: a minimal Direct3D11 renderer for [`egui`](https://crates.io/crates/egui).
//! 
//! This crate aims to provide a *minimal* set of features and APIs to render
//! outputs from `egui` using Direct3D11. We assume you to be familiar with developing
//! graphics applications using Direct3D11, and if not, this crate is not likely
//! useful for you. Besides, this crate cares only about rendering outputs
//! from `egui`, so it is all *your* responsibility to handle things like
//! setting up the window and event loop, creating the device and swap chain, etc.
//! 
//! This crate is built upon the *official* Rust bindings of Direct3D11 and DXGI APIs
//! from the [`windows`](https://crates.io/crates/windows) crate [maintained by
//! Microsoft](https://github.com/microsoft/windows-rs). Using this crate with
//! other Direct3D11 bindings is not recommended and may result in unexpected behavior.
//! 
//! This crate is in early development. It should work in most cases but may lack
//! certain features or functionalities.
//! 
//! To get started, you can check the [`Renderer`] struct provided by this crate.
//! You can also take a look at the [`egui-demo`](https://github.com/Nekomaru-PKU/egui-directx11/blob/main/examples/egui-demo.rs) example, which demonstrates all you need to do to set up a minimal application
//! with Direct3D11 and `egui`. This example uses `winit` for window management and
//! event handling, while native Win32 APIs should also work well.

mod renderer;
mod texture;
mod utils;
use utils::*;

pub use renderer::{
    Renderer,
    RendererOutput,
    split_output,
};
