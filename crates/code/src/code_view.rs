use std::cell::RefCell;
use std::sync::Arc;

use eframe::egui;
use jereide_core::constants::{
    EDITOR_INNER_MARGIN_BOTTOM, EDITOR_INNER_MARGIN_LEFT_EXTRA, EDITOR_INNER_MARGIN_RIGHT,
    EDITOR_INNER_MARGIN_TOP, GUTTER_DIGIT_WIDTH, GUTTER_LINE_NUMBER_RIGHT_OFFSET,
    GUTTER_PADDING_LEFT, GUTTER_PADDING_RIGHT, SCROLL_BAR_WIDTH,
};
use jereide_core::AppState;
use jereide_settings::{EDITOR_FONT_SIZE, SURFACE_BG, TEXT_CURRENT_LINE, TEXT_DEFAULT, TEXT_MUTED};
use jereide_text::char_index_to_line_col;

// -- Syntax highlighter removed for performance --
// use std::collections::HashMap;
// use jereide_syntax::SyntaxHighlighter;
// thread_local! {
//     static HIGHLIGHTERS: RefCell<HashMap<usize, SyntaxHighlighter>> = RefCell::new(HashMap::new());
// }

fn visual_line_count(text: &str) -> usize {
    if text.is_empty() {
        1
    } else {
        text.as_bytes().iter().filter(|&&b| b == b'\n').count() + 1
    }
}

fn digit_count(mut n: usize) -> usize {
    let mut count = 1;
    while n >= 10 {
        n /= 10;
        count += 1;
    }
    count
}

fn gutter_width(line_count: usize) -> f32 {
    GUTTER_PADDING_LEFT + digit_count(line_count) as f32 * GUTTER_DIGIT_WIDTH + GUTTER_PADDING_RIGHT
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
    let _tab_id = state.tabs[active_idx].id;
    // -- Extension-based syntax selection removed --
    // let extension: Option<String> = state.tabs[active_idx]
    //     .file_path.as_ref()
    //     .and_then(|p| std::path::Path::new(p).extension())
    //     .and_then(|ext| ext.to_str()).map(|s| s.to_string());

    // -- Highlighter cache removed --
    // let valid_ids: std::collections::HashSet<usize> = state.tabs.iter().map(|t| t.id).collect();
    // HIGHLIGHTERS.with(|cache| {
    //     let mut cache = cache.borrow_mut();
    //     cache.retain(|id, _| valid_ids.contains(id));
    //     cache.entry(tab_id).or_insert_with(|| SyntaxHighlighter::new(EDITOR_FONT_SIZE, extension.as_deref()));
    // });

    let font_id = egui::FontId::monospace(EDITOR_FONT_SIZE);
    let line_count = visual_line_count(&state.tabs[active_idx].text);
    let gutter_w = gutter_width(line_count);
    let cursor_line = state.tabs[active_idx].cursor_line;

    let last_galley: RefCell<Option<Arc<egui::Galley>>> = RefCell::new(None);

    let mut layouter =
        |layouter_ui: &egui::Ui, text: &dyn egui::widgets::TextBuffer, wrap_width: f32| {
            let text_str = text.as_str();

            // Plain layout job — no syntax highlighting
            let mut layout_job = egui::text::LayoutJob {
                text: text_str.to_string(),
                ..Default::default()
            };
            if !text_str.is_empty() {
                layout_job.sections.push(egui::text::LayoutSection {
                    leading_space: 0.0,
                    byte_range: 0..text_str.len(),
                    format: egui::text::TextFormat::simple(font_id.clone(), TEXT_DEFAULT),
                });
            }

            // Horizontal scrolling — no wrapping, so gutter lines match logical lines
            layout_job.wrap.max_width = f32::INFINITY;
            let galley = layouter_ui.fonts_mut(|f| f.layout_job(layout_job));
            *last_galley.borrow_mut() = Some(galley.clone());
            galley
        };

    // -- Original layouter with syntax highlighting --
    // let mut layouter = |layouter_ui: &egui::Ui,
    //                     text: &dyn egui::widgets::TextBuffer,
    //                     wrap_width: f32| {
    //     let text_str = text.as_str();
    //     let mut layout_job = HIGHLIGHTERS.with(|cache| {
    //         let mut c = cache.borrow_mut();
    //         c.get_mut(&tab_id).unwrap().highlight(text_str)
    //     });
    //     layout_job.wrap.max_width = wrap_width;
    //     let galley = layouter_ui.fonts_mut(|f| f.layout_job(layout_job));
    //     *last_galley.borrow_mut() = Some(galley.clone());
    //     galley
    // };

    let response = egui::ScrollArea::both()
        .auto_shrink(false)
        .show(ui, |ui| {
            let viewport = ui.max_rect().size();
            ui.set_min_size(viewport);

            let widget_top = ui.cursor().min.y;

            let horiz = ui.horizontal_top(|ui| {
                let (gutter_rect, gutter_resp) =
                    ui.allocate_exact_size(egui::vec2(gutter_w, 0.0), egui::Sense::click());

                let text_response = ui.add(
                    egui::TextEdit::code_editor(egui::TextEdit::multiline(
                        &mut state.tabs[active_idx].text,
                    ))
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
                    .layouter(&mut layouter),
                );

                (gutter_rect, gutter_resp, text_response)
            });

            let (gutter_rect, gutter_resp, text_response) = horiz.inner;
            let text_alloc = text_response.rect;

            let g_bottom = text_alloc.bottom().max(ui.clip_rect().bottom());
            let painter = ui.painter();
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(gutter_rect.left(), gutter_rect.top()),
                    egui::vec2(gutter_w, g_bottom - gutter_rect.top()),
                ),
                0.0,
                SURFACE_BG,
            );

            let line_start_y = widget_top + EDITOR_INNER_MARGIN_TOP as f32;
            if let Some(galley) = last_galley.borrow().as_ref() {
                for (i, row) in galley.rows.iter().enumerate() {
                    let line_y = line_start_y + row.pos.y;
                    let line_num = i + 1;
                    let is_current = line_num == cursor_line;
                    let color = if is_current {
                        TEXT_CURRENT_LINE
                    } else {
                        TEXT_MUTED
                    };
                    painter.text(
                        egui::pos2(gutter_w - GUTTER_LINE_NUMBER_RIGHT_OFFSET, line_y),
                        egui::Align2::RIGHT_TOP,
                        line_num.to_string(),
                        font_id.clone(),
                        color,
                    );
                }
            }

            // Fill up the whole Y available space
            let remaining = ui.available_size();
            if remaining.y > 0.0 {
                let (_, bg) = ui.allocate_exact_size(remaining, egui::Sense::click());
                if bg.clicked() || gutter_resp.clicked() {
                    text_response.request_focus();
                }
                bg.on_hover_cursor(egui::CursorIcon::Text);
            }
            gutter_resp.on_hover_cursor(egui::CursorIcon::Text);

            text_response
        })
        .inner;
    state.editor_id = response.id;
    // For the status bar Line/Col indicator
    if let Some(edit_state) = egui::TextEdit::load_state(&ctx, response.id) {
        if let Some(range) = edit_state.cursor.char_range() {
            let (line, col) =
                char_index_to_line_col(&state.tabs[active_idx].text, range.primary.index);
            state.tabs[active_idx].cursor_line = line;
            state.tabs[active_idx].cursor_col = col;
        }
    }

    if !state.editor_focused {
        state.editor_focused = true;
        response.request_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_line_count_empty() {
        assert_eq!(visual_line_count(""), 1);
    }

    #[test]
    fn visual_line_count_single_line() {
        assert_eq!(visual_line_count("hello"), 1);
    }

    #[test]
    fn visual_line_count_multi_line() {
        assert_eq!(visual_line_count("line1\nline2\nline3"), 3);
    }

    #[test]
    fn visual_line_count_trailing_newline() {
        assert_eq!(visual_line_count("line1\nline2\n"), 3);
    }

    #[test]
    fn gutter_width_single_digit() {
        let w = gutter_width(5);
        assert!(w.is_finite() && w > 0.0);
    }

    #[test]
    fn gutter_width_double_digit() {
        let w_single = gutter_width(5);
        let w_double = gutter_width(50);
        assert!(w_double > w_single);
    }

    #[test]
    fn gutter_width_triple_digit() {
        let w_double = gutter_width(50);
        let w_triple = gutter_width(500);
        assert!(w_triple > w_double);
    }

    #[test]
    fn gutter_width_exact_powers_of_ten() {
        let w_9 = gutter_width(9);
        let w_10 = gutter_width(10);
        assert!(w_10 > w_9);
    }
}
