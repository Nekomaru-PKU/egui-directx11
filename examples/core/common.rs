use std::mem;
use std::ptr;
use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

#[allow(clippy::type_complexity)]
struct AppRunner<T> {
    window_attributes: WindowAttributes,
    window: Option<Arc<Window>>,
    app_ctor: Box<dyn Fn(&Arc<Window>) -> T>,
    app_event_handler: Box<dyn Fn(&mut T, &WindowEvent)>,
    app_state: Option<T>,
}

impl<T> AppRunner<T> {
    fn new(
        window_attributes: WindowAttributes,
        app_ctor: impl Fn(&Arc<Window>) -> T + 'static,
        app_event_handler: impl Fn(&mut T, &WindowEvent) + 'static,
    ) -> Self {
        Self {
            window_attributes,
            window: None,
            app_ctor: Box::new(app_ctor),
            app_event_handler: Box::new(app_event_handler),
            app_state: None,
        }
    }
}

impl<T> ApplicationHandler for AppRunner<T> {
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.app_state = None;
        self.window = None;
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(self.window_attributes.clone())
                .expect("Failed to create window"),
        );
        self.app_state = Some((self.app_ctor)(&window));
        self.window = Some(window);
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(window) = self.window.as_ref()
            && window_id == window.id()
        {
            if event == WindowEvent::CloseRequested {
                event_loop.exit()
            } else if let Some(app_state) = self.app_state.as_mut() {
                (self.app_event_handler)(app_state, &event);
            }
        }
    }
}

pub fn run_app<T>(
    window_attributes: WindowAttributes,
    app_ctor: impl Fn(&Arc<Window>) -> T + 'static,
    app_event_handler: impl Fn(&mut T, &WindowEvent) + 'static,
) {
    let mut app_runner =
        AppRunner::new(window_attributes, app_ctor, app_event_handler);
    EventLoop::new()
        .expect("Failed to create event loop")
        .run_app(&mut app_runner)
        .expect("Failed to run event loop");
}

use windows::Win32::{
    Foundation::{HMODULE, HWND},
    Graphics::{
        Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0},
        Direct3D11::*,
        Dxgi::{Common::*, *},
    },
};
use windows::core::BOOL;

pub fn create_device_and_swap_chain(
    window: HWND,
    frame_width: u32,
    frame_height: u32,
    frame_format: DXGI_FORMAT,
) -> windows::core::Result<(ID3D11Device, ID3D11DeviceContext, IDXGISwapChain)>
{
    let dxgi_factory: IDXGIFactory = unsafe { CreateDXGIFactory() }?;
    let dxgi_adapter: IDXGIAdapter = unsafe { dxgi_factory.EnumAdapters(0) }?;

    let mut device = None;
    let mut device_context = None;
    unsafe {
        D3D11CreateDevice(
            &dxgi_adapter,
            D3D_DRIVER_TYPE_UNKNOWN,
            HMODULE(ptr::null_mut()),
            if cfg!(debug_assertions) {
                D3D11_CREATE_DEVICE_DEBUG
            } else {
                D3D11_CREATE_DEVICE_FLAG(0)
            },
            Some(&[D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut device_context),
        )
    }?;
    let device = device.unwrap();
    let device_context = device_context.unwrap();

    let swap_chain_desc = DXGI_SWAP_CHAIN_DESC {
        BufferDesc: DXGI_MODE_DESC {
            Width: frame_width,
            Height: frame_height,
            Format: frame_format,
            ..DXGI_MODE_DESC::default()
        },
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        OutputWindow: window,
        Windowed: BOOL(1),
        SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
        Flags: 0,
    };

    let mut swap_chain = None;
    unsafe {
        dxgi_factory.CreateSwapChain(&device, &swap_chain_desc, &mut swap_chain)
    }
    .ok()?;
    let swap_chain = swap_chain.unwrap();

    unsafe {
        dxgi_factory.MakeWindowAssociation(window, DXGI_MWA_NO_ALT_ENTER)
    }?;
    Ok((device, device_context, swap_chain))
}

pub fn create_render_target_for_swap_chain(
    device: &ID3D11Device,
    swap_chain: &IDXGISwapChain,
) -> windows::core::Result<ID3D11RenderTargetView> {
    let swap_chain_texture =
        unsafe { swap_chain.GetBuffer::<ID3D11Texture2D>(0) }?;
    let mut render_target = None;
    unsafe {
        device.CreateRenderTargetView(
            &swap_chain_texture,
            None,
            Some(&mut render_target),
        )
    }?;
    Ok(render_target.unwrap())
}

pub fn resize_swap_chain_and_render_target(
    device: &ID3D11Device,
    swap_chain: &IDXGISwapChain,
    render_target: &mut ID3D11RenderTargetView,
    new_width: u32,
    new_height: u32,
    new_format: DXGI_FORMAT,
) -> windows::core::Result<()> {
    mem::drop(unsafe { ptr::from_mut(render_target).read() });
    unsafe {
        swap_chain.ResizeBuffers(
            2,
            new_width,
            new_height,
            new_format,
            DXGI_SWAP_CHAIN_FLAG(0),
        )
    }?;
    let new_render_target =
        create_render_target_for_swap_chain(device, swap_chain)?;
    let old_render_target = mem::replace(render_target, new_render_target);
    mem::forget(old_render_target);
    Ok(())
}
