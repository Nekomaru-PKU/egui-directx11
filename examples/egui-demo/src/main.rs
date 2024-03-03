use windows::core::Interface;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;

use winit::dpi::PhysicalSize;
use winit::raw_window_handle::HasDisplayHandle;
use winit::raw_window_handle::HasWindowHandle;
use winit::raw_window_handle::RawWindowHandle;

use winit::event::Event;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

use egui_directx11::Renderer;

const WINDOW_WIDTH: f32 = 1600.0;
const WINDOW_HEIGHT: f32 = 900.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("egui-directx11")
        .with_inner_size(PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .with_resizable(false)
        .build(&event_loop)?;
    let hwnd = if let RawWindowHandle::Win32(raw) =
        window.window_handle()?.as_raw() {
        HWND(raw.hwnd.get())
    } else { unreachable!() };

    let inner_size = window.inner_size();
    let (
        device,
        device_context,
        swap_chain,
        _,
        framebuffer_rtv,
    ) = create_device_and_swap_chain(
        hwnd,
        inner_size.width,
        inner_size.height,
        DXGI_FORMAT_R8G8B8A8_UNORM_SRGB)?;

    let egui_ctx = egui::Context::default();
    let mut egui_renderer: Renderer = Renderer::new(&device)?;
    let mut egui_winit = egui_winit::State::new(
        egui_ctx.clone(),
        egui_ctx.viewport_id(),
        &window.display_handle()?,
        None,
        None);
    let mut egui_demo = egui_demo_lib::DemoWindows::default();

    event_loop.run(move |event, control_flow| match event {
        Event::AboutToWait => window.request_redraw(),
        Event::WindowEvent { window_id, event } => {
            if window_id != window.id() { return; }
            if egui_winit.on_window_event(&window, &event).consumed { return; }
            match event {
                WindowEvent::CloseRequested => control_flow.exit(),
                WindowEvent::RedrawRequested => {
                    let input = egui_winit.take_egui_input(&window);
                    egui_ctx.begin_frame(input);

                    egui_demo.ui(&egui_ctx);

                    let mut output = egui_ctx.end_frame();
                    egui_winit.handle_platform_output(
                        &window,
                        std::mem::take(&mut output.platform_output));

                    unsafe {
                        device_context.ClearRenderTargetView(
                            &framebuffer_rtv, 
                            &[0.0, 0.0, 0.0, 1.0]);
                    }
                    let _ = egui_renderer.render(
                            &device_context,
                            &framebuffer_rtv,
                            &egui_ctx,
                            output,
                            window.scale_factor() as _);
                    let _ = unsafe { swap_chain.Present(1, 0) };
                }
                _ => ()
            }
        }, _ => ()
    })?;
    Ok(())
}


pub fn create_device_and_swap_chain(
    window: HWND,
    frame_width: u32,
    frame_height: u32,
    frame_format: DXGI_FORMAT) -> windows::core::Result<(
    ID3D11Device,
    ID3D11DeviceContext,
    IDXGISwapChain,
    ID3D11Texture2D,
    ID3D11RenderTargetView)> {
    let dxgi_factory: IDXGIFactory = unsafe { CreateDXGIFactory() }?;
    let dxgi_adapter: IDXGIAdapter =
        if let Ok(dxgi_factory) = dxgi_factory.cast::<IDXGIFactory6>() {
            unsafe {
                dxgi_factory.EnumAdapterByGpuPreference(
                    0,
                    DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE)
            }?
        } else {
            unsafe { dxgi_factory.EnumAdapters(0) }?
        };

    let mut device_flags = D3D11_CREATE_DEVICE_FLAG(0);
    if cfg!(debug_assertions) { device_flags |= D3D11_CREATE_DEVICE_DEBUG; }

    let mut device = None;
    let mut device_context = None;
    unsafe { 
        D3D11CreateDevice(
            &dxgi_adapter,
            D3D_DRIVER_TYPE_UNKNOWN,
            None,
            device_flags,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut device_context))
    }?;
    let device = device.unwrap();
    let device_context = device_context.unwrap();

    let swap_chain_desc = DXGI_SWAP_CHAIN_DESC {
        BufferDesc: DXGI_MODE_DESC {
            Width : frame_width,
            Height: frame_height,
            Format: frame_format,
            .. DXGI_MODE_DESC::default()
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
        dxgi_factory.CreateSwapChain(
            &device,
            &swap_chain_desc,
            &mut swap_chain)
    }.unwrap();
    let swap_chain = swap_chain.unwrap();

    unsafe {
        dxgi_factory.MakeWindowAssociation(
            window,
            DXGI_MWA_NO_ALT_ENTER)
    }?;

    let framebuffer = unsafe { swap_chain.GetBuffer(0) }.unwrap();
    let mut framebuffer_rtv = None;
    unsafe {
        device.CreateRenderTargetView(
            &framebuffer,
            None,
            Some(&mut framebuffer_rtv))
    }?;
    let framebuffer_rtv = framebuffer_rtv.unwrap();

    Ok((
        device,
        device_context,
        swap_chain,
        framebuffer,
        framebuffer_rtv,
    ))
}
