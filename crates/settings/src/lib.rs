use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use eframe::egui;
use eframe::egui::Color32;
use serde::{Serialize, Serializer};

#[derive(Clone, Copy)]
struct HexColor(Color32);

impl HexColor {
    fn parse(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('#');
        let bytes = |range: std::ops::Range<usize>| u8::from_str_radix(&s[range], 16).ok();
        match s.len() {
            6 => Some(HexColor(Color32::from_rgb(
                bytes(0..2)?,
                bytes(2..4)?,
                bytes(4..6)?,
            ))),
            8 => Some(HexColor(Color32::from_rgba_unmultiplied(
                bytes(0..2)?,
                bytes(2..4)?,
                bytes(4..6)?,
                bytes(6..8)?,
            ))),
            _ => None,
        }
    }
}

impl Serialize for HexColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let c = self.0;
        serializer.serialize_str(&format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            c.r(),
            c.g(),
            c.b(),
            c.a()
        ))
    }
}

#[derive(Serialize, Clone)]
struct Settings {
    surface_bg: HexColor,
    elevated_bg: HexColor,
    hover_bg: HexColor,
    compose_bg: HexColor,

    text_default: HexColor,
    text_primary: HexColor,
    text_secondary: HexColor,
    text_muted: HexColor,
    current_line_highlighting: HexColor,
    compose_text: HexColor,

    border: HexColor,

    syntax_keyword: HexColor,
    syntax_keyword2: HexColor,
    syntax_string: HexColor,
    syntax_comment: HexColor,
    syntax_number: HexColor,
    syntax_operator: HexColor,
    syntax_function: HexColor,
    syntax_literal: HexColor,
    syntax_heading: HexColor,
    syntax_code: HexColor,
    syntax_emphasis: HexColor,
    syntax_link: HexColor,

    accent: HexColor,
    destructive: HexColor,

    bracket_match: HexColor,

    find_highlight: HexColor,
    find_highlight_current: HexColor,

    bold_folders: bool,

    title_bar_font_size: f32,
    tab_font_size: f32,
    editor_font_size: f32,
    compose_view_font_size: f32,

    window_width: f32,
    window_height: f32,

    dialog_width: f32,

    log_max_file_size: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            surface_bg: HexColor(Color32::WHITE),
            elevated_bg: HexColor(Color32::from_rgb(245, 245, 245)),
            hover_bg: HexColor(Color32::from_rgb(230, 230, 230)),
            compose_bg: HexColor(Color32::from_gray(230)),

            text_default: HexColor(Color32::BLACK),
            text_primary: HexColor(Color32::from_rgb(30, 30, 30)),
            text_secondary: HexColor(Color32::from_rgb(90, 90, 90)),
            text_muted: HexColor(Color32::from_rgb(175, 175, 175)),
            current_line_highlighting: HexColor(Color32::from_rgb(48, 48, 48)),
            compose_text: HexColor(Color32::from_gray(20)),

            border: HexColor(Color32::from_rgb(200, 200, 200)),

            syntax_keyword: HexColor(Color32::from_rgb(175, 0, 0)),
            syntax_keyword2: HexColor(Color32::from_rgb(0, 128, 128)),
            syntax_string: HexColor(Color32::from_rgb(0, 128, 0)),
            syntax_comment: HexColor(Color32::from_rgb(128, 128, 128)),
            syntax_number: HexColor(Color32::from_rgb(128, 0, 128)),
            syntax_operator: HexColor(Color32::from_rgb(100, 100, 100)),
            syntax_function: HexColor(Color32::from_rgb(0, 100, 200)),
            syntax_literal: HexColor(Color32::from_rgb(200, 100, 0)),
            syntax_heading: HexColor(Color32::from_rgb(0, 90, 180)),
            syntax_code: HexColor(Color32::from_rgb(90, 120, 90)),
            syntax_emphasis: HexColor(Color32::from_rgb(160, 80, 160)),
            syntax_link: HexColor(Color32::from_rgb(0, 100, 200)),

            accent: HexColor(Color32::from_rgb(28, 225, 210)),
            destructive: HexColor(Color32::from_rgb(220, 50, 50)),

            bracket_match: HexColor(Color32::from_rgb(255, 220, 80)),

            find_highlight: HexColor(Color32::from_rgb(255, 235, 130)),
            find_highlight_current: HexColor(Color32::from_rgb(255, 170, 40)),

            bold_folders: true,

            title_bar_font_size: 12.0,
            tab_font_size: 12.0,
            editor_font_size: 14.0,
            compose_view_font_size: 18.0,

            window_width: 800.0,
            window_height: 600.0,

            dialog_width: 240.0,

            log_max_file_size: 5 * 1024 * 1024,
        }
    }
}

static SETTINGS: LazyLock<RwLock<Settings>> = LazyLock::new(|| RwLock::new(load()));
static SETTINGS_VERSION: AtomicUsize = AtomicUsize::new(0);

fn config_base_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME")
            && !xdg.is_empty()
        {
            return PathBuf::from(xdg);
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".config");
    }
    PathBuf::from(".")
}

pub fn config_dir() -> PathBuf {
    config_base_dir().join("jereide")
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.toml")
}

fn load() -> Settings {
    let path = settings_path();
    let mut settings = Settings::default();
    match std::fs::read_to_string(&path) {
        Ok(content) => apply_overrides(&mut settings, &content),
        Err(_) => {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            write_template(&path);
        }
    }
    settings
}

fn write_template(path: &PathBuf) {
    if let Ok(rendered) = toml::to_string(&Settings::default()) {
        let commented = rendered
            .lines()
            .map(|line| format!("#{line}"))
            .collect::<Vec<_>>()
            .join("\n");
        let _ = std::fs::write(path, commented);
    }
}

fn apply_overrides(settings: &mut Settings, content: &str) {
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(eq) = line.find('=') else {
            continue;
        };
        let key = line[..eq].trim();
        let mut value = line[eq + 1..].trim();
        if let Some(i) = value.find(" #") {
            value = value[..i].trim();
        }
        apply_override(settings, key, value.trim_matches('"'));
    }
}

fn apply_override(settings: &mut Settings, key: &str, value: &str) {
    macro_rules! set_color {
        ($($field:ident),* $(,)?) => {
            $(
                if key == stringify!($field) {
                    if let Some(c) = HexColor::parse(value) {
                        settings.$field = c;
                    }
                }
            )*
        };
    }
    macro_rules! set_float {
        ($($field:ident),* $(,)?) => {
            $(
                if key == stringify!($field) {
                    if let Ok(f) = value.parse::<f32>() {
                        settings.$field = f;
                    }
                }
            )*
        };
    }
    macro_rules! set_int {
        ($($field:ident),* $(,)?) => {
            $(
                if key == stringify!($field) {
                    if let Ok(i) = value.parse::<usize>() {
                        settings.$field = i;
                    }
                }
            )*
        };
    }

    set_color!(
        surface_bg,
        elevated_bg,
        hover_bg,
        compose_bg,
        text_default,
        text_primary,
        text_secondary,
        text_muted,
        current_line_highlighting,
        compose_text,
        border,
        syntax_keyword,
        syntax_keyword2,
        syntax_string,
        syntax_comment,
        syntax_number,
        syntax_operator,
        syntax_function,
        syntax_literal,
        syntax_heading,
        syntax_code,
        syntax_emphasis,
        syntax_link,
        accent,
        destructive,
        bracket_match,
        find_highlight,
        find_highlight_current,
    );
    set_float!(
        title_bar_font_size,
        tab_font_size,
        editor_font_size,
        compose_view_font_size,
        window_width,
        window_height,
        dialog_width,
    );
    if key == "bold_folders"
        && let Ok(b) = value.parse::<bool>()
    {
        settings.bold_folders = b;
    }
    set_int!(log_max_file_size);
}

pub fn settings_file_path() -> PathBuf {
    let path = settings_path();
    if !path.exists() {
        let _ = load();
    }
    path
}

/// Lock the live settings for mutation. Bumps the version so the app can
/// re-apply visuals that were derived once at startup.
pub(crate) fn update_settings(f: impl FnOnce(&mut Settings)) {
    if let Ok(mut guard) = SETTINGS.write() {
        f(&mut guard);
        SETTINGS_VERSION.fetch_add(1, Ordering::SeqCst);
    }
}

/// Persist the current live settings to `settings.toml` as plain (uncommented)
/// TOML. These values are re-read by `apply_overrides` on the next launch.
pub fn save_settings() {
    if let Ok(guard) = SETTINGS.read() {
        if let Some(parent) = settings_path().parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(rendered) = toml::to_string(&*guard) {
            let _ = std::fs::write(settings_path(), rendered);
        }
    }
}

/// Monotonic counter incremented on every `update_settings`. Lets the app
/// detect when it must re-apply startup-only visuals (selection color, shadow).
pub fn settings_version() -> usize {
    SETTINGS_VERSION.load(Ordering::SeqCst)
}

pub fn surface_bg() -> Color32 {
    SETTINGS.read().unwrap().surface_bg.0
}
pub fn elevated_bg() -> Color32 {
    SETTINGS.read().unwrap().elevated_bg.0
}
pub fn hover_bg() -> Color32 {
    SETTINGS.read().unwrap().hover_bg.0
}
pub fn compose_bg() -> Color32 {
    SETTINGS.read().unwrap().compose_bg.0
}

pub fn text_default() -> Color32 {
    SETTINGS.read().unwrap().text_default.0
}
pub fn text_primary() -> Color32 {
    SETTINGS.read().unwrap().text_primary.0
}
pub fn text_secondary() -> Color32 {
    SETTINGS.read().unwrap().text_secondary.0
}
pub fn text_muted() -> Color32 {
    SETTINGS.read().unwrap().text_muted.0
}
pub fn current_line_highlighting() -> Color32 {
    SETTINGS.read().unwrap().current_line_highlighting.0
}
pub fn compose_text() -> Color32 {
    SETTINGS.read().unwrap().compose_text.0
}

pub fn border() -> Color32 {
    SETTINGS.read().unwrap().border.0
}

pub fn syntax_keyword() -> Color32 {
    SETTINGS.read().unwrap().syntax_keyword.0
}
pub fn syntax_keyword2() -> Color32 {
    SETTINGS.read().unwrap().syntax_keyword2.0
}
pub fn syntax_string() -> Color32 {
    SETTINGS.read().unwrap().syntax_string.0
}
pub fn syntax_comment() -> Color32 {
    SETTINGS.read().unwrap().syntax_comment.0
}
pub fn syntax_number() -> Color32 {
    SETTINGS.read().unwrap().syntax_number.0
}
pub fn syntax_operator() -> Color32 {
    SETTINGS.read().unwrap().syntax_operator.0
}
pub fn syntax_function() -> Color32 {
    SETTINGS.read().unwrap().syntax_function.0
}
pub fn syntax_literal() -> Color32 {
    SETTINGS.read().unwrap().syntax_literal.0
}
pub fn syntax_heading() -> Color32 {
    SETTINGS.read().unwrap().syntax_heading.0
}
pub fn syntax_code() -> Color32 {
    SETTINGS.read().unwrap().syntax_code.0
}
pub fn syntax_emphasis() -> Color32 {
    SETTINGS.read().unwrap().syntax_emphasis.0
}
pub fn syntax_link() -> Color32 {
    SETTINGS.read().unwrap().syntax_link.0
}

pub fn accent() -> Color32 {
    SETTINGS.read().unwrap().accent.0
}
pub fn destructive() -> Color32 {
    SETTINGS.read().unwrap().destructive.0
}

pub fn bracket_match() -> Color32 {
    SETTINGS.read().unwrap().bracket_match.0
}

pub fn find_highlight() -> Color32 {
    SETTINGS.read().unwrap().find_highlight.0
}
pub fn find_highlight_current() -> Color32 {
    SETTINGS.read().unwrap().find_highlight_current.0
}

pub fn bold_folders() -> bool {
    SETTINGS.read().unwrap().bold_folders
}

pub fn title_bar_font_size() -> f32 {
    SETTINGS.read().unwrap().title_bar_font_size
}
pub fn tab_font_size() -> f32 {
    SETTINGS.read().unwrap().tab_font_size
}
pub fn editor_font_size() -> f32 {
    SETTINGS.read().unwrap().editor_font_size
}
pub fn compose_view_font_size() -> f32 {
    SETTINGS.read().unwrap().compose_view_font_size
}

pub fn window_width() -> f32 {
    SETTINGS.read().unwrap().window_width
}
pub fn window_height() -> f32 {
    SETTINGS.read().unwrap().window_height
}

pub fn dialog_width() -> f32 {
    SETTINGS.read().unwrap().dialog_width
}

pub fn log_max_file_size() -> usize {
    SETTINGS.read().unwrap().log_max_file_size
}

/// Inline iOS-style toggle (the canonical widget lives in `jereide-widgets`,
/// which depends on this crate, so we can't depend back on it here).
fn settings_toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter().rect(
            rect,
            radius,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }
    response
}

pub fn render_settings_window(ctx: &egui::Context, open: &mut bool) {
    let screen = ctx
        .input(|i| i.raw.screen_rect)
        .unwrap_or_else(|| ctx.viewport_rect());

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        *open = false;
        return;
    }

    let backdrop_clicked = egui::Area::new(egui::Id::new("settings_modal_blocker"))
        .order(egui::Order::Middle)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, screen.size());
            let r = ui.allocate_rect(rect, egui::Sense::CLICK);
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(110));
            r.clicked()
        })
        .inner;
    if backdrop_clicked {
        *open = false;
        return;
    }

    let max_h = (screen.height() * 0.85).max(360.0);
    egui::Window::new("Settings")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .max_height(max_h)
        .min_width(480.0)
        .max_width(480.0)
        .show(ctx, |ui| {
            ui.set_min_size(egui::vec2(480.0, 360.0));
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(7, 0))
                .show(ui, |ui| {
                    ui.heading("JereIDE Settings");
                    ui.separator();
                });

            let mut snap = SETTINGS.read().unwrap().clone();

            macro_rules! color_row {
                ($ui:expr, $name:expr, $field:ident, $snap:expr) => {
                    $ui.horizontal(|ui| {
                        ui.label($name);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let mut c = $snap.$field.0;
                            let r = egui::color_picker::color_edit_button_srgba(
                                ui,
                                &mut c,
                                egui::color_picker::Alpha::BlendOrAdditive,
                            );
                            if r.changed() {
                                $snap.$field.0 = c;
                                update_settings(|s| s.$field.0 = c);
                            }
                            if r.lost_focus() || r.drag_stopped() {
                                save_settings();
                            }
                        });
                    });
                };
            }
            macro_rules! slider_row {
                ($ui:expr, $name:expr, $field:ident, $range:expr, $snap:expr) => {
                    $ui.horizontal(|ui| {
                        ui.label($name);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let mut v = $snap.$field;
                            let r = ui.add(egui::Slider::new(&mut v, $range));
                            if r.changed() {
                                $snap.$field = v;
                                update_settings(|s| s.$field = v);
                            }
                            if r.lost_focus() || r.drag_stopped() {
                                save_settings();
                            }
                        });
                    });
                };
            }
            macro_rules! toggle_row {
                ($ui:expr, $name:expr, $field:ident, $snap:expr) => {
                    $ui.horizontal(|ui| {
                        ui.label($name);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let mut v = $snap.$field;
                            let r = settings_toggle(ui, &mut v);
                            if r.changed() {
                                $snap.$field = v;
                                update_settings(|s| s.$field = v);
                                save_settings();
                            }
                        });
                    });
                };
            }

            egui::ScrollArea::vertical()
                .max_height(max_h - 150.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let bar = ui.style().spacing.scroll.bar_width;
                        ui.add_space(7.0);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Backgrounds").strong());
                            color_row!(ui, "Surface Background", surface_bg, snap);
                            color_row!(ui, "Elevated Background", elevated_bg, snap);
                            color_row!(ui, "Hover Background", hover_bg, snap);
                            color_row!(ui, "Compose Background", compose_bg, snap);

                            ui.separator();
                            ui.label(egui::RichText::new("Text").strong());
                            color_row!(ui, "Text Default", text_default, snap);
                            color_row!(ui, "Text Primary", text_primary, snap);
                            color_row!(ui, "Text Secondary", text_secondary, snap);
                            color_row!(ui, "Text Muted", text_muted, snap);
                            color_row!(
                                ui,
                                "Current Line Highlight",
                                current_line_highlighting,
                                snap
                            );
                            color_row!(ui, "Compose Text", compose_text, snap);

                            ui.separator();
                            ui.label(egui::RichText::new("UI").strong());
                            color_row!(ui, "Border", border, snap);
                            color_row!(ui, "Accent", accent, snap);
                            color_row!(ui, "Destructive", destructive, snap);
                            color_row!(ui, "Bracket Match", bracket_match, snap);
                            color_row!(ui, "Find Highlight", find_highlight, snap);
                            color_row!(ui, "Find Highlight Current", find_highlight_current, snap);

                            ui.separator();
                            ui.label(egui::RichText::new("Syntax").strong());
                            color_row!(ui, "Syntax Keyword", syntax_keyword, snap);
                            color_row!(ui, "Syntax Keyword 2", syntax_keyword2, snap);
                            color_row!(ui, "Syntax String", syntax_string, snap);
                            color_row!(ui, "Syntax Comment", syntax_comment, snap);
                            color_row!(ui, "Syntax Number", syntax_number, snap);
                            color_row!(ui, "Syntax Operator", syntax_operator, snap);
                            color_row!(ui, "Syntax Function", syntax_function, snap);
                            color_row!(ui, "Syntax Literal", syntax_literal, snap);
                            color_row!(ui, "Syntax Heading", syntax_heading, snap);
                            color_row!(ui, "Syntax Code", syntax_code, snap);
                            color_row!(ui, "Syntax Emphasis", syntax_emphasis, snap);
                            color_row!(ui, "Syntax Link", syntax_link, snap);

                            ui.separator();
                            ui.label(egui::RichText::new("Font Sizes").strong());
                            slider_row!(
                                ui,
                                "Title Bar Font Size",
                                title_bar_font_size,
                                8.0..=32.0,
                                snap
                            );
                            slider_row!(ui, "Tab Font Size", tab_font_size, 8.0..=32.0, snap);
                            slider_row!(ui, "Editor Font Size", editor_font_size, 8.0..=40.0, snap);
                            slider_row!(
                                ui,
                                "Compose Font Size",
                                compose_view_font_size,
                                8.0..=48.0,
                                snap
                            );

                            ui.separator();
                            ui.label(egui::RichText::new("Window").strong());
                            slider_row!(ui, "Window Width", window_width, 400.0..=2400.0, snap);
                            slider_row!(ui, "Window Height", window_height, 400.0..=2400.0, snap);
                            slider_row!(ui, "Dialog Width", dialog_width, 200.0..=600.0, snap);

                            ui.separator();
                            ui.label(egui::RichText::new("Misc").strong());
                            toggle_row!(ui, "Bold Folders", bold_folders, snap);
                            slider_row!(
                                ui,
                                "Log Max File Size",
                                log_max_file_size,
                                1024..=20 * 1024 * 1024,
                                snap
                            );
                        });
                        ui.add_space(7.0 + bar);
                    });
                });

            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(7, 0))
                .show(ui, |ui| {
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Reset to Defaults").clicked() {
                            update_settings(|s| *s = Settings::default());
                            save_settings();
                        }
                    });
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(rgb: u32) -> Color32 {
        Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    }

    #[test]
    fn empty_config_keeps_defaults() {
        let mut s = Settings::default();
        apply_overrides(&mut s, "");
        assert_eq!(s.editor_font_size, 14.0);
        assert_eq!(s.bold_folders, true);
    }

    #[test]
    fn valid_lines_override_defaults() {
        let mut s = Settings::default();
        apply_overrides(
            &mut s,
            "editor_font_size = 18.0\nsurface_bg = \"#112233FF\"\nbold_folders = false\n",
        );
        assert_eq!(s.editor_font_size, 18.0);
        assert_eq!(s.surface_bg.0, hex(0x112233));
        assert_eq!(s.bold_folders, false);
    }

    #[test]
    fn malformed_lines_are_skipped_and_defaults_kept() {
        let mut s = Settings::default();
        apply_overrides(
            &mut s,
            "editor_font_size = 18.0\nthis is not toml at all = { bad\ntext_muted = \"#AABBCC\"\n",
        );
        assert_eq!(s.editor_font_size, 18.0);
        assert_eq!(s.text_muted.0, hex(0xAABBCC));
        assert_eq!(s.current_line_highlighting.0, hex(0x303030));
    }

    #[test]
    fn bad_value_falls_back_to_default() {
        let mut s = Settings::default();
        apply_overrides(
            &mut s,
            "editor_font_size = notanumber\nwindow_height = 999.0\n",
        );
        assert_eq!(s.editor_font_size, 14.0);
        assert_eq!(s.window_height, 999.0);
    }

    #[test]
    fn trailing_comment_is_ignored() {
        let mut s = Settings::default();
        apply_overrides(&mut s, "editor_font_size = 16.0  # line height\n");
        assert_eq!(s.editor_font_size, 16.0);
    }

    #[test]
    fn commented_template_matches_defaults() {
        let mut s = Settings::default();
        let rendered = toml::to_string(&Settings::default()).unwrap();
        let commented = rendered
            .lines()
            .map(|l| format!("#{l}"))
            .collect::<Vec<_>>()
            .join("\n");
        apply_overrides(&mut s, &commented);
        assert_eq!(s.editor_font_size, 14.0);
        assert_eq!(s.surface_bg.0, Color32::WHITE);
    }
}
