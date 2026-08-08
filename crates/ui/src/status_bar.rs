use eframe::egui;
use jereide_core::constants::{STATUS_BAR_MARGIN, TAB_BORDER_WIDTH};
use jereide_core::{AppState, CurrentView};
use jereide_settings::{border, compose_bg, surface_bg, text_secondary};

pub fn render_status_bar(state: &AppState, ui: &mut egui::Ui) -> bool {
    let in_compose = state.current_view == CurrentView::Compose;
    let bg = if in_compose {
        compose_bg()
    } else {
        surface_bg()
    };

    let mut go_to_line_clicked = false;

    let status_bar = egui::Panel::bottom("status_bar")
        .frame(egui::Frame::NONE.fill(bg).inner_margin(STATUS_BAR_MARGIN))
        .show_separator_line(false)
        .show(ui, |ui| {
            if in_compose {
                return;
            }
            ui.horizontal(|ui| {
                ui.colored_label(text_secondary(), format!("v{}", env!("CARGO_PKG_VERSION")));
                if !state.tabs.is_empty() {
                    let tab = state.current_tab();
                    if tab.file_path.is_some() {
                        let lang = jereide_data::lookup_language_by_path(tab.file_path.as_deref());
                        let sep = if lang.is_some() { " · " } else { "" };
                        ui.colored_label(
                            text_secondary(),
                            format!(
                                "{}{}{}",
                                lang.as_ref().map(|l| l.name.as_str()).unwrap_or(""),
                                sep,
                                tab.file_name()
                            ),
                        );
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !state.tabs.is_empty() {
                        let tab = state.current_tab();
                        let resp = ui
                            .add(
                                egui::Label::new(format!("{}:{}", tab.cursor_line, tab.cursor_col))
                                    .sense(egui::Sense::click()),
                            )
                            .on_hover_text("Go to Line")
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if resp.clicked() {
                            go_to_line_clicked = true;
                        }
                    }
                });
            });
        });

    let panel_rect = status_bar.response.rect;
    ui.painter().hline(
        egui::Rangef::new(panel_rect.left(), panel_rect.right()),
        panel_rect.top() + 0.5,
        egui::Stroke::new(TAB_BORDER_WIDTH, border()),
    );

    go_to_line_clicked
}
