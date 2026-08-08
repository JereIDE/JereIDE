use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::time::SystemTime;

use eframe::egui;
use egui::AtomExt;
use jereide_core::AppState;
use jereide_fs::{DirectoryEntry, list_directory};
use jereide_settings::{
    bold_folders, destructive, surface_bg, text_default, text_muted, text_secondary,
};

const INDENT: f32 = 16.0;
const MIN_SIDEBAR_WIDTH: f32 = 150.0;
const MAX_SIDEBAR_WIDTH: f32 = 800.0;
const RESIZE_GRAB: f32 = 6.0;

struct CachedListing {
    entries: Option<Vec<DirectoryEntry>>,
    error: Option<String>,
    modified: Option<SystemTime>,
}

thread_local! {
    static LS_CACHE: RefCell<HashMap<String, CachedListing>> = RefCell::new(HashMap::new());
    static EXPANDED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

pub fn clear_ls_cache() {
    LS_CACHE.with(|cache| cache.borrow_mut().clear());
    EXPANDED.with(|expanded| expanded.borrow_mut().clear());
}

fn directory_modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn cached_entries(dir: &str) -> Result<Vec<DirectoryEntry>, std::io::Error> {
    LS_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let path = Path::new(dir);
        let modified = directory_modified(path);
        let needs_refresh = match cache.get(dir) {
            Some(cached) => cached.modified != modified,
            None => true,
        };
        if needs_refresh {
            match list_directory(path) {
                Ok(entries) => {
                    cache.insert(
                        dir.to_string(),
                        CachedListing {
                            entries: Some(entries),
                            error: None,
                            modified,
                        },
                    );
                }
                Err(err) => {
                    cache.insert(
                        dir.to_string(),
                        CachedListing {
                            entries: None,
                            error: Some(err.to_string()),
                            modified,
                        },
                    );
                }
            }
        }
        let cached = cache.get(dir).unwrap();
        match cached.entries.as_ref() {
            Some(entries) => Ok(entries.clone()),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                cached.error.clone().unwrap_or_default(),
            )),
        }
    })
}

fn render_entries(state: &mut AppState, ui: &mut egui::Ui, dir: &Path, depth: usize) {
    let entries = match cached_entries(&dir.display().to_string()) {
        Ok(entries) => entries,
        Err(err) => {
            let indent = depth as f32 * INDENT;
            let spacer = egui::Atom::default().atom_size(egui::vec2(indent, 0.0));
            let full_width = ui.available_width();
            let button = egui::Button::new((
                spacer,
                egui::RichText::new(format!("⚠ {}", err))
                    .color(destructive())
                    .italics(),
            ))
            .corner_radius(egui::CornerRadius::ZERO)
            .frame_when_inactive(false)
            .min_size(egui::vec2(full_width, 0.0));
            ui.add(button);
            return;
        }
    };
    for entry in &entries {
        let full_path = dir.join(&entry.name);
        let full_path_str = full_path.display().to_string();
        let indent = depth as f32 * INDENT;
        let spacer = egui::Atom::default().atom_size(egui::vec2(indent, 0.0));

        if entry.is_symlink {
            let target_is_dir = std::fs::metadata(&full_path)
                .map(|m| m.is_dir())
                .unwrap_or(false);
            let full_width = ui.available_width();
            let label = format!("📎 {}", entry.name);
            let color = if target_is_dir {
                text_default()
            } else {
                text_secondary()
            };
            let text = egui::RichText::new(label).color(color);
            let text = if target_is_dir && bold_folders() {
                let base = egui::TextStyle::Button.resolve(ui.style());
                text.font(egui::FontId::new(
                    base.size,
                    egui::FontFamily::Name("jereide-bold".into()),
                ))
            } else {
                text
            };
            let button = egui::Button::new((spacer, text))
                .corner_radius(egui::CornerRadius::ZERO)
                .frame_when_inactive(false)
                .min_size(egui::vec2(full_width, 0.0));
            if ui.add(button).clicked() && !target_is_dir {
                state.pending_open_file = Some(full_path_str);
            }
        } else if entry.is_directory {
            let expanded = EXPANDED.with(|set| set.borrow().contains(&full_path_str));
            let label = format!("{} {}", if expanded { "📂" } else { "📁" }, entry.name);
            let text = egui::RichText::new(label).color(text_default());
            let text = if bold_folders() {
                let base = egui::TextStyle::Button.resolve(ui.style());
                text.font(egui::FontId::new(
                    base.size,
                    egui::FontFamily::Name("jereide-bold".into()),
                ))
            } else {
                text
            };
            let full_width = ui.available_width();
            let button = egui::Button::new((spacer, text))
                .corner_radius(egui::CornerRadius::ZERO)
                .frame_when_inactive(false)
                .min_size(egui::vec2(full_width, 0.0));
            if ui.add(button).clicked() {
                EXPANDED.with(|set| {
                    let mut set = set.borrow_mut();
                    if expanded {
                        set.remove(&full_path_str);
                    } else {
                        set.insert(full_path_str);
                    }
                });
            }
            if expanded {
                render_entries(state, ui, &full_path, depth + 1);
            }
        } else {
            let full_width = ui.available_width();
            let label = format!("📄 {}", entry.name);
            let button =
                egui::Button::new((spacer, egui::RichText::new(label).color(text_secondary())))
                    .corner_radius(egui::CornerRadius::ZERO)
                    .frame_when_inactive(false)
                    .min_size(egui::vec2(full_width, 0.0));
            if ui.add(button).clicked() {
                state.pending_open_file = Some(full_path_str);
            }
        }
    }
}

pub fn render_sidebar(state: &mut AppState, ui: &mut egui::Ui) {
    let panel_frame = egui::Frame::side_top_panel(ui.style())
        .fill(surface_bg())
        .inner_margin(egui::Margin {
            left: 0,
            right: 0,
            top: 8,
            bottom: 4,
        });

    state.sidebar_width = state
        .sidebar_width
        .clamp(MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH);

    egui::Panel::left("sidebar")
        .exact_size(state.sidebar_width)
        .resizable(false)
        .frame(panel_frame)
        .show(ui, |ui| {
            ui.set_min_height(ui.available_height());

            match state.current_project_dir.clone() {
                Some(dir) => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        render_entries(state, ui, Path::new(&dir), 0);
                    });
                }
                None => {
                    ui.with_layout(
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            ui.colored_label(text_muted(), "No project open.");
                        },
                    );
                }
            }

            let panel_rect = ui.max_rect();
            let handle_rect = egui::Rect::from_min_max(
                egui::pos2(panel_rect.right() - RESIZE_GRAB, panel_rect.top()),
                egui::pos2(panel_rect.right(), panel_rect.bottom()),
            );
            let handle = ui.interact(
                handle_rect,
                egui::Id::new("sidebar_resize_handle"),
                egui::Sense::drag(),
            );
            if handle.dragged() {
                state.sidebar_width = (state.sidebar_width + handle.drag_delta().x)
                    .clamp(MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH);
            }
            if handle.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
        });
}
