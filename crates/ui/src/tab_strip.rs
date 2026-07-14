use std::sync::Arc;

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use jereide_core::constants::{
    TAB_BORDER_WIDTH, TAB_CLOSE_BTN_RADIUS, TAB_CLOSE_BTN_SIZE, TAB_CLOSE_BTN_SPACING,
    TAB_CLOSE_ICON_HALF, TAB_CLOSE_STROKE, TAB_MODIFIED_DOT_RADIUS, TAB_PAD_LEFT, TAB_PAD_RIGHT,
    TAB_STRIP_HEIGHT,
};
use jereide_core::AppState;
use jereide_settings::{
    ACCENT, BORDER, ELEVATED_BG, HOVER_BG, SURFACE_BG, TAB_FONT_SIZE, TEXT_DEFAULT, TEXT_PRIMARY,
    TEXT_SECONDARY,
};

struct TabLayout {
    rect: Rect,
    close_rect: Rect,
    text_pos: Pos2,
    has_dot: bool,
    dot_pos: Pos2,
    galley: Arc<egui::Galley>,
}

pub fn render_tab_strip(state: &mut AppState, ui: &mut egui::Ui) {
    let available = ui.available_size();
    let (strip_rect, strip_resp) =
        ui.allocate_exact_size(Vec2::new(available.x, TAB_STRIP_HEIGHT), Sense::click());
    let tab_bottom = strip_rect.bottom();
    let tab_top = strip_rect.top();

    let font_id = FontId::monospace(TAB_FONT_SIZE);

    let mut layouts: Vec<TabLayout> = Vec::with_capacity(state.tabs.len());
    let mut cursor_x = strip_rect.left();

    for idx in 0..state.tabs.len() {
        let tab = &state.tabs[idx];
        let name = tab.file_name();
        let galley = ui.fonts_mut(|f| {
            f.layout_job(egui::text::LayoutJob::simple(
                name.clone(),
                font_id.clone(),
                Color32::WHITE,
                f32::INFINITY,
            ))
        });
        let text_w = galley.size().x;
        let text_h = galley.size().y;

        let has_dot = tab.is_modified();
        let dot_extra = if has_dot {
            TAB_MODIFIED_DOT_RADIUS * 2.0
        } else {
            0.0
        };

        let left_extra = TAB_PAD_LEFT + dot_extra;
        let right_extra = TAB_CLOSE_BTN_SPACING + TAB_CLOSE_BTN_SIZE + TAB_PAD_RIGHT;
        let side = left_extra.max(right_extra);
        let tab_w = side + text_w + side;

        let tab_rect = Rect::from_min_size(
            Pos2::new(cursor_x, tab_top),
            Vec2::new(tab_w, TAB_STRIP_HEIGHT),
        );

        let text_pos = Pos2::new(
            tab_rect.center().x - text_w / 2.0,
            tab_rect.center().y - text_h / 2.0,
        );

        let dot_pos = Pos2::new(tab_rect.left() + side / 2.0, tab_rect.center().y);

        let close_rect = Rect::from_center_size(
            Pos2::new(
                tab_rect.right() - TAB_PAD_RIGHT - TAB_CLOSE_BTN_SIZE / 2.0,
                tab_rect.center().y,
            ),
            Vec2::splat(TAB_CLOSE_BTN_SIZE),
        );

        layouts.push(TabLayout {
            rect: tab_rect,
            close_rect,
            text_pos,
            has_dot,
            dot_pos,
            galley,
        });
        cursor_x = tab_rect.right();
    }

    let mut click_tab: Option<usize> = None;
    let mut close_tab: Option<usize> = None;
    let mut close_hovered = vec![false; state.tabs.len()];
    let mut tab_hovered = vec![false; state.tabs.len()];

    for idx in 0..state.tabs.len() {
        let tab_id = egui::Id::new(("tab", idx));
        let close_id = egui::Id::new(("close", idx));

        let tab_resp = ui.interact(layouts[idx].rect, tab_id, Sense::click());
        let close_resp = ui.interact(layouts[idx].close_rect, close_id, Sense::click());

        close_hovered[idx] = close_resp.hovered();
        tab_hovered[idx] = tab_resp.hovered() || close_resp.hovered();

        if close_resp.clicked() {
            close_tab = Some(idx);
        } else if tab_resp.clicked() {
            click_tab = Some(idx);
        }
    }

    let painter = ui.painter();

    painter.rect_filled(strip_rect, 0.0, ELEVATED_BG);

    for idx in 0..state.tabs.len() {
        let layout = &layouts[idx];
        let is_active = idx == state.active_tab_index;
        let bg = if is_active { SURFACE_BG } else { ELEVATED_BG };

        painter.rect_filled(layout.rect, 0.0, bg);

        let text_color = if is_active {
            TEXT_PRIMARY
        } else {
            TEXT_SECONDARY
        };
        painter.galley_with_override_text_color(layout.text_pos, layout.galley.clone(), text_color);

        if layout.has_dot {
            painter.circle_filled(layout.dot_pos, TAB_MODIFIED_DOT_RADIUS, ACCENT);
        }

        if tab_hovered[idx] {
            if close_hovered[idx] {
                painter.rect_filled(layout.close_rect, TAB_CLOSE_BTN_RADIUS, HOVER_BG);
            }
            let icon_color = if close_hovered[idx] {
                TEXT_DEFAULT
            } else {
                TEXT_PRIMARY
            };
            let cx = layout.close_rect.center().x;
            let cy = layout.close_rect.center().y;
            painter.line_segment(
                [
                    Pos2::new(cx - TAB_CLOSE_ICON_HALF, cy - TAB_CLOSE_ICON_HALF),
                    Pos2::new(cx + TAB_CLOSE_ICON_HALF, cy + TAB_CLOSE_ICON_HALF),
                ],
                Stroke::new(TAB_CLOSE_STROKE, icon_color),
            );
            painter.line_segment(
                [
                    Pos2::new(cx + TAB_CLOSE_ICON_HALF, cy - TAB_CLOSE_ICON_HALF),
                    Pos2::new(cx - TAB_CLOSE_ICON_HALF, cy + TAB_CLOSE_ICON_HALF),
                ],
                Stroke::new(TAB_CLOSE_STROKE, icon_color),
            );
        }
    }

    // Top border across the full strip width
    painter.rect_filled(
        Rect::from_min_size(
            Pos2::new(strip_rect.left(), strip_rect.top()),
            Vec2::new(strip_rect.width(), TAB_BORDER_WIDTH),
        ),
        0.0,
        BORDER,
    );

    painter.rect_filled(
        Rect::from_min_size(
            Pos2::new(strip_rect.left(), tab_bottom - TAB_BORDER_WIDTH),
            Vec2::new(strip_rect.width(), TAB_BORDER_WIDTH),
        ),
        0.0,
        BORDER,
    );

    if let Some(active) = layouts.get(state.active_tab_index) {
        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(active.rect.left(), tab_bottom - TAB_BORDER_WIDTH),
                Vec2::new(active.rect.width(), TAB_BORDER_WIDTH),
            ),
            0.0,
            SURFACE_BG,
        );
    }

    for idx in 0..state.tabs.len() {
        painter.vline(
            layouts[idx].rect.left(),
            layouts[idx].rect.y_range(),
            Stroke::new(TAB_BORDER_WIDTH, BORDER),
        );
    }
    if let Some(last) = layouts.last() {
        painter.vline(
            last.rect.right(),
            last.rect.y_range(),
            Stroke::new(TAB_BORDER_WIDTH, BORDER),
        );
    }

    if strip_resp.double_clicked() {
        state.new_tab();
    }
    if let Some(idx) = close_tab {
        if state.tabs[idx].is_modified() {
            state.pending_close_index = Some(idx);
        } else {
            state.close_tab(idx);
        }
    }
    if let Some(idx) = click_tab {
        state.active_tab_index = idx;
    }
}
