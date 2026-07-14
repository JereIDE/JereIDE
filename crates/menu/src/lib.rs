use muda::{
    accelerator::Accelerator, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu,
};

/// A struct about the menu.
pub struct AppMenu {
    menu: Menu,
    receiver: &'static crossbeam_channel::Receiver<MenuEvent>,
    initialized: bool,
}

impl AppMenu {
    /// Creates all these stuff
    pub fn new() -> Self {
        let app_menu = Submenu::with_id("jereide", "jereide", true);
        // Add lots of predefined items and a Star on GitHub
        app_menu
            .append_items(&[
                &PredefinedMenuItem::about(None, None),
                &MenuItem::with_id("githubstar", "Star on GitHub", true, None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::services(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::hide(None),
                &PredefinedMenuItem::hide_others(None),
                &PredefinedMenuItem::show_all(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::quit(None),
            ])
            .ok();

        let file_menu = Submenu::with_id("file", "File", true);
        // The file menu
        file_menu
            .append_items(&[
                &MenuItem::with_id(
                    "new",
                    "New",
                    true,
                    Some("Cmd+N".parse::<Accelerator>().unwrap()),
                ),
                &MenuItem::with_id(
                    "open",
                    "Open...",
                    true,
                    Some("Cmd+O".parse::<Accelerator>().unwrap()),
                ),
                &MenuItem::with_id(
                    "save",
                    "Save",
                    true,
                    Some("Cmd+S".parse::<Accelerator>().unwrap()),
                ),
                &MenuItem::with_id(
                    "save_as",
                    "Save As…",
                    true,
                    Some("Cmd+Shift+S".parse::<Accelerator>().unwrap()),
                ),
            ])
            .ok();

        // The edit menu
        let edit_menu = Submenu::with_id("edit", "Edit", true);
        edit_menu
            .append_items(&[
                &MenuItem::with_id(
                    "undo",
                    "Undo",
                    true,
                    Some("Cmd+Z".parse::<Accelerator>().unwrap()),
                ),
                &MenuItem::with_id(
                    "redo",
                    "Redo",
                    true,
                    Some("Cmd+Shift+Z".parse::<Accelerator>().unwrap()),
                ),
                &PredefinedMenuItem::separator(),
                &MenuItem::with_id(
                    "cut",
                    "Cut",
                    true,
                    Some("Cmd+X".parse::<Accelerator>().unwrap()),
                ),
                &MenuItem::with_id(
                    "copy",
                    "Copy",
                    true,
                    Some("Cmd+C".parse::<Accelerator>().unwrap()),
                ),
                &MenuItem::with_id(
                    "paste",
                    "Paste",
                    true,
                    Some("Cmd+V".parse::<Accelerator>().unwrap()),
                ),
                &PredefinedMenuItem::separator(),
                &MenuItem::with_id(
                    "select_all",
                    "Select All",
                    true,
                    Some("Cmd+A".parse::<Accelerator>().unwrap()),
                ),
            ])
            .ok();

        // The view menu
        let view_menu = Submenu::with_id("view", "View", true);
        view_menu
            .append_items(&[&PredefinedMenuItem::fullscreen(None)])
            .ok();

        // TODO: A help menu

        // Put everything together
        let menu = Menu::new();
        menu.append(&app_menu).ok();
        menu.append(&file_menu).ok();
        menu.append(&edit_menu).ok();
        menu.append(&view_menu).ok();

        let receiver = MenuEvent::receiver();
        Self {
            menu,
            receiver,
            initialized: false,
        }
    }

    pub fn init(&self) {
        #[cfg(target_os = "macos")]
        self.menu.init_for_nsapp();
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
