mod common;

use std::sync::Arc;

use windows::Win32::{
    Foundation::HWND,
    Graphics::{
        Direct3D11::*,
        Dxgi::{Common::*, *},
    },
};

use winit::{
    dpi::PhysicalSize,
    event::WindowEvent,
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::{Window, WindowAttributes},
};

pub fn run_app<T: 'static>(
    window_attributes: WindowAttributes,
    state_ctor: fn() -> T,
    state_ui: fn(&mut T, &egui::Context),
) {
    common::run_app::<AppCore<T>>(
        window_attributes,
        move |window| AppCore::new(window, state_ctor, state_ui),
        AppCore::handle_event,
    );
}

struct AppCore<T> {
    window: Arc<Window>,

    device: ID3D11Device,
    device_context: ID3D11DeviceContext,
    swap_chain: IDXGISwapChain,
    render_target: ID3D11RenderTargetView,

    egui_ctx: egui::Context,
    egui_renderer: egui_directx11::Renderer,
    egui_winit: egui_winit::State,

    state: T,
    state_ui: fn(&mut T, &egui::Context),
}

impl<T> AppCore<T> {
    fn new(
        window: &Arc<Window>,
        state_ctor: fn() -> T,
        state_ui: fn(&mut T, &egui::Context),
    ) -> Self {
        let RawWindowHandle::Win32(window_handle) = window
            .window_handle()
            .expect("Failed to get window handle")
            .as_raw()
        else {
            panic!("Unexpected RawWindowHandle variant");
        };

        let PhysicalSize { width, height } = window.inner_size();
        let (device, device_context, swap_chain) =
            common::create_device_and_swap_chain(
                HWND(window_handle.hwnd.get() as _),
                width,
                height,
                DXGI_FORMAT_R8G8B8A8_UNORM,
            )
            .expect("Failed to create device and swap chain");

        let render_target =
            common::create_render_target_for_swap_chain(&device, &swap_chain)
                .expect("Failed to create render target");

        let egui_ctx = egui::Context::default();
        let egui_renderer = egui_directx11::Renderer::new(&device)
            .expect("Failed to create egui renderer");
        let egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            egui_ctx.viewport_id(),
            &window,
            None,
            None,
            None,
        );

        Self {
            window: Arc::clone(window),

            device,
            device_context,
            swap_chain,
            render_target,

            egui_ctx,
            egui_renderer,
            egui_winit,

            state: (state_ctor)(),
            state_ui,
        }
    }

    fn handle_event(&mut self, event: &WindowEvent) {
        let egui_response =
            self.egui_winit.on_window_event(&self.window, event);
        if !egui_response.consumed {
            match event {
                WindowEvent::Resized(new_size) => {
                    self.resize(new_size.width, new_size.height)
                },
                WindowEvent::RedrawRequested => {
                    self.render();
                    self.present();
                },
                _ => (),
            }
        }
    }

    fn render(&mut self) {
        let egui_input = self.egui_winit.take_egui_input(&self.window);
        let egui_output = self.egui_ctx.run(egui_input, |ctx| {
            (self.state_ui)(&mut self.state, ctx);
        });
        let (renderer_output, platform_output, _) =
            egui_directx11::split_output(egui_output);
        self.egui_winit
            .handle_platform_output(&self.window, platform_output);
        unsafe {
            self.device_context.ClearRenderTargetView(
                &self.render_target,
                &[0.0, 0.0, 0.0, 1.0],
            );
        }
        let _ = self.egui_renderer.render(
            &self.device_context,
            &self.render_target,
            &self.egui_ctx,
            renderer_output,
            self.window.scale_factor() as _,
        );
    }

    fn present(&self) {
        unsafe {
            self.swap_chain
                .Present(1, DXGI_PRESENT(0))
                .ok()
                .expect("Failed to present swap chain");
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        common::resize_swap_chain_and_render_target(
            &self.device,
            &self.swap_chain,
            &mut self.render_target,
            width,
            height,
            DXGI_FORMAT_R8G8B8A8_UNORM,
        )
        .expect("Failed to resize swap chain and render target");
    }
}
