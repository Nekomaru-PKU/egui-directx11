use std::{env, ptr};

use egui::*;

use windows::Win32::{
    Foundation::{HMODULE, HWND},
    Graphics::{
        Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0},
        Direct3D11::*,
        Dxgi::{Common::*, *},
    },
};

use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes, WindowId},
};

fn main() {
    AppRunner::<WinD11WrapApp>::run(
        WindowAttributes::default()
            .with_title("egui-directx11")
            .with_inner_size(LogicalSize::new(800, 600)),
    );
}

struct WinD11WrapApp {
    device: ID3D11Device,
    device_context: ID3D11DeviceContext,
    swap_chain: IDXGISwapChain,
    render_target: Option<ID3D11RenderTargetView>,
    egui_renderer: egui_directx11::Renderer,
    egui_winit: egui_winit::State,
    egui_ctx: Context,
    app: EguiApp,
}

trait App: Sized {
    fn new(window: &Window, event_loop_proxy: EventLoopProxy<()>) -> Self;

    fn on_event(&mut self, window: &Window, event: &WindowEvent)
    -> EventResult;

    fn render(&mut self, window: &Window);
}

impl App for WinD11WrapApp {
    fn new(window: &Window, event_loop_proxy: EventLoopProxy<()>) -> Self {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let RawWindowHandle::Win32(window_handle) = window
            .window_handle()
            .expect("Failed to get window handle")
            .as_raw()
        else {
            panic!("Unexpected RawWindowHandle variant");
        };

        let (device, device_context, swap_chain) = {
            let PhysicalSize { width, height } = window.inner_size();
            WinD11WrapApp::create_device_and_swap_chain(
                HWND(window_handle.hwnd.get() as _),
                width,
                height,
                DXGI_FORMAT_R8G8B8A8_UNORM,
            )
        }
        .expect("Failed to create device and swap chain");

        let render_target = Some(
            WinD11WrapApp::create_render_target_for_swap_chain(
                &device,
                &swap_chain,
            )
            .expect("Failed to create render target"),
        );

        let egui_ctx = egui::Context::default();
        egui_ctx.set_request_repaint_callback(move |v| {
            event_loop_proxy.send_event(()).ok();
        });

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
            app: EguiApp::new(&egui_ctx),

            device,
            device_context,
            swap_chain,
            render_target,
            egui_renderer,
            egui_winit,
            egui_ctx,
            // event_loop_proxy,
        }
    }

    fn on_event(
        &mut self,
        window: &Window,
        event: &WindowEvent,
    ) -> EventResult {
        let mut repaint_asap = false;

        match event {
            WindowEvent::CloseRequested => return EventResult::Exit,
            WindowEvent::Resized(new_size) => {
                self.resize(new_size);
                repaint_asap = true;
                // EventResult::RepaintNow(window.id())
            },
            _ => {},
        }

        let egui_response = self.egui_winit.on_window_event(&window, event);

        if egui_response.repaint {
            if repaint_asap {
                return EventResult::RepaintNow(window.id());
            }
            EventResult::RepaintNext(window.id())
        } else {
            EventResult::Wait
        }
    }

    fn render(&mut self, window: &Window) {
        if let Some(render_target) = &self.render_target {
            let egui_input = self.egui_winit.take_egui_input(window);
            let egui_ctx = self.egui_ctx.clone();
            let egui_output = egui_ctx.run(egui_input, |ctx| {
                self.app.ui(&ctx);
            });
            let (renderer_output, platform_output, _) =
                egui_directx11::split_output(egui_output);
            self.egui_winit
                .handle_platform_output(window, platform_output);
            unsafe {
                self.device_context.ClearRenderTargetView(
                    render_target,
                    &[0.0, 0.0, 0.0, 1.0],
                );
            }
            let _ = self.egui_renderer.render(
                &self.device_context,
                render_target,
                &self.egui_ctx,
                renderer_output,
            );
            let _ = unsafe { self.swap_chain.Present(1, DXGI_PRESENT(0)) };
        } else {
            unreachable!()
        }
    }
}

impl WinD11WrapApp {
    fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        if let Err(err) = self.resize_swap_chain_and_render_target(
            new_size.width,
            new_size.height,
            DXGI_FORMAT_R8G8B8A8_UNORM,
        ) {
            panic!("Failed to resize framebuffers: {err:?}");
        }
    }

    fn create_device_and_swap_chain(
        window: HWND,
        frame_width: u32,
        frame_height: u32,
        frame_format: DXGI_FORMAT,
    ) -> windows::core::Result<(
        ID3D11Device,
        ID3D11DeviceContext,
        IDXGISwapChain,
    )> {
        let dxgi_factory: IDXGIFactory = unsafe { CreateDXGIFactory() }?;
        let dxgi_adapter: IDXGIAdapter =
            unsafe { dxgi_factory.EnumAdapters(0) }?;

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
            Windowed: true.into(),
            SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
            Flags: 0,
        };

        let mut swap_chain = None;
        unsafe {
            dxgi_factory.CreateSwapChain(
                &device,
                &swap_chain_desc,
                &mut swap_chain,
            )
        }
        .ok()?;
        let swap_chain = swap_chain.unwrap();

        unsafe {
            dxgi_factory.MakeWindowAssociation(window, DXGI_MWA_NO_ALT_ENTER)
        }?;
        Ok((device, device_context, swap_chain))
    }

    fn create_render_target_for_swap_chain(
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

    fn resize_swap_chain_and_render_target(
        &mut self,
        new_width: u32,
        new_height: u32,
        new_format: DXGI_FORMAT,
    ) -> windows::core::Result<()> {
        self.render_target.take();
        unsafe {
            self.swap_chain.ResizeBuffers(
                2,
                new_width,
                new_height,
                new_format,
                DXGI_SWAP_CHAIN_FLAG(0),
            )
        }
        .unwrap();
        self.render_target.replace(
            Self::create_render_target_for_swap_chain(
                &self.device,
                &self.swap_chain,
            )
            .unwrap(),
        );
        Ok(())
    }
}

struct AppRunner<T: App> {
    window_attributes: WindowAttributes,
    window: Option<Window>,
    win_d11: Option<T>,
    event_loop_proxy: EventLoopProxy<()>,
}

impl<T: App> AppRunner<T> {
    fn run(window_attributes: WindowAttributes) {
        let event_loop = EventLoop::new().unwrap();
        let event_loop_proxy = event_loop.create_proxy();

        let mut runner = Self {
            window_attributes,
            window: None,
            win_d11: None,
            event_loop_proxy,
        };

        event_loop.run_app(&mut runner).unwrap();
    }
}

impl<T: App> ApplicationHandler for AppRunner<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(self.window_attributes.clone())
            .expect("Failed to create window");

        let event_loop_proxy = self.event_loop_proxy.clone();

        self.win_d11 = Some(T::new(&window, event_loop_proxy));
        self.window = Some(window);
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.win_d11.take();
        self.window.take();
    }

    fn user_event(&mut self, _: &ActiveEventLoop, event: ()) {
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
        // println!("{:?}", event);
        if let Some(window) = self.window.as_ref() {
            if window_id == window.id() {
                if let Some(win_d11) = self.win_d11.as_mut() {
                    match win_d11.on_event(window, &event) {
                        EventResult::Wait => {
                            event_loop.set_control_flow(ControlFlow::Wait);
                        },
                        EventResult::RepaintNow(window_id) => {
                            win_d11.render(window);
                        },
                        EventResult::RepaintNext(window_id) => {
                            win_d11.render(window);
                        },
                        EventResult::RepaintAt(window_id, _) => {},
                        EventResult::Save => {},
                        EventResult::CloseRequested => {
                            event_loop.exit();
                        },
                        EventResult::Exit => {
                            event_loop.exit();
                        },
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventResult {
    Wait,

    /// Causes a synchronous repaint inside the event handler. This should only
    /// be used in special situations if the window must be repainted while
    /// handling a specific event. This occurs on Windows when handling resizes.
    ///
    /// `RepaintNow` creates a new frame synchronously, and should therefore
    /// only be used for extremely urgent repaints.
    RepaintNow(WindowId),

    /// Queues a repaint for once the event loop handles its next redraw. Exists
    /// so that multiple input events can be handled in one frame. Does not
    /// cause any delay like `RepaintNow`.
    RepaintNext(WindowId),

    RepaintAt(WindowId, std::time::Instant),

    /// Causes a save of the client state when the persistence feature is enabled.
    Save,

    /// Starts the process of ending eframe execution whilst allowing for proper
    /// clean up of resources.
    ///
    /// # Warning
    /// This event **must** occur before [`Exit`] to correctly exit eframe code.
    /// If in doubt, return this event.
    ///
    /// [`Exit`]: [EventResult::Exit]
    CloseRequested,

    /// The event loop will exit, now.
    /// The correct circumstance to return this event is in response to a winit "Destroyed" event.
    ///
    /// # Warning
    /// The [`CloseRequested`] **must** occur before this event to ensure that winit
    /// is able to remove any open windows. Otherwise the window(s) will remain open
    /// until the program terminates.
    ///
    /// [`CloseRequested`]: EventResult::CloseRequested
    Exit,
}

struct EguiApp {
    egui_ctx: egui::Context,

    egui_demo: egui_demo_lib::DemoWindows,
    egui_color_test: egui_demo_lib::ColorTest,
}

impl EguiApp {
    fn new(egui_ctx: &Context) -> Self {
        Self {
            egui_ctx: egui_ctx.clone(),
            egui_demo: egui_demo_lib::DemoWindows::default(),
            egui_color_test: egui_demo_lib::ColorTest::default(),
        }
    }
}

impl EguiApp {
    fn ui(&mut self, ctx: &egui::Context) {
        let args = env::args().skip(1).collect::<Vec<_>>();
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        match &args[..] {
            [] => self.egui_demo.ui(ctx),
            ["color-test"] => self.color_test(ctx),
            _ => panic!("Unknown arguments: {:?}", args),
        }
    }

    fn color_test(&mut self, ctx: &egui::Context) {
        use egui::Window;

        const WINDOW_WIDTH: f32 = 640.0;

        let screen_rect = ctx.input(|input| input.screen_rect);
        let window_height = screen_rect.height() - 60.0;

        Window::new("Color Test")
            .resizable(false)
            .collapsible(false)
            .pivot(Align2::CENTER_CENTER)
            .fixed_pos(screen_rect.center())
            .default_width(WINDOW_WIDTH)
            .min_width(WINDOW_WIDTH)
            .max_width(WINDOW_WIDTH)
            .default_height(window_height)
            .min_height(window_height)
            .max_height(window_height)
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .show(ui, |ui| self.egui_color_test.ui(ui))
            });
    }
}
