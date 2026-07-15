use eframe::egui;
use jereide_core::constants::STATUS_BAR_MARGIN;
use jereide_core::{AppState, CurrentView};
use jereide_settings::{COMMAND_BG, SURFACE_BG, TEXT_SECONDARY};

fn language_from_path(path: Option<&str>) -> &'static str {
    let ext = path
        .and_then(|p| std::path::Path::new(p).extension())
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "rs" => "Rust",
        "py" => "Python",
        "js" | "jsx" => "JavaScript",
        "ts" | "tsx" => "TypeScript",
        "json" => "JSON",
        "html" => "HTML",
        "css" => "CSS",
        "md" => "Markdown",
        "toml" => "TOML",
        "yaml" | "yml" => "YAML",
        "sh" | "zsh" => "Shell",
        "java" => "Java",
        "c" | "h" => "C",
        "cpp" | "cxx" | "hpp" | "hxx" => "C++",
        "go" => "Go",
        "rb" => "Ruby",
        "sql" => "SQL",
        _ => "",
    }
}

pub fn render_status_bar(state: &AppState, ui: &mut egui::Ui) {
    let in_command = state.current_view == CurrentView::Command;
    let bg = if in_command { COMMAND_BG } else { SURFACE_BG };

    egui::Panel::bottom("status_bar")
        .frame(egui::Frame::NONE.fill(bg).inner_margin(STATUS_BAR_MARGIN))
        .show_inside(ui, |ui| {
            if in_command {
                return;
            }
            ui.horizontal(|ui| {
                ui.label(format!("JereIDE v{}", env!("CARGO_PKG_VERSION")));
                if !state.tabs.is_empty() {
                    let tab = state.current_tab();
                    if tab.file_path.is_some() {
                        let lang = language_from_path(tab.file_path.as_deref());
                        let sep = if lang.is_empty() { "" } else { " • " };
                        ui.colored_label(
                            TEXT_SECONDARY,
                            format!("{}{}{}", lang, sep, tab.file_name()),
                        );
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if state.tabs.is_empty() {
                        ui.label("--:--");
                    } else {
                        let tab = state.current_tab();
                        ui.label(format!("{}:{}", tab.cursor_line, tab.cursor_col));
                    }
                });
            });
        });
}
