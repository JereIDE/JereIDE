# JereIDE Codebase Documentation

## Overview

JereIDE is a cross-platform code editor built with Rust and [egui](https://github.com/emilk/egui)/[eframe](https://github.com/emilk/egui/tree/master/crates/eframe) (v0.34). It provides syntax highlighting, multi-tab editing, a native menu bar, and file management with dialogs. The project is organized as a Cargo workspace of 11 crates.

**Version:** 0.21.0  
**License:** MIT  
**Target platforms:** macOS 12+, Windows 10+

---

## Workspace Structure

```
jereide/
├── .github/
│   ├── ISSUE_TEMPLATE/
│   │   ├── bug-crash-report.md
│   │   └── feature_request.md
│   ├── images/
│   │   ├── demo.md
│   │   └── macos.jpg
│   ├── workflows/
│   │   ├── release.yml          # Builds & publishes release binaries
│   │   └── test.yml             # Runs tests on CI
│   └── PULL_REQUEST_TEMPLATE.md
├── crates/
│   ├── app/          # Binary entry point
│   ├── core/         # State types & constants
│   ├── settings/     # Colors, font sizes, window dimensions
│   ├── text/         # Text manipulation utilities
│   ├── syntax/       # Syntax highlighting (syntect)
│   ├── menu/         # Native menu bar (muda)
│   ├── main-window/  # Application orchestrator (eframe::App)
│   ├── ui/           # Title bar, tab strip, status bar, dialogs, welcome view
│   ├── code/         # Code editor view & edit actions (undo/redo, clipboard)
│   ├── compose/      # Compose palette view (placeholder)
│   └── fs/           # File system operations & dialogs (rfd)
├── Cargo.toml        # Workspace root
├── AGENTS.md         # Agent-specific instructions
├── README.md
├── RELEASENOTES_FORMAT.md
└── LICENSE
```

---

## Crate Dependency Graph

```
app
 └── settings, main-window

main-window
 ├── core, settings, fs
 ├── menu, ui, code, compose
 └── (macOS-only) objc2, objc2-foundation

code
 ├── core, settings, text, syntax

compose
 └── settings

ui
 ├── core, settings

syntax
 ├── settings
 └── syntect

menu
 ├── crossbeam-channel, muda, raw-window-handle

fs
 └── rfd

core
 └── eframe (for egui types)

text        # no external deps
settings    # no external deps (except eframe for Color32)
```

---

## Crate-by-Crate Breakdown

### `crates/app` — Binary Entry Point

**Files:** `src/main.rs`

The binary crate that starts the application. Sets up an `eframe::NativeOptions` with:

- No native title bar (`with_titlebar_shown(false)`)
- Fullsize content view
- Dimensions from settings (`WINDOW_WIDTH` × `WINDOW_HEIGHT`)

Calls `eframe::run_native` with `JereIDEApp` (from `main-window` crate).

---

### `crates/core` — State & Constants

**Files:** `src/lib.rs`, `src/state.rs`, `src/constants.rs`

Re-exports all state types and layout constants.

#### `AppState`

Central application state owned by `JereIDEApp`. Fields:

- `tabs: Vec<Tab>` — open documents
- `active_tab_index: usize`
- `editor_focused: bool` — used to request initial focus
- `editor_id: egui::Id` — stored for edit action dispatch
- `current_view: CurrentView` — either `Code` or `Compose`
- `was_fullscreen: bool` — macOS traffic light adjustment
- `document_edited: bool` — macOS dirty-dot in window title
- `pending_close_index: Option<usize>` — unsaved-changes modal state
- `pending_large_file_blocked: Option<u64>` — file > 200 MB blocked
- `pending_large_file_warn: Option<(String, u64)>` — file > 100 MB warning

Key methods:

- `open_file(path, content)` — reuses existing tab if same path, else creates new
- `new_tab()` — adds empty untitled tab
- `close_tab(index)` — removes tab, adjusts `active_tab_index`
- `switch_to_view(target)` — toggles code/compose view

#### `Tab`

A single open document:

- `id: usize` — globally unique (atomic counter)
- `text: String` — current buffer content
- `saved_text: String` — last saved state (for modified detection)
- `file_path: Option<String>`
- `cursor_line / cursor_col: usize`
- `is_modified()`, `mark_saved()`, `file_name()`

#### `CurrentView`

```rust
pub enum CurrentView { Code, Compose }
```

#### Layout Constants (`constants.rs`)

All pixel-dimension constants:

- `TITLE_BAR_HEIGHT`, `TAB_STRIP_HEIGHT`
- `TITLE_BAR_TRAFFIC_SPACE` (75px for macOS traffic lights), `TITLE_BAR_FULLSCREEN_SPACE`
- `EDITOR_INNER_MARGIN_*` — margins inside code view
- `GUTTER_DIGIT_WIDTH`, `GUTTER_PADDING_*`, `GUTTER_LINE_NUMBER_RIGHT_OFFSET`
- `SCROLL_BAR_WIDTH`
- `MAX_FILE_SIZE` (200 MB), `WARN_FILE_SIZE` (100 MB)
- `TAB_PAD_*`, `TAB_CLOSE_BTN_*`, `TAB_MODIFIED_DOT_RADIUS`, `TAB_BORDER_WIDTH`
- `TRAFFIC_LIGHT_OFFSET_X/Y` — 2.0, -3.0

---

### `crates/settings` — Colors & Dimensions

**Files:** `src/lib.rs`

Theme constants (hardcoded — TODO: make configurable at runtime):

- Backgrounds: `SURFACE_BG` (white), `ELEVATED_BG` (#f5f5f5), `HOVER_BG` (#e6e6e6), `COMPOSE_BG` (gray 20)
- Text colors: `TEXT_DEFAULT` (black), `TEXT_PRIMARY`, `TEXT_SECONDARY`, `TEXT_MUTED`, `TEXT_CURRENT_LINE`, `COMPOSE_TEXT`
- `ACCENT` — teal (#1ce1d2)
- `BORDER` (#c8c8c8)
- Font sizes: `TITLE_BAR_FONT_SIZE` (12), `TAB_FONT_SIZE` (12), `EDITOR_FONT_SIZE` (14), `COMPOSE_VIEW_FONT_SIZE` (18)
- Window: `WINDOW_WIDTH` (800), `WINDOW_HEIGHT` (600)
- `DIALOG_WIDTH` (240)

---

### `crates/text` — Text Utilities

**Files:** `src/lib.rs`

Pure functions for character-index-based text manipulation:

- `char_index_to_line_col(text, char_index)` — converts byte offset to (line, col)
- `char_range_substring(text, start, end)` — extracts by char index
- `delete_char_range(text, start, end)` — removes range by char index
- `insert_at_char_index(text, char_index, insert)` — inserts at char index

Used by the edit actions (copy/cut) and status bar cursor display.

---

### `crates/syntax` — Syntax Highlighting

**Files:** `src/lib.rs`

Wraps [syntect](https://github.com/trishume/syntect) v5 with:

- `SyntaxSet::load_defaults_newlines()` — bundled syntax definitions
- `ThemeSet::load_defaults()` — defaults, uses "InspiredGitHub" (falls back to "base16-ocean.light")

#### `SyntaxHighlighter`

Struct that holds font, syntax reference, theme, and a line cache for incremental highlighting.

- `new(font_size, extension)` — selects syntax by file extension, falls back to plain text
- `highlight(text) -> LayoutJob` — incremental re-highlight:
  - Compares cached text; returns cached job if unchanged
  - Finds first differing line via `position()` on cached vs new lines
  - Re-highlights from that point forward
  - Short-circuits when highlight state matches old remainder (lines beyond edit are identical)
- `build_job(text)` — converts cached `Vec<CachedLine>` into an egui `LayoutJob` with per-token color sections

#### `CachedLine`

Per-line cache: content string, `Vec<(start, end, Color32)>` sections, `HighlightState`, `ParseState`.

---

### `crates/menu` — Native Menu Bar

**Files:** `src/lib.rs`

Uses [muda](https://crates.io/crates/muda) v0.19 for platform-native menus.

#### `AppMenu`

Menu structure:

- **JereIDE** (app menu): About, Star on GitHub, Services, Hide/Show, Quit
- **File**: New (Cmd+N), Open... (Cmd+O), Save (Cmd+S), Save As… (Cmd+Shift+S)
- **Edit**: Undo (Cmd+Z), Redo (Cmd+Shift+Z), Cut (Cmd+X), Copy (Cmd+C), Paste (Cmd+V), Select All (Cmd+A)
- **View**: Fullscreen

Key methods:

- `init(raw_handle)` — platform-specific: `init_for_nsapp()` on macOS, `init_for_hwnd()` on Windows
- `poll_events() -> Vec<MenuId>` — drains the crossbeam channel of `MenuEvent`s
- `is_initialized() / set_initialized()`

Menu events are processed in `JereIDEApp::ui()` by matching on event ID strings.

---

### `crates/fs` — File Operations

**Files:** `src/lib.rs`

Uses [rfd](https://crates.io/crates/rfd) v0.15 (Rust File Dialogs).

#### `FileManager`

- `pick_file()` — opens native "Open File" dialog
- `read_file_at(path)` — `fs::read_to_string`
- `file_size(path)` — `fs::metadata().len()`
- `save_as_dialog()` — opens "Save File" dialog
- `save_to_path(content, path)` — `fs::write`
- `current_path: Option<PathBuf>` — last used file path

---

### `crates/main-window` — Application Orchestrator

**Files:** `src/lib.rs`

The core application struct `JereIDEApp` implementing `eframe::App`.

#### macOS Native Helpers

- `set_document_edited(frame, edited)` — sets the macOS window document-edited dot via Objective-C `msg_send!`
- `position_traffic_lights(frame, offset_x, offset_y)` — repositions close/minimize/zoom buttons using `objc2-foundation` NSRect manipulation. Caches default positions in `OnceLock`.

#### `JereIDEApp` fields

```rust
state: AppState,
app_menu: AppMenu,
file_manager: FileManager,
visuals_initialized: bool,
traffic_lights_positioned: bool,
prev_fullscreen: bool,
```

#### Event Handling in `ui()`

1. **Visuals init (once):** Sets accent selection color on first frame
2. **macOS document-edited dot:** Syncs with `state.document_edited`
3. **macOS traffic lights:** Repositioned on first show and fullscreen toggle
4. **Menu initialization:** One-time `init_for_nsapp`/`init_for_hwnd`
5. **Non-macOS keyboard shortcuts:** Cmd+N/O/S/Shift+S/Q
6. **Menu event polling:** Dispatches `new`, `open`, `save`, `save_as`, `quit`, `fullscreen`, `githubstar`, and edit actions (`EditAction::from_menu_id`)
7. **UI rendering:** Status bar → CentralPanel → title bar → tab strip → code view or welcome view
8. **Compose overlay:** If `current_view == Compose`, renders compose palette overlay
9. **Modal dialogs:** Unsaved changes confirm, large file blocked/warning

#### File action handlers

- `handle_new()` — adds empty tab
- `handle_open()` — picks file, checks size limits, opens in tab
- `handle_save()` — saves to existing path or delegates to `handle_save_as()`
- `handle_save_as()` — opens save dialog, updates tab file path
- `save_tab(idx)` — saves a specific tab (used by close-confirm dialog)

---

### `crates/ui` — UI Components

**Files:** `src/lib.rs`, `src/title_bar.rs`, `src/tab_strip.rs`, `src/status_bar.rs`, `src/welcome.rs`, `src/dialog.rs`

#### Title Bar (`title_bar.rs`)

Custom title bar with:

- macOS traffic light spacing (75px normal, 7px fullscreen)
- "Choose Project" button with a placeholder popup
- "Code" / "Compose" view toggle buttons (`selectable_label`)
- Layout: left-to-right (macOS spacing → buttons → right-aligned reserved space)

#### Tab Strip (`tab_strip.rs`)

Custom-drawn tab strip (no egui tabs widget):

- Tab layout: modified dot (accent circle) → centered file name → close button (×)
- Close button appears on hover as a circular X
- Active tab connected to content area (bottom border removed for active tab)
- Vertical dividers between tabs
- Double-click strip creates new tab
- Click to activate, close button triggers unsaved-changes modal if modified

#### Status Bar (`status_bar.rs`)

Bottom bar showing:

- Left: app version, language, file name
- Right: cursor position (Line:Col) or "--:--"
- Language detection via file extension (Rust, Python, JS, TS, etc.)
- In Compose view mode, status bar shows as compose background and returns early

#### Welcome View (`welcome.rs`)

Shown when no tabs are open. Displays:

- `[LOGO]` placeholder
- "Welcome back to JereIDE"
- "The editor for what's next"

#### Dialogs (`dialog.rs`)

Three modal dialogs using egui `Window` with a dimmer overlay:

1. **Close Confirm** — "Unsaved Changes" with Save / Don't Save / Cancel
2. **Large File Blocked** — files > 200 MB cannot be opened
3. **Large File Warning** — files > 100 MB show "Open Anyway" / Cancel

Each dialog creates a full-viewport dimmer layer (`Color32::from_black_alpha(120)`) with a click-catcher.

---

### `crates/code` — Code Editor

**Files:** `src/lib.rs`, `src/code_view.rs`, `src/edit.rs`

#### Code View (`code_view.rs`)

Renders the main code editor area:

- Thread-local `HIGHLIGHTERS` cache: `HashMap<tab_id, SyntaxHighlighter>`
  - Cleans defunct tab IDs on each render
- `visual_line_count(text)` — counts `\n` + 1
- `gutter_width(line_count)` — dynamic width based on digit count
- Uses `egui::TextEdit::code_editor` with a custom `layouter` closure that calls `SyntaxHighlighter::highlight()`
- Left gutter: line numbers drawn by position (`galley.rows` iteration)
- Current line number rendered in `TEXT_CURRENT_LINE`, others in `TEXT_MUTED`
- Extra clickable surface area below text to request focus
- Reads cursor position from `TextEdit::load_state` for status bar

#### Edit Actions (`edit.rs`)

`EditAction` enum dispatched from the menu system:

```rust
pub enum EditAction { SelectAll, Copy, Cut, Paste, Undo, Redo }
```

- `from_menu_id(id)` — maps menu event IDs to actions
- `handle_edit_action(state, ctx, action)`
- **SelectAll:** Sets `CCursorRange` from 0 to text length
- **Copy:** Reads selection range, calls `ctx.copy_text()`
- **Cut:** Copies selection then deletes range from text
- **Paste:** Sends `ViewportCommand::RequestPaste`
- **Undo/Redo:** Uses egui's built-in `TextEdit::undoer()` with char-range snapshots

---

### `crates/compose` — Compose Palette

**Files:** `src/lib.rs`, `src/compose_view.rs`

Currently a placeholder. Renders a full-viewport overlay with `COMPOSE_BG` and "Needs implementation" text.

---

## Data Flow

### Opening a file

```
Menu "Open" → handle_open()
  → FileManager::pick_file() → native dialog
  → FileManager::file_size() → check MAX_FILE_SIZE / WARN_FILE_SIZE
  → FileManager::read_file_at() → read_to_string
  → state.open_file(path, content)
     → checks if already open (reuses tab)
     → else creates new Tab with path+content
```

### Saving a file

```
Menu "Save" → handle_save()
  → if path exists: FileManager::save_to_path() + mark_saved()
  → else: handle_save_as()
     → FileManager::save_as_dialog()
     → FileManager::save_to_path()
     → updates tab.file_path + mark_saved()
```

### Typing in the editor

```
User types → egui TextEdit handles input
  → custom layouter calls SyntaxHighlighter::highlight()
     → incremental: finds first changed line
     → re-highlights from there
     → returns LayoutJob with colored sections
  → cursor position read from TextEdit::load_state
  → status bar updated with line:col
```

### Closing a modified tab

```
Close button clicked → pending_close_index = Some(idx)
  → render_close_confirm_modal()
  → user selects: Save (save_tab then close), Discard (close), Cancel
```

---

## CI/CD

### Test Workflow (`.github/workflows/test.yml`)

Runs on push to main and PRs:

- **macOS:** installs sdl3, freetype, pcre2 via brew, `cargo test --all-features`
- **Windows:** installs vcpkg packages, `cargo test --all-features`
- Linux tests are currently commented out

### Release Workflow (`.github/workflows/release.yml`)

Triggered by:

- Push to main with commit message matching `[RELEASE v{version}#{title}]`
- Manual `workflow_dispatch` with version and title inputs

Stages:

1. **parse-commit:** Extracts version, creates git tag, creates draft GitHub Release
2. **build-windows:** `cargo build --release -p JereIDE`, zips binary + LICENSE + README
3. **build-macos:** Matrix of aarch64 + x86_64, links with `libclang_rt.osx.a`, zips
4. **release:** Downloads all artifacts, uploads to draft release

Linux builds are commented out.

---

## Key Design Decisions

1. **Native title bar disabled** — JereIDE renders its own title bar for a custom look, with macOS traffic lights repositioned via Objective-C runtime calls.

2. **Custom tab strip** — Tabs are drawn manually with `egui::Painter` rather than using egui's widget library, giving full control over appearance and interaction.

3. **Incremental syntax highlighting** — The `SyntaxHighlighter` caches per-line highlight state and re-highlights only from the first changed line, reusing identical trailing lines when the parser state matches.

4. **Menu-driven edit actions** — Cut/Copy/Paste/Undo/Redo are routed through the native menu system and dispatched to egui's `TextEdit` state, ensuring menu and keyboard shortcut parity.

5. **Size-gated file opening** — Files > 200 MB are blocked outright; files > 100 MB warn the user before opening.

6. **macOS integration** — Uses `objc2` directly for window manipulation (document-edited dot, traffic light positioning) rather than egui abstractions.

7. **Thread-local highlighter cache** — `SyntaxHighlighter` instances are stored per tab in a `thread_local!` `RefCell<HashMap>` to avoid lifetime complexity and allow mutable access from the layouter closure.

---

## Tests

Tests are located in `crates/code/src/code_view.rs` and cover:

- `visual_line_count` — empty, single, multi-line, trailing newline
- `gutter_width` — single, double, triple digit, power-of-ten boundaries

Run with: `cargo test`

---

## TODO / Stubbed Areas

- **Compose palette** (`crates/compose/compose_view.rs`) — "Needs implementation"
- **"Choose Project"** title bar button — "Needs Implementation"
- **Settings persistence** — colors/dimensions are hardcoded in `settings/src/lib.rs` with TODO to load from a JSON file
- **Error handling** — `FileManager::save_to_path` has a TODO for proper error handling
- **Help menu** — commented out in menu construction
- **Linux** — build and test CI jobs are commented out (not a planned target)
