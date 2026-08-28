use eframe::egui;
use jereide_core::{
    AppState, CurrentView, ITEM_SPACING_Y, MAX_FILE_SIZE, PaneLayout, SplitDirection,
    TITLE_BAR_HEIGHT, TRAFFIC_LIGHT_OFFSET_X, TRAFFIC_LIGHT_OFFSET_Y, WARN_FILE_SIZE,
};
use jereide_fs::{
    file_size, pick_directory, pick_file, read_file_at, save_as_dialog, save_to_path,
};
use jereide_menu::AppMenu;
use jereide_settings::{accent, surface_bg};
use jereide_ui::find_replace_palette::{FindReplaceAction, FindReplacePalette};
use jereide_ui::go_to_line_palette::GoToLinePalette;
use jereide_ui::sidebar::clear_ls_cache;
use jereide_widgets::toggle_switch::toggle_ui;
use jereide_widgets::widget_palette::WidgetPalette;
use raw_window_handle::HasWindowHandle;

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

use jereide_widgets::palette::Palette;

pub struct JereIDEApp {
    state: AppState,
    app_menu: AppMenu,
    visuals_initialized: bool,
    palette: Option<Palette>,
    compose: jereide_compose::compose_view::Compose,
    widget_palette: WidgetPalette,
    widget_palette_open: bool,
    find_palette: FindReplacePalette,
    find_palette_open: bool,
    go_to_line_palette: Option<GoToLinePalette>,
    go_to_line_open: bool,
    settings_window_open: bool,
    last_settings_version: usize,
}

impl JereIDEApp {
    pub fn new() -> Self {
        Self {
            state: AppState::new(),
            app_menu: AppMenu::new(),
            visuals_initialized: false,
            palette: None,
            compose: jereide_compose::compose_view::Compose::new(),
            widget_palette: WidgetPalette::new(),
            widget_palette_open: false,
            find_palette: FindReplacePalette::new(),
            find_palette_open: false,
            go_to_line_palette: None,
            go_to_line_open: false,
            settings_window_open: false,
            last_settings_version: 0,
        }
    }

    fn handle_new(&mut self) {
        self.state.new_tab();
    }

    fn begin_quit(&mut self, ctx: &egui::Context) {
        if let Some(idx) = self.state.first_dirty_tab_index() {
            self.state.pending_quit = true;
            self.state.pending_close_index = Some(idx);
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn toggle_find_palette(&mut self) {
        if self.state.tabs.is_empty() {
            return;
        }
        self.find_palette_open = !self.find_palette_open;
        log::debug!("find palette open = {}", self.find_palette_open);
        if self.find_palette_open {
            if let Some(sel) = &self.state.selected_text {
                if !sel.is_empty() {
                    self.find_palette.set_find(sel);
                }
            }
        }
    }

    fn toggle_go_to_line(&mut self) {
        if self.state.tabs.is_empty() {
            return;
        }
        if self.go_to_line_open {
            self.go_to_line_open = false;
            self.go_to_line_palette = None;
        } else {
            self.go_to_line_open = true;
            self.go_to_line_palette = Some(GoToLinePalette::new());
        }
        log::debug!("go-to-line palette open = {}", self.go_to_line_open);
    }

    fn open_path(&mut self, path: std::path::PathBuf) {
        let Some(size) = file_size(&path) else {
            log::warn!("could not stat file, refusing to open: {}", path.display());
            return;
        };

        if size > MAX_FILE_SIZE {
            log::warn!(
                "file too large to open ({} bytes > {} max), blocked: {}",
                size,
                MAX_FILE_SIZE,
                path.display()
            );
            self.state.pending_large_file_blocked = Some(size);
            return;
        }

        if size > WARN_FILE_SIZE {
            log::warn!(
                "file is large ({} bytes > {} warn threshold): {}",
                size,
                WARN_FILE_SIZE,
                path.display()
            );
            self.state.pending_large_file_warn = Some((path.display().to_string(), size));
            return;
        }

        let Some(content) = read_file_at(&path) else {
            log::error!("failed to read file contents: {}", path.display());
            self.state.pending_binary_file = Some(path.display().to_string());
            return;
        };
        log::debug!("read {} bytes from {}", content.len(), path.display());
        let path_str = path.display().to_string();
        self.state.open_file(path_str, content);
    }

    fn handle_open(&mut self) {
        log::info!("user requested 'open file' dialog");
        let Some(path) = pick_file() else {
            log::info!("open file dialog cancelled");
            return;
        };
        log::info!("user picked file: {}", path.display());
        self.open_path(path);
    }

    fn handle_open_project(&mut self) {
        log::info!("user requested 'open project' dialog");
        let Some(path) = pick_directory() else {
            log::info!("open project dialog cancelled");
            return;
        };
        log::info!("opening project directory: {}", path.display());
        self.state.current_project_dir = Some(path.to_string_lossy().into_owned());
        clear_ls_cache();
    }

    fn handle_settings(&mut self) {
        let path = jereide_settings::settings_file_path();
        log::info!("opening settings file: {}", path.display());
        let Some(content) = read_file_at(&path) else {
            log::error!("could not read settings file: {}", path.display());
            return;
        };
        self.state.open_file(path.display().to_string(), content);
    }

    fn handle_view_log(&mut self) {
        let Some(path) = jereide_logging::current_log_path() else {
            log::warn!("view log requested but no log file is open");
            return;
        };
        let content = read_file_at(&path).unwrap_or_default();
        log::info!(
            "opening log file {:?} in read-only tab ({} chars)",
            path,
            content.chars().count()
        );
        self.state
            .open_read_only(path.display().to_string(), content);
    }

    fn handle_save(&mut self) {
        if self.state.tabs.is_empty() {
            log::debug!("save requested but no tabs open");
            return;
        }
        if self.state.current_tab().read_only {
            log::info!("save requested on read-only tab; ignoring");
            return;
        }
        let path = self.state.current_tab().file_path.clone();
        match path {
            Some(p) => {
                if let Err(e) = save_to_path(
                    &self.state.current_tab().text,
                    &std::path::PathBuf::from(&p),
                ) {
                    log::error!("failed to save file {}: {}", p, e);
                } else {
                    log::info!(
                        "saved {} bytes to {}",
                        self.state.current_tab().text.chars().count(),
                        p
                    );
                    self.state.mark_saved();
                }
            }
            None => {
                log::debug!("save requested on unnamed tab; falling back to save-as");
                self.handle_save_as();
            }
        }
    }

    fn handle_save_as(&mut self) {
        if self.state.tabs.is_empty() {
            log::debug!("save-as requested but no tabs open");
            return;
        }
        if self.state.current_tab().read_only {
            log::info!("save-as requested on read-only tab; ignoring");
            return;
        }
        log::info!("user requested 'save as' dialog");
        if let Some(path) = save_as_dialog() {
            if let Err(e) = save_to_path(&self.state.current_tab().text, &path) {
                log::error!("failed to save file as {}: {}", path.display(), e);
            } else {
                log::info!(
                    "saved file as {} ({} chars)",
                    path.display(),
                    self.state.current_tab().text.chars().count()
                );
                let path_str = path.display().to_string();
                let idx = self.state.focused_tab_index();
                self.state.tabs[idx].file_path = Some(path_str);
                self.state.mark_saved();
            }
        } else {
            log::info!("save-as dialog cancelled");
        }
    }

    fn save_tab(&mut self, idx: usize) -> bool {
        let path = self.state.tabs[idx].file_path.clone();
        match path {
            Some(p) => {
                if let Err(e) =
                    save_to_path(&self.state.tabs[idx].text, &std::path::PathBuf::from(&p))
                {
                    log::error!("failed to save tab {idx} to {}: {}", p, e);
                    false
                } else {
                    log::info!("saved tab {idx} to {}", p);
                    true
                }
            }
            None => {
                if let Some(path) = save_as_dialog() {
                    if let Err(e) = save_to_path(&self.state.tabs[idx].text, &path) {
                        log::error!("failed to save tab {idx} as {}: {}", path.display(), e);
                        false
                    } else {
                        let path_str = path.display().to_string();
                        self.state.tabs[idx].file_path = Some(path_str);
                        true
                    }
                } else {
                    log::debug!("save-as dialog cancelled; tab {idx} not saved");
                    false
                }
            }
        }
    }

    /// Handles stuff
    fn handle_action(&mut self, action: &str, ctx: &egui::Context, frame: &mut eframe::Frame) {
        match action {
            "file: new" => self.handle_new(),
            "file: open" => self.handle_open(),
            "file: save" => self.handle_save(),
            "file: save as" => self.handle_save_as(),
            "file: open project" => self.handle_open_project(),
            "file: close tab" => {
                if !self.state.tabs.is_empty() {
                    let idx = self.state.focused_tab_index();
                    if self.state.tabs[idx].is_modified() {
                        self.state.pending_close_index = Some(idx);
                    } else {
                        self.state.close_tab(idx);
                    }
                }
            }
            "view: toggle sidebar" => {
                self.state.sidebar_open = !self.state.sidebar_open;
                log::debug!("sidebar open = {}", self.state.sidebar_open);
            }
            "view: code" => self.state.switch_to_view(CurrentView::Code),
            "view: compose" => self.state.switch_to_view(CurrentView::Compose),
            "view: split right" => {
                self.state
                    .split_pane(jereide_core::SplitDirection::Vertical);
            }
            "view: split down" => {
                self.state
                    .split_pane(jereide_core::SplitDirection::Horizontal);
            }
            "view: close pane" => {
                self.state.close_current_pane();
            }
            "jereide: open settings file" => self.handle_settings(),
            "jereide: quit" => {
                self.begin_quit(&ctx);
            }
            "jereide: toggle fullscreen" => {
                toggle_fullscreen(ctx, frame);
            }
            "jereide: open docs" => {
                ctx.open_url(egui::OpenUrl {
                    url: String::from("https://jereide.github.io/docs"),
                    new_tab: true,
                });
            }
            "jereide: star on github" => {
                ctx.open_url(egui::OpenUrl {
                    url: String::from("https://github.com/jeremy-qian/jereide"),
                    new_tab: true,
                });
            }
            "jereide: view log" => self.handle_view_log(),
            "jereide: about" => {
                self.state.show_about_dialog = true;
            }
            "editor: find replace" => {
                self.toggle_find_palette();
            }
            "editor: go to line" => {
                self.toggle_go_to_line();
            }
            _ => {
                jereide_code::edit::handle_edit_action(&mut self.state, ctx, action);
            }
        }
    }
}

impl eframe::App for JereIDEApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if !self.visuals_initialized
            || jereide_settings::settings_version() != self.last_settings_version
        {
            let mut visuals = ctx.global_style().visuals.clone();
            visuals.selection.bg_fill = accent();
            visuals.selection.stroke = egui::Stroke::new(1.0, jereide_settings::text_default());
            visuals.window_shadow = egui::Shadow {
                offset: [1, 2],
                blur: 6,
                spread: 0,
                color: egui::Color32::from_black_alpha(90),
            };
            ctx.set_visuals(visuals);
            self.last_settings_version = jereide_settings::settings_version();
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

        {
            let input = ctx.input(|i| {
                let cmd = i.modifiers.command;
                (
                    cmd && i.key_pressed(egui::Key::N),
                    cmd && i.key_pressed(egui::Key::O) && !i.modifiers.shift,
                    cmd && i.modifiers.shift && i.key_pressed(egui::Key::O),
                    cmd && i.key_pressed(egui::Key::S) && !i.modifiers.shift,
                    cmd && i.modifiers.shift && i.key_pressed(egui::Key::S),
                    cmd && i.key_pressed(egui::Key::Q),
                    cmd && i.key_pressed(egui::Key::Z) && !i.modifiers.shift,
                    cmd && i.modifiers.shift && i.key_pressed(egui::Key::Z),
                    cmd && i.key_pressed(egui::Key::X),
                    cmd && i.key_pressed(egui::Key::C),
                    cmd && i.key_pressed(egui::Key::V),
                    cmd && i.key_pressed(egui::Key::A),
                    cmd && i.key_pressed(egui::Key::W),
                    cmd && i.modifiers.shift && i.key_pressed(egui::Key::P),
                    cmd && i.key_pressed(egui::Key::B),
                    cmd && i.modifiers.shift && i.key_pressed(egui::Key::W),
                    cmd && i.key_pressed(egui::Key::F),
                    cmd && i.key_pressed(egui::Key::Comma),
                    cmd && i.key_pressed(egui::Key::G),
                    cmd && i.key_pressed(egui::Key::Backslash) && !i.modifiers.shift,
                    cmd && i.modifiers.shift && i.key_pressed(egui::Key::Backslash),
                    cmd && i.key_pressed(egui::Key::Backslash) && i.modifiers.shift,
                )
            });
            let (
                want_new,
                want_open,
                want_open_project,
                want_save,
                want_save_as,
                want_quit,
                want_undo,
                want_redo,
                want_cut,
                want_copy,
                want_paste,
                want_select_all,
                want_close_tab,
                want_command_palette,
                want_toggle_sidebar,
                want_widget_palette,
                want_find_replace,
                want_open_settings,
                want_go_to_line,
                want_split_right,
                want_split_down,
                want_split_down2,
            ) = input;
            if want_new {
                self.handle_action("file: new", &ctx, frame);
            }
            if want_open {
                self.handle_action("file: open", &ctx, frame);
            }
            if want_open_project {
                self.handle_action("file: open project", &ctx, frame);
            }
            if want_save {
                self.handle_action("file: save", &ctx, frame);
            }
            if want_save_as {
                self.handle_action("file: save as", &ctx, frame);
            }
            if want_quit {
                self.handle_action("jereide: quit", &ctx, frame);
            }
            if want_undo {
                self.handle_action("editor: undo", &ctx, frame);
            }
            if want_redo {
                self.handle_action("editor: redo", &ctx, frame);
            }
            if want_cut {
                self.handle_action("editor: cut", &ctx, frame);
            }
            if want_copy {
                self.handle_action("editor: copy", &ctx, frame);
            }
            if want_paste {
                self.handle_action("editor: paste", &ctx, frame);
            }
            if want_select_all {
                self.handle_action("editor: select all", &ctx, frame);
            }
            if want_close_tab {
                self.handle_action("file: close tab", &ctx, frame);
            }
            if want_toggle_sidebar {
                self.handle_action("view: toggle sidebar", &ctx, frame);
            }
            if want_widget_palette && !self.settings_window_open {
                self.widget_palette_open = !self.widget_palette_open;
            }
            if want_find_replace && !self.settings_window_open {
                self.toggle_find_palette();
            }
            if want_open_settings {
                if self.settings_window_open {
                    self.settings_window_open = false;
                } else {
                    self.settings_window_open = true;
                    ctx.memory_mut(|m| {
                        if let Some(id) = m.focused() {
                            m.surrender_focus(id);
                        }
                    });
                    self.state.command_palette_open = false;
                    self.palette = None;
                    self.widget_palette_open = false;
                    self.find_palette_open = false;
                    self.go_to_line_open = false;
                    self.go_to_line_palette = None;
                }
            }
            if want_go_to_line && !self.settings_window_open {
                self.toggle_go_to_line();
            }
            if want_command_palette && !self.settings_window_open {
                self.state.command_palette_open = !self.state.command_palette_open;
                if self.state.command_palette_open {
                    self.palette = Some(Palette::new(jereide_ui::command_palette::items()));
                }
            }
            if want_split_right && !self.settings_window_open {
                self.state
                    .split_pane(jereide_core::SplitDirection::Vertical);
            }
            if (want_split_down || want_split_down2) && !self.settings_window_open {
                self.state
                    .split_pane(jereide_core::SplitDirection::Horizontal);
            }

            if self.state.pending_open {
                self.handle_action("file: open", &ctx, frame);
                self.state.pending_open = false;
            } else if self.state.pending_open_project {
                self.handle_action("file: open project", &ctx, frame);
                self.state.pending_open_project = false;
            }

            #[cfg(target_os = "windows")]
            {
                let want_fullscreen = ctx.input(|i| i.key_pressed(egui::Key::F11));
                if want_fullscreen {
                    self.handle_action("jereide: toggle fullscreen", &ctx, frame);
                }
            }
        }

        for event_id in self.app_menu.poll_events() {
            match event_id.as_ref() {
                "command palette: toggle" => {
                    if !self.settings_window_open {
                        self.state.command_palette_open = !self.state.command_palette_open;
                        if self.state.command_palette_open {
                            self.palette = Some(Palette::new(jereide_ui::command_palette::items()));
                        }
                    }
                }
                other => self.handle_action(other, &ctx, frame),
            }
        }

        let go_to_line_clicked;
        let pending_sidebar_open: Option<String>;

        {
            let state = &mut self.state;

            go_to_line_clicked = jereide_ui::status_bar::render_status_bar(state, ui);

            let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            egui::Panel::top("title_bar")
                .exact_size(TITLE_BAR_HEIGHT)
                .frame(egui::Frame::NONE)
                .show_separator_line(false)
                .show(ui, |ui| {
                    jereide_ui::title_bar::render_title_bar(state, ui, is_fullscreen);
                });

            if state.sidebar_open {
                jereide_ui::sidebar::render_sidebar(state, ui);
            }
            pending_sidebar_open = state.pending_open_file.take();

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(surface_bg()))
                .show(ui, |ui| {
                    let style = ui.style_mut();
                    style.visuals.extreme_bg_color = surface_bg();
                    style.spacing.item_spacing.y = ITEM_SPACING_Y;

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
                        render_pane_layout(state, &mut code_ui);
                    }
                });

            if state.current_view == CurrentView::Compose {
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
                self.compose.render(&mut overlay_ui);
            }
        }

        if go_to_line_clicked {
            self.toggle_go_to_line();
        }

        if let Some(path) = pending_sidebar_open {
            self.open_path(std::path::PathBuf::from(path));
        }

        use jereide_ui::dialog::{CloseConfirmAction, LargeFileAction};

        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if close_requested && !self.state.pending_quit && !self.state.pending_close_index.is_some()
        {
            if let Some(idx) = self.state.first_dirty_tab_index() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.state.pending_quit = true;
                self.state.pending_close_index = Some(idx);
            }
        }

        if let Some(action) = jereide_ui::dialog::render_close_confirm_modal(&mut self.state, &ctx)
        {
            match action {
                CloseConfirmAction::Save(idx) => {
                    if self.save_tab(idx) {
                        self.state.close_tab(idx);
                    }
                }
                CloseConfirmAction::Discard(idx) => {
                    self.state.close_tab(idx);
                }
                CloseConfirmAction::Cancel => {
                    self.state.pending_quit = false;
                    self.state.pending_close_index = None;
                }
            }

            if self.state.pending_quit {
                if let Some(next) = self.state.first_dirty_tab_index() {
                    self.state.pending_close_index = Some(next);
                } else {
                    self.state.pending_quit = false;
                    self.state.pending_close_index = None;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            } else {
                self.state.pending_close_index = None;
            }
        }

        if let Some(size) = self.state.pending_large_file_blocked {
            if jereide_ui::dialog::render_large_file_blocked(&ctx, size) {
                self.state.pending_large_file_blocked = None;
            }
        }

        if let Some(ref pending) = self.state.pending_large_file_warn {
            let action = jereide_ui::dialog::render_large_file_warning(&ctx, &pending.0, pending.1);
            if let Some(lfa) = action {
                self.state.pending_large_file_warn = None;
                match lfa {
                    LargeFileAction::OpenAnyway(path_str) => {
                        log::info!("user chose to open large file anyway: {}", path_str);
                        let pb = std::path::PathBuf::from(&path_str);
                        if let Some(content) = read_file_at(&pb) {
                            self.state.open_file(path_str.clone(), content);
                        } else {
                            log::error!("failed to read file contents: {}", path_str);
                            self.state.pending_binary_file = Some(path_str);
                        }
                    }
                    LargeFileAction::Cancel => {}
                }
            }
        }

        if let Some(path) = self.state.pending_binary_file.clone() {
            if jereide_ui::dialog::render_binary_file_dialog(&ctx, &path) {
                self.state.pending_binary_file = None;
            }
        }

        if let Some(ref mut palette) = self.palette {
            if let Some(action) = palette.render(
                &ctx,
                "Command Palette",
                &mut self.state.command_palette_open,
            ) {
                self.handle_action(action, &ctx, frame);
            }
            if !self.state.command_palette_open {
                self.palette = None;
            }
        }

        // Seriously, this isn't supposed to be here now that the palette is done
        // but I just really like fresh widgets.
        if self.widget_palette_open {
            self.widget_palette.render(
                &ctx,
                "Widget Palette",
                &mut self.widget_palette_open,
                |palette, ui, filter| {
                    ui.heading("Widget Palette");
                    ui.add_space(4.0);
                    ui.label("A label and some buttons:");
                    ui.horizontal(|ui| {
                        if ui.button("Button A").clicked() {
                            log::info!("widget palette 'Button A' clicked");
                        }
                        if ui.button("Button B").clicked() {
                            log::info!("widget palette 'Button B' clicked");
                        }
                        if ui.button("Button C").clicked() {
                            log::info!("widget palette 'Button C' clicked");
                        }
                    });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.label("Checkboxes:");
                    let mut a = false;
                    let mut b = true;
                    let mut c = false;
                    ui.checkbox(&mut a, "Option A");
                    ui.checkbox(&mut b, "Option B");
                    ui.checkbox(&mut c, "Option C");
                    ui.add_space(8.0);
                    ui.separator();
                    ui.label("Toggle switch:");
                    ui.horizontal(|ui| {
                        let on = palette.toggle_state();
                        toggle_ui(ui, on);
                        ui.label(if *on { "On" } else { "Off" });
                    });
                    ui.separator();
                    ui.label("A slider and a progress bar:");
                    let mut value = 0.5;
                    ui.add(egui::Slider::new(&mut value, 0.0..=1.0).text("value"));
                    ui.add(egui::ProgressBar::new(value).show_percentage());
                    ui.add_space(8.0);
                    ui.separator();
                    let mut text = String::new();
                    ui.add(
                        egui::TextEdit::multiline(&mut text)
                            .hint_text("Multiline text input…")
                            .desired_rows(2)
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(format!("You searched: {}", filter));
                },
            );
        }

        if self.find_palette_open && !self.state.tabs.is_empty() {
            let idx = self.state.focused_tab_index();
            let text = self.state.tabs[idx].text.clone();
            let action = self
                .find_palette
                .render(&ctx, &text, &mut self.find_palette_open);
            let mut scroll_to: Option<usize> = None;
            if let Some(action) = action {
                let state = &mut self.state;
                match action {
                    FindReplaceAction::Select(s, e) => {
                        state.pending_find_selection = Some((s, e));
                        scroll_to = Some(s);
                    }
                    FindReplaceAction::Replace(s, e) => {
                        state.pending_find_selection = None;
                        let replacement = self.find_palette.replace_text();
                        jereide_code::find_replace::replace_range(state, &ctx, s, e, replacement);
                        scroll_to = Some(s + replacement.chars().count());
                    }
                    FindReplaceAction::ReplaceAll => {
                        state.pending_find_selection = None;
                        let find = self.find_palette.find_text().to_string();
                        let replace = self.find_palette.replace_text().to_string();
                        let match_case = self.find_palette.match_case();
                        let whole_word = self.find_palette.whole_word();
                        jereide_code::find_replace::replace_all(
                            state, &ctx, &find, &replace, match_case, whole_word,
                        );
                        scroll_to = Some(0);
                    }
                }
            }
            self.state.find_highlight = Some(jereide_core::FindHighlight {
                query: self.find_palette.find_text().to_string(),
                match_case: self.find_palette.match_case(),
                whole_word: self.find_palette.whole_word(),
                current_match: self.find_palette.current_match(),
                scroll_to,
            });
        } else {
            self.state.find_highlight = None;
            self.find_palette_open = false;
            if let Some((s, e)) = self.state.pending_find_selection.take() {
                jereide_code::find_replace::select_match(&mut self.state, &ctx, s, e);
            }
        }

        if self.go_to_line_open && !self.state.tabs.is_empty() {
            let idx = self.state.focused_tab_index();
            let total_lines = jereide_text::count_lines(&self.state.tabs[idx].text);
            if let Some(palette) = &mut self.go_to_line_palette {
                if let Some(line) = palette.render(&ctx, total_lines, &mut self.go_to_line_open) {
                    let tab_text = self.state.tabs[idx].text.clone();
                    let target = jereide_text::line_start_char_index(&tab_text, line);
                    jereide_code::go_to_line::go_to_line(&mut self.state, &ctx, target);
                    self.state.go_to_line_scroll_to = Some(target);
                }
            }
        } else {
            self.go_to_line_open = false;
            self.go_to_line_palette = None;
        }

        jereide_ui::dialog::render_about_dialog(&ctx, &mut self.state.show_about_dialog);

        if self.settings_window_open {
            jereide_settings::render_settings_window(&ctx, &mut self.settings_window_open);
        }
    }
}

fn toggle_fullscreen(ctx: &egui::Context, _frame: &mut eframe::Frame) {
    #[cfg(target_os = "macos")]
    {
        let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
    }

    #[cfg(target_os = "windows")]
    {
        let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
    }
}

fn render_pane_layout(state: &mut AppState, ui: &mut egui::Ui) {
    render_pane_layout_inner(state, ui, &mut state.pane_layout.clone());
}

fn render_pane_layout_inner(state: &mut AppState, ui: &mut egui::Ui, layout: &PaneLayout) {
    match layout {
        PaneLayout::Single(pane) => {
            if state.tabs.is_empty() {
                jereide_ui::welcome::render_welcome_view(ui);
                return;
            }

            let pane_id = pane.id;
            let pane_rect = ui.max_rect();

            // Render per-pane tab strip
            jereide_ui::tab_strip::render_tab_strip(state, ui, &pane.tab_indices, pane_id);

            let tab_index = pane
                .active_tab_index
                .min(state.tabs.len().saturating_sub(1));
            jereide_code::code_view::render_code_view(state, ui, tab_index, pane_id);

            if ui.input(|i| i.pointer.any_click()) {
                if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                    if pane_rect.contains(pos) {
                        state.set_focused_pane(pane_id);
                    }
                }
            }
        }
        PaneLayout::Split {
            direction,
            first,
            second,
            split_ratio,
        } => {
            let available = ui.available_rect_before_wrap();
            let ratio = *split_ratio;

            match direction {
                SplitDirection::Horizontal => {
                    let split_y = available.top() + available.height() * ratio;
                    let top_rect = egui::Rect::from_min_max(
                        available.min,
                        egui::pos2(available.right(), split_y),
                    );
                    let bottom_rect = egui::Rect::from_min_max(
                        egui::pos2(available.left(), split_y),
                        available.max,
                    );

                    let divider_rect = egui::Rect::from_min_max(
                        egui::pos2(available.left(), split_y - 1.0),
                        egui::pos2(available.right(), split_y + 1.0),
                    );

                    {
                        let mut top_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(top_rect)
                                .layout(egui::Layout::top_down(egui::Align::LEFT)),
                        );
                        top_ui.set_clip_rect(top_rect);
                        render_pane_layout_inner(state, &mut top_ui, first);
                    }

                    ui.painter()
                        .rect_filled(divider_rect, 0.0, jereide_settings::border());

                    {
                        let mut bottom_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(bottom_rect)
                                .layout(egui::Layout::top_down(egui::Align::LEFT)),
                        );
                        bottom_ui.set_clip_rect(bottom_rect);
                        render_pane_layout_inner(state, &mut bottom_ui, second);
                    }

                    let drag_resp = ui.interact(
                        divider_rect,
                        egui::Id::new("split_divider_h"),
                        egui::Sense::click_and_drag(),
                    );
                    if drag_resp.dragged() {
                        let delta = ui.input(|i| i.pointer.delta().y);
                        let new_ratio = (ratio + delta / available.height()).clamp(0.1, 0.9);
                        if let Some(pane_layout) =
                            get_split_mut(&mut state.pane_layout, first, second)
                        {
                            pane_layout.set_split_ratio(new_ratio);
                        }
                    }
                }
                SplitDirection::Vertical => {
                    let split_x = available.left() + available.width() * ratio;
                    let left_rect = egui::Rect::from_min_max(
                        available.min,
                        egui::pos2(split_x, available.bottom()),
                    );
                    let right_rect = egui::Rect::from_min_max(
                        egui::pos2(split_x, available.top()),
                        available.max,
                    );

                    let divider_rect = egui::Rect::from_min_max(
                        egui::pos2(split_x - 1.0, available.top()),
                        egui::pos2(split_x + 1.0, available.bottom()),
                    );

                    {
                        let mut left_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(left_rect)
                                .layout(egui::Layout::top_down(egui::Align::LEFT)),
                        );
                        left_ui.set_clip_rect(left_rect);
                        render_pane_layout_inner(state, &mut left_ui, first);
                    }

                    ui.painter()
                        .rect_filled(divider_rect, 0.0, jereide_settings::border());

                    {
                        let mut right_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(right_rect)
                                .layout(egui::Layout::top_down(egui::Align::LEFT)),
                        );
                        right_ui.set_clip_rect(right_rect);
                        render_pane_layout_inner(state, &mut right_ui, second);
                    }

                    let drag_resp = ui.interact(
                        divider_rect,
                        egui::Id::new("split_divider_v"),
                        egui::Sense::click_and_drag(),
                    );
                    if drag_resp.dragged() {
                        let delta = ui.input(|i| i.pointer.delta().x);
                        let new_ratio = (ratio + delta / available.width()).clamp(0.1, 0.9);
                        if let Some(pane_layout) =
                            get_split_mut(&mut state.pane_layout, first, second)
                        {
                            pane_layout.set_split_ratio(new_ratio);
                        }
                    }
                }
            }
        }
    }
}

fn get_split_mut<'a>(
    layout: &'a mut PaneLayout,
    first: &PaneLayout,
    second: &PaneLayout,
) -> Option<&'a mut PaneLayout> {
    match layout {
        PaneLayout::Split {
            first: l_first,
            second: l_second,
            ..
        } => {
            let first_ptr = &**l_first as *const PaneLayout as usize;
            let second_ptr = &**l_second as *const PaneLayout as usize;
            let target_first = first as *const PaneLayout as usize;
            let target_second = second as *const PaneLayout as usize;
            if first_ptr == target_first && second_ptr == target_second {
                Some(layout)
            } else {
                let ptr = layout as *mut PaneLayout;
                let (f, s) = unsafe { (*ptr).as_split_mut() }?;
                get_split_mut(f, first, second).or_else(|| get_split_mut(s, first, second))
            }
        }
        _ => None,
    }
}
