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

    let cx = rect.center().x;
    let cy = rect.center().y;
    let icon_size = 48.0;

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

    let sub_text = "The ready-to-use editor that nobody ever uses";
    let sub_galley = ui.fonts_mut(|f| {
        f.layout_job(egui::text::LayoutJob::simple(
            sub_text.to_owned(),
            egui::FontId::proportional(editor_font_size()),
            text_muted(),
            f32::INFINITY,
        ))
    });

    let icon_cx = cx - 110.0;
    let icon_cy = cy + 13.0;
    let text_x = cx - 70.0;
    let text_y = cy - galley.size().y / 2.0;
    let sub_x = cx - 70.0;
    let sub_cy = cy + 26.0;

    let mut min_x = icon_cx - icon_size / 2.0;
    let mut max_x = icon_cx + icon_size / 2.0;
    min_x = min_x.min(text_x).min(sub_x);
    max_x = max_x
        .max(text_x + galley.size().x)
        .max(sub_x + sub_galley.size().x);
    let mut min_y = icon_cy - icon_size / 2.0;
    let mut max_y = icon_cy + icon_size / 2.0;
    min_y = min_y.min(text_y).min(sub_cy - sub_galley.size().y / 2.0);
    max_y = max_y
        .max(text_y + galley.size().y)
        .max(sub_cy + sub_galley.size().y / 2.0);

    let dx = cx - (min_x + max_x) / 2.0;
    let dy = cy - (min_y + max_y) / 2.0;

    ui.put(
        egui::Rect::from_center_size(
            egui::pos2(icon_cx + dx, icon_cy + dy),
            egui::vec2(icon_size, icon_size),
        ),
        app_icon(),
    );

    ui.painter().galley(
        egui::pos2(text_x + dx, text_y + dy),
        galley,
        text_primary(),
    );

    ui.painter().text(
        egui::pos2(sub_x + dx, sub_cy + dy),
        egui::Align2::LEFT_CENTER,
        sub_text,
        egui::FontId::proportional(editor_font_size()),
        text_muted(),
    );
}
