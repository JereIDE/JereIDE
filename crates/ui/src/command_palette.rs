use jereide_widgets::palette::PaletteItem;

pub fn items() -> Vec<PaletteItem> {
    vec![
        PaletteItem { code: "file: new" },
        PaletteItem { code: "file: open" },
        PaletteItem {
            code: "file: open project",
        },
        PaletteItem { code: "file: save" },
        PaletteItem {
            code: "file: save as",
        },
        PaletteItem {
            code: "file: close tab",
        },
        PaletteItem {
            code: "editor: undo",
        },
        PaletteItem {
            code: "editor: redo",
        },
        PaletteItem {
            code: "editor: cut",
        },
        PaletteItem {
            code: "editor: copy",
        },
        PaletteItem {
            code: "editor: paste",
        },
        PaletteItem {
            code: "editor: select all",
        },
        PaletteItem {
            code: "editor: find replace",
        },
        PaletteItem {
            code: "editor: go to line",
        },
        PaletteItem {
            code: "command palette: toggle",
        },
        PaletteItem {
            code: "view: toggle sidebar",
        },
        PaletteItem { code: "view: code" },
        PaletteItem {
            code: "view: compose",
        },
        PaletteItem {
            code: "jereide: open settings file",
        },
        PaletteItem {
            code: "jereide: open docs",
        },
        PaletteItem {
            code: "jereide: view log",
        },
        PaletteItem {
            code: "jereide: toggle fullscreen",
        },
        PaletteItem {
            code: "jereide: quit",
        },
        PaletteItem {
            code: "jereide: about",
        },
        PaletteItem {
            code: "jereide: star on github",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_palette_no_duplicate_codes() {
        let items = items();
        let mut codes: Vec<&str> = items.iter().map(|i| i.code).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), items.len());
    }

    #[test]
    fn command_palette_all_codes_have_colon() {
        for item in items() {
            assert!(
                item.code.contains(": "),
                "code {:?} is missing ': ' separator",
                item.code
            );
        }
    }

    #[test]
    fn command_palette_items_have_no_whitespace_prefix() {
        for item in items() {
            assert!(
                !item.code.starts_with(' '),
                "code {:?} starts with space",
                item.code
            );
            assert!(
                !item.code.ends_with(' '),
                "code {:?} ends with space",
                item.code
            );
        }
    }
}
