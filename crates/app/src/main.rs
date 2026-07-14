use jereide_main_window::JereIDEApp;
use jereide_settings::{WINDOW_HEIGHT, WINDOW_WIDTH};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
            .with_titlebar_shown(false)
            .with_title_shown(false)
            .with_fullsize_content_view(true),
        ..Default::default()
    };

    eframe::run_native(
        "jereide",
        options,
        Box::new(|_cc| Ok(Box::new(JereIDEApp::new()))),
    )
}
