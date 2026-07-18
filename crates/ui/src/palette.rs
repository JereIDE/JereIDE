use eframe::egui;
use jereide_settings::{ACCENT, DIALOG_WIDTH, SURFACE_BG, TEXT_DEFAULT, TEXT_MUTED};

pub struct PaletteItem<T> {
    pub label: &'static str,
    pub description: &'static str,
    pub shortcut: &'static str,
    pub data: T,
}

pub struct Palette<T> {
    items: Vec<PaletteItem<T>>,
    filter: String,
    selected_index: usize,
}

impl<T> Palette<T> {
    pub fn new(items: Vec<PaletteItem<T>>) -> Self {
        Self {
            items,
            filter: String::new(),
            selected_index: 0,
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
                .filter(|(_, item)| {
                    item.label.to_lowercase().contains(&lower)
                        || item.description.to_lowercase().contains(&lower)
                })
                .map(|(i, _)| i)
                .collect()
        }
    }

    pub fn render(&mut self, ctx: &egui::Context, title: &str, open: &mut bool) -> Option<T>
    where
        T: Clone,
    {
        if !*open {
            return None;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            *open = false;
            return None;
        }

        let mut chosen: Option<T> = None;

        let dim_rect = ctx.viewport_rect();
        let clicked_outside = egui::Area::new(egui::Id::new("palette_dismiss"))
            .order(egui::Order::Foreground)
            .fixed_pos(dim_rect.min)
            .show(ctx, |ui| ui.allocate_rect(dim_rect, egui::Sense::click()))
            .inner
            .clicked();

        if clicked_outside {
            *open = false;
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
                        .desired_width(f32::INFINITY),
                );
                resp.request_focus();

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

                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .show(ui, |ui| {
                        for (i, &item_idx) in indices.iter().enumerate() {
                            let item = &self.items[item_idx];
                            let selected = i == self.selected_index;
                            let bg = if selected { ACCENT } else { SURFACE_BG };
                            let text_color = if selected { SURFACE_BG } else { TEXT_DEFAULT };
                            let desc_color = if selected { SURFACE_BG } else { TEXT_MUTED };

                            let resp = ui.add_sized(
                                egui::vec2(ui.available_width(), 28.0),
                                egui::Button::new(
                                    egui::RichText::new(item.label).color(text_color),
                                )
                                .fill(bg)
                                .frame(selected),
                            );

                            if !item.shortcut.is_empty() {
                                ui.painter().text(
                                    egui::pos2(resp.rect.right() - 8.0, resp.rect.center().y),
                                    egui::Align2::RIGHT_CENTER,
                                    item.shortcut,
                                    egui::FontId::proportional(11.0),
                                    desc_color,
                                );
                            }

                            if resp.clicked() {
                                chosen = Some(item.data.clone());
                            }
                        }
                    });

                if confirm && !indices.is_empty() {
                    chosen = Some(self.items[indices[self.selected_index]].data.clone());
                }
            });

        if chosen.is_some() {
            *open = false;
        }

        chosen
    }
}
