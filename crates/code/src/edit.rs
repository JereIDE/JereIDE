use eframe::egui;
use jereide_core::AppState;
use jereide_text::{char_range_substring, delete_char_range};

pub fn handle_edit_action(state: &mut AppState, ctx: &egui::Context, action: &str) {
    if state.tabs.is_empty() {
        return;
    }
    match action {
        "editor: select all" => action_select_all(state, ctx),
        "editor: copy" => action_copy(state, ctx),
        "editor: cut" => action_cut(state, ctx),
        "editor: paste" => action_paste(state, ctx),
        "editor: undo" => action_undo(state, ctx),
        "editor: redo" => action_redo(state, ctx),
        _ => {}
    }
}

fn action_select_all(state: &AppState, ctx: &egui::Context) {
    if let Some(mut edit_state) = egui::TextEdit::load_state(ctx, state.editor_id) {
        let len = state.current_tab().text.chars().count();
        use egui::text::{CCursor, CCursorRange};
        edit_state
            .cursor
            .set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(len))));
        edit_state.store(ctx, state.editor_id);
    }
}

fn action_copy(state: &AppState, ctx: &egui::Context) {
    if let Some(edit_state) = egui::TextEdit::load_state(ctx, state.editor_id) {
        if let Some(range) = edit_state.cursor.char_range() {
            let start = range.primary.index.min(range.secondary.index);
            let end = range.primary.index.max(range.secondary.index);
            if end > start {
                let text = char_range_substring(state.current_tab().text.as_str(), start, end);
                ctx.copy_text(text);
            }
        }
    }
}

fn action_cut(state: &mut AppState, ctx: &egui::Context) {
    if let Some(mut edit_state) = egui::TextEdit::load_state(ctx, state.editor_id) {
        if let Some(range) = edit_state.cursor.char_range() {
            let start = range.primary.index.min(range.secondary.index);
            let end = range.primary.index.max(range.secondary.index);
            if end > start {
                let idx = state.active_tab_index;
                let text = char_range_substring(&state.tabs[idx].text, start, end);
                ctx.copy_text(text);
                let new_text = delete_char_range(&state.tabs[idx].text, start, end);
                state.tabs[idx].text = new_text;
            }
            edit_state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(
                    egui::text::CCursor::new(start),
                )));
            edit_state.store(ctx, state.editor_id);
        }
    }
}

fn action_paste(_state: &mut AppState, ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
}

fn action_undo(state: &mut AppState, ctx: &egui::Context) {
    if let Some(mut edit_state) = egui::TextEdit::load_state(ctx, state.editor_id) {
        let idx = state.active_tab_index;
        let current = (
            edit_state
                .cursor
                .char_range()
                .unwrap_or(egui::text::CCursorRange::one(egui::text::CCursor::new(0))),
            state.tabs[idx].text.clone(),
        );
        let mut undoer = edit_state.undoer();
        if let Some((cursor_range, text)) = undoer.undo(&current).cloned() {
            state.tabs[idx].text = text;
            edit_state.cursor.set_char_range(Some(cursor_range));
            edit_state.set_undoer(undoer);
            edit_state.store(ctx, state.editor_id);
        }
    }
}

fn action_redo(state: &mut AppState, ctx: &egui::Context) {
    if let Some(mut edit_state) = egui::TextEdit::load_state(ctx, state.editor_id) {
        let idx = state.active_tab_index;
        let current = (
            edit_state
                .cursor
                .char_range()
                .unwrap_or(egui::text::CCursorRange::one(egui::text::CCursor::new(0))),
            state.tabs[idx].text.clone(),
        );
        let mut undoer = edit_state.undoer();
        if let Some((cursor_range, text)) = undoer.redo(&current).cloned() {
            state.tabs[idx].text = text;
            edit_state.cursor.set_char_range(Some(cursor_range));
            edit_state.set_undoer(undoer);
            edit_state.store(ctx, state.editor_id);
        }
    }
}
