use std::sync::Arc;

use eframe::egui::{
    self, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2, WidgetInfo, WidgetType,
};
use jereide_core::AppState;
use jereide_core::constants::{
    TAB_BORDER_WIDTH, TAB_CLOSE_BTN_RADIUS, TAB_CLOSE_BTN_SIZE, TAB_CLOSE_BTN_SPACING,
    TAB_CLOSE_ICON_HALF, TAB_CLOSE_STROKE, TAB_MODIFIED_DOT_RADIUS, TAB_PAD_LEFT, TAB_PAD_RIGHT,
    TAB_STRIP_HEIGHT,
};
use jereide_settings::{
    accent, border, elevated_bg, hover_bg, surface_bg, tab_font_size, text_default, text_primary,
    text_secondary,
};

struct TabLayout {
    rect: Rect,
    close_rect: Rect,
    text_pos: Pos2,
    has_dot: bool,
    dot_pos: Pos2,
    is_read_only: bool,
    lock_rect: Rect,
    galley: Arc<egui::Galley>,
}

const TAB_LOCK_ICON_SIZE: f32 = 12.0;

fn lock_image() -> egui::Image<'static> {
    let bytes = jereide_data::data_dir()
        .map(|dir| std::fs::read(dir.join("lock.png")).unwrap_or_default())
        .unwrap_or_default();
    egui::Image::from_bytes("lock.png", bytes).max_height(TAB_LOCK_ICON_SIZE)
}

pub fn render_tab_strip(state: &mut AppState, ui: &mut egui::Ui) {
    let sidebar_open = state.sidebar_open;
    let available_w = ui.available_width();

    let font_id = FontId::monospace(tab_font_size());
    let lock_image = lock_image();

    // Layout tabs in content coordinates (starting from x = 0).
    let mut layouts: Vec<TabLayout> = Vec::with_capacity(state.tabs.len());
    let mut cursor_x = 0.0;

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
        let is_read_only = tab.read_only;
        let dot_extra = if has_dot {
            TAB_MODIFIED_DOT_RADIUS * 2.0
        } else if is_read_only {
            TAB_LOCK_ICON_SIZE
        } else {
            0.0
        };

        let left_extra = TAB_PAD_LEFT + dot_extra;
        let right_extra = TAB_CLOSE_BTN_SPACING + TAB_CLOSE_BTN_SIZE + TAB_PAD_RIGHT;
        let side = left_extra.max(right_extra);
        let tab_w = side + text_w + side;

        let tab_rect =
            Rect::from_min_size(Pos2::new(cursor_x, 0.0), Vec2::new(tab_w, TAB_STRIP_HEIGHT));

        let text_pos = Pos2::new(
            tab_rect.center().x - text_w / 2.0,
            tab_rect.center().y - text_h / 2.0,
        );

        let dot_pos = Pos2::new(tab_rect.left() + side / 2.0, tab_rect.center().y);

        let lock_rect = Rect::from_center_size(
            Pos2::new(tab_rect.left() + side / 2.0, tab_rect.center().y),
            Vec2::splat(TAB_LOCK_ICON_SIZE),
        );

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
            is_read_only,
            lock_rect,
            galley,
        });
        cursor_x = tab_rect.right();
    }
    let total_w = cursor_x;

    let mut click_tab: Option<usize> = None;
    let mut close_tab: Option<usize> = None;
    let mut new_tab = false;

    egui::ScrollArea::horizontal()
        .id_salt("tab_strip_scroll")
        .max_height(TAB_STRIP_HEIGHT)
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            let origin = ui.cursor().min;
            let content_w = total_w.max(available_w);
            let content_rect = Rect::from_min_size(origin, Vec2::new(content_w, TAB_STRIP_HEIGHT));
            let (_, content_resp) = ui.allocate_exact_size(content_rect.size(), Sense::click());

            let painter = ui.painter();
            painter.rect_filled(content_rect, 0.0, elevated_bg());

            for idx in 0..state.tabs.len() {
                let l = &layouts[idx];
                let rect = Rect::from_min_size(origin + l.rect.min.to_vec2(), l.rect.size());
                let close_rect =
                    Rect::from_min_size(origin + l.close_rect.min.to_vec2(), l.close_rect.size());
                let text_pos = origin + l.text_pos.to_vec2();
                let dot_pos = origin + l.dot_pos.to_vec2();
                let lock_rect =
                    Rect::from_min_size(origin + l.lock_rect.min.to_vec2(), l.lock_rect.size());

                let is_active = idx == state.active_tab_index;
                let bg = if is_active {
                    surface_bg()
                } else {
                    elevated_bg()
                };

                painter.rect_filled(rect, 0.0, bg);

                let text_color = if is_active {
                    text_primary()
                } else {
                    text_secondary()
                };
                painter.galley_with_override_text_color(text_pos, l.galley.clone(), text_color);

                if l.has_dot {
                    painter.circle_filled(dot_pos, TAB_MODIFIED_DOT_RADIUS, accent());
                } else if l.is_read_only {
                    lock_image.paint_at(ui, lock_rect);
                }

                let tab_resp = ui
                    .interact(rect, egui::Id::new(("tab", idx)), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                let close_resp = ui
                    .interact(close_rect, egui::Id::new(("close", idx)), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("Close Tab");

                tab_resp.widget_info(|| {
                    WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), l.galley.text())
                });
                close_resp.widget_info(|| {
                    WidgetInfo::labeled(
                        WidgetType::Button,
                        ui.is_enabled(),
                        format!("Close {}", l.galley.text()),
                    )
                });

                let close_h = close_resp.hovered();
                let tab_h = tab_resp.hovered() || close_h;

                if close_resp.clicked() {
                    close_tab = Some(idx);
                } else if tab_resp.clicked() {
                    click_tab = Some(idx);
                }

                if tab_h {
                    if close_h {
                        painter.rect_filled(close_rect, TAB_CLOSE_BTN_RADIUS, hover_bg());
                    }
                    let icon_color = if close_h {
                        text_default()
                    } else {
                        text_primary()
                    };
                    let cx = close_rect.center().x;
                    let cy = close_rect.center().y;
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

            for idx in 0..state.tabs.len() {
                if idx == 0 && sidebar_open {
                    continue;
                }
                painter.vline(
                    origin.x + layouts[idx].rect.left(),
                    egui::Rangef::new(origin.y, origin.y + TAB_STRIP_HEIGHT),
                    Stroke::new(TAB_BORDER_WIDTH, border()),
                );
            }
            if let Some(last) = layouts.last() {
                painter.vline(
                    origin.x + last.rect.right(),
                    egui::Rangef::new(origin.y, origin.y + TAB_STRIP_HEIGHT),
                    Stroke::new(TAB_BORDER_WIDTH, border()),
                );
            }

            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(
                        content_rect.left(),
                        content_rect.bottom() - TAB_BORDER_WIDTH,
                    ),
                    egui::vec2(content_rect.width(), TAB_BORDER_WIDTH),
                ),
                0.0,
                border(),
            );

            if let Some(active) = layouts.get(state.active_tab_index) {
                let active_rect =
                    Rect::from_min_size(origin + active.rect.min.to_vec2(), active.rect.size());
                painter.rect_filled(
                    Rect::from_min_size(
                        Pos2::new(active_rect.left(), active_rect.bottom() - TAB_BORDER_WIDTH),
                        Vec2::new(active_rect.width(), TAB_BORDER_WIDTH),
                    ),
                    0.0,
                    surface_bg(),
                );
            }

            if content_resp.double_clicked() {
                new_tab = true;
            }
        });

    if new_tab {
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
