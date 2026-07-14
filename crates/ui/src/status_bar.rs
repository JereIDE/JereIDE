use eframe::egui;
use jereide_core::constants::STATUS_BAR_MARGIN;
use jereide_core::AppState;
use jereide_settings::SURFACE_BG;

pub fn render_status_bar(state: &AppState, ui: &mut egui::Ui) {
    egui::Panel::bottom("status_bar")
        .frame(
            egui::Frame::NONE
                .fill(SURFACE_BG)
                .inner_margin(STATUS_BAR_MARGIN),
        )
        .show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Ready");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if state.tabs.is_empty() {
                        ui.label("--:--");
                    } else {
                        let tab = state.current_tab();
                        ui.label(format!("{}:{}", tab.cursor_line, tab.cursor_col));
                    }
                });
            });
        });
}
