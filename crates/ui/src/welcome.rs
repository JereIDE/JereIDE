use eframe::egui;
use jereide_settings::{COMMAND_VIEW_FONT_SIZE, SURFACE_BG, TEXT_SECONDARY};

pub fn render_welcome_view(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, SURFACE_BG);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Welcome to JereIDE!",
        egui::FontId::proportional(COMMAND_VIEW_FONT_SIZE),
        TEXT_SECONDARY,
    );
}
