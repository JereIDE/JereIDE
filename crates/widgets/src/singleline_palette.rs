use eframe::egui;
use jereide_settings::DIALOG_WIDTH;

pub struct SinglelinePalette {
    input: String,
    previous_focus: Option<egui::Id>,
    was_open: bool,
}

impl SinglelinePalette {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            previous_focus: None,
            was_open: false,
        }
    }

    /// Renders a centered palette with a single text entry. Returns the
    /// entered text when the user submits with Enter, or `None` while the
    /// palette is still open.
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        title: &str,
        hint: &str,
        open: &mut bool,
    ) -> Option<String> {
        if !*open {
            if self.was_open {
                self.was_open = false;
                self.input.clear();
                if let Some(id) = self.previous_focus {
                    ctx.memory_mut(|m| m.request_focus(id));
                }
            }
            return None;
        }

        let fresh_open = !self.was_open;
        self.was_open = true;
        if fresh_open {
            self.previous_focus = ctx.memory(|m| m.focused());
        }

        let close = |ctx: &egui::Context,
                     previous_focus: Option<egui::Id>,
                     input: &mut String,
                     open: &mut bool| {
            *open = false;
            input.clear();
            if let Some(id) = previous_focus {
                ctx.memory_mut(|m| m.request_focus(id));
            }
        };

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close(ctx, self.previous_focus, &mut self.input, open);
            return None;
        }

        let dim_rect = ctx.viewport_rect();
        let clicked_outside = egui::Area::new(egui::Id::new("singleline_palette_dismiss"))
            .order(egui::Order::Foreground)
            .fixed_pos(dim_rect.min)
            .show(ctx, |ui| ui.allocate_rect(dim_rect, egui::Sense::click()))
            .inner
            .clicked();

        if clicked_outside {
            close(ctx, self.previous_focus, &mut self.input, open);
            return None;
        }

        let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
        let window_width = DIALOG_WIDTH + 120.0;
        let input_id = egui::Id::new(title).with("input");

        egui::Window::new(title)
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .fixed_size([window_width, 0.0])
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(window_width);
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.input)
                        .id_source(input_id)
                        .hint_text(hint)
                        .desired_width(f32::INFINITY),
                );
                resp.request_focus();
            });

        if enter_pressed {
            let result = self.input.clone();
            close(ctx, self.previous_focus, &mut self.input, open);
            return Some(result);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_empty_input() {
        let palette = SinglelinePalette::new();
        assert_eq!(palette.input, "");
        assert!(palette.previous_focus.is_none());
        assert!(!palette.was_open);
    }

    #[test]
    fn closed_palette_returns_none() {
        let mut palette = SinglelinePalette::new();
        let ctx = egui::Context::default();
        let mut open = false;
        assert_eq!(palette.render(&ctx, "Title", "hint", &mut open), None);
        assert!(!open);
    }
}
