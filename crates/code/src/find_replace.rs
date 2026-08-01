use eframe::egui;
use jereide_core::AppState;
use jereide_text::find_matches;

pub fn select_match(state: &mut AppState, ctx: &egui::Context, start: usize, end: usize) {
    if state.tabs.is_empty() {
        return;
    }
    let id = state.editor_id;
    if let Some(mut edit_state) = jereide_editor::TextEdit::load_state(ctx, id) {
        edit_state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(start),
                egui::text::CCursor::new(end),
            )));
        edit_state.store(ctx, id);
    }
}

pub fn replace_range(
    state: &mut AppState,
    ctx: &egui::Context,
    start: usize,
    end: usize,
    replacement: &str,
) -> bool {
    if state.tabs.is_empty() {
        return false;
    }
    let idx = state.active_tab_index;
    let old_text = state.tabs[idx].text.clone();
    let char_count = old_text.chars().count();
    if start >= end || start > char_count || end > char_count {
        return false;
    }
    let before: String = old_text.chars().take(start).collect();
    let after: String = old_text.chars().skip(end).collect();
    let new_text = format!("{}{}{}", before, replacement, after);
    state.tabs[idx].text = new_text;
    let cursor = start + replacement.chars().count();
    record_edit(state, ctx, old_text, cursor);
    true
}

pub fn replace_all(
    state: &mut AppState,
    ctx: &egui::Context,
    find: &str,
    replace: &str,
    match_case: bool,
    whole_word: bool,
) -> usize {
    if state.tabs.is_empty() || find.is_empty() {
        return 0;
    }
    let idx = state.active_tab_index;
    let old_text = state.tabs[idx].text.clone();
    let matches = find_matches(&old_text, find, match_case, whole_word);
    if matches.is_empty() {
        return 0;
    }
    let chars: Vec<char> = old_text.chars().collect();
    let mut out = String::new();
    let mut last = 0;
    for (s, e) in &matches {
        out.extend(chars[last..*s].iter());
        out.push_str(replace);
        last = *e;
    }
    out.extend(chars[last..].iter());
    let new_text = out;
    let replaced_count = matches.len();
    let cursor = new_text.chars().count();
    state.tabs[idx].text = new_text;
    record_edit(state, ctx, old_text, cursor);
    replaced_count
}

fn record_edit(state: &mut AppState, ctx: &egui::Context, old_text: String, cursor: usize) {
    let id = state.editor_id;
    if let Some(mut edit_state) = jereide_editor::TextEdit::load_state(ctx, id) {
        let old_cursor = edit_state
            .cursor
            .char_range()
            .unwrap_or(egui::text::CCursorRange::one(egui::text::CCursor::new(0)));
        let mut undoer = edit_state.undoer();
        undoer.add_undo(&(old_cursor, old_text));
        edit_state.set_undoer(undoer);
        edit_state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(cursor),
            )));
        edit_state.store(ctx, id);
    }
}
