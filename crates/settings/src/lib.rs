//! Includes stuff like colors, dimensions, and other constants.
// TODO: Change all of these to variables soon
// TODO: Load all of these from a settings.json or something
// like that at runtime instead of hardcoding them.

use eframe::egui::Color32;

pub const SURFACE_BG: Color32 = Color32::WHITE;
pub const ELEVATED_BG: Color32 = Color32::from_rgb(245, 245, 245);
pub const HOVER_BG: Color32 = Color32::from_rgb(230, 230, 230);
pub const COMMAND_BG: Color32 = Color32::from_gray(20);

pub const TEXT_DEFAULT: Color32 = Color32::BLACK;
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(30, 30, 30);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(130, 130, 130);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(145, 145, 145);
pub const TEXT_CURRENT_LINE: Color32 = Color32::from_rgb(48, 48, 48);
pub const COMMAND_TEXT: Color32 = Color32::from_gray(250);

pub const BORDER: Color32 = Color32::from_rgb(200, 200, 200);

pub const ACCENT: Color32 = Color32::from_rgb(28, 225, 210);

pub const TITLE_BAR_FONT_SIZE: f32 = 12.0;
pub const TAB_FONT_SIZE: f32 = 12.0;
pub const EDITOR_FONT_SIZE: f32 = 14.0;
pub const COMMAND_VIEW_FONT_SIZE: f32 = 18.0;

pub const WINDOW_WIDTH: f32 = 800.0;
pub const WINDOW_HEIGHT: f32 = 600.0;

pub const DIALOG_WIDTH: f32 = 240.0;
