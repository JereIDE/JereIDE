use eframe::egui;
use jereide_settings::{ACCENT, DIALOG_WIDTH, HOVER_BG, SURFACE_BG, TEXT_DEFAULT};

pub struct PaletteItem {
    pub code: &'static str,
}

pub struct Palette {
    items: Vec<PaletteItem>,
    filter: String,
    selected_index: usize,
    search_focused: bool,
    last_mouse_pos: Option<egui::Pos2>,
    previous_focus: Option<egui::Id>,
    was_open: bool,
}

impl Palette {
    pub fn new(items: Vec<PaletteItem>) -> Self {
        Self {
            items,
            filter: String::new(),
            selected_index: 0,
            search_focused: false,
            last_mouse_pos: None,
            previous_focus: None,
            was_open: false,
        }
    }

    fn filtered_indices(&self) -> Vec<usize> {
        if self.filter.is_empty() {
            (0..self.items.len()).collect()
        } else {
            let lower = self.filter.to_lowercase();
            self.items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.code.to_lowercase().contains(&lower))
                .map(|(i, _)| i)
                .collect()
        }
    }

    pub fn render(&mut self, ctx: &egui::Context, title: &str, open: &mut bool) -> Option<&'static str> {
        if !*open {
            if self.was_open {
                self.was_open = false;
                if let Some(id) = self.previous_focus {
                    ctx.memory_mut(|m| m.request_focus(id));
                }
            }
            return None;
        }

        self.was_open = true;
        if !self.search_focused {
            self.previous_focus = ctx.memory(|m| m.focused());
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            *open = false;
            if let Some(id) = self.previous_focus {
                ctx.memory_mut(|m| m.request_focus(id));
            }
            return None;
        }

        let mut chosen: Option<&'static str> = None;

        let dim_rect = ctx.viewport_rect();
        let clicked_outside = egui::Area::new(egui::Id::new("palette_dismiss"))
            .order(egui::Order::Foreground)
            .fixed_pos(dim_rect.min)
            .show(ctx, |ui| ui.allocate_rect(dim_rect, egui::Sense::click()))
            .inner
            .clicked();

        if clicked_outside {
            *open = false;
            if let Some(id) = self.previous_focus {
                ctx.memory_mut(|m| m.request_focus(id));
            }
            return None;
        }

        let window_width = DIALOG_WIDTH + 120.0;

        egui::Window::new(title)
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .fixed_size([window_width, 300.0])
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                let resp = ui.add_sized(
                    egui::vec2(ui.available_width(), 0.0),
                    egui::TextEdit::singleline(&mut self.filter)
                        .hint_text("Search...")
                        .desired_width(f32::INFINITY)
                        .return_key(None),
                );
                if !self.search_focused {
                    resp.request_focus();
                    self.search_focused = true;
                }

                ui.add_space(6.0);

                let indices = self.filtered_indices();
                if self.selected_index >= indices.len() && !indices.is_empty() {
                    self.selected_index = indices.len() - 1;
                }

                let nav_up = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));
                let nav_down = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
                let confirm = ctx.input(|i| i.key_pressed(egui::Key::Enter));

                if nav_up {
                    self.selected_index = self.selected_index.saturating_sub(1);
                }
                if nav_down {
                    self.selected_index =
                        (self.selected_index + 1).min(indices.len().saturating_sub(1));
                }

                let mouse_pos = ctx.input(|i| i.pointer.hover_pos());
                let mouse_moved = mouse_pos != self.last_mouse_pos;
                if mouse_moved {
                    self.last_mouse_pos = mouse_pos;
                }

                let nav_key = nav_up || nav_down;
                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .show(ui, |ui| {
                        let mut selected_rect: Option<egui::Rect> = None;
                        for (i, &item_idx) in indices.iter().enumerate() {
                            let item = &self.items[item_idx];
                            let selected = i == self.selected_index;
                            let text_color = if selected { SURFACE_BG } else { TEXT_DEFAULT };

                            let (rect, resp) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 28.0),
                                egui::Sense::click(),
                            );

                            if selected {
                                ui.painter().rect_filled(rect, 4.0, ACCENT);
                            } else if resp.hovered() {
                                ui.painter().rect_filled(rect, 4.0, HOVER_BG);
                            }

                            if selected {
                                selected_rect = Some(rect);
                            }

                            if resp.hovered() && mouse_moved && !nav_key {
                                self.selected_index = i;
                            }

                            ui.painter().text(
                                egui::pos2(rect.min.x + 8.0, rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                item.code,
                                egui::FontId::proportional(14.0),
                                text_color,
                            );

                            if resp.clicked() {
                                chosen = Some(item.code);
                            }
                        }
                        if nav_key {
                            if let Some(rect) = selected_rect {
                                ui.scroll_to_rect(rect, None);
                            }
                        }
                    });

                if confirm && !indices.is_empty() {
                    chosen = Some(self.items[indices[self.selected_index]].code);
                }
            });

        if chosen.is_some() {
            *open = false;
            if let Some(id) = self.previous_focus {
                ctx.memory_mut(|m| m.request_focus(id));
            }
        }

        chosen
    }
}
