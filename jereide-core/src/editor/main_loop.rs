//! Editor main loop.
//!
//! The module is split into two visually-distinct sections: `pub fn run`
//! and the big `#[cfg(feature = "sdl")] fn run` it delegates to live
//! near the top; the bottom 1.4k lines are supporting helpers most of
//! which only make sense when the `sdl` feature is on. Those helpers
//! are bulk-gated via the `sdl_only!` macro below so each one doesn't
//! need its own `#[cfg(feature = "sdl")]` attribute.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crossbeam_channel::Receiver;
use notify::{Event, RecursiveMode, Watcher};

// Module-level editor mode. Set once at startup, read by helper functions.
thread_local! {
    static SINGLE_FILE_MODE: Cell<bool> = const { Cell::new(false) };
}

/// Whether the editor is running in single-file mode.
pub(crate) fn is_single_file() -> bool {
    SINGLE_FILE_MODE.with(|c| c.get())
}

/// Whether git integration is active (inverse of single-file mode).
fn use_git() -> bool {
    !is_single_file()
}

use crate::editor::buffer;
use crate::editor::config::NativeConfig;
use crate::editor::context_menu::{ContextMenu, MenuItem};
use crate::editor::doc_view::{
    DocView, RenderLine, build_render_lines, click_to_doc_pos, syntax_color,
};
use crate::editor::empty_view::EmptyView;
use crate::editor::event::{EditorEvent, MouseButton};
use crate::editor::git_helpers;
use crate::editor::keymap::NativeKeymap;
use crate::editor::project_search;
use crate::editor::lsp;
use crate::editor::lsp_client::*;
use crate::editor::status_view::{StatusItem, StatusView};
use crate::editor::storage;
use crate::editor::terminal_panel::*;
use crate::editor::tokenizer::{self, CompiledSyntax};
use crate::editor::view::{UpdateContext, View};
use crate::editor::word_index::WordIndex;

/// Append a timestamped message to the log file in the user directory.
#[cfg(feature = "sdl")]
fn log_to_file(userdir: &str, msg: &str) {
    let path = Path::new(userdir).join("jereide.log");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        use std::io::Write;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = writeln!(f, "[{ts}] {msg}");
    }
}

/// A single entry in the file tree sidebar.
struct SidebarEntry {
    name: String,
    path: String,
    is_dir: bool,
    depth: usize,
    expanded: bool,
}

/// Width of the sidebar in logical pixels.
const DEFAULT_SIDEBAR_W: f64 = 200.0;
const MIN_SIDEBAR_W: f64 = 100.0;
/// Editor's share of the editor|markdown-preview split, as a fraction of the
/// shared content area. Stored as a fraction (not pixels) so the split tracks
/// the window width across resizes; persisted per app via the `session` store.
const DEFAULT_PREVIEW_SPLIT: f64 = 0.5;
const MIN_PREVIEW_SPLIT: f64 = 0.2;
const MAX_PREVIEW_SPLIT: f64 = 0.8;
/// Collapse redundant `.` segments in a path string. Preserves a single
/// leading `./` for relative paths and leaves absolute paths intact.
/// Does not touch `..` segments (we don't want to silently traverse symlinks).
pub(crate) fn normalize_path(p: &str) -> String {
    use std::path::Component;
    let path = Path::new(p);
    let mut out = PathBuf::new();
    let mut started_with_curdir = false;
    let mut has_anchor = false;
    for comp in path.components() {
        match comp {
            Component::CurDir => {
                if !has_anchor && !started_with_curdir {
                    out.push(".");
                    started_with_curdir = true;
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                out.push(comp.as_os_str());
                has_anchor = true;
            }
            _ => {
                out.push(comp.as_os_str());
                has_anchor = true;
            }
        }
    }
    if out.as_os_str().is_empty() {
        ".".to_string()
    } else {
        out.to_string_lossy().to_string()
    }
}

/// Filter + sort `sidebar_entries` for notes-mode display.
/// Returns indices into `sidebar_entries` in the order they should be
/// rendered. `sort_mode`: 0 = A-Z asc, 1 = A-Z desc, 2 = recent-first,
/// 3 = oldest-first. Filter is a case-insensitive substring match on
/// the entry name (empty = no filter).
fn compute_notes_display_order(
    entries: &[SidebarEntry],
    search: &str,
    sort_mode: u8,
) -> Vec<usize> {
    let needle = search.to_lowercase();
    let mut indices: Vec<usize> = (0..entries.len())
        .filter(|&i| {
            if needle.is_empty() {
                true
            } else {
                entries[i].name.to_lowercase().contains(&needle)
            }
        })
        .collect();
    match sort_mode {
        0 => indices.sort_by(|&a, &b| {
            entries[a]
                .name
                .to_lowercase()
                .cmp(&entries[b].name.to_lowercase())
        }),
        1 => indices.sort_by(|&a, &b| {
            entries[b]
                .name
                .to_lowercase()
                .cmp(&entries[a].name.to_lowercase())
        }),
        2 | 3 => {
            let mtime = |path: &str| -> std::time::SystemTime {
                std::fs::metadata(path)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            };
            indices.sort_by(|&a, &b| {
                let ta = mtime(&entries[a].path);
                let tb = mtime(&entries[b].path);
                if sort_mode == 2 {
                    tb.cmp(&ta)
                } else {
                    ta.cmp(&tb)
                }
            });
        }
        _ => {}
    }
    indices
}

/// Wrapper around `scan_directory` that, in notes-mode, flattens to a
/// `*.md`-only top-level list (no folders, no recursion).
fn scan_for_sidebar(notes_mode: bool, dir: &str, show_hidden: bool) -> Vec<SidebarEntry> {
    let entries = scan_directory(dir, 0, show_hidden);
    if notes_mode {
        entries
            .into_iter()
            .filter(|e| !e.is_dir && e.name.to_lowercase().ends_with(".md"))
            .collect()
    } else {
        entries
    }
}

/// Scan a directory non-recursively and return sorted sidebar entries at the given depth.
fn scan_directory(dir: &str, depth: usize, show_hidden: bool) -> Vec<SidebarEntry> {
    let mut entries = Vec::new();
    let Ok(read) = std::fs::read_dir(dir) else {
        return entries;
    };
    for entry in read.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().to_string();
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        entries.push(SidebarEntry {
            name,
            path: entry.path().to_string_lossy().to_string(),
            is_dir: meta.is_dir(),
            depth,
            expanded: false,
        });
    }
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    entries
}

/// Expand previously-saved sidebar folders, inserting children as needed.
fn restore_expanded_folders(
    sidebar_entries: &mut Vec<SidebarEntry>,
    userdir: &std::path::Path,
    show_hidden: bool,
    session_key: &str,
) {
    let key = format!("{session_key}_expanded");
    let Ok(Some(data)) = storage::load_text(userdir, "project_session", &key) else {
        return;
    };
    let Ok(expanded) = serde_json::from_str::<Vec<String>>(&data) else {
        return;
    };
    let expanded_set: HashSet<&str> = expanded.iter().map(|s| s.as_str()).collect();
    // Iterate by index because expanding inserts children, shifting subsequent indices.
    let mut i = 0;
    while i < sidebar_entries.len() {
        if sidebar_entries[i].is_dir
            && !sidebar_entries[i].expanded
            && expanded_set.contains(sidebar_entries[i].path.as_str())
        {
            sidebar_entries[i].expanded = true;
            let children = scan_directory(
                &sidebar_entries[i].path,
                sidebar_entries[i].depth + 1,
                show_hidden,
            );
            let insert_at = i + 1;
            for (j, child) in children.into_iter().enumerate() {
                sidebar_entries.insert(insert_at + j, child);
            }
        }
        i += 1;
    }
}

/// Save the set of expanded sidebar folder paths for a project.
fn save_expanded_folders(
    sidebar_entries: &[SidebarEntry],
    userdir: &std::path::Path,
    session_key: &str,
) {
    let expanded: Vec<&str> = sidebar_entries
        .iter()
        .filter(|e| e.is_dir && e.expanded)
        .map(|e| e.path.as_str())
        .collect();
    if expanded.is_empty() {
        let _ = storage::clear(
            userdir,
            "project_session",
            Some(&format!("{session_key}_expanded")),
        );
    } else {
        let _ = storage::save_text(
            userdir,
            "project_session",
            &format!("{session_key}_expanded"),
            &serde_json::to_string(&expanded).unwrap_or_default(),
        );
    }
}

/// Re-expand sidebar directories from an in-memory set of previously expanded paths.
fn expand_sidebar_from_set(
    sidebar_entries: &mut Vec<SidebarEntry>,
    expanded: &HashSet<String>,
    show_hidden: bool,
) {
    let mut i = 0;
    while i < sidebar_entries.len() {
        if sidebar_entries[i].is_dir
            && !sidebar_entries[i].expanded
            && expanded.contains(&sidebar_entries[i].path)
        {
            sidebar_entries[i].expanded = true;
            let children = scan_directory(
                &sidebar_entries[i].path,
                sidebar_entries[i].depth + 1,
                show_hidden,
            );
            let insert_at = i + 1;
            for (j, child) in children.into_iter().enumerate() {
                sidebar_entries.insert(insert_at + j, child);
            }
        }
        i += 1;
    }
}

/// A file-type icon: Seti font codepoint + color.
struct FileIcon {
    /// Unicode codepoint in the Seti icon font.
    codepoint: u32,
    color: [u8; 4],
}

/// Load file-extension to icon mapping from the JSON config.
fn load_file_icons(datadir: &str) -> std::collections::HashMap<String, FileIcon> {
    let path = Path::new(datadir).join("assets").join("file_icons.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return std::collections::HashMap::new();
    };
    let Ok(map) =
        serde_json::from_str::<std::collections::HashMap<String, serde_json::Value>>(&text)
    else {
        return std::collections::HashMap::new();
    };
    map.into_iter()
        .filter_map(|(ext, val)| {
            let obj = val.as_object()?;
            let codepoint = obj.get("codepoint")?.as_u64()? as u32;
            let color = obj.get("color")?.as_str().and_then(parse_hex_color)?;
            Some((ext, FileIcon { codepoint, color }))
        })
        .collect()
}

/// Parse "#rrggbb" into [r, g, b, 255].
fn parse_hex_color(s: &str) -> Option<[u8; 4]> {
    let hex = s.strip_prefix('#')?;
    if hex.len() < 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b, 255])
}

/// File watcher state for autoreload on external changes.
///
/// We watch each file's *parent directory* (not the file inode) and
/// filter events by filename. This is the standard robust pattern for
/// inotify-backed watchers: an external editor saving via write-to-temp
/// and atomic rename replaces the file's inode, which silently breaks a
/// file-inode watch (only the first save fires and all subsequent ones
/// miss). Watching the directory sidesteps that entirely.
pub(crate) struct AutoreloadState {
    watcher: Option<notify::RecommendedWatcher>,
    rx: Option<Receiver<notify::Result<Event>>>,
    /// Watched file paths mapped to the directory registered with
    /// notify. Used to filter events and to look up which dir to
    /// decrement in `unwatch`.
    watched_files: HashMap<String, PathBuf>,
    /// Reference count per watched directory so the last file in a
    /// directory unwatches it, but shared dirs stay watched while any
    /// of their files are open.
    watched_dirs: HashMap<PathBuf, usize>,
}

impl AutoreloadState {
    fn new() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        let watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })
        .ok();
        Self {
            watcher,
            rx: Some(rx),
            watched_files: HashMap::new(),
            watched_dirs: HashMap::new(),
        }
    }

    /// Start watching a file path for external changes.
    pub(crate) fn watch(&mut self, path: &str) {
        if self.watched_files.contains_key(path) {
            return;
        }
        let file_path = PathBuf::from(path);
        let dir = match file_path.parent() {
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => return,
        };
        let count = self.watched_dirs.entry(dir.clone()).or_insert(0);
        if *count == 0 {
            if let Some(ref mut w) = self.watcher {
                if w.watch(&dir, RecursiveMode::NonRecursive).is_err() {
                    self.watched_dirs.remove(&dir);
                    return;
                }
            }
        }
        *self.watched_dirs.get_mut(&dir).expect("just inserted") += 1;
        self.watched_files.insert(path.to_string(), dir);
    }

    /// Stop watching a file path.
    pub(crate) fn unwatch(&mut self, path: &str) {
        let Some(dir) = self.watched_files.remove(path) else {
            return;
        };
        if let Some(count) = self.watched_dirs.get_mut(&dir) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.watched_dirs.remove(&dir);
                if let Some(ref mut w) = self.watcher {
                    let _ = w.unwatch(&dir);
                }
            }
        }
    }

    /// Drain pending events and return paths of modified files.
    fn poll_changed(&self) -> Vec<String> {
        let mut changed = Vec::new();
        if let Some(ref rx) = self.rx {
            while let Ok(event) = rx.try_recv() {
                if let Ok(ev) = event {
                    use notify::EventKind;
                    // Creates count too: an atomic save rename replaces
                    // the target with a fresh inode, which arrives as a
                    // Create event on the dir watcher.
                    let is_interesting =
                        matches!(ev.kind, EventKind::Modify(_) | EventKind::Create(_));
                    if !is_interesting {
                        continue;
                    }
                    for p in &ev.paths {
                        if let Some(s) = p.to_str() {
                            if self.watched_files.contains_key(s) {
                                changed.push(s.to_string());
                            }
                        }
                    }
                }
            }
        }
        changed
    }
}

/// Watches project directories so the sidebar refreshes when the filesystem changes.
struct SidebarWatcher {
    watcher: Option<notify::RecommendedWatcher>,
    rx: Option<Receiver<notify::Result<Event>>>,
    watched_dirs: HashSet<PathBuf>,
}

impl SidebarWatcher {
    fn new() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        let watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })
        .ok();
        Self {
            watcher,
            rx: Some(rx),
            watched_dirs: HashSet::new(),
        }
    }

    fn watch_dir(&mut self, dir: &str) {
        let path = PathBuf::from(dir);
        if self.watched_dirs.contains(&path) {
            return;
        }
        if let Some(ref mut w) = self.watcher {
            if w.watch(&path, RecursiveMode::NonRecursive).is_ok() {
                self.watched_dirs.insert(path);
            }
        }
    }

    fn unwatch_dir(&mut self, dir: &str) {
        let path = PathBuf::from(dir);
        if self.watched_dirs.remove(&path) {
            if let Some(ref mut w) = self.watcher {
                let _ = w.unwatch(&path);
            }
        }
    }

    fn unwatch_all(&mut self) {
        let dirs: Vec<PathBuf> = self.watched_dirs.drain().collect();
        if let Some(ref mut w) = self.watcher {
            for dir in &dirs {
                let _ = w.unwatch(dir);
            }
        }
    }

    /// Returns true if any directory-listing change (create/remove/rename) was detected.
    fn poll_changed(&self) -> bool {
        let Some(ref rx) = self.rx else {
            return false;
        };
        let mut changed = false;
        while let Ok(event) = rx.try_recv() {
            if let Ok(ev) = event {
                use notify::EventKind;
                if matches!(
                    ev.kind,
                    EventKind::Create(_)
                        | EventKind::Remove(_)
                        | EventKind::Modify(notify::event::ModifyKind::Name(_))
                ) {
                    changed = true;
                }
            }
        }
        changed
    }
}

/// Comment style chosen for the toggle-line-comments command.
#[derive(Debug, Clone)]
pub(crate) enum CommentMarker {
    /// `prefix` is prepended after the indent (e.g. `//` for Rust, `#` for Python).
    Line(String),
    /// `(open, close)` wraps each line individually (e.g. `<!-- ... -->` for HTML).
    /// Used for languages that have no line-comment form.
    Block(String, String),
}

/// Resolve the comment marker for a document based on its filename's matched
/// syntax. Returns `None` when no syntax matches or when the language has
/// neither a line- nor a block-comment form — callers must treat that as
/// "no-op" rather than substituting a default.
fn comment_marker_for_path(
    path: &str,
    index: &[crate::editor::syntax::SyntaxEntry],
) -> Option<CommentMarker> {
    if path.is_empty() {
        return None;
    }
    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path);
    let entry = crate::editor::syntax::match_syntax_entry(filename, index)?;
    if let Some(line) = &entry.comment {
        return Some(CommentMarker::Line(line.clone()));
    }
    entry
        .block_comment
        .as_ref()
        .map(|(o, c)| CommentMarker::Block(o.clone(), c.clone()))
}

/// Truncate a tab name to `max_chars` characters, appending an ellipsis when
/// the original is longer. Operates on Unicode scalar values so multi-byte
/// filenames don't get cut mid-codepoint.
fn truncate_tab_name(name: &str, max_chars: usize) -> String {
    if name.chars().count() <= max_chars {
        return name.to_string();
    }
    let prefix: String = name.chars().take(max_chars).collect();
    format!("{prefix}...")
}

/// Map a Markdown fenced-code `lang` tag (e.g. from ```` ```python ````) to
/// the file extension our bundled syntax index keys on. Unknown or empty
/// tags fall back to the tag itself so anything the index already matches
/// directly (like `sh`, `rs`, `go`) still works without a special case.
fn markdown_lang_to_ext(lang: &str) -> &str {
    match lang.to_ascii_lowercase().as_str() {
        "rust" => "rs",
        "gossamer" => "gos",
        "python" | "python3" => "py",
        "javascript" | "node" => "js",
        "typescript" => "ts",
        "shell" | "bash" | "zsh" => "sh",
        "c++" | "cplusplus" => "cpp",
        "c#" | "csharp" => "cs",
        "golang" => "go",
        "yaml" => "yml",
        "markdown" => "md",
        "ruby" => "rb",
        "kotlin" => "kt",
        "ocaml" => "ml",
        "perl" => "pl",
        "elixir" => "ex",
        _ => lang,
    }
}

/// Run the editor main loop. Returns true if restart requested.
#[cfg(feature = "sdl")]
pub fn run(
    mut config: NativeConfig,
    _args: &[String],
    datadir: &str,
    userdir: &str,
    subsystems: crate::editor::subsystems::EditorSubsystems,
) -> bool {
    let single_file_mode = !subsystems.has_sidebar();
    SINGLE_FILE_MODE.with(|c| c.set(single_file_mode));
    if single_file_mode {
        crate::renderer::font::set_glyph_cache_limit(1024);
        crate::renderer::font::set_skip_prewarm(true);
        config.max_undos = 100;
    }
    // macOS: aggressively lower the glyph-cache ceiling + skip ASCII
    // prewarm on every auxiliary font (h1/h2/h3/big/icon_big/seti).
    // Only `ui` and `code` see sustained glyph traffic; the rest barely
    // touch their cache, so warming 95 ASCII glyphs per face wastes ~2-3 MB
    // upfront. macOS pays the highest price for this since Metal keeps
    // each glyph's backing bitmap resident in the GPU's shared memory.
    #[cfg(target_os = "macos")]
    {
        crate::renderer::font::set_glyph_cache_limit(512);
        crate::renderer::font::set_skip_prewarm(true);
    }

    // Create window.
    if !crate::window::restore_window() {
        let window_title = "JereIDE";
        if let Err(e) = crate::window::create_window(window_title) {
            log::error!("Window creation failed: {e}");
            return false;
        }
    }

    // Restore saved window size/position.
    let userdir_path = Path::new(userdir);
    if let Ok(Some(win_json)) = storage::load_text(userdir_path, "session", "window") {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&win_json) {
            if let (Some(w), Some(h), Some(x), Some(y)) = (
                val["w"].as_i64(),
                val["h"].as_i64(),
                val["x"].as_i64(),
                val["y"].as_i64(),
            ) {
                crate::window::set_window_size(w as i32, h as i32, x as i32, y as i32);
            }
        }
    }

    // Enable text input events from SDL.
    crate::window::start_text_input();

    // Load fonts and build style from config.
    // Restore saved font size if available.
    let mut config = config;
    let userdir_path = std::path::Path::new(userdir);
    if let Ok(Some(size_str)) =
        crate::editor::storage::load_text(userdir_path, "session", "font_size")
    {
        if let Ok(size) = size_str.trim().parse::<f32>() {
            let base_size = (size / crate::window::get_display_scale() as f32) as u32;
            config.fonts.ui.size = base_size;
            config.fonts.code.size = base_size;
        }
    }

    let mut font_warning: Option<String> = None;
    let mut draw_ctx = match load_fonts(&config) {
        Ok(ctx) => ctx,
        Err(e) => {
            // Font loading failed (custom path or missing data dir). Try
            // resetting to the built-in defaults before giving up entirely.
            let msg = format!("Font loading failed: {e} -- falling back to defaults");
            log::warn!("{msg}");
            font_warning = Some(msg);
            config.fonts = crate::editor::config::FontsConfig::default();
            config.resolve_font_paths(datadir);
            match load_fonts(&config) {
                Ok(ctx) => ctx,
                Err(e2) => {
                    log::error!("Default font loading also failed: {e2}");
                    eprintln!("Error: could not load any fonts. {e2}");
                    return false;
                }
            }
        }
    };
    let display_scale = crate::window::get_display_scale();
    let mut style = build_style(&config, &draw_ctx);

    // Load theme colors from JSON.
    let theme_name = &config.theme;
    let theme_path = Path::new(datadir)
        .join("assets")
        .join("themes")
        .join(format!("{theme_name}.json"))
        .to_string_lossy()
        .into_owned();
    if let Ok(palette) = crate::editor::style::load_theme_palette(&theme_path) {
        apply_theme_to_style(&mut style, &palette);
    } else {
        eprintln!("Theme not found: {theme_path}, using defaults");
    }
    // Build list of available themes.
    let available_themes: Vec<String> = {
        let themes_dir = Path::new(datadir)
            .join("assets")
            .join("themes")
            .to_string_lossy()
            .into_owned();
        let mut themes = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&themes_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if let Some(stem) = name.strip_suffix(".json") {
                        themes.push(stem.to_string());
                    }
                }
            }
        }
        themes.sort();
        themes
    };
    let mut current_theme_idx = available_themes
        .iter()
        .position(|t| t == theme_name)
        .unwrap_or(0);
    style.scale = display_scale;
    style.padding_x *= display_scale;
    style.padding_y *= display_scale;
    style.divider_size = (style.divider_size * display_scale).ceil();
    style.scrollbar_size *= display_scale;
    style.caret_width = (style.caret_width * display_scale).ceil();
    style.tab_width *= display_scale;
    crate::editor::style_ctx::set_current_style(style.clone());

    // Build native keymap.
    let mut keymap = NativeKeymap::with_defaults();
    keymap.add_from_config(&config.keybindings);

    // Create the view tree: EmptyView (center) + StatusView (bottom).
    // No TitleView -- the OS window title bar is sufficient.

    let mut empty_view = EmptyView::new();
    empty_view.version = format!("{} v{}", "JereIDE", env!("CARGO_PKG_VERSION"),);
    for (fmt, cmd) in EmptyView::commands() {
        if let Some(binding) = keymap.get_binding_display(cmd) {
            empty_view
                .display_commands
                .push(fmt.replace("%s", &binding));
        }
    }

    let mut status_view = StatusView::new();
    status_view.left_items.push(StatusItem {
        text: "JereIDE".to_string(),
        color: None,
        command: None,
    });
    status_view.right_items.push(StatusItem {
        text: format!("v{}", env!("CARGO_PKG_VERSION")),
        color: None,
        command: None,
    });

    // Open files from CLI args. Per-tab state and session/file I/O live
    // in `crate::editor::open_doc`.
    use crate::editor::open_doc::{
        BG_LOAD_THRESHOLD, OpenDoc, check_file_size_limit, doc_is_modified, nag_msg_close,
        nag_msg_quit, open_file_into, project_session_key, restore_project_session,
        save_project_session, scroll_new_doc_to_line, split_path_line,
    };

    let mut docs: Vec<OpenDoc> = Vec::new();
    let mut active_tab: usize = 0;

    let line_h_for_scroll = style.line_height();
    let mut has_cli_files = false;
    let mut cli_project_root: Option<String> = None;
    for arg in _args.iter().skip(1) {
        if arg.starts_with('-') {
            continue;
        }
        // If the argument is a directory, open it as the project folder.
        let p = std::path::Path::new(arg);
        if p.is_dir() {
            has_cli_files = true;
            let abs = std::path::absolute(p)
                .map(|a| a.to_string_lossy().to_string())
                .unwrap_or_else(|_| arg.to_string());
            cli_project_root = Some(abs);
            continue;
        }
        // Nano-Anvil: single file only -- skip additional args.
        if single_file_mode && has_cli_files {
            break;
        }
        has_cli_files = true;
        let (path, goto_line) = split_path_line(arg);
        // If path:N doesn't exist as-is but path does, use the split version.
        let (actual_path, line) = if goto_line.is_some()
            && !std::path::Path::new(arg).is_file()
            && std::path::Path::new(path).is_file()
        {
            (path, goto_line)
        } else {
            (arg.as_str(), None)
        };
        if open_file_into(actual_path, &mut docs, use_git()) {
            if let Some(ln) = line {
                scroll_new_doc_to_line(&mut docs, ln, line_h_for_scroll);
            }
        }
    }

    // Session restore: JereIDE restores previous session.
    let mut restored_project = String::new();
    if !single_file_mode && !has_cli_files {
        if let Ok(Some(data)) = storage::load_text(userdir_path, "session", "files") {
            if let Ok(session) = serde_json::from_str::<crate::editor::open_doc::SessionData>(&data)
            {
                for (i, file) in session.files.iter().enumerate() {
                    if file == "__untitled__" {
                        let buf_id = buffer::insert_buffer(buffer::default_buffer_state());
                        if let Some(content) = session.unsaved_content.get(i) {
                            if !content.is_empty() {
                                let _ = buffer::with_buffer_mut(buf_id, |b| {
                                    b.lines = content.lines().map(|l| format!("{l}\n")).collect();
                                    if b.lines.is_empty() {
                                        b.lines.push("\n".to_string());
                                    }
                                    b.change_id += 1;
                                    Ok(())
                                });
                            }
                        }
                        let mut dv = DocView::new();
                        dv.buffer_id = Some(buf_id);
                        docs.push(OpenDoc {
                            view: dv,
                            path: String::new(),
                            name: "untitled".to_string(),
                            saved_change_id: 1,
                            saved_signature: buffer::content_signature(&["\n".to_string()]),
                            indent_type: "soft".to_string(),
                            indent_size: 2,
                            git_changes: std::collections::HashMap::new(),
                            cached_render: std::sync::Arc::new(Vec::new()),
                            cached_change_id: -1,
                            cached_scroll_y: -1.0,
                            cached_hint_count: 0,
                            cached_rect_w: -1.0,
                            cached_rect_h: -1.0,
                            dirty_cache: std::cell::Cell::new(None),
                            token_cache: std::cell::RefCell::new(
                                crate::editor::open_doc::TokenCache::default(),
                            ),
                            preview: crate::editor::markdown_preview::MarkdownPreviewState::default(
                            ),
                        });
                    } else {
                        open_file_into(file, &mut docs, use_git());
                    }
                }
                if session.active < docs.len() {
                    active_tab = session.active;
                }
                restored_project = session.active_project;
            }
        }
    }

    // Nano-Anvil: always ensure exactly one document (blank if no CLI file).
    if single_file_mode && docs.is_empty() {
        let buf_state = buffer::default_buffer_state();
        let initial_change_id = buf_state.change_id;
        let buf_id = buffer::insert_buffer(buf_state);
        let mut dv = DocView::new();
        dv.buffer_id = Some(buf_id);
        docs.push(OpenDoc {
            view: dv,
            path: String::new(),
            name: "untitled".to_string(),
            saved_change_id: initial_change_id,
            saved_signature: 0,
            indent_type: "soft".to_string(),
            indent_size: 4,
            git_changes: std::collections::HashMap::new(),
            cached_render: std::sync::Arc::new(Vec::new()),
            cached_change_id: -1,
            cached_scroll_y: -1.0,
            cached_hint_count: 0,
            cached_rect_w: -1.0,
            cached_rect_h: -1.0,
            dirty_cache: std::cell::Cell::new(None),
            token_cache: std::cell::RefCell::new(crate::editor::open_doc::TokenCache::default()),
            preview: crate::editor::markdown_preview::MarkdownPreviewState::default(),
        });
    }

    // Sidebar state.
    // Load saved sidebar width.
    let mut sidebar_width: f64 =
        crate::editor::storage::load_text(userdir_path, "session", "sidebar_width")
            .ok()
            .flatten()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(DEFAULT_SIDEBAR_W);
    let mut sidebar_dragging = false;
    // Terminal panel height override from user drag. `None` means use the
    // default 30% of window height.
    let mut terminal_h_override: Option<f64> =
        crate::editor::storage::load_text(userdir_path, "session", "terminal_height")
            .ok()
            .flatten()
            .and_then(|s| s.trim().parse().ok());
    let mut terminal_divider_dragging = false;
    // Editor|markdown-preview split fraction, persisted per app. Loaded the
    // same way the sidebar width is, then clamped so neither pane collapses.
    let mut preview_split: f64 =
        crate::editor::storage::load_text(userdir_path, "session", "preview_split")
            .ok()
            .flatten()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(DEFAULT_PREVIEW_SPLIT)
            .clamp(MIN_PREVIEW_SPLIT, MAX_PREVIEW_SPLIT);
    let mut preview_dragging = false;
    // Held while the mouse is pressed on the editor's vertical
    // scrollbar; lets drag-scrolling track the cursor until release.
    // The drag offset is the pixel gap between the top of the thumb and
    // the click y at the moment the press began, so the thumb stays
    // anchored to the grip point as the mouse moves.
    let mut editor_sb_dragging = false;
    let mut editor_sb_drag_offset: f64 = 0.0;
    let mut terminal_sb_dragging = false;
    let mut terminal_sb_drag_offset: f64 = 0.0;
    let mut sidebar_sb_dragging = false;
    let mut sidebar_sb_drag_offset: f64 = 0.0;
    let mut editor_mouse_down = false;
    // Last (buffer, selection range) mirrored into the X11 PRIMARY selection.
    // Keyed so a non-empty selection is pushed once per change rather than
    // every frame, and a re-selection after a foreign app grabbed PRIMARY
    // re-asserts ownership (the intervening caret changes the key).
    let mut last_primary_key: (u64, Vec<usize>) = (0, Vec::new());
    // Local shift-key tracker. SDL's mouse events don't carry modifier state,
    // so tracking it from keyboard events directly by key name makes shift+click
    // robust against any SDL_GetModState quirks on different platforms/WMs.
    let mut shift_held = false;
    let mut tab_dragging: Option<usize> = None;
    // Dropdown menu shown when the tab bar overflows — lists every open tab
    // so they stay reachable even when their labels don't fit on screen.
    let mut tab_dropdown_open: bool = false;
    // Suppresses the hover tooltip after a tab click until the mouse leaves
    // the tab bar; prevents the tooltip from lingering on the selected tab.
    let mut tab_tooltip_suppressed: bool = false;
    // Tab targeted by the most recent right-click; consumed by the
    // tab:close / close-left / close-right / close-all dispatch in the
    // context-menu click handler.
    let mut tab_menu_target: Option<usize> = None;
    let mut mouse_x: f64 = 0.0;
    let mut mouse_y: f64 = 0.0;
    let mut sidebar_entries: Vec<SidebarEntry>;
    let mut sidebar_watcher = SidebarWatcher::new();
    let mut sidebar_scroll: f64 = 0.0;
    // Content height + scrollbar track geometry captured during the sidebar
    // draw so the click/drag paths can reuse the same numbers instead of
    // recomputing the filtered notes-mode entry list.
    let mut sidebar_content_h: f64 = 0.0;
    let mut sidebar_sb_top: f64 = 0.0;
    let mut sidebar_sb_h: f64 = 0.0;
    let mut sidebar_hovered_index: Option<usize>;
    let mut sidebar_menu_pinned_index: Option<usize> = None;

    // Determine project root for sidebar.
    // Notes-mode forces the configured notes folder so the sidebar
    // always reflects the user's notes dir even after the user changes
    // NOTE_ANVIL_DIR. Otherwise CLI folder overrides everything, then
    // restored project, then nothing. If a file was passed via CLI (no
    // folder), don't open a project.
    let mut project_root: String = if let Some(folder) = subsystems.notes_folder() {
        folder.to_string()
    } else if let Some(root) = cli_project_root {
        root
    } else if has_cli_files {
        // Files passed on CLI -- no project folder.
        String::new()
    } else if !restored_project.is_empty() && Path::new(&restored_project).is_dir() {
        restored_project
    } else {
        String::new()
    };

    let mut sidebar_show_hidden = false;
    // Set while the window is occluded or hidden. We skip the render
    // pass entirely while true so Metal/IOSurface/RenCache buffers
    // don't get touched for frames nobody will see. Reset on Exposed
    // / Shown / FocusGained.
    let mut window_hidden: bool = false;
    // Idle-drop: if the user hasn't interacted for a while, release the
    // glyph / render caches. They'll be rebuilt on the next draw.
    let mut last_activity: Instant = Instant::now();
    let mut dropped_caches_for_idle: bool = false;
    const IDLE_DROP_SECS: u64 = 60;
    // macOS memory-pressure poll: check every N seconds. If the kernel
    // reports WARN or CRITICAL, drop caches immediately.
    let mut last_mem_pressure_check: Instant = Instant::now();
    const MEM_PRESSURE_CHECK_SECS: u64 = 5;
    // Notes-mode sidebar: search filter + sort. NoteSquirrel parity.
    // sort_mode: 0 = A-Z asc, 1 = A-Z desc, 2 = Recent (newest first),
    //            3 = Recent (oldest first).
    let mut notes_search: String = String::new();
    let mut notes_search_focused: bool = false;
    let mut notes_sort_mode: u8 =
        crate::editor::storage::load_text(userdir_path, "session", "notes_sort_mode")
            .ok()
            .flatten()
            .and_then(|s| s.trim().parse::<u8>().ok())
            .filter(|v| *v <= 3)
            .unwrap_or(0);
    let file_icons = load_file_icons(datadir);
    sidebar_entries = if subsystems.has_sidebar() && !project_root.is_empty() {
        scan_for_sidebar(
            subsystems.has_notes_mode(),
            &project_root,
            sidebar_show_hidden,
        )
    } else {
        Vec::new()
    };
    let mut sidebar_visible = subsystems.has_sidebar() && !project_root.is_empty();
    if subsystems.has_sidebar() {
        restore_expanded_folders(
            &mut sidebar_entries,
            userdir_path,
            sidebar_show_hidden,
            &project_session_key(&project_root),
        );
        if !project_root.is_empty() {
            sidebar_watcher.watch_dir(&project_root);
            for entry in &sidebar_entries {
                if entry.is_dir && entry.expanded {
                    sidebar_watcher.watch_dir(&entry.path);
                }
            }
        }
    }

    // Recent projects list (persisted).
    let mut recent_projects: Vec<String> =
        crate::editor::storage::load_text(userdir_path, "session", "recent_projects")
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
    // Add current project to recents.
    {
        let abs = std::fs::canonicalize(&project_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| project_root.clone());
        if !abs.is_empty() && !recent_projects.contains(&abs) {
            recent_projects.insert(0, abs);
            if recent_projects.len() > 20 {
                recent_projects.truncate(20);
            }
        }
    }

    // Recent files list (persisted, max 100).
    let mut recent_files: Vec<String> =
        crate::editor::storage::load_text(userdir_path, "session", "recent_files")
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

    let fps = config.fps as f64;
    let mut redraw = true;
    let mut quit = false;
    let mut window_title = String::new();
    // Cached (path, language-name) for the status bar so the syntax-entry glob
    // scan is not repeated on every redraw while the same file stays active.
    let mut status_lang_cache: (String, String) = (String::new(), String::new());
    let frame_interval = 1.0 / fps;
    // When the UI is idle - nothing drawn recently, no terminal panel open, no
    // background job running - the loop blocks this long between timer-driven
    // wakeups instead of spinning at the frame interval. Input and pushed
    // wake-up events still return from the blocked wait immediately, so this
    // lowers idle CPU/battery without affecting responsiveness.
    let idle_wait = 0.5_f64;
    let mut last_draw = Instant::now();
    // Deferred render-line cache: written at the top of the next frame to
    // avoid borrow-checker conflicts with the immutable doc borrow during
    // rendering. Includes the tab index so we write to the correct doc even
    // if the user switched tabs between frames.
    // Tuple layout: (tab_idx, buffer_id, lines, change_id, scroll_y). The
    // `buffer_id` is the only stable identity for the doc being rendered —
    // `tab_idx` aliases once the docs list is swapped (e.g. Open Recent
    // replaces the project), so the consumer uses `buffer_id` to skip
    // applying a render captured against a now-defunct doc.
    // The trailing `usize` is the number of LSP inlay hints actually folded
    // into the render. Recording the count used (after URI filtering) rather
    // than the global `lsp_state.inlay_hints.len()` keeps the cache key honest
    // when the active doc's URI doesn't match the held hints.
    // The last two `f64`s are the view width and height so the cache is
    // invalidated when the window is resized.
    type PendingRenderCache = Option<(
        usize,
        u64,
        std::sync::Arc<Vec<RenderLine>>,
        i64,
        f64,
        usize,
        f64,
        f64,
    )>;
    let mut pending_render_cache: PendingRenderCache = None;

    // Background file load job. When a large file is being loaded on a
    // background thread, this holds the progress atomics and the join handle.
    struct LoadJob {
        path: String,
        name: String,
        bytes_read: std::sync::Arc<std::sync::atomic::AtomicU64>,
        total_bytes: u64,
        handle: Option<std::thread::JoinHandle<Result<buffer::BufferState, String>>>,
    }
    let mut load_job: Option<LoadJob> = None;

    /// Spawn a background file load. Returns a LoadJob to poll each frame.
    fn spawn_load(path: &str, total: u64) -> LoadJob {
        use std::sync::atomic::{AtomicU64, Ordering};
        let bytes_read = std::sync::Arc::new(AtomicU64::new(0));
        let bytes_read_clone = bytes_read.clone();
        let path_owned = path.to_string();
        let name = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());
        let handle = std::thread::spawn(move || {
            let mut state = buffer::default_buffer_state();
            buffer::load_file_with_progress(&mut state, &path_owned, |bytes, _total| {
                bytes_read_clone.store(bytes, Ordering::Relaxed);
            })
            .map_err(|e| e.to_string())?;
            Ok(state)
        });
        LoadJob {
            path: path.to_string(),
            name,
            bytes_read,
            total_bytes: total,
            handle: Some(handle),
        }
    }

    // Find bar state.
    let mut find_active = false;
    let mut find_query = String::new();
    let mut replace_active = false;
    let mut replace_query = String::new();
    let mut find_focus_on_replace = false;
    let mut find_use_regex = false;
    let mut find_whole_word = false;
    let mut find_case_insensitive = false;
    // All current matches as (line, col, end_col) with 1-based columns.
    let mut find_matches: Vec<(usize, usize, usize, usize)> = Vec::new();
    let mut find_current: Option<usize> = None;
    // Anchor (line, col) captured when find is opened — live-search re-centers here
    // so typing a longer query doesn't skip past matches the user hasn't seen yet.
    let mut find_anchor: (usize, usize) = (1, 1);
    // Find-in-selection: when true, matches are limited to the captured range.
    let mut find_in_selection = false;
    // The selection range captured when find-in-selection was activated:
    // (start_line, start_col, end_line, end_col), all 1-based.
    let mut find_selection_range: Option<(usize, usize, usize, usize)> = None;

    // Nag bar state. The three prompts the editor can raise —
    // "unsaved changes on close/quit?", "file changed on disk, reload?",
    // and "parent directory missing, create and save?" — are modelled as
    // a single enum instead of three independent `bool + data` sets so
    // only one nag can be active at a time and the draw code can match
    // on it once.
    #[derive(Default)]
    enum Nag {
        #[default]
        None,
        /// "FOO has unsaved changes, close/quit anyway?" — `tab_to_close`
        /// is `Some(i)` to close that tab on Yes, `None` to quit.
        UnsavedChanges {
            message: String,
            tab_to_close: Option<usize>,
        },
        /// "File changed on disk, reload?" — applies to the doc at `path`.
        ReloadFromDisk { path: String },
        /// "Directory does not exist: PARENT. Create it and save?" — on
        /// Yes, mkdir -p the parent and save to `save_path`; other fields
        /// are needed to complete the interrupted Save / Save As.
        CreateDir {
            parent: String,
            save_path: String,
            doc_tab: usize,
            from_save_as: bool,
        },
        /// "FILE already exists, overwrite?" — Save As targeted an existing
        /// file that isn't the current doc's own path. Yes performs the
        /// save; No returns to the Save As picker so the user can pick a
        /// different name. Guards against autocomplete races where a
        /// late-arriving suggestion silently retargets Enter.
        OverwriteFile { save_path: String, doc_tab: usize },
        /// "No extension detected, save anyway?" — Save As target has no
        /// trailing `.ext`. Yes proceeds (and still checks for overwrite
        /// next); No returns to the picker so the user can add one.
        NoExtension { save_path: String, doc_tab: usize },
        /// "Delete FILE?" — sidebar Delete confirmation. Yes removes the
        /// file from disk and closes any open tab pointing to it; No
        /// dismisses without touching anything.
        DeleteFile { path: String },
    }
    impl Nag {
        fn is_unsaved(&self) -> bool {
            matches!(self, Nag::UnsavedChanges { .. })
        }
    }
    let mut nag = Nag::None;
    // Set by the KeyDown-side nag handlers so the immediately-following
    // SDL_TEXTINPUT event (which fires on every printable keystroke,
    // including Y / N) doesn't leak into the active document.
    let mut eat_next_text_input: bool = false;
    let mut info_message: Option<(String, Instant)> = font_warning.map(|msg| (msg, Instant::now()));

    // Command palette state.
    let mut palette_active = false;
    let mut palette_query = String::new();
    let mut palette_results: Vec<(String, String)> = Vec::new(); // (cmd_name, display_name)
    let mut palette_selected: usize = 0;

    // Theme picker state.
    let mut theme_picker_active = false;
    let mut theme_picker_query = String::new();
    let mut theme_picker_results: Vec<(String, String)> = Vec::new(); // (theme_name, display_name)
    let mut theme_picker_selected: usize = 0;
    let mut theme_picker_original_style: Option<crate::editor::style_ctx::StyleContext> = None;
    let mut theme_picker_original_idx: usize = 0;

    // Build command list for palette from keymap.
    let all_commands: Vec<(String, String)> = {
        let mut cmds = Vec::new();
        // Extract unique command names from keymap bindings, skipping the
        // raw key-input commands that aren't meaningful in the palette.
        let mut seen = std::collections::HashSet::new();
        for (stroke, cmd_names) in keymap.iter_bindings() {
            for cmd in cmd_names {
                if !crate::editor::keymap::is_palette_command(cmd) {
                    continue;
                }
                if seen.insert(cmd.clone()) {
                    let display = crate::editor::keymap::prettify_name(cmd);
                    cmds.push((cmd.clone(), format!("{display}  ({stroke})")));
                }
            }
        }
        // Commands available in the palette without a keybinding.
        let palette_extras: &[&str] = &[
            "core:sort-lines",
            "doc:reopen-closed-tab",
            "doc:convert-indentation",
            "doc:toggle-line-endings",
            "core:open-user-settings",
            "about:version",
            "core:force-quit",
            "core:toggle-hidden-files",
            "core:check-for-updates",
            "doc:upper-case",
            "doc:lower-case",
            "doc:reload",
            "git:pull",
            "git:push",
            "git:commit",
            "git:stash",
            "git:blame",
            "git:log",
            "root:close-all",
            "root:close-all-others",
            "root:close-or-quit",
            "doc:save-as",
            "core:toggle-markdown-preview",
            "notes:delete-current",
            "test:run-all",
            "test:run-in-current-file",
        ];
        for cmd in palette_extras {
            if seen.insert((*cmd).to_string()) {
                let display = crate::editor::keymap::prettify_name(cmd);
                cmds.push(((*cmd).to_string(), display));
            }
        }
        cmds.sort_by(|a, b| a.1.cmp(&b.1));
        // Filter commands for disabled subsystems.
        if !subsystems.has_git() {
            cmds.retain(|c| !c.0.starts_with("git:") && c.0 != "core:git-status");
        }
        if !subsystems.has_lsp() {
            cmds.retain(|c| !c.0.starts_with("lsp:"));
        }
        if !subsystems.has_terminal() {
            cmds.retain(|c| !c.0.contains("terminal"));
        }
        if !subsystems.has_sidebar() {
            cmds.retain(|c| !c.0.contains("sidebar") && c.0 != "core:toggle-hidden-files");
        }
        if !subsystems.has_find_in_files() {
            cmds.retain(|c| c.0 != "core:project-search" && c.0 != "core:project-replace");
        }
        if !subsystems.has_update_check() {
            cmds.retain(|c| c.0 != "core:check-for-updates");
        }
        if !subsystems.has_picker() {
            // Nano-Anvil (single_file_mode) still supports core:open-recent
            // as a files-only list, so only strip open-project-folder.
            let keep_recent = single_file_mode;
            cmds.retain(|c| {
                c.0 != "core:open-project-folder" && (keep_recent || c.0 != "core:open-recent")
            });
        }
        if !subsystems.has_bookmarks() {
            cmds.retain(|c| !c.0.contains("bookmark"));
        }
        if !subsystems.has_folding() {
            cmds.retain(|c| c.0 != "doc:fold" && c.0 != "doc:unfold" && c.0 != "doc:unfold-all");
        }
        if single_file_mode {
            cmds.retain(|c| {
                !c.0.contains("tab") && c.0 != "root:close-all" && c.0 != "root:close-all-others"
            });
        }
        // Notes-mode: drop project / folder / multi-tab / preview-toggle
        // commands. Keep only what NoteSquirrel users would expect.
        if subsystems.has_notes_mode() {
            cmds.retain(|c| {
                let n = c.0.as_str();
                !n.contains("tab")
                    && !n.contains("project")
                    && !n.contains("folder")
                    && n != "core:toggle-markdown-preview"
                    && n != "core:toggle-hidden-files"
                    && n != "doc:save"
                    && n != "doc:save-as"
                    && n != "doc:reload"
                    && n != "core:open-file"
                    && n != "core:find-file"
                    && n != "root:close-all"
                    && n != "root:close-all-others"
            });
        } else {
            // Outside notes-mode the delete-current command is a no-op
            // and would only confuse the palette.
            cmds.retain(|c| c.0 != "notes:delete-current");
        }
        cmds
    };

    // Command view state. Helpers and the `CmdViewMode` enum live in
    // `crate::editor::cmdview`.
    #[cfg(feature = "sdl")]
    use crate::editor::cmdview::truncate_left_to_width;
    use crate::editor::cmdview::{
        CmdViewMode, dir_with_trailing_sep, effective_root, path_suggest,
        refresh_cmdview_suggestions, remember_recent_file, update_recent,
    };
    let mut cmdview_active = false;
    let mut cmdview_mode = CmdViewMode::OpenFile;
    let mut cmdview_text = String::new();
    // Byte position of the input caret within cmdview_text. Always lands on a UTF-8 boundary.
    let mut cmdview_cursor: usize = 0;
    let mut cmdview_suggestions: Vec<String> = Vec::new();
    let mut cmdview_selected: usize = 0;
    let mut cmdview_label = String::new();
    // Pending LSP rename target: (uri, 0-based line, 0-based character). Set
    // when the rename input opens; consumed when the new name is submitted.
    let mut lsp_rename_pos: Option<(String, usize, usize)> = None;
    // Code-action picker state: overlay of (title, action-json) awaiting choice.
    let mut code_action_active = false;
    let mut code_actions: Vec<(String, serde_json::Value)> = Vec::new();
    let mut code_action_selected: usize = 0;

    // Per-field undo/redo history for the dialog text inputs. Each input keeps
    // its own stack so the undo/redo shortcuts edit the focused field rather
    // than the document buffer (VS Code input-box behaviour).
    use crate::editor::field_history::{FieldEdit, FieldHistory};
    let mut find_history = FieldHistory::default();
    let mut replace_history = FieldHistory::default();
    let mut cmdview_history = FieldHistory::default();
    let mut project_search_history = FieldHistory::default();
    let mut project_replace_search_history = FieldHistory::default();
    let mut project_replace_with_history = FieldHistory::default();
    let mut palette_history = FieldHistory::default();
    let mut notes_search_history = FieldHistory::default();
    let mut sidebar_new_file_history = FieldHistory::default();

    // Project-wide search state.
    // Git status view.
    let mut git_status_active = false;
    let mut git_status_entries: Vec<(String, String, String)> = Vec::new();
    let mut git_status_selected: usize = 0;
    // Background git-status refresh job, polled each frame.
    let mut git_status_job: Option<std::thread::JoinHandle<Vec<(String, String, String)>>> = None;
    let mut git_blame_job: Option<std::thread::JoinHandle<Vec<String>>> = None;
    let mut git_log_job: Option<std::thread::JoinHandle<Vec<(String, String, String)>>> = None;
    let mut update_check_job: Option<std::thread::JoinHandle<String>> = None;
    // Paths of recently closed tabs (most-recent last) for reopen-closed-tab.
    let mut closed_tabs: Vec<String> = Vec::new();

    // Git blame: per-line annotations shown inline at the right edge.
    let mut git_blame_active = false;
    let mut git_blame_lines: Vec<String> = Vec::new();

    // Git history (log) for the current file.
    let mut git_log_active = false;
    let mut git_log_entries: Vec<(String, String, String)> = Vec::new(); // (hash, date, message)
    let mut git_log_selected: usize = 0;

    let mut project_search_active = false;
    let mut project_search_query = String::new();
    let mut project_search_results: Vec<(String, usize, String)> = Vec::new();
    let mut project_search_selected: usize = 0;
    // Shared toggles for both project search and project replace.
    let mut project_use_regex = false;
    let mut project_whole_word = false;
    let mut project_case_insensitive = true;

    // Project-wide replace state.
    let mut project_replace_active = false;
    let mut project_replace_search = String::new();
    let mut project_replace_with = String::new();
    let mut project_replace_focus_on_replace = false;
    let mut project_replace_results: Vec<(String, usize, String)> = Vec::new();
    // Background project-wide replace-all (sed) job, polled each frame.
    let mut replace_job: Option<std::thread::JoinHandle<usize>> = None;
    let mut project_replace_selected: usize = 0;

    // Context menu state.
    let mut context_menu = ContextMenu::new();
    // (doc_path, test_name) to run; set by the badge-click hit-test and
    // consumed by the `test:run-single` command dispatch.
    let mut pending_single_test: Option<(String, String)> = None;
    // Per-frame discovered tests for the active doc; rebuilt each frame.
    let mut active_tests: Vec<crate::editor::test_runner::DiscoveredTest> = Vec::new();
    // Per-frame badge rects for the active doc.
    let mut test_badges: Vec<crate::editor::test_runner::TestBadgeRegion> = Vec::new();
    // (path, change_id) the badges were last scanned for, so the discovery scan
    // (which clones the document and probes the filesystem) runs only when the
    // file or its content changes, not on every redraw.
    let mut test_scan_cache: (String, i64) = (String::new(), -1);
    // Sidebar entry targeted by the current context menu (path, is_dir).
    // Set when right-clicking a sidebar row; consumed by the rename flow.
    let mut sidebar_menu_target: Option<(String, bool)> = None;
    // Path being renamed (source). Read by the CmdViewMode::Rename
    // confirm handler to `fs::rename` the file.
    let mut rename_source: String = String::new();
    // Folder path for the in-progress inline new-file creation (`None` = inactive).
    let mut sidebar_new_file_dir: Option<String> = None;
    // Filename currently being typed into the inline new-file input.
    let mut sidebar_new_file_name: String = String::new();
    // Byte-offset cursor position within `sidebar_new_file_name`.
    let mut sidebar_new_file_cursor: usize = 0;

    // LSP completion, hover, and go-to-definition state.
    let mut completion = CompletionState::new();
    // Document-word autocomplete index (used when LSP is not active).
    let mut word_index = WordIndex::new();
    // Momentum-scroll velocity for wheel-driven smooth scrolling.
    let mut editor_scroll_vel: f64 = 0.0;
    let mut sidebar_scroll_vel: f64 = 0.0;
    let mut preview_scroll_vel: f64 = 0.0;

    let mut hover = HoverState::new();
    // Signature-help popup reuses the hover popup shape (text + visibility).
    let mut signature_help = HoverState::new();
    // Mouse-tracked hover state: `mouse_doc_pos` is the (1-based line,
    // 1-based col) under the cursor when over the active doc, or None
    // otherwise. `mouse_idle_since` records when the cursor settled at
    // that position so we can debounce the `textDocument/hover` LSP
    // request — diagnostic tooltips fire immediately, type-info tooltips
    // wait for the cursor to stop moving for ~600ms. `last_lsp_hover_pos`
    // dedupes repeat requests for the same position.
    let mut mouse_doc_pos: Option<(usize, usize)> = None;
    let mut mouse_idle_since: Option<Instant> = None;
    let mut last_lsp_hover_pos: Option<(usize, usize)> = None;

    // Terminal emulator panel (multi-terminal).
    let mut terminal = TerminalPanel::new();

    // Minimap state.
    let mut minimap_visible = false;
    // Line wrap is on by default, and the preference is persisted across
    // sessions so a user who explicitly disables it still sees no wrap the
    // next time they launch.
    let mut line_wrapping =
        match crate::editor::storage::load_text(userdir_path, "session", "line_wrapping") {
            Ok(Some(v)) => v.trim() != "false",
            _ => true,
        };
    let mut overwrite_mode = false;
    let mut cursor_blink_reset = Instant::now();
    let blink_period = 0.5;

    // Autoreload: watch open files for external changes.
    let mut autoreload = AutoreloadState::new();

    // Notes-mode: restore the per-notes-folder session (the previously
    // open note) when no doc was opened from the CLI. Mirrors NoteSquirrel's
    // "remember last open note" behavior so launching drops
    // the user back into whatever they were editing last.
    if subsystems.has_notes_mode() && docs.is_empty() && !project_root.is_empty() {
        if let Some(tab) = crate::editor::open_doc::restore_project_session(
            userdir_path,
            &project_root,
            &mut docs,
            &mut autoreload,
            use_git(),
        ) {
            active_tab = tab;
        }
    }

    for doc in &docs {
        autoreload.watch(&doc.path);
    }

    // Syntax highlighting: load lightweight index for file matching, defer
    // full definition parsing to first use per extension.
    let syntax_index = crate::editor::syntax::load_syntax_index(datadir);
    let mut compiled_syntax_cache: HashMap<String, Option<CompiledSyntax>> = HashMap::new();
    // MRU ordering for `compiled_syntax_cache`: `compiled_syntax_mru[0]`
    // is the most recently used extension. Lets us cap the cache at
    // `SYNTAX_CACHE_CAP` entries and evict the oldest instead of
    // growing unbounded on sessions that touch many file types.
    let mut compiled_syntax_mru: Vec<String> = Vec::new();
    const SYNTAX_CACHE_CAP: usize = 8;

    // LSP state.
    let mut lsp_state = LspState::new();
    let lsp_specs = if subsystems.has_lsp() {
        lsp::builtin_specs()
    } else {
        Vec::new()
    };

    /// Try to start LSP for a file path if not already running for this filetype.
    fn try_start_lsp(
        file_path: &str,
        lsp_state: &mut LspState,
        lsp_specs: &[crate::editor::lsp::LspSpec],
        userdir: &str,
        verbose: bool,
    ) {
        if lsp_state.transport_id.is_some() {
            return;
        }
        let ext = file_path.rsplit('.').next().unwrap_or("");
        let Some(filetype) = ext_to_lsp_filetype(ext) else {
            return;
        };
        let Some(spec) = find_lsp_spec(filetype, lsp_specs) else {
            return;
        };
        let root = find_project_root(
            Path::new(file_path)
                .parent()
                .map(|p| p.to_str().unwrap_or("."))
                .unwrap_or("."),
            &spec.root_patterns,
        );
        let Some(root_dir) = root else { return };
        let cmd: Vec<String> = spec
            .command
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        if cmd.is_empty() {
            return;
        }
        match lsp::spawn_transport(&cmd, &root_dir, &[]) {
            Ok(tid) => {
                lsp_state.transport_id = Some(tid);
                lsp_state.root_uri = path_to_uri(&root_dir);
                lsp_state.filetype = filetype.to_string();
                let req_id = lsp_state.next_id();
                lsp_state
                    .pending_requests
                    .insert(req_id, "initialize".to_string());
                let _ =
                    lsp::send_message(tid, &lsp_initialize_request(req_id, &lsp_state.root_uri));
            }
            Err(e) => {
                lsp_state.note_spawn_failure();
                log_to_file(userdir, &format!("Failed to spawn LSP: {e}"));
                if verbose {
                    eprintln!("Failed to spawn LSP: {e}");
                }
            }
        }
    }

    // Try to start LSP for the first open file.
    if subsystems.has_lsp() {
        if let Some(doc) = docs.first() {
            try_start_lsp(
                &doc.path,
                &mut lsp_state,
                &lsp_specs,
                userdir,
                config.verbose,
            );
        }
    }

    // Clear any stale shutdown signal from prior runs.
    if crate::signal::shutdown_requested() {
        crate::signal::clear_shutdown();
    }

    // Unified command dispatch. The match body lives in
    // `commands_dispatch.rs` and is pulled in textually via `include!()`
    // so its ~830 lines of arms run in this scope and can read/write
    // every local variable directly. (A `macro_rules!` wrapper would
    // break: its `let cmd: String = $cmd_arg` binding is hygienic, so
    // the included `match cmd.as_str()` can't see it.) The three
    // invocations below each declare a local `cmd: String` before the
    // include so the dispatch body has it in scope.

    loop {
        if crate::signal::shutdown_requested() {
            crate::signal::clear_shutdown();
            if docs.iter().any(doc_is_modified) {
                nag = Nag::UnsavedChanges {
                    message: nag_msg_quit(&docs),
                    tab_to_close: None,
                };
                redraw = true;
            } else {
                quit = true;
            }
        }

        // Idle-drop: after N seconds with no events, release cached
        // glyph bitmaps and command buffers. Next interactive frame
        // rebuilds them lazily. This is most of the benefit of the
        // macOS memory-pressure hook without needing platform FFI.
        if !dropped_caches_for_idle && last_activity.elapsed().as_secs() >= IDLE_DROP_SECS {
            crate::renderer::drop_caches();
            dropped_caches_for_idle = true;
        }

        // macOS memory-pressure probe. `None` on other platforms.
        if last_mem_pressure_check.elapsed().as_secs() >= MEM_PRESSURE_CHECK_SECS {
            last_mem_pressure_check = Instant::now();
            if let Some(level) = crate::renderer::macos_memory_pressure_level() {
                if level >= 1 {
                    // WARN or CRITICAL -- release everything reclaimable.
                    crate::renderer::drop_caches();
                    compiled_syntax_cache.retain(|k, _| {
                        docs.iter()
                            .any(|d| d.path.rsplit('.').next().unwrap_or("") == k)
                    });
                    compiled_syntax_mru.retain(|k| compiled_syntax_cache.contains_key(k));
                }
            }
        }

        // Poll all pending events.
        let mut had_input_events = false;

        include!("event_polling.rs");



        include!("post_event.rs");


        include!("render_pass.rs");

        if quit {
            break;
        }

        // Block until the next event. While there is recent on-screen motion, a
        // terminal panel open (its PTY is polled here), or a background job
        // running, poll at the frame interval so output and animation stay
        // smooth. Otherwise sleep longer: input and worker-pushed wake-up
        // events return from the blocked wait immediately, so only idle CPU is
        // affected, never responsiveness.
        let busy = last_draw.elapsed().as_secs_f64() < 0.3
            || terminal.visible
            || load_job.is_some()
            || replace_job.is_some()
            || git_status_job.is_some()
            || git_blame_job.is_some()
            || git_log_job.is_some()
            || update_check_job.is_some();
        let timeout = if busy { frame_interval } else { idle_wait };
        crate::window::wait_event(Some(timeout));
    }

    // Persist recent files: add all currently open docs to recent_files.
    for doc in &docs {
        if !doc.path.is_empty() {
            update_recent(&mut recent_files, &doc.path, 100);
        }
    }
    let _ = crate::editor::storage::save_text(
        userdir_path,
        "session",
        "recent_files",
        &serde_json::to_string(&recent_files).unwrap_or_default(),
    );
    let _ = crate::editor::storage::save_text(
        userdir_path,
        "session",
        "recent_projects",
        &serde_json::to_string(&recent_projects).unwrap_or_default(),
    );

    // Persist expanded sidebar folders for this project.
    if subsystems.has_sidebar() {
        save_expanded_folders(
            &sidebar_entries,
            userdir_path,
            &project_session_key(&project_root),
        );
    }

    // Notes-mode: persist the per-folder session so the next launch
    // reopens the same note. The global "session/files" path below is
    // not used because notes-mode never keeps multiple
    // tabs and a per-folder key keeps switching `NOTE_ANVIL_DIR` clean.
    if subsystems.has_notes_mode() && !project_root.is_empty() {
        save_project_session(userdir_path, &project_root, &docs, active_tab);
    }

    // Session save: persist open files, active tab, and project root via storage.
    // Save session state (JereIDE only -- Nano-Anvil has no session).
    if !single_file_mode {
        let mut open_files = Vec::new();
        let mut unsaved_content = Vec::new();
        for doc in &docs {
            if doc.path.is_empty() {
                open_files.push("__untitled__".to_string());
                let content = doc
                    .view
                    .buffer_id
                    .and_then(|id| buffer::with_buffer(id, |b| Ok(b.lines.join(""))).ok())
                    .unwrap_or_default();
                unsaved_content.push(content);
            } else {
                open_files.push(doc.path.clone());
                unsaved_content.push(String::new());
            }
        }
        let project_root_meaningful = !project_root.is_empty()
            && project_root != "."
            && std::path::Path::new(&project_root).is_dir();
        if !open_files.is_empty() || project_root_meaningful {
            let session = crate::editor::open_doc::SessionData {
                files: open_files,
                active: active_tab,
                active_project: project_root.clone(),
                unsaved_content,
            };
            if let Ok(json) = serde_json::to_string_pretty(&session) {
                if let Err(e) = storage::save_text(userdir_path, "session", "files", &json) {
                    eprintln!("Failed to save session: {e}");
                }
            }
        } else if let Err(e) = storage::clear(userdir_path, "session", Some("files")) {
            eprintln!("Failed to clear session: {e}");
        }
    }

    // Save window size and position.
    let (pw, ph, wx, wy) = crate::window::get_window_size();
    let win_json = serde_json::json!({ "w": pw, "h": ph, "x": wx, "y": wy });
    if let Err(e) = storage::save_text(userdir_path, "session", "window", &win_json.to_string()) {
        eprintln!("Failed to save window size: {e}");
    }

    // Shut down all terminals.
    for inst in &mut terminal.terminals {
        inst.inner.cleanup();
    }

    // Shut down every LSP transport (kill + reap, closing writer threads).
    lsp::clear_all_transports();

    false
}

#[cfg(not(feature = "sdl"))]
pub fn run(_config: NativeConfig, _args: &[String], _datadir: &str, _userdir: &str) -> bool {
    false
}

use crate::editor::main_helpers::*;
