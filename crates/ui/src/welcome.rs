use std::sync::OnceLock;

use eframe::egui;
use jereide_data::data_dir;
use jereide_settings::{
    compose_view_font_size, editor_font_size, surface_bg, text_muted, text_primary, text_secondary,
};

fn app_icon() -> egui::Image<'static> {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    let bytes = BYTES.get_or_init(|| {
        data_dir()
            .map(|dir| std::fs::read(dir.join("AppIcon.png")).unwrap_or_default())
            .unwrap_or_default()
    });
    egui::Image::from_bytes("AppIcon.png", bytes.clone()).max_height(48.0)
}

pub fn render_welcome_view(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, surface_bg());
    ui.put(
        egui::Rect::from_center_size(
            egui::pos2(rect.center().x - 110.0, rect.center().y + 13.0),
            egui::vec2(48.0, 48.0),
        ),
        app_icon(),
    );
    let font = egui::FontId::proportional(compose_view_font_size());
    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let main = "Welcome back to JereIDE ";
    let full_text = format!("{}{}", main, version);
    let mut job = egui::text::LayoutJob::default();
    job.text = full_text;
    let main_end = main.len();
    job.sections.push(egui::text::LayoutSection {
        leading_space: 0.0,
        byte_range: egui::text::ByteIndex(0)..egui::text::ByteIndex(main_end),
        format: egui::TextFormat::simple(font.clone(), text_primary()),
    });
    job.sections.push(egui::text::LayoutSection {
        leading_space: 0.0,
        byte_range: egui::text::ByteIndex(main_end)..egui::text::ByteIndex(job.text.len()),
        format: egui::TextFormat::simple(font, text_secondary()),
    });
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let text_pos = egui::pos2(
        rect.center().x - 70.0,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, galley, text_primary());

    ui.painter().text(
        egui::Pos2::new(rect.center().x - 70.0, rect.center().y + 26.0),
        egui::Align2::LEFT_CENTER,
        "The ready-to-use editor that nobody ever uses",
        egui::FontId::proportional(editor_font_size()),
        text_muted(),
    );
}
