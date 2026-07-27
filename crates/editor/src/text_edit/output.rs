use std::sync::Arc;

use eframe::egui;
use egui::text::CCursorRange;

pub struct TextEditOutput {
    pub response: egui::AtomLayoutResponse,
    pub galley: Arc<egui::Galley>,
    pub galley_pos: egui::Pos2,
    pub text_clip_rect: egui::Rect,
    pub state: super::TextEditState,
    pub cursor_range: Option<CCursorRange>,
}
