use eframe::egui;
use jereide_settings::{
    COMPOSE_VIEW_FONT_SIZE, EDITOR_FONT_SIZE, SURFACE_BG, TEXT_MUTED, TEXT_PRIMARY,
    TEXT_SECONDARY,
};

pub fn render_welcome_view(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, SURFACE_BG);
    ui.painter().text(
        egui::Pos2::new(rect.center().x - 110.0, rect.center().y + 13.0),
        egui::Align2::CENTER_CENTER,
        "[LOGO]",
        egui::FontId::proportional(COMPOSE_VIEW_FONT_SIZE),
        TEXT_PRIMARY,
    );
    let font = egui::FontId::proportional(COMPOSE_VIEW_FONT_SIZE);
    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let main = "Welcome back to JereIDE ";
    let full_text = format!("{}{}", main, version);
    let mut job = egui::text::LayoutJob::default();
    job.text = full_text;
    let main_end = main.len();
    job.sections.push(egui::text::LayoutSection {
        leading_space: 0.0,
        byte_range: 0..main_end,
        format: egui::TextFormat::simple(font.clone(), TEXT_PRIMARY),
    });
    job.sections.push(egui::text::LayoutSection {
        leading_space: 0.0,
        byte_range: main_end..job.text.len(),
        format: egui::TextFormat::simple(font, TEXT_SECONDARY),
    });
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let text_pos = egui::pos2(
        rect.center().x - 70.0,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, galley, TEXT_PRIMARY);

    ui.painter().text(
        egui::Pos2::new(rect.center().x - 70.0, rect.center().y + 26.0),
        egui::Align2::LEFT_CENTER,
        "The editor for what's next",
        egui::FontId::proportional(EDITOR_FONT_SIZE),
        TEXT_MUTED,
    );
}
