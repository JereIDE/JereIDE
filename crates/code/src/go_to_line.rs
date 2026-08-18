use eframe::egui;
use jereide_core::AppState;

pub fn go_to_line(state: &mut AppState, ctx: &egui::Context, char_index: usize) {
    if state.tabs.is_empty() {
        return;
    }
    let id = state.editor_id();
    if let Some(mut edit_state) = jereide_editor::TextEdit::load_state(ctx, id) {
        edit_state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(char_index),
            )));
        edit_state.store(ctx, id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn go_to_line_empty_tabs_is_noop() {
        let mut state = AppState::new();
        state.tabs.clear();
        let ctx = egui::Context::default();
        go_to_line(&mut state, &ctx, 0);
    }

    #[test]
    fn go_to_line_with_tab_no_panic() {
        let mut state = AppState::new();
        state.new_tab();
        state.tabs[0].text = "a\nb\nc".to_string();
        let ctx = egui::Context::default();
        go_to_line(&mut state, &ctx, 2);
    }
}
