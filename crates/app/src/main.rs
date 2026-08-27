use jereide_data::data_dir;
use jereide_main_window::JereIDEApp;
use jereide_settings::{window_height, window_width};

fn main() -> Result<(), eframe::Error> {
    jereide_logging::init();
    log::info!("JereIDE {} starting up", env!("CARGO_PKG_VERSION"));
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
            log::info!("egui image loaders installed");

            if let Some(dir) = data_dir() {
                log::info!("data dir found: {}", dir.display());
                let mut fonts = eframe::egui::FontDefinitions::default();

                let prop_path = dir.join("IBMPlexSans-Regular.ttf");
                if prop_path.exists() {
                    if let Ok(bytes) = std::fs::read(&prop_path) {
                        log::info!(
                            "loading proportional font from {} ({} bytes)",
                            prop_path.display(),
                            bytes.len()
                        );
                        fonts.font_data.insert(
                            "IBMPlexSans-Regular".into(),
                            std::sync::Arc::new(eframe::egui::FontData::from_owned(bytes)),
                        );
                        fonts
                            .families
                            .get_mut(&eframe::egui::FontFamily::Proportional)
                            .map(|list| list.insert(0, "IBMPlexSans-Regular".into()));
                    } else {
                        log::warn!(
                            "proportional font {} exists but could not be read",
                            prop_path.display()
                        );
                    }
                } else {
                    log::warn!(
                        "proportional font {} not found; using defaults",
                        prop_path.display()
                    );
                }

                let bold_path = dir.join("IBMPlexSans-Bold.ttf");
                if bold_path.exists() {
                    if let Ok(bytes) = std::fs::read(&bold_path) {
                        log::info!(
                            "loading bold font from {} ({} bytes)",
                            bold_path.display(),
                            bytes.len()
                        );
                        fonts.font_data.insert(
                            "IBMPlexSans-Bold".into(),
                            std::sync::Arc::new(eframe::egui::FontData::from_owned(bytes)),
                        );
                        fonts.families.insert(
                            eframe::egui::FontFamily::Name("jereide-bold".into()),
                            vec!["IBMPlexSans-Bold".into(), "NotoEmoji-Regular".into()],
                        );
                    } else {
                        log::warn!(
                            "bold font {} exists but could not be read",
                            bold_path.display()
                        );
                    }
                } else {
                    log::warn!("bold font {} not found", bold_path.display());
                }

                let mono_path = dir.join("SF-Mono-Regular.otf");
                if mono_path.exists() {
                    if let Ok(bytes) = std::fs::read(&mono_path) {
                        log::info!(
                            "loading monospace font from {} ({} bytes)",
                            mono_path.display(),
                            bytes.len()
                        );
                        fonts.font_data.insert(
                            "SF-Mono-Regular".into(),
                            std::sync::Arc::new(eframe::egui::FontData::from_owned(bytes)),
                        );
                        fonts
                            .families
                            .get_mut(&eframe::egui::FontFamily::Monospace)
                            .map(|list| list.insert(0, "SF-Mono-Regular".into()));
                    } else {
                        log::warn!(
                            "monospace font {} exists but could not be read",
                            mono_path.display()
                        );
                    }
                } else {
                    log::warn!("monospace font {} not found", mono_path.display());
                }

                cc.egui_ctx.set_fonts(fonts);
                log::info!("fonts applied");
            } else {
                log::warn!("no data dir found; skipping custom font loading");
            }

            Ok(Box::new(JereIDEApp::new()))
        }),
    )
}
