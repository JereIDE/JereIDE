use muda::{
    accelerator::Accelerator, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu,
};
use raw_window_handle::RawWindowHandle;

/// A struct about the menu.
pub struct AppMenu {
    menu: Menu,
    receiver: &'static crossbeam_channel::Receiver<MenuEvent>,
    initialized: bool,
}

impl AppMenu {
    /// Creates all these stuff
    pub fn new() -> Self {
        let app_menu = Submenu::with_id("JereIDE", "JereIDE", true);
        // Add lots of predefined items and a Star on GitHub
        app_menu
            .append_items(&[
                &MenuItem::with_id("jereide: about", "About JereIDE", true, None),
                &PredefinedMenuItem::separator(),
                #[cfg(target_os = "macos")]
                &PredefinedMenuItem::services(None),
                #[cfg(target_os = "macos")]
                &PredefinedMenuItem::separator(),
                #[cfg(target_os = "macos")]
                &PredefinedMenuItem::hide(None),
                #[cfg(target_os = "macos")]
                &PredefinedMenuItem::hide_others(None),
                #[cfg(target_os = "macos")]
                &PredefinedMenuItem::show_all(None),
                #[cfg(target_os = "macos")]
                &PredefinedMenuItem::separator(),
                &MenuItem::with_id(
                    "jereide: quit",
                    "Quit",
                    true,
                    "CmdOrCtrl+Q".parse::<Accelerator>().ok(),
                ),
            ])
            .ok();

        let file_menu = Submenu::with_id("file", "File", true);
        // The file menu
        file_menu
            .append_items(&[
                &MenuItem::with_id(
                    "file: new",
                    "New",
                    true,
                    "CmdOrCtrl+N".parse::<Accelerator>().ok(),
                ),
                &MenuItem::with_id(
                    "file: open",
                    "Open...",
                    true,
                    "CmdOrCtrl+O".parse::<Accelerator>().ok(),
                ),
                &MenuItem::with_id(
                    "file: save",
                    "Save",
                    true,
                    "CmdOrCtrl+S".parse::<Accelerator>().ok(),
                ),
                &MenuItem::with_id(
                    "file: save as",
                    "Save As…",
                    true,
                    "CmdOrCtrl+Shift+S".parse::<Accelerator>().ok(),
                ),
                &PredefinedMenuItem::separator(),
                &MenuItem::with_id(
                    "file: close tab",
                    "Close Tab",
                    true,
                    "CmdOrCtrl+W".parse::<Accelerator>().ok(),
                ),
            ])
            .ok();

        // The edit menu
        let edit_menu = Submenu::with_id("edit", "Edit", true);
        edit_menu
            .append_items(&[
                &MenuItem::with_id(
                    "editor: undo",
                    "Undo",
                    true,
                    "CmdOrCtrl+Z".parse::<Accelerator>().ok(),
                ),
                &MenuItem::with_id(
                    "editor: redo",
                    "Redo",
                    true,
                    "CmdOrCtrl+Shift+Z".parse::<Accelerator>().ok(),
                ),
                &PredefinedMenuItem::separator(),
                &MenuItem::with_id(
                    "editor: cut",
                    "Cut",
                    true,
                    "CmdOrCtrl+X".parse::<Accelerator>().ok(),
                ),
                &MenuItem::with_id(
                    "editor: copy",
                    "Copy",
                    true,
                    "CmdOrCtrl+C".parse::<Accelerator>().ok(),
                ),
                &MenuItem::with_id(
                    "editor: paste",
                    "Paste",
                    true,
                    "CmdOrCtrl+V".parse::<Accelerator>().ok(),
                ),
                &PredefinedMenuItem::separator(),
                &MenuItem::with_id(
                    "editor: select all",
                    "Select All",
                    true,
                    "CmdOrCtrl+A".parse::<Accelerator>().ok(),
                ),
            ])
            .ok();

        // The view menu
        let view_menu = Submenu::with_id("view", "View", true);
        view_menu
            .append_items(&[
                &MenuItem::with_id(
                    "command palette: toggle",
                    "Command Palette",
                    true,
                    "CmdOrCtrl+Shift+P".parse::<Accelerator>().ok(),
                ),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::fullscreen(None),
            ])
            .ok();

        let help_menu = Submenu::with_id("help", "Help", true);
        help_menu
            .append_items(&[&MenuItem::with_id(
                "jereide: star on github",
                "Star on GitHub",
                true,
                None,
            )])
            .ok();

        // Put everything together
        let menu = Menu::new();
        menu.append(&app_menu).ok();
        menu.append(&file_menu).ok();
        menu.append(&edit_menu).ok();
        menu.append(&view_menu).ok();
        menu.append(&help_menu).ok();

        let receiver = MenuEvent::receiver();
        Self {
            menu,
            receiver,
            initialized: false,
        }
    }

    pub fn init(&self, _raw: Option<RawWindowHandle>) {
        #[cfg(target_os = "macos")]
        self.menu.init_for_nsapp();

        #[cfg(target_os = "windows")]
        if let Some(RawWindowHandle::Win32(win32)) = _raw {
            unsafe { self.menu.init_for_hwnd(win32.hwnd.get()) };
        }
    }

    pub fn poll_events(&self) -> Vec<MenuId> {
        let mut events = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event.id);
        }
        events
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn set_initialized(&mut self) {
        self.initialized = true;
    }
}
