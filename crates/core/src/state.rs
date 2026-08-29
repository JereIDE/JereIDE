use eframe::egui;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TAB_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_PANE_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_SPLIT_ID: AtomicUsize = AtomicUsize::new(0);

fn next_tab_id() -> usize {
    NEXT_TAB_ID.fetch_add(1, Ordering::Relaxed)
}

fn next_pane_id() -> usize {
    NEXT_PANE_ID.fetch_add(1, Ordering::Relaxed)
}

fn next_split_id() -> usize {
    NEXT_SPLIT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum CurrentView {
    Code,
    Compose,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Clone)]
pub struct Pane {
    pub id: usize,
    pub active_tab_index: usize,
    pub tab_indices: Vec<usize>,
}

impl Pane {
    pub fn new(tab_index: usize) -> Self {
        Self {
            id: next_pane_id(),
            active_tab_index: tab_index,
            tab_indices: vec![tab_index],
        }
    }

    pub fn new_empty() -> Self {
        Self {
            id: next_pane_id(),
            active_tab_index: 0,
            tab_indices: vec![],
        }
    }
}

#[derive(Clone)]
pub enum PaneLayout {
    Single(Pane),
    Split {
        id: usize,
        direction: SplitDirection,
        first: Box<PaneLayout>,
        second: Box<PaneLayout>,
        split_ratio: f32,
    },
}

impl PaneLayout {
    pub fn new() -> Self {
        PaneLayout::Single(Pane::new_empty())
    }

    pub fn split(&mut self, direction: SplitDirection) {
        let current = std::mem::replace(self, PaneLayout::new());
        let current_tab = current.get_active_pane().active_tab_index;
        let new_pane = Pane::new(current_tab);
        *self = PaneLayout::Split {
            id: next_split_id(),
            direction,
            first: Box::new(current),
            second: Box::new(PaneLayout::Single(new_pane)),
            split_ratio: 0.5,
        };
    }

    pub fn close_pane(&mut self, pane_id: usize) -> bool {
        match self {
            PaneLayout::Single(pane) if pane.id == pane_id => {
                return false;
            }
            PaneLayout::Split { first, second, .. } => {
                if first.contains_pane(pane_id) {
                    if let PaneLayout::Single(_) = **first {
                        *self = (**second).clone();
                        return true;
                    }
                    return first.close_pane(pane_id);
                } else if second.contains_pane(pane_id) {
                    if let PaneLayout::Single(_) = **second {
                        *self = (**first).clone();
                        return true;
                    }
                    return second.close_pane(pane_id);
                }
            }
            _ => {}
        }
        false
    }

    pub fn contains_pane(&self, pane_id: usize) -> bool {
        match self {
            PaneLayout::Single(pane) => pane.id == pane_id,
            PaneLayout::Split { first, second, .. } => {
                first.contains_pane(pane_id) || second.contains_pane(pane_id)
            }
        }
    }

    pub fn get_active_pane(&self) -> &Pane {
        match self {
            PaneLayout::Single(pane) => pane,
            PaneLayout::Split { first, .. } => first.get_active_pane(),
        }
    }

    pub fn get_active_pane_mut(&mut self) -> &mut Pane {
        match self {
            PaneLayout::Single(pane) => pane,
            PaneLayout::Split { first, .. } => first.get_active_pane_mut(),
        }
    }

    pub fn set_focus(&mut self, pane_id: usize) -> bool {
        match self {
            PaneLayout::Single(pane) => pane.id == pane_id,
            PaneLayout::Split { first, second, .. } => {
                first.set_focus(pane_id) || second.set_focus(pane_id)
            }
        }
    }

    pub fn iter_panes(&self) -> Vec<&Pane> {
        match self {
            PaneLayout::Single(pane) => vec![pane],
            PaneLayout::Split { first, second, .. } => {
                let mut panes = first.iter_panes();
                panes.extend(second.iter_panes());
                panes
            }
        }
    }

    pub fn empty_pane_ids(&self) -> Vec<usize> {
        self.iter_panes()
            .iter()
            .filter(|p| p.tab_indices.is_empty())
            .map(|p| p.id)
            .collect()
    }

    pub fn iter_panes_mut(&mut self) -> Vec<&mut Pane> {
        match self {
            PaneLayout::Single(pane) => vec![pane],
            PaneLayout::Split { first, second, .. } => {
                let mut panes = first.iter_panes_mut();
                panes.extend(second.iter_panes_mut());
                panes
            }
        }
    }

    pub fn set_split_ratio(&mut self, ratio: f32) {
        if let PaneLayout::Split { split_ratio, .. } = self {
            *split_ratio = ratio;
        }
    }

    pub fn as_split_mut(&mut self) -> Option<(&mut PaneLayout, &mut PaneLayout)> {
        match self {
            PaneLayout::Split { first, second, .. } => Some((first, second)),
            _ => None,
        }
    }
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
    pub editor_focused: bool,
    pub current_view: CurrentView,
    pub was_fullscreen: bool,
    pub document_edited: bool,

    pub pending_close_index: Option<usize>,

    pub pending_quit: bool,

    pub pending_large_file_blocked: Option<u64>,
    pub pending_large_file_warn: Option<(String, u64)>,
    pub pending_binary_file: Option<String>,

    pub command_palette_open: bool,

    pub sidebar_open: bool,

    pub sidebar_width: f32,

    pub show_about_dialog: bool,

    pub pending_open: bool,
    pub pending_open_project: bool,
    pub pending_open_file: Option<String>,

    /// Split pane layout
    pub pane_layout: PaneLayout,
    /// Currently focused pane ID
    pub focused_pane_id: usize,
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
        let pane_layout = PaneLayout::new();
        let focused_pane_id = pane_layout.get_active_pane().id;
        Self {
            tabs: vec![],
            current_project_dir: None,
            selected_text: None,
            find_highlight: None,
            pending_find_selection: None,
            go_to_line_scroll_to: None,
            editor_focused: false,
            current_view: CurrentView::Code,
            was_fullscreen: false,
            document_edited: false,
            pending_close_index: None,
            pending_quit: false,
            pending_large_file_blocked: None,
            pending_large_file_warn: None,
            pending_binary_file: None,
            command_palette_open: false,
            sidebar_open: false,
            sidebar_width: 280.0,
            show_about_dialog: false,
            pending_open: false,
            pending_open_project: false,
            pending_open_file: None,
            pane_layout,
            focused_pane_id,
        }
    }

    pub fn current_tab(&self) -> &Tab {
        let active_pane = self.get_focused_pane();
        &self.tabs[active_pane.active_tab_index]
    }

    pub fn editor_id(&self) -> egui::Id {
        let active_pane = self.get_focused_pane();
        egui::Id::new((
            "editor",
            active_pane.id,
            self.tabs[active_pane.active_tab_index].id,
        ))
    }

    pub fn current_tab_mut(&mut self) -> &mut Tab {
        let active_tab_index = self.focused_tab_index();
        &mut self.tabs[active_tab_index]
    }

    pub fn is_modified(&self) -> bool {
        self.current_tab().is_modified()
    }

    pub fn mark_saved(&mut self) {
        self.current_tab_mut().mark_saved();
    }

    pub fn get_focused_pane(&self) -> &Pane {
        self.pane_layout.get_active_pane()
    }

    pub fn get_focused_pane_mut(&mut self) -> &mut Pane {
        self.pane_layout.get_active_pane_mut()
    }

    pub fn focused_tab_index(&self) -> usize {
        self.get_focused_pane().active_tab_index
    }

    pub fn get_pane(&self, pane_id: usize) -> Option<&Pane> {
        self.pane_layout
            .iter_panes()
            .into_iter()
            .find(|p| p.id == pane_id)
    }

    pub fn get_pane_mut(&mut self, pane_id: usize) -> Option<&mut Pane> {
        self.pane_layout
            .iter_panes_mut()
            .into_iter()
            .find(|p| p.id == pane_id)
    }

    pub fn set_focused_pane(&mut self, pane_id: usize) {
        self.focused_pane_id = pane_id;
        self.pane_layout.set_focus(pane_id);
    }

    pub fn split_pane(&mut self, direction: SplitDirection) {
        self.pane_layout.split(direction);
    }

    pub fn close_current_pane(&mut self) {
        let focused_pane_id = self.focused_pane_id;
        if !self.pane_layout.close_pane(focused_pane_id) {
            return;
        }

        if let Some(pane) = self.pane_layout.iter_panes().first() {
            self.focused_pane_id = pane.id;
        }
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
                let pane = self.get_focused_pane_mut();
                pane.active_tab_index = i;
                if !pane.tab_indices.contains(&i) {
                    pane.tab_indices.push(i);
                }
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
        let pane = self.get_focused_pane_mut();
        pane.active_tab_index = idx;
        pane.tab_indices.push(idx);
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
        let pane = self.get_focused_pane_mut();
        pane.active_tab_index = idx;
        pane.tab_indices.push(idx);
        log::info!(
            "created new blank tab {idx} ({} tabs total)",
            self.tabs.len()
        );
        idx
    }

    pub fn close_tab(&mut self, index: usize, pane_id: usize) {
        let path = self.tabs[index].file_path.clone();

        // Remove the tab index from the closing pane's tab_indices only
        if let Some(pane) = self.get_pane_mut(pane_id) {
            pane.tab_indices.retain(|&i| i != index);
        }

        // Check if any pane still references this tab
        let still_referenced = self
            .pane_layout
            .iter_panes()
            .iter()
            .any(|p| p.tab_indices.contains(&index));

        if !still_referenced {
            self.tabs.remove(index);

            // Adjust indices in all panes
            let new_tab_count = self.tabs.len();
            for pane in self.pane_layout.iter_panes_mut() {
                pane.tab_indices.iter_mut().for_each(|i| {
                    if *i > index {
                        *i -= 1;
                    }
                });
                if pane.active_tab_index == index {
                    if new_tab_count == 0 {
                        pane.active_tab_index = 0;
                    } else if index >= new_tab_count {
                        pane.active_tab_index = new_tab_count - 1;
                    } else {
                        pane.active_tab_index = index;
                    }
                } else if pane.active_tab_index > index {
                    pane.active_tab_index -= 1;
                }
                // Ensure active_tab_index is in tab_indices
                if !pane.tab_indices.is_empty()
                    && !pane.tab_indices.contains(&pane.active_tab_index)
                {
                    pane.active_tab_index = *pane.tab_indices.last().unwrap_or(&0);
                }
            }
        } else {
            // Tab is still referenced by other panes; just fix the closing pane
            if let Some(pane) = self.get_pane_mut(pane_id) {
                if pane.active_tab_index == index {
                    if let Some(&last) = pane.tab_indices.last() {
                        pane.active_tab_index = last;
                    }
                } else if pane.active_tab_index > index {
                    pane.active_tab_index -= 1;
                }
            }
        }

        log::info!(
            "closed tab {index} from pane {pane_id} (was {:?}); now {} tabs",
            path,
            self.tabs.len()
        );

        // Close any panes that have no tabs left
        let empty_ids = self.pane_layout.empty_pane_ids();
        for pid in empty_ids {
            self.pane_layout.close_pane(pid);
        }

        // Update focused pane if needed
        if let Some(pane) = self.pane_layout.iter_panes().first() {
            self.focused_pane_id = pane.id;
        }
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
