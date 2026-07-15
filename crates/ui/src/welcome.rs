use eframe::egui;
use jereide_settings::{
    COMMAND_VIEW_FONT_SIZE, EDITOR_FONT_SIZE, SURFACE_BG, TEXT_PRIMARY, TEXT_SECONDARY,
};

pub fn render_welcome_view(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, SURFACE_BG);
    ui.painter().text(
        egui::Pos2::new(rect.center().x - 110.0, rect.center().y + 13.0),
        egui::Align2::CENTER_CENTER,
        "[LOGO]",
        egui::FontId::proportional(COMMAND_VIEW_FONT_SIZE),
        TEXT_PRIMARY,
    );
    ui.painter().text(
        egui::Pos2::new(rect.center().x - 70.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        "Welcome back to to JereIDE",
        egui::FontId::proportional(COMMAND_VIEW_FONT_SIZE),
        TEXT_PRIMARY,
    );

    ui.painter().text(
        egui::Pos2::new(rect.center().x - 70.0, rect.center().y + 26.0),
        egui::Align2::LEFT_CENTER,
        "The editor for what's next",
        egui::FontId::proportional(EDITOR_FONT_SIZE),
        TEXT_SECONDARY,
    );
}
