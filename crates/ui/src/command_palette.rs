use super::palette::{Palette, PaletteItem};

#[derive(Clone)]
pub enum Command {
    NewFile,
    OpenFile,
    Save,
    SaveAs,
    CloseTab,
    Quit,
    ToggleFullscreen,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    OpenGithub,
}

pub struct CommandPalette {
    palette: Palette<Command>,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            palette: Palette::new(Self::items()),
        }
    }

    fn items() -> Vec<PaletteItem<Command>> {
        vec![
            PaletteItem {
                label: "New File",
                description: "Create a new file",
                shortcut: "⌘N",
                data: Command::NewFile,
            },
            PaletteItem {
                label: "Open File",
                description: "Open a file from disk",
                shortcut: "⌘O",
                data: Command::OpenFile,
            },
            PaletteItem {
                label: "Save",
                description: "Save the current file",
                shortcut: "⌘S",
                data: Command::Save,
            },
            PaletteItem {
                label: "Save As",
                description: "Save the current file with a new name",
                shortcut: "⌘⇧S",
                data: Command::SaveAs,
            },
            PaletteItem {
                label: "Close Tab",
                description: "Close the current tab",
                shortcut: "⌘W",
                data: Command::CloseTab,
            },
            PaletteItem {
                label: "Quit",
                description: "Quit JereIDE",
                shortcut: "⌘Q",
                data: Command::Quit,
            },
            PaletteItem {
                label: "Toggle Fullscreen",
                description: "Toggle fullscreen mode",
                shortcut: "",
                data: Command::ToggleFullscreen,
            },
            PaletteItem {
                label: "Undo",
                description: "Undo the last action",
                shortcut: "⌘Z",
                data: Command::Undo,
            },
            PaletteItem {
                label: "Redo",
                description: "Redo the last undone action",
                shortcut: "⌘⇧Z",
                data: Command::Redo,
            },
            PaletteItem {
                label: "Cut",
                description: "Cut selected text",
                shortcut: "⌘X",
                data: Command::Cut,
            },
            PaletteItem {
                label: "Copy",
                description: "Copy selected text",
                shortcut: "⌘C",
                data: Command::Copy,
            },
            PaletteItem {
                label: "Paste",
                description: "Paste from clipboard",
                shortcut: "⌘V",
                data: Command::Paste,
            },
            PaletteItem {
                label: "Select All",
                description: "Select all text",
                shortcut: "⌘A",
                data: Command::SelectAll,
            },
            PaletteItem {
                label: "Open GitHub",
                description: "Open JereIDE repository on GitHub",
                shortcut: "",
                data: Command::OpenGithub,
            },
        ]
    }

    pub fn render(&mut self, ctx: &eframe::egui::Context, open: &mut bool) -> Option<Command> {
        self.palette.render(ctx, "Command Palette", open)
    }
}
