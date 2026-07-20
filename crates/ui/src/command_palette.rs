use super::palette::PaletteItem;

pub fn items() -> Vec<PaletteItem> {
    vec![
        PaletteItem { code: "file: new" },
        PaletteItem { code: "file: open" },
        PaletteItem { code: "file: save" },
        PaletteItem { code: "file: save as" },
        PaletteItem { code: "file: close tab" },
        PaletteItem { code: "editor: undo" },
        PaletteItem { code: "editor: redo" },
        PaletteItem { code: "editor: cut" },
        PaletteItem { code: "editor: copy" },
        PaletteItem { code: "editor: paste" },
        PaletteItem { code: "editor: select all" },
        PaletteItem { code: "command palette: toggle" },
        PaletteItem { code: "jereide: toggle fullscreen" },
        PaletteItem { code: "jereide: quit" },
        PaletteItem { code: "jereide: about" },
        PaletteItem { code: "jereide: star on github" },
    ]
}
