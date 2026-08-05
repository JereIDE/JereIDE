use std::path::PathBuf;
use std::sync::LazyLock;

use eframe::egui::Color32;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy)]
struct HexColor(Color32);

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

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s.trim_start_matches('#');
        let bytes = |range: std::ops::Range<usize>| u8::from_str_radix(&s[range], 16).ok();

        if s.len() == 6 {
            let r = bytes(0..2).ok_or_else(|| serde::de::Error::custom("invalid color"))?;
            let g = bytes(2..4).ok_or_else(|| serde::de::Error::custom("invalid color"))?;
            let b = bytes(4..6).ok_or_else(|| serde::de::Error::custom("invalid color"))?;
            Ok(HexColor(Color32::from_rgb(r, g, b)))
        } else if s.len() == 8 {
            let r = bytes(0..2).ok_or_else(|| serde::de::Error::custom("invalid color"))?;
            let g = bytes(2..4).ok_or_else(|| serde::de::Error::custom("invalid color"))?;
            let b = bytes(4..6).ok_or_else(|| serde::de::Error::custom("invalid color"))?;
            let a = bytes(6..8).ok_or_else(|| serde::de::Error::custom("invalid color"))?;
            Ok(HexColor(Color32::from_rgba_unmultiplied(r, g, b, a)))
        } else {
            Err(serde::de::Error::custom("invalid color"))
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    surface_bg: HexColor,
    elevated_bg: HexColor,
    hover_bg: HexColor,
    compose_bg: HexColor,

    text_default: HexColor,
    text_primary: HexColor,
    text_secondary: HexColor,
    text_muted: HexColor,
    text_current_line: HexColor,
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

    title_bar_font_size: f32,
    tab_font_size: f32,
    editor_font_size: f32,
    compose_view_font_size: f32,

    window_width: f32,
    window_height: f32,

    dialog_width: f32,
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
            text_current_line: HexColor(Color32::from_rgb(48, 48, 48)),
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

            title_bar_font_size: 12.0,
            tab_font_size: 12.0,
            editor_font_size: 14.0,
            compose_view_font_size: 18.0,

            window_width: 800.0,
            window_height: 600.0,

            dialog_width: 240.0,
        }
    }
}

static SETTINGS: LazyLock<Settings> = LazyLock::new(load);

fn config_base_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            if !xdg.is_empty() {
                return PathBuf::from(xdg);
            }
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".config");
    }
    PathBuf::from(".")
}

pub fn settings_path() -> PathBuf {
    config_base_dir().join("jereide").join("settings.toml")
}

fn load() -> Settings {
    let path = settings_path();
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(settings) = toml::from_str::<Settings>(&content) {
                return settings;
            }
        }
    }
    let defaults = Settings::default();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(rendered) = toml::to_string(&defaults) {
        let _ = std::fs::write(&path, rendered);
    }
    defaults
}

pub fn settings_file_path() -> PathBuf {
    let path = settings_path();
    if !path.exists() {
        let _ = load();
    }
    path
}

pub fn surface_bg() -> Color32 {
    SETTINGS.surface_bg.0
}
pub fn elevated_bg() -> Color32 {
    SETTINGS.elevated_bg.0
}
pub fn hover_bg() -> Color32 {
    SETTINGS.hover_bg.0
}
pub fn compose_bg() -> Color32 {
    SETTINGS.compose_bg.0
}

pub fn text_default() -> Color32 {
    SETTINGS.text_default.0
}
pub fn text_primary() -> Color32 {
    SETTINGS.text_primary.0
}
pub fn text_secondary() -> Color32 {
    SETTINGS.text_secondary.0
}
pub fn text_muted() -> Color32 {
    SETTINGS.text_muted.0
}
pub fn text_current_line() -> Color32 {
    SETTINGS.text_current_line.0
}
pub fn compose_text() -> Color32 {
    SETTINGS.compose_text.0
}

pub fn border() -> Color32 {
    SETTINGS.border.0
}

pub fn syntax_keyword() -> Color32 {
    SETTINGS.syntax_keyword.0
}
pub fn syntax_keyword2() -> Color32 {
    SETTINGS.syntax_keyword2.0
}
pub fn syntax_string() -> Color32 {
    SETTINGS.syntax_string.0
}
pub fn syntax_comment() -> Color32 {
    SETTINGS.syntax_comment.0
}
pub fn syntax_number() -> Color32 {
    SETTINGS.syntax_number.0
}
pub fn syntax_operator() -> Color32 {
    SETTINGS.syntax_operator.0
}
pub fn syntax_function() -> Color32 {
    SETTINGS.syntax_function.0
}
pub fn syntax_literal() -> Color32 {
    SETTINGS.syntax_literal.0
}
pub fn syntax_heading() -> Color32 {
    SETTINGS.syntax_heading.0
}
pub fn syntax_code() -> Color32 {
    SETTINGS.syntax_code.0
}
pub fn syntax_emphasis() -> Color32 {
    SETTINGS.syntax_emphasis.0
}
pub fn syntax_link() -> Color32 {
    SETTINGS.syntax_link.0
}

pub fn accent() -> Color32 {
    SETTINGS.accent.0
}
pub fn destructive() -> Color32 {
    SETTINGS.destructive.0
}

pub fn bracket_match() -> Color32 {
    SETTINGS.bracket_match.0
}

pub fn find_highlight() -> Color32 {
    SETTINGS.find_highlight.0
}
pub fn find_highlight_current() -> Color32 {
    SETTINGS.find_highlight_current.0
}

pub fn title_bar_font_size() -> f32 {
    SETTINGS.title_bar_font_size
}
pub fn tab_font_size() -> f32 {
    SETTINGS.tab_font_size
}
pub fn editor_font_size() -> f32 {
    SETTINGS.editor_font_size
}
pub fn compose_view_font_size() -> f32 {
    SETTINGS.compose_view_font_size
}

pub fn window_width() -> f32 {
    SETTINGS.window_width
}
pub fn window_height() -> f32 {
    SETTINGS.window_height
}

pub fn dialog_width() -> f32 {
    SETTINGS.dialog_width
}
