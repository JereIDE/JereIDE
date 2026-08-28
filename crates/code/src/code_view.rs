use std::cell::RefCell;

use eframe::egui;
use egui::text::CCursor;
use jereide_core::AppState;
use jereide_core::constants::{
    EDITOR_INNER_MARGIN_BOTTOM, EDITOR_INNER_MARGIN_LEFT_EXTRA, EDITOR_INNER_MARGIN_RIGHT,
    EDITOR_INNER_MARGIN_TOP, GUTTER_DIGIT_WIDTH, GUTTER_LINE_NUMBER_RIGHT_OFFSET,
    GUTTER_PADDING_LEFT, GUTTER_PADDING_RIGHT, SCROLL_BAR_WIDTH,
};
use jereide_settings::{
    bracket_match, current_line_highlighting, editor_font_size, find_highlight,
    find_highlight_current, surface_bg, text_muted,
};
use jereide_text::{char_index_to_line_col, char_range_substring, find_matches};

use jereide_syntax::SyntaxHighlighter;
use std::collections::{HashMap, HashSet};
thread_local! {
    static HIGHLIGHTERS: RefCell<HashMap<usize, SyntaxHighlighter>> = RefCell::new(HashMap::new());
    static FIND_CACHE: RefCell<HashMap<usize, FindCache>> = RefCell::new(HashMap::new());
    static PREV_TEXT: RefCell<HashMap<usize, String>> = RefCell::new(HashMap::new());
    static EDITOR_STATE_TABS: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
}

struct FindCache {
    text: String,
    query: String,
    match_case: bool,
    whole_word: bool,
    matches: Vec<(usize, usize)>,
}

pub fn render_code_view(state: &mut AppState, ui: &mut egui::Ui, tab_index: usize, pane_id: usize) {
    if state.tabs.is_empty() {
        return;
    }

    let ctx = ui.ctx().clone();

    let style = ui.style_mut();
    style.visuals.extreme_bg_color = surface_bg();
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
    style.spacing.scroll = {
        let mut s = egui::style::ScrollStyle::floating();
        s.bar_width = SCROLL_BAR_WIDTH;
        s
    };

    let active_idx = tab_index;
    let tab_id = state.tabs[active_idx].id;
    let read_only = state.tabs[active_idx].read_only;
    let extension: Option<String> = state.tabs[active_idx]
        .file_path
        .as_ref()
        .and_then(|p| std::path::Path::new(p).extension())
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_string());

    let syntax_file = extension
        .as_deref()
        .and_then(|ext| jereide_data::lookup_language(Some(ext)))
        .and_then(|info| info.syntax_file);

    let valid_ids: std::collections::HashSet<usize> = state.tabs.iter().map(|t| t.id).collect();
    HIGHLIGHTERS.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.retain(|id, _| valid_ids.contains(id));
        cache
            .entry(tab_id)
            .or_insert_with(|| SyntaxHighlighter::new(editor_font_size(), syntax_file.as_deref()));
    });
    FIND_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.retain(|id, _| valid_ids.contains(id));
    });
    PREV_TEXT.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.retain(|id, _| valid_ids.contains(id));
    });
    EDITOR_STATE_TABS.with(|tabs| {
        let mut tabs = tabs.borrow_mut();
        for &closed in tabs.iter().filter(|id| !valid_ids.contains(id)) {
            let id = egui::Id::new(("editor", closed));
            ctx.data_mut(|d| d.remove::<jereide_editor::TextEditState>(id));
        }
        tabs.retain(|id| valid_ids.contains(id));
        tabs.insert(tab_id);
    });

    let font_id = egui::FontId::monospace(editor_font_size());
    let cursor_line = state.tabs[active_idx].cursor_line;

    let mut layouter =
        |layouter_ui: &egui::Ui, text: &dyn jereide_editor::TextBuffer, _wrap_width: f32| {
            let text_str = text.as_str();
            HIGHLIGHTERS.with(|cache| {
                let mut c = cache.borrow_mut();
                let hl = c.get_mut(&tab_id).unwrap();
                layouter_ui.fonts_mut(|f| hl.highlight_galley(text_str, f))
            })
        };

    let scroll_area_id = ui.make_persistent_id(egui::IdSalt::new(("editor_scroll", tab_id)));
    let gutter_scroll_x = egui::containers::scroll_area::State::load(ui.ctx(), scroll_area_id)
        .map(|s| s.offset.x)
        .unwrap_or(0.0);

    let text_edit_output = egui::ScrollArea::both()
        .id_salt(("editor_scroll", tab_id))
        .auto_shrink(false)
        .show(ui, |ui| {
            let viewport = ui.max_rect().size();
            ui.set_min_size(viewport);

            let text_output = jereide_editor::TextEdit::code_editor(
                jereide_editor::TextEdit::multiline(&mut state.tabs[active_idx].text),
            )
            .id(egui::Id::new(("editor", tab_id)))
            .editable(!read_only)
            .desired_width(f32::INFINITY)
            .min_size(egui::vec2(0.0, viewport.y))
            .frame(egui::Frame {
                inner_margin: egui::Margin {
                    left: EDITOR_INNER_MARGIN_LEFT_EXTRA,
                    right: EDITOR_INNER_MARGIN_RIGHT,
                    top: EDITOR_INNER_MARGIN_TOP,
                    bottom: EDITOR_INNER_MARGIN_BOTTOM,
                },
                ..egui::Frame::NONE
            })
            .layouter(&mut layouter)
            .gutter_scroll_x(gutter_scroll_x)
            .show_gutter(jereide_editor::GutterConfig {
                padding_left: GUTTER_PADDING_LEFT,
                padding_right: GUTTER_PADDING_RIGHT,
                digit_width: GUTTER_DIGIT_WIDTH,
                line_number_right_offset: GUTTER_LINE_NUMBER_RIGHT_OFFSET,
                current_line_color: current_line_highlighting(),
                muted_color: text_muted(),
                background_color: surface_bg(),
                font_id: font_id.clone(),
                current_line: cursor_line,
            })
            .show(ui);

            let text_response = &text_output.response;
            let galley = text_output.galley.clone();
            let galley_pos = text_output.galley_pos;

            if let Some(hl) = state.find_highlight.clone() {
                if !hl.query.is_empty() {
                    let tab_text = &state.tabs[active_idx].text;
                    let matches = FIND_CACHE.with(|cache| {
                        let mut cache = cache.borrow_mut();
                        let entry = cache.entry(tab_id).or_insert_with(|| FindCache {
                            text: String::new(),
                            query: String::new(),
                            match_case: false,
                            whole_word: false,
                            matches: Vec::new(),
                        });
                        if entry.text != *tab_text
                            || entry.query != hl.query
                            || entry.match_case != hl.match_case
                            || entry.whole_word != hl.whole_word
                        {
                            entry.text = tab_text.clone();
                            entry.query = hl.query.clone();
                            entry.match_case = hl.match_case;
                            entry.whole_word = hl.whole_word;
                            entry.matches =
                                find_matches(tab_text, &hl.query, hl.match_case, hl.whole_word);
                        }
                        entry.matches.clone()
                    });
                    paint_find_highlights(
                        ui,
                        &galley,
                        galley_pos,
                        &matches,
                        hl.current_match,
                        text_output.text_clip_rect,
                    );
                    if let Some(target) = hl.scroll_to {
                        let gutter_w = gutter_width(galley.rows.len());
                        scroll_to_char(ui, &galley, galley_pos, target, gutter_w);
                    }
                }
            }

            if let Some(target) = state.go_to_line_scroll_to {
                let gutter_w = gutter_width(galley.rows.len());
                scroll_to_char(ui, &galley, galley_pos, target, gutter_w);
                state.go_to_line_scroll_to = None;
            }

            if let Some(cursor_range) = text_output.cursor_range {
                let tab_text = &state.tabs[active_idx].text;
                if let Some((open_idx, close_idx)) =
                    find_matching_bracket(tab_text, cursor_range.primary.index.into())
                {
                    let bracket_painter = ui
                        .painter_at(text_output.text_clip_rect.expand(1.0))
                        .with_layer_id(egui::LayerId::new(
                            egui::Order::Background,
                            egui::Id::new("bracket_highlight"),
                        ));
                    let highlight_at = |char_index: usize| {
                        if char_index >= tab_text.chars().count() {
                            return;
                        }
                        let lc = galley.layout_from_cursor(CCursor::new(char_index));
                        if let Some(placed_row) = galley.rows.get(lc.row) {
                            if let Some(glyph) = placed_row.glyphs.get(lc.column.0) {
                                let screen_x = galley_pos.x + placed_row.pos.x + glyph.pos.x;
                                let screen_y = galley_pos.y + placed_row.pos.y;
                                let w = glyph.advance_width;
                                let h = placed_row.height();
                                bracket_painter.rect_filled(
                                    egui::Rect::from_min_size(
                                        egui::pos2(screen_x, screen_y),
                                        egui::vec2(w, h),
                                    ),
                                    2.0,
                                    bracket_match().linear_multiply(0.3),
                                );
                            }
                        }
                    };
                    highlight_at(open_idx);
                    highlight_at(close_idx);
                }
            }

            // Fill up the whole Y available space
            let remaining = ui.available_size();
            let bg_response = if remaining.y > 0.0 {
                let (_, bg) = ui.allocate_exact_size(remaining, egui::Sense::click());
                let bg = bg.on_hover_cursor(egui::CursorIcon::Text);
                if bg.clicked() {
                    text_response.request_focus();
                }
                Some(bg)
            } else {
                None
            };

            let mut context_action: Option<&'static str> = None;
            let mut add_context_menu = |menu_ui: &mut egui::Ui| {
                if menu_ui.button("Undo").clicked() {
                    context_action = Some("editor: undo");
                }
                if menu_ui.button("Redo").clicked() {
                    context_action = Some("editor: redo");
                }
                menu_ui.separator();
                if menu_ui.button("Cut").clicked() {
                    context_action = Some("editor: cut");
                }
                if menu_ui.button("Copy").clicked() {
                    context_action = Some("editor: copy");
                }
                if menu_ui.button("Paste").clicked() {
                    context_action = Some("editor: paste");
                }
                menu_ui.separator();
                if menu_ui.button("Select All").clicked() {
                    context_action = Some("editor: select all");
                }
            };

            let tracker_id = egui::Id::new("editor_context_menu");
            let was_menu_open = ctx
                .data(|d| d.get_temp::<bool>(tracker_id))
                .unwrap_or(false);

            text_response.context_menu(&mut add_context_menu);
            if let Some(bg) = bg_response.as_ref() {
                bg.context_menu(&mut add_context_menu);
            }

            let is_menu_open = text_response.context_menu_opened()
                || bg_response
                    .as_ref()
                    .map_or(false, |bg| bg.context_menu_opened());

            if was_menu_open && !is_menu_open {
                text_response.request_focus();
            }
            ctx.data_mut(|d| d.insert_temp(tracker_id, is_menu_open));

            if let Some(action) = context_action {
                crate::edit::handle_edit_action(state, &ctx, action);
            }

            text_output
        })
        .inner;
    state.selected_text = text_edit_output
        .state
        .cursor
        .char_range()
        .and_then(|range| {
            let start: usize = range.primary.index.min(range.secondary.index).into();
            let end: usize = range.primary.index.max(range.secondary.index).into();
            if end > start {
                Some(char_range_substring(
                    &state.tabs[active_idx].text,
                    start,
                    end,
                ))
            } else {
                None
            }
        });

    if let Some(cursor_range) = text_edit_output.cursor_range {
        let cursor_idx: usize = cursor_range.primary.index.into();

        // For the status bar Line/Col indicator, some stuff to help it
        let (line, col) = char_index_to_line_col(&state.tabs[active_idx].text, cursor_idx);
        state.tabs[active_idx].cursor_line = line;
        state.tabs[active_idx].cursor_col = col;

        if text_edit_output.response.changed() {
            let old_text = PREV_TEXT
                .with(|c| c.borrow().get(&tab_id).cloned())
                .unwrap_or_else(|| state.tabs[active_idx].text.clone());

            // Smart bracket deletion, so deleting will delete... like, deleting the opening will delete
            // the closing bracket.
            if should_delete_bracket_pair(&old_text, &state.tabs[active_idx].text, cursor_idx) {
                let byte_idx = state.tabs[active_idx]
                    .text
                    .char_indices()
                    .nth(cursor_idx)
                    .map(|(i, _)| i)
                    .unwrap_or(state.tabs[active_idx].text.len());
                state.tabs[active_idx].text.remove(byte_idx);
                if let Some(mut edit_state) =
                    jereide_editor::TextEdit::load_state(&ctx, text_edit_output.response.id)
                {
                    edit_state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::one(CCursor::new(
                            cursor_idx,
                        ))));

                    if let Some(cursor_range) = edit_state.cursor.char_range() {
                        edit_state.undoer().feed_state(
                            ctx.input(|i| i.time),
                            &(cursor_range, state.tabs[active_idx].text.clone()),
                        );
                    }
                    edit_state.store(&ctx, text_edit_output.response.id);
                }
            }

            PREV_TEXT.with(|c| {
                c.borrow_mut()
                    .insert(tab_id, state.tabs[active_idx].text.clone())
            });
        }
    }

    if !state.editor_focused {
        state.editor_focused = true;
        text_edit_output.response.request_focus();
    }
}

fn char_row_x(galley: &egui::Galley, char_index: usize) -> Option<(usize, f32)> {
    let lc = galley.layout_from_cursor(CCursor::new(char_index));
    let row = galley.rows.get(lc.row)?;
    let x = if lc.column.0 < row.glyphs.len() {
        row.pos.x + row.glyphs[lc.column.0].pos.x
    } else {
        row.pos.x + row.size.x
    };
    Some((lc.row, x))
}

fn paint_find_highlights(
    ui: &egui::Ui,
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    matches: &[(usize, usize)],
    current: usize,
    text_clip_rect: egui::Rect,
) {
    let painter = ui
        .painter_at(text_clip_rect.expand(1.0))
        .with_layer_id(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("find_highlight"),
        ));
    for (i, &(s, e)) in matches.iter().enumerate() {
        if s >= e {
            continue;
        }
        let color = if i == current {
            find_highlight_current().linear_multiply(0.4)
        } else {
            find_highlight().linear_multiply(0.25)
        };
        let Some((start_row, start_x)) = char_row_x(galley, s) else {
            continue;
        };
        let Some((end_row, end_x)) = char_row_x(galley, e) else {
            continue;
        };
        if start_row > end_row {
            continue;
        }
        if start_row == end_row {
            if let Some(row) = galley.rows.get(start_row) {
                let rect = egui::Rect::from_min_max(
                    egui::pos2(galley_pos.x + start_x, galley_pos.y + row.pos.y),
                    egui::pos2(
                        galley_pos.x + end_x,
                        galley_pos.y + row.pos.y + row.height(),
                    ),
                );
                painter.rect_filled(rect, 2.0, color);
            }
        } else {
            if let Some(row) = galley.rows.get(start_row) {
                let rect = egui::Rect::from_min_max(
                    egui::pos2(galley_pos.x + start_x, galley_pos.y + row.pos.y),
                    egui::pos2(
                        galley_pos.x + row.pos.x + row.size.x,
                        galley_pos.y + row.pos.y + row.height(),
                    ),
                );
                painter.rect_filled(rect, 2.0, color);
            }
            for r in (start_row + 1)..end_row {
                if let Some(row) = galley.rows.get(r) {
                    let rect = egui::Rect::from_min_max(
                        egui::pos2(galley_pos.x + row.pos.x, galley_pos.y + row.pos.y),
                        egui::pos2(
                            galley_pos.x + row.pos.x + row.size.x,
                            galley_pos.y + row.pos.y + row.height(),
                        ),
                    );
                    painter.rect_filled(rect, 2.0, color);
                }
            }
            if let Some(row) = galley.rows.get(end_row) {
                let rect = egui::Rect::from_min_max(
                    egui::pos2(galley_pos.x + row.pos.x, galley_pos.y + row.pos.y),
                    egui::pos2(
                        galley_pos.x + end_x,
                        galley_pos.y + row.pos.y + row.height(),
                    ),
                );
                painter.rect_filled(rect, 2.0, color);
            }
        }
    }
}

fn scroll_to_char(
    ui: &egui::Ui,
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    char_index: usize,
    gutter_w: f32,
) {
    if let Some((row_idx, x)) = char_row_x(galley, char_index) {
        if let Some(row) = galley.rows.get(row_idx) {
            const MARGIN_X: f32 = 24.0;
            const MARGIN_Y: f32 = 16.0;
            let rect = egui::Rect::from_min_max(
                egui::pos2(
                    galley_pos.x + x - gutter_w - MARGIN_X,
                    (galley_pos.y + row.pos.y - MARGIN_Y).max(galley_pos.y),
                ),
                egui::pos2(
                    galley_pos.x + x + 1.0,
                    galley_pos.y + row.pos.y + row.height(),
                ),
            );
            ui.scroll_to_rect(rect, Some(egui::Align::TOP));
        }
    }
}

fn gutter_width(line_count: usize) -> f32 {
    let digit_count = if line_count < 10 {
        1
    } else {
        line_count.ilog10() as usize + 1
    };
    GUTTER_PADDING_LEFT + digit_count as f32 * GUTTER_DIGIT_WIDTH + GUTTER_PADDING_RIGHT
}

fn find_matching_bracket(text: &str, cursor_char_index: usize) -> Option<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();

    let check_at = |idx: usize| -> Option<(usize, usize)> {
        match chars[idx] {
            '(' => find_match_forward(text, idx + 1, '(', ')').map(|m| (idx, m)),
            ')' => find_match_backward(text, idx, '(', ')').map(|m| (m, idx)),
            '[' => find_match_forward(text, idx + 1, '[', ']').map(|m| (idx, m)),
            ']' => find_match_backward(text, idx, '[', ']').map(|m| (m, idx)),
            '{' => find_match_forward(text, idx + 1, '{', '}').map(|m| (idx, m)),
            '}' => find_match_backward(text, idx, '{', '}').map(|m| (m, idx)),
            _ => None,
        }
    };

    if cursor_char_index < chars.len() {
        if matches!(chars[cursor_char_index], '(' | ')' | '[' | ']' | '{' | '}') {
            if let Some(pair) = check_at(cursor_char_index) {
                return Some(pair);
            }
        }
    }

    if cursor_char_index > 0 {
        if matches!(
            chars[cursor_char_index - 1],
            '(' | ')' | '[' | ']' | '{' | '}'
        ) {
            if let Some(pair) = check_at(cursor_char_index - 1) {
                return Some(pair);
            }
        }
    }

    None
}

fn find_match_forward(text: &str, start: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 1;
    for (i, c) in text.chars().enumerate().skip(start) {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn find_match_backward(text: &str, start: usize, open: char, close: char) -> Option<usize> {
    let chars: Vec<char> = text.chars().collect();
    let mut depth = 1;
    for i in (0..start).rev() {
        let c = chars[i];
        if c == close {
            depth += 1;
        } else if c == open {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn should_delete_bracket_pair(old_text: &str, new_text: &str, cursor_idx: usize) -> bool {
    if new_text.chars().count() + 1 != old_text.chars().count() {
        return false;
    }
    let old: Vec<char> = old_text.chars().collect();
    let closing = match old.get(cursor_idx) {
        Some('(') => Some(')'),
        Some('[') => Some(']'),
        Some('{') => Some('}'),
        Some('"') => Some('"'),
        Some('\'') => Some('\''),
        Some('`') => Some('`'),
        _ => None,
    };
    match closing {
        Some(closing) => old.get(cursor_idx + 1) == Some(&closing),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_code_view_with_tab_no_panic() {
        let mut state = AppState::new();
        state.new_tab();
        state.tabs[0].text = "fn main() {}".to_string();
        let pane_id = state.get_focused_pane().id;
        egui::__run_test_ui(|ui| {
            render_code_view(&mut state, ui, 0, pane_id);
        });
    }

    #[test]
    fn render_code_view_empty_tabs_no_panic() {
        let mut state = AppState::new();
        state.tabs.clear();
        let pane_id = state.get_focused_pane().id;
        egui::__run_test_ui(|ui| {
            render_code_view(&mut state, ui, 0, pane_id);
        });
    }

    #[test]
    fn context_menu_actions_are_valid_edit_actions() {
        let actions = [
            "editor: undo",
            "editor: redo",
            "editor: cut",
            "editor: copy",
            "editor: paste",
            "editor: select all",
        ];
        let mut state = AppState::new();
        let ctx = egui::Context::default();
        for action in &actions {
            crate::edit::handle_edit_action(&mut state, &ctx, action);
        }
    }

    #[test]
    fn context_menu_tracker_round_trips() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("editor_context_menu");
        assert!(!ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false));
        ctx.data_mut(|d| d.insert_temp(id, true));
        assert!(ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false));
        ctx.data_mut(|d| d.insert_temp(id, false));
        assert!(!ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false));
    }

    #[test]
    fn find_match_forward_basic_parens() {
        let text = "(a + b)";
        assert_eq!(find_match_forward(text, 1, '(', ')'), Some(6));
    }

    #[test]
    fn find_match_forward_nested() {
        let text = "(a + (b * c))";
        assert_eq!(find_match_forward(text, 1, '(', ')'), Some(12));
    }

    #[test]
    fn find_match_forward_no_match() {
        let text = "(a + b";
        assert_eq!(find_match_forward(text, 1, '(', ')'), None);
    }

    #[test]
    fn find_match_forward_at_end() {
        let text = "(";
        assert_eq!(find_match_forward(text, 0, '(', ')'), None);
    }

    #[test]
    fn find_match_backward_basic_parens() {
        let text = "(a + b)";
        assert_eq!(find_match_backward(text, 6, '(', ')'), Some(0));
    }

    #[test]
    fn find_match_backward_nested() {
        let text = "((a + b) * c)";
        assert_eq!(find_match_backward(text, 7, '(', ')'), Some(1));
    }

    #[test]
    fn find_match_backward_no_match() {
        let text = "a + b)";
        assert_eq!(find_match_backward(text, 4, '(', ')'), None);
    }

    #[test]
    fn find_match_backward_at_start() {
        let text = ")";
        assert_eq!(find_match_backward(text, 0, '(', ')'), None);
    }

    #[test]
    fn find_matching_bracket_forward_paren() {
        let text = "(hello)";
        assert_eq!(find_matching_bracket(text, 0), Some((0, 6)));
    }

    #[test]
    fn find_matching_bracket_backward_paren() {
        let text = "(hello)";
        assert_eq!(find_matching_bracket(text, 6), Some((0, 6)));
    }

    #[test]
    fn find_matching_bracket_curly() {
        let text = "{ fn main() }";
        assert_eq!(find_matching_bracket(text, 0), Some((0, 12)));
    }

    #[test]
    fn find_matching_bracket_square() {
        let text = "let v = [1, 2, 3]";
        assert_eq!(find_matching_bracket(text, 8), Some((8, 16)));
    }

    #[test]
    fn find_matching_bracket_no_bracket() {
        let text = "hello world";
        assert_eq!(find_matching_bracket(text, 3), None);
    }

    #[test]
    fn find_matching_bracket_empty_text() {
        assert_eq!(find_matching_bracket("", 0), None);
    }

    #[test]
    fn find_matching_bracket_unmatched_open() {
        let text = "(hello";
        assert_eq!(find_matching_bracket(text, 0), None);
    }

    #[test]
    fn find_matching_bracket_checks_char_before_cursor() {
        let text = "(hello)";
        assert_eq!(find_matching_bracket(text, 7), Some((0, 6)));
    }

    #[test]
    fn should_delete_bracket_pair_empty_curly() {
        assert!(should_delete_bracket_pair("{}", "}", 0));
    }

    #[test]
    fn should_delete_bracket_pair_empty_paren() {
        assert!(should_delete_bracket_pair("()", ")", 0));
    }

    #[test]
    fn should_delete_bracket_pair_empty_square() {
        assert!(should_delete_bracket_pair("[]", "]", 0));
    }

    #[test]
    fn should_delete_bracket_pair_non_empty() {
        assert!(!should_delete_bracket_pair("{a}", "a}", 0));
    }

    #[test]
    fn should_delete_bracket_pair_with_space() {
        assert!(!should_delete_bracket_pair("{ }", " }", 0));
    }

    #[test]
    fn should_delete_bracket_pair_closing_deleted() {
        assert!(!should_delete_bracket_pair("{}", "{", 1));
    }

    #[test]
    fn should_delete_bracket_pair_wrong_deleted_char() {
        assert!(!should_delete_bracket_pair("a}", "}", 0));
    }

    #[test]
    fn should_delete_bracket_pair_multibyte_paren() {
        assert!(should_delete_bracket_pair("😀()", "😀)", 1));
    }

    #[test]
    fn should_delete_bracket_pair_multibyte_quote() {
        assert!(should_delete_bracket_pair("é\"\"", "é\"", 1));
    }

    #[test]
    fn should_delete_bracket_pair_multibyte_not_pair() {
        assert!(!should_delete_bracket_pair("😀(a)", "😀a)", 1));
    }

    #[test]
    fn find_matching_bracket_multibyte_before_paren() {
        let text = "é(x)中";
        assert_eq!(find_matching_bracket(text, 4), Some((1, 3)));
    }

    #[test]
    fn find_match_forward_multibyte_paren() {
        let text = "é( | )";
        assert_eq!(find_match_forward(text, 2, '(', ')'), Some(5));
    }

    #[test]
    fn find_match_backward_multibyte_paren() {
        let text = "é( )";
        assert_eq!(find_match_backward(text, 3, '(', ')'), Some(1));
    }
}
