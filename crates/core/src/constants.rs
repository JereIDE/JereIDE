//! Central constants for JereIDE.
//! Changing might break!

use eframe::egui::Margin;

pub const TITLE_BAR_HEIGHT: f32 = 34.0;
pub const TAB_STRIP_HEIGHT: f32 = 30.0;
pub const TITLE_BAR_TRAFFIC_SPACE: f32 = 75.0;
pub const TITLE_BAR_FULLSCREEN_SPACE: f32 = 7.0;
pub const TITLE_BAR_POPUP_GAP: f32 = 4.0;
pub const EDITOR_INNER_MARGIN_TOP: i8 = 10;
pub const EDITOR_INNER_MARGIN_BOTTOM: i8 = 10;
pub const EDITOR_INNER_MARGIN_RIGHT: i8 = 10;
pub const EDITOR_INNER_MARGIN_LEFT_EXTRA: i8 = 6;
pub const GUTTER_DIGIT_WIDTH: f32 = 12.0;
pub const GUTTER_PADDING_LEFT: f32 = 10.0;
pub const GUTTER_PADDING_RIGHT: f32 = 6.0;
pub const GUTTER_LINE_NUMBER_RIGHT_OFFSET: f32 = 5.0;
pub const SCROLL_BAR_WIDTH: f32 = 12.0;
pub const MAX_FILE_SIZE: u64 = 200 * 1024 * 1024;
pub const WARN_FILE_SIZE: u64 = 100 * 1024 * 1024;
pub const STATUS_BAR_MARGIN: Margin = Margin::symmetric(8, 4);
pub const TAB_PAD_LEFT: f32 = 8.0;
pub const TAB_PAD_RIGHT: f32 = 8.0;
pub const TAB_CLOSE_BTN_SIZE: f32 = 18.0;
pub const TAB_CLOSE_BTN_RADIUS: u8 = 3;
pub const TAB_CLOSE_BTN_SPACING: f32 = 6.0;
pub const TAB_CLOSE_ICON_HALF: f32 = 3.0;
pub const TAB_CLOSE_STROKE: f32 = 0.9;
pub const TAB_MODIFIED_DOT_RADIUS: f32 = 3.0;
pub const TAB_BORDER_WIDTH: f32 = 1.0;
pub const ITEM_SPACING_Y: f32 = 0.0;
pub const TRAFFIC_LIGHT_OFFSET_X: f64 = 2.0;
pub const TRAFFIC_LIGHT_OFFSET_Y: f64 = -3.0;
