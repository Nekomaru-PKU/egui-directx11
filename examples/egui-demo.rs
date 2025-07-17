mod core;

fn main() {
    core::run_app(
        winit::window::WindowAttributes::default()
            .with_title("egui-directx11")
            .with_inner_size(winit::dpi::PhysicalSize::new(1920, 1080)),
        egui_demo_lib::DemoWindows::default,
        egui_demo_lib::DemoWindows::ui,
    );
}
