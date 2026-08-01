use std::cell::RefCell;

use eframe::egui;
use egui::text::CCursor;
use jereide_core::constants::{
    EDITOR_INNER_MARGIN_BOTTOM, EDITOR_INNER_MARGIN_LEFT_EXTRA, EDITOR_INNER_MARGIN_RIGHT,
    EDITOR_INNER_MARGIN_TOP, GUTTER_DIGIT_WIDTH, GUTTER_LINE_NUMBER_RIGHT_OFFSET,
    GUTTER_PADDING_LEFT, GUTTER_PADDING_RIGHT, SCROLL_BAR_WIDTH,
};
use jereide_core::AppState;
use jereide_settings::{
    BRACKET_MATCH, EDITOR_FONT_SIZE, SURFACE_BG, TEXT_CURRENT_LINE, TEXT_MUTED,
};
use jereide_text::char_index_to_line_col;

use jereide_syntax::SyntaxHighlighter;
use std::collections::HashMap;
thread_local! {
    static HIGHLIGHTERS: RefCell<HashMap<usize, SyntaxHighlighter>> = RefCell::new(HashMap::new());
}

pub fn render_code_view(state: &mut AppState, ui: &mut egui::Ui) {
    if state.tabs.is_empty() {
        return;
    }

    let ctx = ui.ctx().clone();

    let style = ui.style_mut();
    style.visuals.extreme_bg_color = SURFACE_BG;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
    style.spacing.scroll = {
        let mut s = egui::style::ScrollStyle::floating();
        s.bar_width = SCROLL_BAR_WIDTH;
        s
    };

    let active_idx = state.active_tab_index;
    let tab_id = state.tabs[active_idx].id;
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
            .or_insert_with(|| SyntaxHighlighter::new(EDITOR_FONT_SIZE, syntax_file.as_deref()));
    });

    let font_id = egui::FontId::monospace(EDITOR_FONT_SIZE);
    let cursor_line = state.tabs[active_idx].cursor_line;

    let mut layouter =
        |layouter_ui: &egui::Ui, text: &dyn jereide_editor::TextBuffer, _wrap_width: f32| {
            let text_str = text.as_str();

            let mut layout_job = HIGHLIGHTERS.with(|cache| {
                let mut c = cache.borrow_mut();
                c.get_mut(&tab_id).unwrap().highlight(text_str).clone()
            });
            layout_job.wrap.max_width = f32::INFINITY;
            layouter_ui.fonts_mut(|f| f.layout_job(layout_job))
        };

    let old_text = state.tabs[active_idx].text.clone();

    let text_edit_output = egui::ScrollArea::both()
        .auto_shrink(false)
        .show(ui, |ui| {
            let viewport = ui.max_rect().size();
            ui.set_min_size(viewport);

            let text_output = jereide_editor::TextEdit::code_editor(
                jereide_editor::TextEdit::multiline(&mut state.tabs[active_idx].text),
            )
            .id_source("editor")
            .desired_width(f32::INFINITY)
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
            .show_gutter(jereide_editor::GutterConfig {
                padding_left: GUTTER_PADDING_LEFT,
                padding_right: GUTTER_PADDING_RIGHT,
                digit_width: GUTTER_DIGIT_WIDTH,
                line_number_right_offset: GUTTER_LINE_NUMBER_RIGHT_OFFSET,
                current_line_color: TEXT_CURRENT_LINE,
                muted_color: TEXT_MUTED,
                background_color: SURFACE_BG,
                font_id: font_id.clone(),
                current_line: cursor_line,
            })
            .show(ui);

            let text_response = &text_output.response;
            let galley = text_output.galley.clone();
            let galley_pos = text_output.galley_pos;

            if let Some(cursor_range) = text_output.cursor_range {
                let tab_text = &state.tabs[active_idx].text;
                if let Some((open_idx, close_idx)) =
                    find_matching_bracket(tab_text, cursor_range.primary.index)
                {
                    let highlight_at = |char_index: usize| {
                        if char_index >= tab_text.len() {
                            return;
                        }
                        let lc = galley.layout_from_cursor(CCursor::new(char_index));
                        if let Some(placed_row) = galley.rows.get(lc.row) {
                            if let Some(glyph) = placed_row.glyphs.get(lc.column) {
                                let screen_x = galley_pos.x + placed_row.pos.x + glyph.pos.x;
                                let screen_y = galley_pos.y + placed_row.pos.y;
                                let w = glyph.advance_width;
                                let h = placed_row.height();
                                let bg_painter = egui::Painter::new(
                                    ui.ctx().clone(),
                                    egui::LayerId::new(
                                        egui::Order::Background,
                                        egui::Id::new("bracket_highlight"),
                                    ),
                                    ui.clip_rect(),
                                );
                                bg_painter.rect_filled(
                                    egui::Rect::from_min_size(
                                        egui::pos2(screen_x, screen_y),
                                        egui::vec2(w, h),
                                    ),
                                    2.0,
                                    BRACKET_MATCH.linear_multiply(0.3),
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
    state.editor_id = text_edit_output.response.id;

    if let Some(cursor_range) = text_edit_output.cursor_range {
        let cursor_idx = cursor_range.primary.index;

        // For the status bar Line/Col indicator
        let (line, col) = char_index_to_line_col(&state.tabs[active_idx].text, cursor_idx);
        state.tabs[active_idx].cursor_line = line;
        state.tabs[active_idx].cursor_col = col;

        // Auto-indent on Enter
        let text_len = state.tabs[active_idx].text.len();
        if text_len > old_text.len()
            && cursor_idx > 0
            && state.tabs[active_idx].text.as_bytes()[cursor_idx - 1] == b'\n'
            && (cursor_idx > old_text.len()
                || old_text.as_bytes().get(cursor_idx - 1) != Some(&b'\n'))
        {
            let indent = {
                let t = &state.tabs[active_idx].text;
                compute_indent(t, cursor_idx)
            };
            if !indent.is_empty() {
                state.tabs[active_idx].text.insert_str(cursor_idx, &indent);
                let new_cursor = cursor_idx + indent.len();
                if let Some(mut edit_state) =
                    jereide_editor::TextEdit::load_state(&ctx, text_edit_output.response.id)
                {
                    edit_state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::one(CCursor::new(
                            new_cursor,
                        ))));
                    edit_state.store(&ctx, text_edit_output.response.id);
                }
            }
        }

        // Auto-pair brackets and quotes
        if text_len == old_text.len() + 1 && cursor_idx > 0 {
            let bytes = state.tabs[active_idx].text.as_bytes();
            let c = bytes[cursor_idx - 1] as char;
            let (pair, is_opening) = match c {
                '(' => (Some(')'), true),
                ')' => (Some(')'), false),
                '[' => (Some(']'), true),
                ']' => (Some(']'), false),
                '{' => (Some('}'), true),
                '}' => (Some('}'), false),
                '"' => (Some('"'), true),
                '\'' => (Some('\''), true),
                '`' => (Some('`'), true),
                _ => (None, false),
            };
            if let Some(pair_char) = pair {
                let store_cursor = |edit_state: &mut jereide_editor::TextEditState| {
                    edit_state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::one(CCursor::new(
                            cursor_idx,
                        ))));
                };
                if cursor_idx < text_len && bytes[cursor_idx] as char == pair_char {
                    state.tabs[active_idx].text.remove(cursor_idx);
                    if let Some(mut edit_state) =
                        jereide_editor::TextEdit::load_state(&ctx, text_edit_output.response.id)
                    {
                        store_cursor(&mut edit_state);
                        edit_state.store(&ctx, text_edit_output.response.id);
                    }
                } else if is_opening {
                    state.tabs[active_idx].text.insert(cursor_idx, pair_char);
                    if let Some(mut edit_state) =
                        jereide_editor::TextEdit::load_state(&ctx, text_edit_output.response.id)
                    {
                        store_cursor(&mut edit_state);
                        edit_state.store(&ctx, text_edit_output.response.id);
                    }
                }
            }
        }
    }

    if !state.editor_focused {
        state.editor_focused = true;
        text_edit_output.response.request_focus();
    }
}

fn find_matching_bracket(text: &str, cursor_char_index: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();

    let check_at = |idx: usize| -> Option<(usize, usize)> {
        let c = bytes[idx] as char;
        match c {
            '(' => find_match_forward(text, idx + 1, '(', ')').map(|m| (idx, m)),
            ')' => find_match_backward(text, idx, '(', ')').map(|m| (m, idx)),
            '[' => find_match_forward(text, idx + 1, '[', ']').map(|m| (idx, m)),
            ']' => find_match_backward(text, idx, '[', ']').map(|m| (m, idx)),
            '{' => find_match_forward(text, idx + 1, '{', '}').map(|m| (idx, m)),
            '}' => find_match_backward(text, idx, '{', '}').map(|m| (m, idx)),
            _ => None,
        }
    };

    if cursor_char_index < text.len() {
        let c = bytes[cursor_char_index] as char;
        if matches!(c, '(' | ')' | '[' | ']' | '{' | '}') {
            if let Some(pair) = check_at(cursor_char_index) {
                return Some(pair);
            }
        }
    }

    if cursor_char_index > 0 {
        let c = bytes[cursor_char_index - 1] as char;
        if matches!(c, '(' | ')' | '[' | ']' | '{' | '}') {
            if let Some(pair) = check_at(cursor_char_index - 1) {
                return Some(pair);
            }
        }
    }

    None
}

fn find_match_forward(text: &str, start: usize, open: char, close: char) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 1;
    for i in start..text.len() {
        let c = bytes[i] as char;
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
    let bytes = text.as_bytes();
    let mut depth = 1;
    for i in (0..start).rev() {
        let c = bytes[i] as char;
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

fn compute_indent(text: &str, cursor_idx: usize) -> String {
    let before = &text[..cursor_idx];
    let prev_start = before[..before.len() - 1]
        .rfind('\n')
        .map(|pos| pos + 1)
        .unwrap_or(0);
    let prev_line = &text[prev_start..cursor_idx - 1];
    let indent_len = prev_line.len() - prev_line.trim_start().len();
    prev_line[..indent_len].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_code_view_with_tab_no_panic() {
        let mut state = AppState::new();
        state.new_tab();
        state.tabs[0].text = "fn main() {}".to_string();
        egui::__run_test_ui(|ui| {
            render_code_view(&mut state, ui);
        });
    }

    #[test]
    fn render_code_view_empty_tabs_no_panic() {
        let mut state = AppState::new();
        state.tabs.clear();
        egui::__run_test_ui(|ui| {
            render_code_view(&mut state, ui);
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
        // cursor at index 7 (past the closing paren) should check char before
        assert_eq!(find_matching_bracket(text, 7), Some((0, 6)));
    }

    #[test]
    fn compute_indent_basic() {
        let text = "    hello\n";
        assert_eq!(compute_indent(text, 10), "    ");
    }

    #[test]
    fn compute_indent_two_levels() {
        let text = "        hello\n";
        assert_eq!(compute_indent(text, 14), "        ");
    }

    #[test]
    fn compute_indent_tabs() {
        let text = "\t\thello\n";
        assert_eq!(compute_indent(text, 8), "\t\t");
    }

    #[test]
    fn compute_indent_no_indent() {
        let text = "hello\n";
        assert_eq!(compute_indent(text, 6), "");
    }

    #[test]
    fn compute_indent_mixed() {
        let text = "  \t  hello\n";
        assert_eq!(compute_indent(text, 11), "  \t  ");
    }
}
