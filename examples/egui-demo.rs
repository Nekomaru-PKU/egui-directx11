use windows::Win32::{
    Foundation::{
        BOOL, HWND
    },
    Graphics::{
        Dxgi::Common::*,
        Dxgi::*,
        Direct3D::*,
        Direct3D11::*,
    },
};

use winit::{
    dpi::PhysicalSize,
    event::{
        Event,
        WindowEvent,
    },
    event_loop::EventLoop,
    raw_window_handle::{
        HasWindowHandle,
        RawWindowHandle,
    },
    window::WindowBuilder,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("egui-directx11")
        .with_inner_size(PhysicalSize::new(1600, 900))
        .build(&event_loop)?;
    let hwnd = if let RawWindowHandle::Win32(raw) =
        window.window_handle()?.as_raw() {
        HWND(raw.hwnd.get() as _)
    } else {
        panic!("unexpected RawWindowHandle variant");
    };

    let frame_size = window.inner_size();
    let (
        device,
        device_context,
        swap_chain,
    ) = create_device_and_swap_chain(
        hwnd,
        frame_size.width,
        frame_size.height,
        DXGI_FORMAT_R8G8B8A8_UNORM_SRGB)?;
    let mut render_target = Some(
        create_render_target_for_swap_chain(&device, &swap_chain)?);

    let egui_ctx = egui::Context::default();
    let mut egui_renderer = egui_directx11::Renderer::new(&device)?;
    let mut egui_winit = egui_winit::State::new(
        egui_ctx.clone(),
        egui_ctx.viewport_id(),
        &window,
        None,
        None);
    let mut egui_demo = egui_demo_lib::DemoWindows::default();

    event_loop.run(move |event, event_loop| match event {
        Event::AboutToWait => window.request_redraw(),
        Event::WindowEvent { window_id, event } => {
            if window_id != window.id() { return; }
            if egui_winit.on_window_event(&window, &event).consumed { return; }
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(PhysicalSize {
                    width: new_width,
                    height: new_height,
                }) => if let Err(err) = resize_swap_chain_and_render_target(
                    &device,
                    &swap_chain,
                    &mut render_target,
                    new_width,
                    new_height,
                    DXGI_FORMAT_R8G8B8A8_UNORM_SRGB) {
                    panic!("fail to resize framebuffers: {err:?}");
                },
                WindowEvent::RedrawRequested => if let Some(render_target) = &render_target {
                    let egui_input = egui_winit.take_egui_input(&window);
                    let egui_output = egui_ctx.run(egui_input, |ctx| {
                        egui_demo.ui(ctx);
                    });
                    let (
                        renderer_output,
                        platform_output,
                        _,
                    ) = egui_directx11::split_output(egui_output);
                    egui_winit.handle_platform_output(&window, platform_output);

                    unsafe {
                        device_context.ClearRenderTargetView(
                            render_target, 
                            &[0.0, 0.0, 0.0, 1.0]);
                    }
                    let _ = egui_renderer.render(
                        &device_context,
                        render_target,
                        &egui_ctx,
                        renderer_output,
                        window.scale_factor() as _);
                    let _ = unsafe { swap_chain.Present(1, DXGI_PRESENT(0)) };
                } else { unreachable!() }, _ => ()
            }
        }, _ => ()
    })?;
    Ok(())
}

fn resize_swap_chain_and_render_target(
    device: &ID3D11Device,
    swap_chain: &IDXGISwapChain,
    render_target: &mut Option<ID3D11RenderTargetView>,
    new_width: u32,
    new_height: u32,
    new_format: DXGI_FORMAT,
)-> windows::core::Result<()> {
    render_target.take();
    unsafe {
        swap_chain.ResizeBuffers(
            2,
            new_width,
            new_height,
            new_format,
            DXGI_SWAP_CHAIN_FLAG(0))
    }?;
    render_target.replace(
        create_render_target_for_swap_chain(device, swap_chain)?);
    Ok(())
}

fn create_device_and_swap_chain(
    window: HWND,
    frame_width: u32,
    frame_height: u32,
    frame_format: DXGI_FORMAT,
)-> windows::core::Result<(
    ID3D11Device,
    ID3D11DeviceContext,
    IDXGISwapChain)> {
    let dxgi_factory: IDXGIFactory = unsafe { CreateDXGIFactory() }?;
    let dxgi_adapter: IDXGIAdapter = unsafe { dxgi_factory.EnumAdapters(0) }?;

    let mut device = None;
    let mut device_context = None;
    unsafe { 
        D3D11CreateDevice(
            &dxgi_adapter,
            D3D_DRIVER_TYPE_UNKNOWN,
            None,
            if cfg!(debug_assertions) {
                D3D11_CREATE_DEVICE_DEBUG
            } else {
                D3D11_CREATE_DEVICE_FLAG(0)
            },
            Some(&[D3D_FEATURE_LEVEL_11_0]),
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
    }.ok()?;
    let swap_chain = swap_chain.unwrap();

    unsafe {
        dxgi_factory.MakeWindowAssociation(
            window,
            DXGI_MWA_NO_ALT_ENTER)
    }?;
    Ok((device, device_context, swap_chain))
}

fn create_render_target_for_swap_chain(
    device: &ID3D11Device,
    swap_chain: &IDXGISwapChain,
)-> windows::core::Result<ID3D11RenderTargetView> {
    let swap_chain_texture = unsafe {
        swap_chain.GetBuffer::<ID3D11Texture2D>(0)
    }?;
    let mut render_target = None;
    unsafe {
        device.CreateRenderTargetView(
            &swap_chain_texture,
            None,
            Some(&mut render_target))
    }?;
    Ok(render_target.unwrap())
}
