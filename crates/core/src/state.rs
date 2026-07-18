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

/// A single open document (tab) in the IDE.
#[derive(Clone)]
pub struct Tab {
    pub id: usize,

    pub text: String,

    pub saved_text: String,
    pub file_path: Option<String>,
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
            cursor_line: 1,
            cursor_col: 1,
        }
    }

    pub fn is_modified(&self) -> bool {
        self.text != self.saved_text
    }

    pub fn mark_saved(&mut self) {
        self.saved_text = self.text.clone();
    }

    /// Returns the file name to display (e.g. "main.rs") or "Untitled".
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

    pub active_tab_index: usize,
    pub editor_focused: bool,
    pub editor_id: egui::Id,
    pub current_view: CurrentView,
    pub was_fullscreen: bool,
    pub document_edited: bool,

    pub pending_close_index: Option<usize>,

    pub pending_large_file_blocked: Option<u64>,
    pub pending_large_file_warn: Option<(String, u64)>,

    pub command_palette_open: bool,
}

/// Another new method.
impl AppState {
    pub fn new() -> Self {
        Self {
            tabs: vec![Tab::new()],
            active_tab_index: 0,
            editor_focused: false,
            editor_id: egui::Id::new("editor"),
            current_view: CurrentView::Code,
            was_fullscreen: false,
            document_edited: false,
            pending_close_index: None,
            pending_large_file_blocked: None,
            pending_large_file_warn: None,
            command_palette_open: false,
        }
    }

    /// Returns a shared reference to the active tab.
    pub fn current_tab(&self) -> &Tab {
        &self.tabs[self.active_tab_index]
    }

    /// Returns a mutable reference to the active tab.
    pub fn current_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab_index]
    }

    pub fn is_modified(&self) -> bool {
        self.current_tab().is_modified()
    }

    /// Marks the active tab's text as "saved".
    pub fn mark_saved(&mut self) {
        self.current_tab_mut().mark_saved();
    }

    pub fn open_file(&mut self, path: String, content: String) -> usize {
        // Check if this file is already open
        for (i, tab) in self.tabs.iter().enumerate() {
            if tab.file_path.as_deref() == Some(&path) {
                self.active_tab_index = i;
                return i;
            }
        }
        // Otherwise create a new tab
        let tab = Tab::with_path_and_content(path, content);
        self.tabs.push(tab);
        let idx = self.tabs.len() - 1;
        self.active_tab_index = idx;
        idx
    }

    /// Adds a new empty tab and returns its index.
    pub fn new_tab(&mut self) -> usize {
        self.tabs.push(Tab::new());
        let idx = self.tabs.len() - 1;
        self.active_tab_index = idx;
        idx
    }

    pub fn close_tab(&mut self, index: usize) {
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.active_tab_index = 0;
        } else if self.active_tab_index >= self.tabs.len() {
            self.active_tab_index = self.tabs.len() - 1;
        } else if index < self.active_tab_index {
            self.active_tab_index -= 1;
        }
    }

    pub fn switch_to_view(&mut self, target: CurrentView) {
        if target != self.current_view {
            self.current_view = target;
        }
    }
}
