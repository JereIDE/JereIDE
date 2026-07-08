//! Trait-based subsystem definitions for optional editor features.
//!
//! Each subsystem represents an optional editor feature.
//! The `EditorSubsystems` container holds an
//! `Option` for each, and the event loop dispatches conditionally.

/// Sidebar file tree panel.
pub trait SidebarSubsystem {
    /// Whether the sidebar should be active at all.
    fn is_enabled(&self) -> bool {
        true
    }
}

/// Integrated terminal emulator.
pub trait TerminalSubsystem {
    fn is_enabled(&self) -> bool {
        true
    }
}

/// Language Server Protocol client.
pub trait LspSubsystem {
    fn is_enabled(&self) -> bool {
        true
    }
}

/// Git integration (status, blame, log, gutter decorations).
pub trait GitSubsystem {
    fn is_enabled(&self) -> bool {
        true
    }
}

/// Fuzzy file picker and command view for file/folder/recent navigation.
pub trait PickerSubsystem {
    fn is_enabled(&self) -> bool {
        true
    }
}

/// Project-wide search and replace.
pub trait FindInFilesSubsystem {
    fn is_enabled(&self) -> bool {
        true
    }
}

/// Toolbar with icon buttons.
pub trait ToolbarSubsystem {
    fn is_enabled(&self) -> bool {
        true
    }
}

/// Bookmark support (per-file line bookmarks).
pub trait BookmarkSubsystem {
    fn is_enabled(&self) -> bool {
        true
    }
}

/// Code folding support.
pub trait FoldingSubsystem {
    fn is_enabled(&self) -> bool {
        true
    }
}

/// Check-for-updates feature.
pub trait UpdateCheckSubsystem {
    fn is_enabled(&self) -> bool {
        true
    }
}

/// Marker type for enabled subsystems.
pub struct Enabled;

impl SidebarSubsystem for Enabled {}
impl TerminalSubsystem for Enabled {}
impl LspSubsystem for Enabled {}
impl GitSubsystem for Enabled {}
impl PickerSubsystem for Enabled {}
impl FindInFilesSubsystem for Enabled {}
impl ToolbarSubsystem for Enabled {}
impl BookmarkSubsystem for Enabled {}
impl FoldingSubsystem for Enabled {}
impl UpdateCheckSubsystem for Enabled {}

/// Optional editor subsystems injected at startup.
///
/// JereIDE populates all fields by default. Minimal configurations
/// may leave most fields `None`. The native event loop checks each
/// `Option` before dispatching to subsystem-specific code paths.
pub struct EditorSubsystems {
    pub sidebar: Option<Box<dyn SidebarSubsystem>>,
    pub terminal: Option<Box<dyn TerminalSubsystem>>,
    pub lsp: Option<Box<dyn LspSubsystem>>,
    pub git: Option<Box<dyn GitSubsystem>>,
    pub picker: Option<Box<dyn PickerSubsystem>>,
    pub find_in_files: Option<Box<dyn FindInFilesSubsystem>>,
    pub toolbar: Option<Box<dyn ToolbarSubsystem>>,
    pub bookmarks: Option<Box<dyn BookmarkSubsystem>>,
    pub folding: Option<Box<dyn FoldingSubsystem>>,
    pub update_check: Option<Box<dyn UpdateCheckSubsystem>>,
}

impl EditorSubsystems {
    /// All subsystems disabled (minimal mode).
    pub fn none() -> Self {
        Self {
            sidebar: None,
            terminal: None,
            lsp: None,
            git: None,
            picker: None,
            find_in_files: None,
            toolbar: None,
            bookmarks: None,
            folding: None,
            update_check: None,
        }
    }

    /// All subsystems enabled (JereIDE).
    pub fn all() -> Self {
        Self {
            sidebar: Some(Box::new(Enabled)),
            terminal: Some(Box::new(Enabled)),
            lsp: Some(Box::new(Enabled)),
            git: Some(Box::new(Enabled)),
            picker: Some(Box::new(Enabled)),
            find_in_files: Some(Box::new(Enabled)),
            toolbar: Some(Box::new(Enabled)),
            bookmarks: Some(Box::new(Enabled)),
            folding: Some(Box::new(Enabled)),
            update_check: Some(Box::new(Enabled)),
        }
    }

    pub fn has_sidebar(&self) -> bool {
        self.sidebar.is_some()
    }
    pub fn has_terminal(&self) -> bool {
        self.terminal.is_some()
    }
    pub fn has_lsp(&self) -> bool {
        self.lsp.is_some()
    }
    pub fn has_git(&self) -> bool {
        self.git.is_some()
    }
    pub fn has_picker(&self) -> bool {
        self.picker.is_some()
    }
    pub fn has_find_in_files(&self) -> bool {
        self.find_in_files.is_some()
    }
    pub fn has_toolbar(&self) -> bool {
        self.toolbar.is_some()
    }
    pub fn has_bookmarks(&self) -> bool {
        self.bookmarks.is_some()
    }
    pub fn has_folding(&self) -> bool {
        self.folding.is_some()
    }
    pub fn has_update_check(&self) -> bool {
        self.update_check.is_some()
    }
}


