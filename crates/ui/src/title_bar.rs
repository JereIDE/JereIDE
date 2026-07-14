use eframe::egui;
use jereide_core::constants::{
    TITLE_BAR_FULLSCREEN_SPACE, TITLE_BAR_HEIGHT, TITLE_BAR_POPUP_GAP, TITLE_BAR_TRAFFIC_SPACE,
};
use jereide_core::{AppState, CurrentView};
use jereide_settings::{ELEVATED_BG, TITLE_BAR_FONT_SIZE};

pub fn render_title_bar(state: &mut AppState, ui: &mut egui::Ui, is_fullscreen: bool) {
    let available = ui.available_size();
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(available.x, TITLE_BAR_HEIGHT),
        egui::Sense::hover(),
    );
    ui.painter().rect_filled(rect, 0.0, ELEVATED_BG);
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.style_mut().text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::proportional(TITLE_BAR_FONT_SIZE),
        );

        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            if is_fullscreen {
                ui.add_space(TITLE_BAR_FULLSCREEN_SPACE);
            } else {
                ui.add_space(TITLE_BAR_TRAFFIC_SPACE);
            }
            let choose_project_resp = ui.button("Choose Project");
            egui::Popup::menu(&choose_project_resp)
                .gap(TITLE_BAR_POPUP_GAP)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
                .show(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Needs Implementation");
                    });
                });

            if ui
                .selectable_label(state.current_view == CurrentView::Code, "Code")
                .clicked()
            {
                state.switch_to_view(CurrentView::Code);
            }
            if ui
                .selectable_label(state.current_view == CurrentView::Command, "Command")
                .clicked()
            {
                state.switch_to_view(CurrentView::Command);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |_ui| {
                // Reserved for future right-side title bar content.
            });
        });
    });
}
