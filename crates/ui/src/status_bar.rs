use eframe::egui;
use jereide_core::constants::STATUS_BAR_MARGIN;
use jereide_core::{AppState, CurrentView};
use jereide_settings::{COMMAND_BG, SURFACE_BG};

pub fn render_status_bar(state: &AppState, ui: &mut egui::Ui) {
    let in_command = state.current_view == CurrentView::Command;
    let bg = if in_command { COMMAND_BG } else { SURFACE_BG };

    egui::Panel::bottom("status_bar")
        .frame(
            egui::Frame::NONE
                .fill(bg)
                .inner_margin(STATUS_BAR_MARGIN),
        )
        .show_inside(ui, |ui| {
            if in_command {
                return;
            }
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
