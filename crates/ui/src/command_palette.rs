use super::palette::PaletteItem;

pub fn items() -> Vec<PaletteItem> {
    vec![
        PaletteItem { code: "file: new" },
        PaletteItem { code: "file: open" },
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
            code: "command palette: toggle",
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
    fn command_palette_has_16_items() {
        let items = items();
        assert_eq!(items.len(), 16);
    }

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
    fn command_palette_has_file_operations() {
        let items = items();
        assert!(items.iter().any(|i| i.code == "file: new"));
        assert!(items.iter().any(|i| i.code == "file: open"));
        assert!(items.iter().any(|i| i.code == "file: save"));
        assert!(items.iter().any(|i| i.code == "file: save as"));
        assert!(items.iter().any(|i| i.code == "file: close tab"));
    }

    #[test]
    fn command_palette_has_editor_operations() {
        let items = items();
        assert!(items.iter().any(|i| i.code == "editor: undo"));
        assert!(items.iter().any(|i| i.code == "editor: redo"));
        assert!(items.iter().any(|i| i.code == "editor: cut"));
        assert!(items.iter().any(|i| i.code == "editor: copy"));
        assert!(items.iter().any(|i| i.code == "editor: paste"));
        assert!(items.iter().any(|i| i.code == "editor: select all"));
    }

    #[test]
    fn command_palette_has_jereide_operations() {
        let items = items();
        assert!(items.iter().any(|i| i.code == "jereide: quit"));
        assert!(items.iter().any(|i| i.code == "jereide: about"));
        assert!(items.iter().any(|i| i.code == "jereide: toggle fullscreen"));
        assert!(items.iter().any(|i| i.code == "jereide: star on github"));
    }

    #[test]
    fn command_palette_has_toggle() {
        let items = items();
        assert!(items.iter().any(|i| i.code == "command palette: toggle"));
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
