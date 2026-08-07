use jereide_data::data_dir;
use jereide_main_window::JereIDEApp;
use jereide_settings::{window_height, window_width};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([window_width(), window_height()])
            .with_min_inner_size([360.0, 240.0])
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
                            std::sync::Arc::new(eframe::egui::FontData::from_owned(
                                bytes.clone(),
                            )),
                        );
                        fonts
                            .families
                            .get_mut(&eframe::egui::FontFamily::Proportional)
                            .map(|list| list.insert(0, "iAWriterQuattroV".into()));

                        let bold_tweak = eframe::egui::FontTweak {
                            coords: eframe::egui::epaint::text::VariationCoords::new([
                                (b"wght", 700.0),
                            ]),
                            ..Default::default()
                        };
                        fonts.font_data.insert(
                            "iAWriterQuattroV-Bold".into(),
                            std::sync::Arc::new(
                                eframe::egui::FontData::from_owned(bytes).tweak(bold_tweak),
                            ),
                        );
                        fonts.families.insert(
                            eframe::egui::FontFamily::Name("jereide-bold".into()),
                            vec![
                                "iAWriterQuattroV-Bold".into(),
                                "NotoEmoji-Regular".into(),
                            ],
                        );
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
