use eframe::egui;
use jereide_settings::dialog_width;

pub struct WidgetPalette {
    filter: String,
    search_focused: bool,
    previous_focus: Option<egui::Id>,
    was_open: bool,
    toggle_state: bool,
}

impl WidgetPalette {
    pub fn new() -> Self {
        Self {
            filter: String::new(),
            search_focused: false,
            previous_focus: None,
            was_open: false,
            toggle_state: false,
        }
    }

    pub fn filter(&self) -> &str {
        &self.filter
    }

    pub fn toggle_state(&mut self) -> &mut bool {
        &mut self.toggle_state
    }

    pub fn render<F>(&mut self, ctx: &egui::Context, title: &str, open: &mut bool, mut content: F)
    where
        F: FnMut(&mut Self, &mut egui::Ui, &str),
    {
        if !*open {
            if self.was_open {
                self.was_open = false;
                if let Some(id) = self.previous_focus {
                    ctx.memory_mut(|m| m.request_focus(id));
                }
            }
            return;
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
            return;
        }

        let dim_rect = ctx.viewport_rect();
        let clicked_outside = egui::Area::new(egui::Id::new("widget_palette_dismiss"))
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
            return;
        }

        let window_width = dialog_width() + 120.0;

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
                        .hint_text("Search…")
                        .desired_width(f32::INFINITY)
                        .return_key(None),
                );
                if !self.search_focused {
                    resp.request_focus();
                    self.search_focused = true;
                }

                ui.add_space(6.0);

                let filter = self.filter.clone();
                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        content(self, ui, &filter);
                    });
            });
    }
}
