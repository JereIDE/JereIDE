use std::sync::Arc;

use eframe::egui;
use egui::mutex::Mutex;

use egui::{
    text_selection::{CCursorRange, TextCursorState},
    Context, Id, Vec2,
};

pub type TextEditUndoer = egui::util::undoer::Undoer<(CCursorRange, String)>;

#[derive(Clone, Default)]
pub struct TextEditState {
    pub cursor: TextCursorState,

    pub(crate) undoer: Arc<Mutex<TextEditUndoer>>,

    pub(crate) ime_enabled: bool,

    pub(crate) ime_cursor_range: CCursorRange,

    pub(crate) text_offset: Vec2,

    pub(crate) last_interaction_time: f64,
}

impl TextEditState {
    pub fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_persisted(id))
    }

    pub fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_persisted(id, self));
    }

    pub fn undoer(&self) -> TextEditUndoer {
        self.undoer.lock().clone()
    }

    pub fn set_undoer(&mut self, undoer: TextEditUndoer) {
        *self.undoer.lock() = undoer;
    }

    pub fn clear_undoer(&mut self) {
        self.set_undoer(TextEditUndoer::default());
    }
}
