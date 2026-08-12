use std::sync::OnceLock;

use eframe::egui;
use jereide_data::data_dir;
use jereide_settings::dialog_width;
use jereide_text::find_matches;

const FIND_ID: &str = "find_replace_palette_find";
const REPLACE_ID: &str = "find_replace_palette_replace";

fn prev_arrow_image() -> egui::Image<'static> {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    let bytes = BYTES.get_or_init(|| {
        data_dir()
            .map(|dir| std::fs::read(dir.join("arrow-left.png")).unwrap_or_default())
            .unwrap_or_default()
    });
    egui::Image::from_bytes("arrow-left.png", bytes.clone()).max_height(10.0)
}

fn next_arrow_image() -> egui::Image<'static> {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    let bytes = BYTES.get_or_init(|| {
        data_dir()
            .map(|dir| std::fs::read(dir.join("arrow-right.png")).unwrap_or_default())
            .unwrap_or_default()
    });
    egui::Image::from_bytes("arrow-right.png", bytes.clone()).max_height(10.0)
}

pub enum FindReplaceAction {
    Select(usize, usize),
    Replace(usize, usize),
    ReplaceAll,
}

pub struct FindReplacePalette {
    find: String,
    replace: String,
    match_case: bool,
    whole_word: bool,
    current_match: usize,
    search_focused: bool,
    previous_focus: Option<egui::Id>,
    was_open: bool,
    matches: Vec<(usize, usize)>,
    last_query: Option<(String, bool, bool)>,
    last_text_len: usize,
}

impl FindReplacePalette {
    pub fn new() -> Self {
        Self {
            find: String::new(),
            replace: String::new(),
            match_case: false,
            whole_word: false,
            current_match: 0,
            search_focused: false,
            previous_focus: None,
            was_open: false,
            matches: Vec::new(),
            last_query: None,
            last_text_len: 0,
        }
    }

    pub fn replace_text(&self) -> &str {
        &self.replace
    }

    pub fn match_case(&self) -> bool {
        self.match_case
    }

    pub fn whole_word(&self) -> bool {
        self.whole_word
    }

    pub fn find_text(&self) -> &str {
        &self.find
    }

    pub fn current_match(&self) -> usize {
        self.current_match
    }

    pub fn set_find(&mut self, text: &str) {
        self.find = text.to_string();
        self.current_match = 0;
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        text: &str,
        open: &mut bool,
    ) -> Option<FindReplaceAction> {
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
        let focus_before = ctx.memory(|m| m.focused());

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            *open = false;
            if let Some(id) = self.previous_focus {
                ctx.memory_mut(|m| m.request_focus(id));
            }
            return None;
        }

        let dim_rect = ctx.viewport_rect();
        let clicked_outside = egui::Area::new(egui::Id::new("find_replace_dismiss"))
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

        let query = (self.find.clone(), self.match_case, self.whole_word);
        let text_len = text.chars().count();
        let query_changed = self.last_query.as_ref() != Some(&query);
        let text_changed = self.last_text_len != text_len;
        if query_changed || text_changed {
            self.matches = find_matches(text, &self.find, self.match_case, self.whole_word);
            if query_changed {
                self.current_match = 0;
            } else if self.current_match >= self.matches.len() {
                self.current_match = self.matches.len().saturating_sub(1);
            }
            self.last_query = Some(query);
            self.last_text_len = text_len;
        }

        let mut action = None;
        let mut find_id = egui::Id::NULL;
        let mut replace_id = egui::Id::NULL;
        let window_width = dialog_width() + 120.0;
        let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));

        egui::Window::new("Find / Replace")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .fixed_size([window_width, 0.0])
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(window_width);
                let find_resp = ui.add(
                    egui::TextEdit::singleline(&mut self.find)
                        .id_source(FIND_ID)
                        .hint_text("Find")
                        .desired_width(f32::INFINITY),
                );
                find_id = find_resp.id;
                if !self.search_focused {
                    find_resp.request_focus();
                    self.search_focused = true;
                }

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.match_case, "Match case");
                    ui.checkbox(&mut self.whole_word, "Whole word");
                });

                let match_count = self.matches.len();
                ui.horizontal(|ui| {
                    let label = if match_count == 0 {
                        "No matches".to_string()
                    } else {
                        format!("{}/{}", self.current_match + 1, match_count)
                    };
                    ui.label(label);
                    if ui.add(egui::Button::new(prev_arrow_image())).clicked() {
                        if match_count > 0 {
                            self.current_match =
                                (self.current_match + match_count - 1) % match_count;
                            let (s, e) = self.matches[self.current_match];
                            action = Some(FindReplaceAction::Select(s, e));
                        }
                    }
                    if ui.add(egui::Button::new(next_arrow_image())).clicked() {
                        if match_count > 0 {
                            self.current_match = (self.current_match + 1) % match_count;
                            let (s, e) = self.matches[self.current_match];
                            action = Some(FindReplaceAction::Select(s, e));
                        }
                    }
                });

                ui.add_space(6.0);
                let replace_resp = ui.add(
                    egui::TextEdit::singleline(&mut self.replace)
                        .id_source(REPLACE_ID)
                        .hint_text("Replace with")
                        .desired_width(f32::INFINITY),
                );
                replace_id = replace_resp.id;

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Replace").clicked() {
                        if match_count > 0 {
                            let (s, e) = self.matches[self.current_match];
                            action = Some(FindReplaceAction::Replace(s, e));
                        }
                    }
                    if ui.button("Replace All").clicked() {
                        action = Some(FindReplaceAction::ReplaceAll);
                    }
                });
            });

        if enter_pressed {
            if focus_before == Some(find_id) {
                if !self.matches.is_empty() {
                    self.current_match = (self.current_match + 1) % self.matches.len();
                    let (s, e) = self.matches[self.current_match];
                    action = Some(FindReplaceAction::Select(s, e));
                }
                ctx.memory_mut(|m| m.request_focus(find_id));
            } else if focus_before == Some(replace_id) && !self.matches.is_empty() {                let (s, e) = self.matches[self.current_match];
                action = Some(FindReplaceAction::Replace(s, e));
            }
        }

        if action.is_none() && query_changed && !self.matches.is_empty() {
            let (s, e) = self.matches[self.current_match];
            action = Some(FindReplaceAction::Select(s, e));
        }

        action
    }
}
