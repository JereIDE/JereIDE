use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

use eframe::egui;
use jereide_core::AppState;
use jereide_fs::{DirectoryEntry, list_directory};
use jereide_settings::{text_default, text_muted, text_secondary};

struct CachedListing {
    entries: Vec<DirectoryEntry>,
    modified: Option<SystemTime>,
}

thread_local! {
    static LS_CACHE: RefCell<HashMap<String, CachedListing>> = RefCell::new(HashMap::new());
}

pub fn clear_ls_cache() {
    LS_CACHE.with(|cache| cache.borrow_mut().clear());
}

fn directory_modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

pub fn render_sidebar(state: &mut AppState, ui: &mut egui::Ui) {
    egui::Panel::left("sidebar")
        .resizable(true)
        .default_size(200.0)
        .min_size(80.0)
        .show(ui, |ui| {
            ui.set_min_height(ui.available_height());
            ui.vertical(|ui| {
                ui.add_space(12.0);
                ui.colored_label(text_muted(), "Explorer");
                ui.add_space(4.0);

                match state.current_project_dir.clone() {
                    Some(dir) => {
                        ui.colored_label(text_secondary(), &dir);
                        ui.separator();

                        let entries = LS_CACHE.with(|cache| {
                            let mut cache = cache.borrow_mut();
                            let path = Path::new(&dir);
                            let modified = directory_modified(path);
                            let needs_refresh = match cache.get(&dir) {
                                Some(cached) => cached.modified != modified,
                                None => true,
                            };
                            if needs_refresh {
                                cache.insert(
                                    dir.clone(),
                                    CachedListing {
                                        entries: list_directory(path).unwrap_or_default(),
                                        modified,
                                    },
                                );
                            }
                            cache
                                .get(&dir)
                                .map(|c| c.entries.clone())
                                .unwrap_or_default()
                        });

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for entry in &entries {
                                if entry.is_directory {
                                    ui.colored_label(text_default(), format!("{}/", entry.name));
                                } else {
                                    ui.colored_label(text_secondary(), &entry.name);
                                }
                            }
                        });
                    }
                    None => {
                        ui.add_space(4.0);
                        ui.colored_label(text_muted(), "No project open.");
                    }
                }
            });
        });
}
