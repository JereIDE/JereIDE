use jereide_data::data_dir;
use jereide_main_window::JereIDEApp;
use jereide_settings::{WINDOW_HEIGHT, WINDOW_WIDTH};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
            .with_titlebar_shown(false)
            .with_title_shown(false)
            .with_fullsize_content_view(true),
        ..Default::default()
    };

    eframe::run_native(
        "JereIDE",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            if let Some(dir) = data_dir() {
                let mut fonts = eframe::egui::FontDefinitions::default();

                let prop_path = dir.join("iAWriterQuattroV.ttf");
                if prop_path.exists() {
                    if let Ok(bytes) = std::fs::read(&prop_path) {
                        fonts.font_data.insert(
                            "iAWriterQuattroV".into(),
                            std::sync::Arc::new(eframe::egui::FontData::from_owned(bytes)),
                        );
                        fonts
                            .families
                            .get_mut(&eframe::egui::FontFamily::Proportional)
                            .map(|list| list.insert(0, "iAWriterQuattroV".into()));
                    }
                }

                let mono_path = dir.join("SF-Mono-Regular.otf");
                if mono_path.exists() {
                    if let Ok(bytes) = std::fs::read(&mono_path) {
                        fonts.font_data.insert(
                            "SF-Mono-Regular".into(),
                            std::sync::Arc::new(eframe::egui::FontData::from_owned(bytes)),
                        );
                        fonts
                            .families
                            .get_mut(&eframe::egui::FontFamily::Monospace)
                            .map(|list| list.insert(0, "SF-Mono-Regular".into()));
                    }
                }

                cc.egui_ctx.set_fonts(fonts);
            }

            Ok(Box::new(JereIDEApp::new()))
        }),
    )
}
