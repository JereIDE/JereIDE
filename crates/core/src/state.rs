use eframe::egui;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TAB_ID: AtomicUsize = AtomicUsize::new(0);

fn next_tab_id() -> usize {
    NEXT_TAB_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum CurrentView {
    Code,
    Compose,
}

#[derive(Clone)]
pub struct Tab {
    pub id: usize,

    pub text: String,

    pub saved_text: String,
    pub file_path: Option<String>,
    pub read_only: bool,
    pub cursor_line: usize,
    pub cursor_col: usize,
}

impl Tab {
    pub fn new() -> Self {
        Self {
            id: next_tab_id(),
            text: String::new(),
            saved_text: String::new(),
            file_path: None,
            read_only: false,
            cursor_line: 1,
            cursor_col: 1,
        }
    }

    pub fn with_path_and_content(path: String, content: String) -> Self {
        Self {
            id: next_tab_id(),
            saved_text: content.clone(),
            text: content,
            file_path: Some(path),
            read_only: false,
            cursor_line: 1,
            cursor_col: 1,
        }
    }

    pub fn with_path_and_content_read_only(path: String, content: String) -> Self {
        Self {
            id: next_tab_id(),
            saved_text: content.clone(),
            text: content,
            file_path: Some(path),
            read_only: true,
            cursor_line: 1,
            cursor_col: 1,
        }
    }

    pub fn is_modified(&self) -> bool {
        !self.read_only && self.text != self.saved_text
    }

    pub fn mark_saved(&mut self) {
        self.saved_text = self.text.clone();
    }

    /// Returns the file name to display (e.g. "main.rs") or "Untitled" if the file isn't a file,
    /// like, if it was created fresh.
    pub fn file_name(&self) -> String {
        self.file_path
            .as_ref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Untitled".to_string())
    }
}

/// Includes the cursor line/col, the current code text, the focusing stuff, etc
pub struct AppState {
    /// All open documents.
    pub tabs: Vec<Tab>,
    pub current_project_dir: Option<String>,
    pub selected_text: Option<String>,
    pub find_highlight: Option<FindHighlight>,
    pub pending_find_selection: Option<(usize, usize)>,
    pub go_to_line_scroll_to: Option<usize>,
    pub active_tab_index: usize,
    pub editor_focused: bool,
    pub current_view: CurrentView,
    pub was_fullscreen: bool,
    pub document_edited: bool,

    pub pending_close_index: Option<usize>,

    pub pending_quit: bool,

    pub pending_large_file_blocked: Option<u64>,
    pub pending_large_file_warn: Option<(String, u64)>,

    pub command_palette_open: bool,

    pub sidebar_open: bool,

    pub sidebar_width: f32,

    pub show_about_dialog: bool,

    pub pending_open: bool,
    pub pending_open_project: bool,
    pub pending_open_file: Option<String>,
}

#[derive(Clone)]
pub struct FindHighlight {
    pub query: String,
    pub match_case: bool,
    pub whole_word: bool,
    pub current_match: usize,
    pub scroll_to: Option<usize>,
}

/// Starts an AppState with all the default stuff
impl AppState {
    pub fn new() -> Self {
        Self {
            tabs: vec![],
            current_project_dir: None,
            selected_text: None,
            find_highlight: None,
            pending_find_selection: None,
            go_to_line_scroll_to: None,
            active_tab_index: 0,
            editor_focused: false,
            current_view: CurrentView::Code,
            was_fullscreen: false,
            document_edited: false,
            pending_close_index: None,
            pending_quit: false,
            pending_large_file_blocked: None,
            pending_large_file_warn: None,
            command_palette_open: false,
            sidebar_open: false,
            sidebar_width: 280.0,
            show_about_dialog: false,
            pending_open: false,
            pending_open_project: false,
            pending_open_file: None,
        }
    }

    pub fn current_tab(&self) -> &Tab {
        &self.tabs[self.active_tab_index]
    }

    pub fn editor_id(&self) -> egui::Id {
        egui::Id::new(("editor", self.tabs[self.active_tab_index].id))
    }

    pub fn current_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab_index]
    }

    pub fn is_modified(&self) -> bool {
        self.current_tab().is_modified()
    }

    pub fn mark_saved(&mut self) {
        self.current_tab_mut().mark_saved();
    }

    pub fn open_file(&mut self, path: String, content: String) -> usize {
        self.open_path(path, content, false)
    }

    pub fn open_read_only(&mut self, path: String, content: String) -> usize {
        self.open_path(path, content, true)
    }

    fn open_path(&mut self, path: String, content: String, read_only: bool) -> usize {
        // This thingie checks if this file is already open
        for (i, tab) in self.tabs.iter().enumerate() {
            if tab.file_path.as_deref() == Some(&path) {
                log::info!(
                    "switched to already-open tab {i} for {:?} ({} chars)",
                    path,
                    content.chars().count()
                );
                self.active_tab_index = i;
                return i;
            }
        }
        // If it isn't, I probably need a new tab
        let tab = if read_only {
            Tab::with_path_and_content_read_only(path.clone(), content.clone())
        } else {
            Tab::with_path_and_content(path.clone(), content.clone())
        };
        self.tabs.push(tab);
        let idx = self.tabs.len() - 1;
        self.active_tab_index = idx;
        log::info!(
            "opened {:?} in new tab {idx} ({} tabs, {} chars)",
            path,
            self.tabs.len(),
            content.chars().count()
        );
        idx
    }

    pub fn new_tab(&mut self) -> usize {
        self.tabs.push(Tab::new());
        let idx = self.tabs.len() - 1;
        self.active_tab_index = idx;
        log::info!(
            "created new blank tab {idx} ({} tabs total)",
            self.tabs.len()
        );
        idx
    }

    pub fn close_tab(&mut self, index: usize) {
        let path = self.tabs[index].file_path.clone();
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.active_tab_index = 0;
        } else if self.active_tab_index >= self.tabs.len() {
            self.active_tab_index = self.tabs.len() - 1;
        } else if index < self.active_tab_index {
            self.active_tab_index -= 1;
        }
        log::info!(
            "closed tab {index} (was {:?}); now {} tabs, active tab {}",
            path,
            self.tabs.len(),
            self.active_tab_index
        );
    }

    pub fn first_dirty_tab_index(&self) -> Option<usize> {
        self.tabs.iter().position(|t| t.is_modified())
    }

    pub fn switch_to_view(&mut self, target: CurrentView) {
        if target != self.current_view {
            log::info!(
                "switching view from {:?} to {:?}",
                self.current_view,
                target
            );
            self.current_view = target;
        }
    }
}
