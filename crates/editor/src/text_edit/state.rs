use std::sync::Arc;

use eframe::egui;
use egui::mutex::Mutex;
use egui::text::CCursor;

use egui::{
    Context, Id, Vec2,
    text_selection::{CCursorRange, TextCursorState},
};

pub type TextEditUndoer = egui::util::undoer::Undoer<(CCursorRange, String)>;

#[derive(Clone, Default)]
pub struct TextEditState {
    pub cursor: TextCursorState,

    pub(crate) cursor_purpose: TextEditCursorPurpose,

    pub(crate) undoer: Arc<Mutex<TextEditUndoer>>,

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

#[derive(Clone, Default)]
pub(crate) enum TextEditCursorPurpose {
    #[default]
    Selection,

    ImeComposition {
        #[allow(dead_code)]
        active_range: Option<std::ops::Range<CCursor>>,
    },
}

impl TextEditCursorPurpose {
    pub(crate) fn is_selection(&self) -> bool {
        matches!(self, Self::Selection)
    }

    pub(crate) fn is_ime_composition(&self) -> bool {
        matches!(self, Self::ImeComposition { .. })
    }
}
