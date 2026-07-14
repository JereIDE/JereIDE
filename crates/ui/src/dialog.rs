use eframe::egui;
use jereide_core::AppState;
use jereide_settings::{ACCENT, TEXT_DEFAULT, TEXT_MUTED, DIALOG_WIDTH};

pub enum CloseConfirmAction {
    Save(usize),
    Discard(usize),
    Cancel,
}

pub fn render_close_confirm_modal(
    state: &mut AppState,
    ctx: &egui::Context,
) -> Option<CloseConfirmAction> {
    let idx = state.pending_close_index?;
    let file_name = state.tabs[idx].file_name();
    let dim_rect = ctx.viewport_rect();

    let dim_layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("modal_dimmer"));
    let dim_painter = ctx.layer_painter(dim_layer);
    dim_painter.rect_filled(dim_rect, 0.0, egui::Color32::from_black_alpha(120));

    // No clicks outside!
    egui::Area::new(egui::Id::new("modal_dimmer_interact"))
            .order(egui::Order::Foreground)
            .fixed_pos(dim_rect.min)
            .show(ctx, |ui| {
                ui.allocate_rect(dim_rect, egui::Sense::click());
            });

    let response = egui::Window::new("Unsaved Changes")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .max_width(DIALOG_WIDTH)
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.label(format!(
                    "\"{}\" has unsaved changes.\nDo you want to save them before closing?",
                    file_name
                ));
            });

            ui.add_space(10.0);

            let btn_w = ui.available_width();
            let mut result: Option<CloseConfirmAction> = None;

            if ui
                .add_sized(
                    egui::vec2(btn_w, 0.0),
                    egui::Button::new("Save").fill(ACCENT),
                )
                .clicked()
            {
                result = Some(CloseConfirmAction::Save(idx));
            }

            if ui
                .add_sized(egui::vec2(btn_w, 0.0), egui::Button::new("Don't Save"))
                .clicked()
            {
                result = Some(CloseConfirmAction::Discard(idx));
            }

            if ui
                .add_sized(egui::vec2(btn_w, 0.0), egui::Button::new("Cancel"))
                .clicked()
            {
                result = Some(CloseConfirmAction::Cancel);
            }

            result
        });

    match response {
        Some(ir) => ir.inner.flatten(),
        None => None,
    }
}

// ---------------------------------------------------------------------------
// Large file dialogs
// ---------------------------------------------------------------------------

pub enum LargeFileAction {
    OpenAnyway(String),
    Cancel,
}

pub fn render_large_file_blocked(ctx: &egui::Context, size: u64) -> bool {
    let dim_rect = ctx.viewport_rect();
    let dim_layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("modal_dimmer"));
    let dim_painter = ctx.layer_painter(dim_layer);
    dim_painter.rect_filled(dim_rect, 0.0, egui::Color32::from_black_alpha(120));
    // No clicks outside!
    egui::Area::new(egui::Id::new("modal_dimmer_interact"))
            .order(egui::Order::Foreground)
            .fixed_pos(dim_rect.min)
            .show(ctx, |ui| {
                ui.allocate_rect(dim_rect, egui::Sense::click());
            });

    let mut dismissed = false;

    egui::Window::new("File Too Large")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .max_width(DIALOG_WIDTH)
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.colored_label(
                    TEXT_DEFAULT,
                    format!("This file is {:.1} MB.", size as f64 / 1024.0 / 1024.0),
                );
                ui.colored_label(TEXT_MUTED, "Files larger than 200 MB cannot be opened.");
            });

            ui.add_space(10.0);

            let btn_w = ui.available_width();
            if ui
                .add_sized(egui::vec2(btn_w, 0.0), egui::Button::new("OK").fill(ACCENT))
                .clicked()
            {
                dismissed = true;
            }
        });

    dismissed
}

pub fn render_large_file_warning(
    ctx: &egui::Context,
    path: &str,
    size: u64,
) -> Option<LargeFileAction> {
    let dim_rect = ctx.viewport_rect();
    let dim_layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("modal_dimmer"));
    let dim_painter = ctx.layer_painter(dim_layer);
    dim_painter.rect_filled(dim_rect, 0.0, egui::Color32::from_black_alpha(120));
    // No clicks outside!
    egui::Area::new(egui::Id::new("modal_dimmer_interact"))
            .order(egui::Order::Foreground)
            .fixed_pos(dim_rect.min)
            .show(ctx, |ui| {
                ui.allocate_rect(dim_rect, egui::Sense::click());
            });


    let response = egui::Window::new("Large File")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .max_width(DIALOG_WIDTH)
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.colored_label(
                    TEXT_DEFAULT,
                    format!("This file is {:.1} MB.", size as f64 / 1024.0 / 1024.0),
                );
                ui.colored_label(
                    TEXT_MUTED,
                    "Opening very large files may cause performance issues.",
                );
            });

            ui.add_space(10.0);

            let btn_w = ui.available_width();
            let mut result: Option<LargeFileAction> = None;

            if ui
                .add_sized(
                    egui::vec2(btn_w, 0.0),
                    egui::Button::new("Open Anyway").fill(ACCENT),
                )
                .clicked()
            {
                result = Some(LargeFileAction::OpenAnyway(path.to_string()));
            }

            if ui
                .add_sized(egui::vec2(btn_w, 0.0), egui::Button::new("Cancel"))
                .clicked()
            {
                result = Some(LargeFileAction::Cancel);
            }

            result
        });

    match response {
        Some(ir) => ir.inner.flatten(),
        None => None,
    }
}
