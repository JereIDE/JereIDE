use eframe::egui;
use jereide_core::{
    AppState, CurrentView, ITEM_SPACING_Y, MAX_FILE_SIZE, TITLE_BAR_HEIGHT, TRAFFIC_LIGHT_OFFSET_X,
    TRAFFIC_LIGHT_OFFSET_Y, WARN_FILE_SIZE,
};
use jereide_fs::FileManager;
use jereide_menu::AppMenu;
use jereide_settings::{ACCENT, SURFACE_BG};
use raw_window_handle::HasWindowHandle;

// ---------------------------------------------------------------------------
// macOS native window helpers
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
pub fn set_document_edited(frame: &eframe::Frame, edited: bool) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = frame.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return;
    };

    let ns_view = appkit.ns_view.as_ptr() as *mut AnyObject;

    unsafe {
        let ns_window: *mut AnyObject = msg_send![ns_view, window];
        if ns_window.is_null() {
            return;
        }
        let _: () = msg_send![ns_window, setDocumentEdited: edited];
    }
}

#[cfg(target_os = "macos")]
pub fn position_traffic_lights(frame: &eframe::Frame, offset_x: f64, offset_y: f64) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use objc2_foundation::{NSPoint, NSRect};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::sync::OnceLock;

    let Ok(handle) = frame.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return;
    };

    let ns_view = appkit.ns_view.as_ptr() as *mut AnyObject;

    unsafe {
        let ns_window: *mut AnyObject = msg_send![ns_view, window];
        if ns_window.is_null() {
            return;
        }

        static DEFAULTS: OnceLock<[(f64, f64); 3]> = OnceLock::new();

        let mut origins = [(0.0f64, 0.0f64); 3];
        let mut any_found = false;

        for tag in 0i64..3 {
            let button: *mut AnyObject = msg_send![ns_window, standardWindowButton: tag];
            if button.is_null() {
                continue;
            }
            any_found = true;
            let frame: NSRect = msg_send![button, frame];
            origins[tag as usize] = (frame.origin.x, frame.origin.y);
        }

        if any_found {
            let _ = DEFAULTS.set(origins);
        }

        let Some(defaults) = DEFAULTS.get() else {
            return;
        };

        for tag in 0i64..3 {
            let button: *mut AnyObject = msg_send![ns_window, standardWindowButton: tag];
            if button.is_null() {
                continue;
            }

            let (base_x, base_y) = defaults[tag as usize];
            let frame: NSRect = msg_send![button, frame];

            let new_frame = NSRect {
                origin: NSPoint {
                    x: base_x + offset_x,
                    y: base_y + offset_y,
                },
                size: frame.size,
            };
            let _: () = msg_send![button, setFrame: new_frame];
        }
    }
}

// ---------------------------------------------------------------------------
// Main application struct — ties together state, menu, file manager, and UI
// ---------------------------------------------------------------------------

pub struct JereIDEApp {
    state: AppState,
    app_menu: AppMenu,
    file_manager: FileManager,
    visuals_initialized: bool,
}

impl JereIDEApp {
    pub fn new() -> Self {
        Self {
            state: AppState::new(),
            app_menu: AppMenu::new(),
            file_manager: FileManager::new(),
            visuals_initialized: false,
        }
    }

    fn handle_new(&mut self) {
        self.state.new_tab();
    }

    fn handle_open(&mut self) {
        let Some(path) = FileManager::pick_file() else {
            return;
        };

        let Some(size) = FileManager::file_size(&path) else {
            return;
        };

        if size > MAX_FILE_SIZE {
            self.state.pending_large_file_blocked = Some(size);
            return;
        }

        if size > WARN_FILE_SIZE {
            self.state.pending_large_file_warn = Some((path.display().to_string(), size));
            return;
        }

        let Some(content) = FileManager::read_file_at(&path) else {
            return;
        };
        let path_str = path.display().to_string();
        self.state.open_file(path_str, content);
        self.file_manager.current_path = Some(path);
    }

    fn handle_save(&mut self) {
        if self.state.tabs.is_empty() {
            return;
        }
        let path = self.state.current_tab().file_path.clone();
        match path {
            Some(p) => {
                let text = self.state.current_tab().text.clone();
                if let Err(e) = FileManager::save_to_path(&text, &std::path::PathBuf::from(&p)) {
                    eprintln!("Failed to save file: {}", e);
                } else {
                    self.state.mark_saved();
                }
            }
            None => self.handle_save_as(),
        }
    }

    fn handle_save_as(&mut self) {
        if self.state.tabs.is_empty() {
            return;
        }
        if let Some(path) = FileManager::save_as_dialog() {
            let text = self.state.current_tab().text.clone();
            if let Err(e) = FileManager::save_to_path(&text, &path) {
                eprintln!("Failed to save file: {}", e);
            } else {
                let path_str = path.display().to_string();
                let idx = self.state.active_tab_index;
                self.state.tabs[idx].file_path = Some(path_str);
                self.state.mark_saved();
            }
        }
    }

    fn save_tab(&mut self, idx: usize) -> bool {
        let path = self.state.tabs[idx].file_path.clone();
        match path {
            Some(p) => {
                let text = self.state.tabs[idx].text.clone();
                if let Err(e) = FileManager::save_to_path(&text, &std::path::PathBuf::from(&p)) {
                    eprintln!("Failed to save file: {}", e);
                    false
                } else {
                    true
                }
            }
            None => {
                if let Some(path) = FileManager::save_as_dialog() {
                    let text = self.state.tabs[idx].text.clone();
                    if let Err(e) = FileManager::save_to_path(&text, &path) {
                        eprintln!("Failed to save file: {}", e);
                        false
                    } else {
                        let path_str = path.display().to_string();
                        self.state.tabs[idx].file_path = Some(path_str);
                        true
                    }
                } else {
                    false
                }
            }
        }
    }
}

impl eframe::App for JereIDEApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Apply the accent theme once. Re-setting visuals every frame makes egui
        // treat the style as changed and re-layout the whole UI each frame, which
        // makes the entire app (and even while dialogs are open) feel sluggish.
        if !self.visuals_initialized {
            let mut visuals = ctx.global_style().visuals.clone();
            visuals.selection.bg_fill = ACCENT;
            visuals.selection.stroke = egui::Stroke::new(1.0, jereide_settings::TEXT_DEFAULT);
            ctx.set_visuals(visuals);
            self.visuals_initialized = true;
        }

        #[cfg(target_os = "macos")]
        {
            let is_modified = !self.state.tabs.is_empty() && self.state.is_modified();
            if is_modified != self.state.document_edited {
                self.state.document_edited = is_modified;
                set_document_edited(frame, is_modified);
            }

            let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            position_traffic_lights(frame, TRAFFIC_LIGHT_OFFSET_X, TRAFFIC_LIGHT_OFFSET_Y);
            self.state.was_fullscreen = is_fullscreen;
        }

        if !self.app_menu.is_initialized() {
            let raw = frame.window_handle().ok().map(|h| h.as_raw());
            self.app_menu.init(raw);
            self.app_menu.set_initialized();
        }

        #[cfg(not(target_os = "macos"))]
        {
            let (want_new, want_open, want_save, want_save_as, want_quit) = ctx.input(|i| {
                let cmd = i.modifiers.command;
                (
                    cmd && i.key_pressed(egui::Key::N),
                    cmd && i.key_pressed(egui::Key::O),
                    cmd && i.key_pressed(egui::Key::S) && !i.modifiers.shift,
                    cmd && i.modifiers.shift && i.key_pressed(egui::Key::S),
                    cmd && i.key_pressed(egui::Key::Q),
                )
            });
            if want_new {
                self.handle_new();
            }
            if want_open {
                self.handle_open();
            }
            if want_save {
                self.handle_save();
            }
            if want_save_as {
                self.handle_save_as();
            }
            if want_quit {
                std::process::exit(0);
            }
        }

        for event_id in self.app_menu.poll_events() {
            match event_id.as_ref() {
                "new" => self.handle_new(),
                "open" => self.handle_open(),
                "save" => self.handle_save(),
                "save_as" => self.handle_save_as(),
                "quit" => std::process::exit(0),
                "fullscreen" => {
                    let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
                }
                "githubstar" => {
                    ctx.open_url(egui::OpenUrl {
                        url: String::from("https://github.com/jeremy-qian/jereide"),
                        new_tab: true,
                    });
                }
                _ => {
                    if let Some(action) = jereide_code::EditAction::from_menu_id(event_id.as_ref())
                    {
                        jereide_code::edit::handle_edit_action(&mut self.state, &ctx, action);
                    }
                }
            }
        }

        {
            let state = &mut self.state;

            jereide_ui::status_bar::render_status_bar(state, ui);

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(SURFACE_BG))
                .show_inside(ui, |ui| {
                    let style = ui.style_mut();
                    style.visuals.extreme_bg_color = SURFACE_BG;
                    style.spacing.item_spacing.y = ITEM_SPACING_Y;

                    let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                    jereide_ui::title_bar::render_title_bar(state, ui, is_fullscreen);
                    if !state.tabs.is_empty() {
                        jereide_ui::tab_strip::render_tab_strip(state, ui);
                    }

                    let content_rect = ui.available_rect_before_wrap();
                    let mut code_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(content_rect)
                            .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    );
                    code_ui.set_clip_rect(content_rect);
                    if state.tabs.is_empty() {
                        jereide_ui::welcome::render_welcome_view(&mut code_ui);
                    } else {
                        jereide_code::code_view::render_code_view(state, &mut code_ui);
                    }
                });

            if state.current_view == CurrentView::Command {
                let title_bar_height = TITLE_BAR_HEIGHT;
                let full_area = ui.ctx().content_rect();
                let overlay_rect = egui::Rect::from_min_size(
                    egui::pos2(full_area.left(), full_area.top() + title_bar_height),
                    egui::vec2(full_area.width(), full_area.height() - title_bar_height),
                );

                let mut overlay_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(overlay_rect)
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                );
                jereide_command::command_view::render_command_view(&mut overlay_ui);
            }
        }

        use jereide_ui::dialog::{CloseConfirmAction, LargeFileAction};

        if let Some(action) = jereide_ui::dialog::render_close_confirm_modal(&mut self.state, &ctx)
        {
            match action {
                CloseConfirmAction::Save(idx) => {
                    if self.save_tab(idx) {
                        self.state.close_tab(idx);
                    }
                    self.state.pending_close_index = None;
                }
                CloseConfirmAction::Discard(idx) => {
                    self.state.close_tab(idx);
                    self.state.pending_close_index = None;
                }
                CloseConfirmAction::Cancel => {
                    self.state.pending_close_index = None;
                }
            }
        }

        if let Some(size) = self.state.pending_large_file_blocked {
            if jereide_ui::dialog::render_large_file_blocked(&ctx, size) {
                self.state.pending_large_file_blocked = None;
            }
        }

        if let Some((ref path, _size)) = self.state.pending_large_file_warn.clone() {
            let action = jereide_ui::dialog::render_large_file_warning(&ctx, path, _size);
            if let Some(lfa) = action {
                self.state.pending_large_file_warn = None;
                match lfa {
                    LargeFileAction::OpenAnyway(path_str) => {
                        let pb = std::path::PathBuf::from(&path_str);
                        if let Some(content) = FileManager::read_file_at(&pb) {
                            self.state.open_file(path_str, content);
                            self.file_manager.current_path = Some(pb);
                        }
                    }
                    LargeFileAction::Cancel => {}
                }
            }
        }
    }
}
