use eframe::egui;
use jereide_settings::COMPOSE_BG;

pub struct Compose {
    pub prompt: String,
}

impl Compose {
    pub fn new() -> Compose {
        Compose {
            prompt: String::new(),
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, COMPOSE_BG);

        let avail = ui.available_rect_before_wrap();
        let text_width = avail.width() * 0.8;

        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(avail)
                .layout(egui::Layout::top_down(egui::Align::Center)),
        );
        child_ui.vertical_centered(|ui| {
            let total_height = ui.spacing().interact_size.y * 2.0 + ui.spacing().item_spacing.y;
            let extra = ui.available_height();
            if extra > total_height {
                ui.add_space((extra - total_height) / 2.0);
            }

            ui.add(egui::Label::new(
                egui::RichText::new("What would you like to compose?").heading(),
            ));

            let h = ui.spacing().interact_size.y;
            let input_rect = egui::Rect::from_min_size(
                egui::pos2(
                    ui.available_rect_before_wrap().center().x - text_width / 2.0,
                    ui.cursor().min.y,
                ),
                egui::vec2(text_width, h),
            );
            let mut input_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(input_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            input_ui.add_sized(
                egui::vec2(text_width - 64.0, h),
                egui::TextEdit::singleline(&mut self.prompt).hint_text("Needs Implementation"),
            );
            input_ui.add_sized(egui::vec2(64.0, h), egui::Button::new("Send"));
        });
    }
}
