use eframe::egui;
use jereide_core::AppState;
use jereide_settings::TEXT_MUTED;

pub fn render_sidebar(state: &mut AppState, ui: &mut egui::Ui) {
    egui::Panel::left("sidebar")
        .resizable(true)
        .default_size(200.0)
        .min_size(80.0)
        .show_inside(ui, |ui| {
            ui.set_min_height(ui.available_height());
            ui.vertical_centered(|ui| {
                ui.add_space(12.0);
                ui.colored_label(
                    TEXT_MUTED,
                    format!(
                        "current directory opened: {}",
                        state
                            .current_project_dir
                            .clone()
                            .unwrap_or(String::from("none"))
                    ),
                );
            });
        });
}
