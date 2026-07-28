use eframe::egui;
use jereide_core::{
    AppState, CurrentView, ITEM_SPACING_Y, MAX_FILE_SIZE, TITLE_BAR_HEIGHT, TRAFFIC_LIGHT_OFFSET_X,
    TRAFFIC_LIGHT_OFFSET_Y, WARN_FILE_SIZE,
};
use jereide_fs::FileManager;
use jereide_menu::AppMenu;
use jereide_settings::{ACCENT, SURFACE_BG};
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

use jereide_ui::palette::Palette;

pub struct JereIDEApp {
    state: AppState,
    app_menu: AppMenu,
    file_manager: FileManager,
    visuals_initialized: bool,
    palette: Option<Palette>,
    compose: jereide_compose::compose_view::Compose,
}

impl JereIDEApp {
    pub fn new() -> Self {
        Self {
            state: AppState::new(),
            app_menu: AppMenu::new(),
            file_manager: FileManager::new(),
            visuals_initialized: false,
            palette: None,
            compose: jereide_compose::compose_view::Compose::new(),
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
                if let Err(e) = FileManager::save_to_path(
                    &self.state.current_tab().text,
                    &std::path::PathBuf::from(&p),
                ) {
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
            if let Err(e) = FileManager::save_to_path(&self.state.current_tab().text, &path) {
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
                if let Err(e) = FileManager::save_to_path(
                    &self.state.tabs[idx].text,
                    &std::path::PathBuf::from(&p),
                ) {
                    eprintln!("Failed to save file: {}", e);
                    false
                } else {
                    true
                }
            }
            None => {
                if let Some(path) = FileManager::save_as_dialog() {
                    if let Err(e) = FileManager::save_to_path(&self.state.tabs[idx].text, &path) {
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

    fn handle_action(&mut self, action: &str, ctx: &egui::Context, frame: &mut eframe::Frame) {
        match action {
            "file: new" => self.handle_new(),
            "file: open" => self.handle_open(),
            "file: save" => self.handle_save(),
            "file: save as" => self.handle_save_as(),
            "file: close tab" => {
                if !self.state.tabs.is_empty() {
                    let idx = self.state.active_tab_index;
                    if self.state.tabs[idx].is_modified() {
                        self.state.pending_close_index = Some(idx);
                    } else {
                        self.state.close_tab(idx);
                    }
                }
            }
            "view: toggle sidebar" => {
                self.state.sidebar_open = !self.state.sidebar_open;
            }
            "jereide: quit" => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            "jereide: toggle fullscreen" => {
                toggle_fullscreen(ctx, frame);
            }
            "jereide: star on github" => {
                ctx.open_url(egui::OpenUrl {
                    url: String::from("https://github.com/jeremy-qian/jereide"),
                    new_tab: true,
                });
            }
            "jereide: about" => {
                self.state.show_about_dialog = true;
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

        {
            let input = ctx.input(|i| {
                let cmd = i.modifiers.command;
                (
                    cmd && i.key_pressed(egui::Key::N),
                    cmd && i.key_pressed(egui::Key::O),
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
                )
            });
            let (
                want_new,
                want_open,
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
            ) = input;
            if want_new {
                self.handle_action("file: new", &ctx, frame);
            }
            if want_open {
                self.handle_action("file: open", &ctx, frame);
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
            if want_command_palette {
                self.state.command_palette_open = !self.state.command_palette_open;
                if self.state.command_palette_open {
                    self.palette = Some(Palette::new(jereide_ui::command_palette::items()));
                }
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
                    self.state.command_palette_open = !self.state.command_palette_open;
                    if self.state.command_palette_open {
                        self.palette = Some(Palette::new(jereide_ui::command_palette::items()));
                    }
                }
                other => self.handle_action(other, &ctx, frame),
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

                    if state.sidebar_open {
                        jereide_ui::sidebar::render_sidebar(state, ui);
                    }

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

        if let Some(ref pending) = self.state.pending_large_file_warn {
            let action = jereide_ui::dialog::render_large_file_warning(&ctx, &pending.0, pending.1);
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

        jereide_ui::dialog::render_about_dialog(&ctx, &mut self.state.show_about_dialog);
    }
}

#[cfg_attr(not(target_os = "windows"), allow(unused_variables))]
fn toggle_fullscreen(ctx: &egui::Context, frame: &mut eframe::Frame) {
    #[cfg(target_os = "macos")]
    {
        let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
    }

    #[cfg(target_os = "windows")]
    {
        use raw_window_handle::HasWindowHandle;
        use windows_sys::Win32::{
            Foundation::{HWND, RECT},
            Graphics::Gdi::{
                GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
            },
            UI::WindowsAndMessaging::{
                GetWindowLongW, GetWindowRect, SetWindowLongW, SetWindowPos, GWL_STYLE, HWND_TOP,
                SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW,
                WS_OVERLAPPEDWINDOW, WS_POPUP,
            },
        };

        use std::sync::OnceLock;
        static SAVED_PLACEMENT: OnceLock<(i32, i32, i32, i32)> = OnceLock::new();
        static IS_FULLSCREEN: OnceLock<std::sync::Mutex<bool>> = OnceLock::new();
        let is_fs = IS_FULLSCREEN.get_or_init(|| std::sync::Mutex::new(false));

        let Ok(handle) = frame.window_handle() else {
            return;
        };
        let raw = handle.as_raw();
        let raw_window_handle::RawWindowHandle::Win32(win32) = raw else {
            return;
        };
        let hwnd: HWND = win32.hwnd.get();

        let mut fs_guard = is_fs.lock().unwrap();
        if !*fs_guard {
            // Enter fullscreen
            let mut rect: RECT = unsafe { std::mem::zeroed() };
            if unsafe { GetWindowRect(hwnd, &mut rect as *mut RECT as *mut _) } == 0 {
                return;
            }
            let _ = SAVED_PLACEMENT.set((
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
            ));

            let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) };
            let new_style = (style & !WS_OVERLAPPEDWINDOW) | WS_POPUP;
            unsafe { SetWindowLongW(hwnd, GWL_STYLE, new_style) };

            let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
            let mut mi: MONITORINFO = unsafe { std::mem::zeroed() };
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if unsafe { GetMonitorInfoW(monitor, &mut mi as *mut MONITORINFO as *mut _) } == 0 {
                return;
            }

            unsafe {
                SetWindowPos(
                    hwnd,
                    HWND_TOP,
                    mi.rcMonitor.left,
                    mi.rcMonitor.top,
                    mi.rcMonitor.right - mi.rcMonitor.left,
                    mi.rcMonitor.bottom - mi.rcMonitor.top,
                    SWP_FRAMECHANGED | SWP_NOZORDER | SWP_SHOWWINDOW,
                );
            }

            *fs_guard = true;
        } else {
            // Exit fullscreen
            let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) };
            let new_style = (style & !WS_POPUP) | WS_OVERLAPPEDWINDOW;
            unsafe { SetWindowLongW(hwnd, GWL_STYLE, new_style) };

            if let Some(&(x, y, w, h)) = SAVED_PLACEMENT.get() {
                unsafe {
                    SetWindowPos(
                        hwnd,
                        HWND_TOP,
                        x,
                        y,
                        w,
                        h,
                        SWP_FRAMECHANGED | SWP_SHOWWINDOW,
                    );
                }
            }

            *fs_guard = false;
        }

        // Redraw the window to apply the changed style
        unsafe {
            SetWindowPos(
                hwnd,
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
            );
        }
    }
}
