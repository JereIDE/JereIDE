use eframe::egui;
use jereide_settings::{COMPOSE_BG, COMPOSE_TEXT, COMPOSE_VIEW_FONT_SIZE};

// Renders the whole command view.
pub fn render_compose_view(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, COMPOSE_BG);
    // TODO: Still needs implementation
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Needs implementation",
        egui::FontId::proportional(COMPOSE_VIEW_FONT_SIZE),
        COMPOSE_TEXT,
    );
}
