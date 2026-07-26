use std::sync::OnceLock;

use eframe::egui;
use jereide_core::constants::{
    TITLE_BAR_FULLSCREEN_SPACE, TITLE_BAR_HEIGHT, TITLE_BAR_POPUP_GAP, TITLE_BAR_TRAFFIC_SPACE,
};
use jereide_core::{AppState, CurrentView};
use jereide_data::data_dir;
use jereide_settings::{ELEVATED_BG, TITLE_BAR_FONT_SIZE};

fn alpha_image() -> egui::Image<'static> {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    let bytes = BYTES.get_or_init(|| {
        data_dir()
            .map(|dir| std::fs::read(dir.join("alpha.png")).unwrap_or_default())
            .unwrap_or_default()
    });
    egui::Image::from_bytes("alpha.png", bytes.clone()).max_height(19.0)
}

fn user_icon_image() -> egui::Image<'static> {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    let bytes = BYTES.get_or_init(|| {
        data_dir()
            .map(|dir| std::fs::read(dir.join("usericon.png")).unwrap_or_default())
            .unwrap_or_default()
    });
    // TODO: I probably need the actual user icon thingy, perhaps from GitHub or Gravatar?
    egui::Image::from_bytes("usericon.png", bytes.clone()).max_height(19.0)
}

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
            #[cfg(target_os = "macos")]
            {
                if is_fullscreen {
                    ui.add_space(TITLE_BAR_FULLSCREEN_SPACE);
                } else {
                    ui.add_space(TITLE_BAR_TRAFFIC_SPACE);
                }
            }
            #[cfg(not(target_os = "macos"))]
            ui.add_space(TITLE_BAR_FULLSCREEN_SPACE);

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
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                state.switch_to_view(CurrentView::Code);
            }
            if ui
                .add(egui::Button::selectable(
                    state.current_view == CurrentView::Compose,
                    ("Compose", alpha_image()),
                ))
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                state.switch_to_view(CurrentView::Compose);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(TITLE_BAR_FULLSCREEN_SPACE);

                let d = 20.0;
                let usericon_clicked = ui.add_sized(
                    egui::vec2(d, d),
                    egui::Button::new(user_icon_image()).corner_radius(d / 2.0),
                );
                egui::Popup::menu(&usericon_clicked)
                    .gap(TITLE_BAR_POPUP_GAP)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
                    .show(|ui| {
                        ui.vertical(|ui| {
                            let _ = ui.button("Username");
                            let _ = ui.button("Star us on GitHub");
                        });
                    });
            });
        });
    });
}
