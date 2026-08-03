use std::cell::RefCell;
use std::collections::HashMap;

use eframe::egui;
use jereide_backend::{list_directory, DirectoryEntry};
use jereide_core::AppState;
use jereide_settings::{TEXT_DEFAULT, TEXT_MUTED, TEXT_SECONDARY};

thread_local! {
    static LS_CACHE: RefCell<HashMap<String, Vec<DirectoryEntry>>> = RefCell::new(HashMap::new());
}

pub fn render_sidebar(state: &mut AppState, ui: &mut egui::Ui) {
    egui::Panel::left("sidebar")
        .resizable(true)
        .default_size(200.0)
        .min_size(80.0)
        .show_inside(ui, |ui| {
            ui.set_min_height(ui.available_height());
            ui.vertical(|ui| {
                ui.add_space(12.0);
                ui.colored_label(TEXT_MUTED, "Explorer");
                ui.add_space(4.0);

                match state.current_project_dir.clone() {
                    Some(dir) => {
                        ui.colored_label(TEXT_SECONDARY, &dir);
                        ui.separator();

                        let entries = LS_CACHE.with(|cache| {
                            let mut cache = cache.borrow_mut();
                            cache
                                .entry(dir.clone())
                                .or_insert_with(|| {
                                    list_directory(std::path::Path::new(&dir)).unwrap_or_default()
                                })
                                .clone()
                        });

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for entry in &entries {
                                if entry.is_directory {
                                    ui.colored_label(TEXT_DEFAULT, format!("{}/", entry.name));
                                } else {
                                    ui.colored_label(TEXT_SECONDARY, &entry.name);
                                }
                            }
                        });
                    }
                    None => {
                        ui.add_space(4.0);
                        ui.colored_label(TEXT_MUTED, "No project open.");
                    }
                }
            });
        });
}
