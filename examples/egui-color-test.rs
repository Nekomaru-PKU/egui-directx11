mod core;

fn main() {
    core::run_app(
        winit::window::WindowAttributes::default()
            .with_title("egui-directx11")
            .with_inner_size(winit::dpi::PhysicalSize::new(1920, 1080)),
        egui_demo_lib::ColorTest::default,
        |state, ctx| {
            use egui::*;
            let screen_rect = ctx.input(|input| input.screen_rect);
            let window_height = screen_rect.height() - 60.0;

            Window::new("Color Test")
                .pivot(Align2::CENTER_CENTER)
                .fixed_pos(screen_rect.center())
                .default_height(window_height)
                .min_height(window_height)
                .max_height(window_height)
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    ScrollArea::vertical().show(ui, |ui| state.ui(ui))
                });
        },
    );
}
