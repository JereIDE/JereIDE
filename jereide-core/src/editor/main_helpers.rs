//! Standalone helper functions extracted from main_loop.rs for
//! parallel compilation.

use crate::editor::buffer;
use crate::editor::config::NativeConfig;
use crate::editor::doc_view::{
    DocView, SYNTAX_COLORS,
};
use crate::editor::keymap::NativeKeymap;
use crate::editor::picker;
use crate::editor::style_ctx::StyleContext;
use crate::editor::view::View;

macro_rules! sdl_only {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "sdl")]
            $item
        )*
    };
}

sdl_only! {
/// Filter command list using fuzzy matching from the picker module.
pub(crate) fn fuzzy_filter_commands(query: &str, all_commands: &[(String, String)]) -> Vec<(String, String)> {
    if query.is_empty() {
        return all_commands.to_vec();
    }
    // Rank on the pretty name only (the part before the "  (ctrl+...)"
    // keybinding tail). `fuzzy_match`'s score includes a -length
    // penalty, so if we rank on the full display string an entry with
    // a keybinding ("Open File  (ctrl+o)") gets pushed below one
    // without a binding ("Open User Settings") on the query "open" —
    // which is exactly backwards for users who are typing the name of
    // a command they already know has a shortcut.
    let strip = |d: &str| d.split("  (").next().unwrap_or(d).to_string();
    let names: Vec<String> = all_commands.iter().map(|(_, d)| strip(d)).collect();
    let ranked = picker::rank_strings(names.clone(), query, false, &[], None);
    ranked
        .into_iter()
        .filter_map(|name| {
            names
                .iter()
                .position(|n| n == &name)
                .and_then(|i| all_commands.get(i).cloned())
        })
        .collect()
}

/// Escape a literal string for safe inclusion in a PCRE2 pattern.
fn regex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if "\\.+*?()|[]{}^$".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Compile the find pattern based on the current toggle state.
fn build_find_regex(
    query: &str,
    use_regex: bool,
    whole_word: bool,
    case_insensitive: bool,
) -> Option<crate::editor::regex::NativeRegex> {
    if query.is_empty() {
        return None;
    }
    let mut pat = if use_regex {
        query.to_string()
    } else {
        regex_escape(query)
    };
    if whole_word {
        pat = format!(r"\b(?:{pat})\b");
    }
    // The whole document is searched as one subject, so `multiline` keeps
    // `^`/`$` anchored to line boundaries rather than the document ends.
    let flags = crate::editor::regex::CompileFlags {
        caseless: case_insensitive,
        multiline: true,
        ..Default::default()
    };
    crate::editor::regex::NativeRegex::compile(&pat, flags).ok()
}

/// Map a keystroke to a dialog-input undo/redo action via the keymap, so the
/// bindings the user configured for `doc:undo` / `doc:redo` drive the focused
/// dialog field too. Returns `Some(true)` when the key is bound to `doc:redo`,
/// `Some(false)` for `doc:undo`, and `None` for anything else.
pub(crate) fn keymap_field_undo(
    keymap: &NativeKeymap,
    key: &str,
    mods: crate::editor::event::Modifiers,
) -> Option<bool> {
    keymap.commands_for(key, mods).and_then(|cmds| {
        if cmds.iter().any(|c| c == "doc:redo") {
            Some(true)
        } else if cmds.iter().any(|c| c == "doc:undo") {
            Some(false)
        } else {
            None
        }
    })
}

/// A clipboard action resolved from the keymap for a focused dialog input.
#[derive(Clone, Copy)]
pub(crate) enum FieldClipboard {
    Copy,
    Cut,
    Paste,
}

/// Map a keystroke to a dialog-input clipboard action via the keymap, so the
/// bindings the user configured for `doc:copy` / `doc:cut` / `doc:paste` drive
/// the focused dialog field too. Returns `None` for anything else.
pub(crate) fn keymap_field_clipboard(
    keymap: &NativeKeymap,
    key: &str,
    mods: crate::editor::event::Modifiers,
) -> Option<FieldClipboard> {
    keymap.commands_for(key, mods).and_then(|cmds| {
        if cmds.iter().any(|c| c == "doc:paste") {
            Some(FieldClipboard::Paste)
        } else if cmds.iter().any(|c| c == "doc:cut") {
            Some(FieldClipboard::Cut)
        } else if cmds.iter().any(|c| c == "doc:copy") {
            Some(FieldClipboard::Copy)
        } else {
            None
        }
    })
}

/// Append clipboard text onto a single-line input buffer, dropping line breaks
/// so a multi-line paste collapses onto one line instead of corrupting a search
/// query. Used by the append-only find/replace, project-search, palette, and
/// notes inputs, none of which model more than one line.
pub(crate) fn append_clipboard_line(buf: &mut String, clip: &str) {
    buf.extend(clip.chars().filter(|&c| c != '\n' && c != '\r'));
}

/// Insert clipboard text into a caret-tracked single-line input at byte offset
/// `cursor`, dropping line breaks. Returns the caret position after the text.
pub(crate) fn insert_clipboard_line(buf: &mut String, cursor: usize, clip: &str) -> usize {
    let line: String = clip.chars().filter(|&c| c != '\n' && c != '\r').collect();
    buf.insert_str(cursor, &line);
    cursor + line.len()
}

/// Scan the document and return every match as (line, col, end_line, end_col).
/// All values are 1-based. Matches may span newlines; a match consuming a
/// line's trailing `\n` ends at column 1 of the next line.
fn compute_find_matches(
    dv: &DocView,
    query: &str,
    use_regex: bool,
    whole_word: bool,
    case_insensitive: bool,
) -> Vec<(usize, usize, usize, usize)> {
    let Some(re) = build_find_regex(query, use_regex, whole_word, case_insensitive) else {
        return Vec::new();
    };
    let Some(buf_id) = dv.buffer_id else {
        return Vec::new();
    };
    // Reuse the memoized subject across keystrokes, then run the scan without
    // holding the buffer lock.
    let Ok(subject) = buffer::with_buffer_mut(buf_id, |b| Ok(buffer::search_subject_cached(b)))
    else {
        return Vec::new();
    };
    subject.find_all(&re)
}

/// Like `compute_find_matches` but optionally restricts results to the lines
/// covered by `selection`. The range is `(start_line, start_col, end_line,
/// end_col)`, all 1-based.
pub(crate) fn compute_find_matches_filtered(
    dv: &DocView,
    query: &str,
    use_regex: bool,
    whole_word: bool,
    case_insensitive: bool,
    selection: Option<(usize, usize, usize, usize)>,
) -> Vec<(usize, usize, usize, usize)> {
    let all = compute_find_matches(dv, query, use_regex, whole_word, case_insensitive);
    let Some((sl, sc, el, ec)) = selection else {
        return all;
    };
    all.into_iter()
        .filter(|&(line, col, end_line, end_col)| {
            (line, col) >= (sl, sc) && (end_line, end_col) <= (el, ec)
        })
        .collect()
}

/// Index of the first match at or after (line, col). Wraps to 0 if nothing
/// later exists. Returns None only for an empty match list.
pub(crate) fn find_match_at_or_after(
    matches: &[(usize, usize, usize, usize)],
    line: usize,
    col: usize,
) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    for (i, m) in matches.iter().enumerate() {
        if m.0 > line || (m.0 == line && m.1 >= col) {
            return Some(i);
        }
    }
    Some(0)
}

/// Index of the last match strictly before (line, col). Wraps to the final
/// match if nothing earlier exists. Returns None only for an empty match list.
pub(crate) fn find_match_before(
    matches: &[(usize, usize, usize, usize)],
    line: usize,
    col: usize,
) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    let mut last = None;
    for (i, m) in matches.iter().enumerate() {
        if m.0 < line || (m.0 == line && m.1 < col) {
            last = Some(i);
        } else {
            break;
        }
    }
    Some(last.unwrap_or(matches.len() - 1))
}

/// Move the caret to the given match and scroll the view so it is visible.
pub(crate) fn select_find_match(dv: &mut DocView, m: (usize, usize, usize, usize), replace_active: bool) {
    let (line, col, end_line, end_col) = m;
    let Some(buf_id) = dv.buffer_id else { return };
    let _ = buffer::with_buffer_mut(buf_id, |b| {
        b.selections = vec![line, col, end_line, end_col];
        Ok(())
    });
    // Use the real line height from the current style, not a hardcoded
    // 20.0 — that was off by ~50% at typical fonts, so the computed
    // scroll target landed nowhere near the match and F3 / Enter
    // appeared to do nothing when the match was off-screen.
    let style = crate::editor::style_ctx::current_style();
    let line_h = style.code_font_height * 1.2;
    // The find bar overlays the top of the doc view (2 rows normally,
    // 3 with Replace open). Subtract its height so "centered" means
    // centered in the *visible* area rather than under the bar.
    let bar_row_h = style.font_height + style.padding_y * 2.0;
    let bar_h = bar_row_h * if replace_active { 3.0 } else { 2.0 };
    let cursor_y = (line as f64 - 1.0) * line_h;
    // Always center, unconditionally. The previous "only if off-screen"
    // check used the wrong line_h so it misjudged visibility; forcing a
    // center on every F3 / Enter is both simpler and what users expect.
    let view_h = dv.rect().h;
    dv.target_scroll_y = (cursor_y - (view_h + bar_h) / 2.0).max(0.0);
}

/// Current caret as (line, col) using the "cursor end" of the selection.
pub(crate) fn doc_cursor(dv: &DocView) -> (usize, usize) {
    dv.buffer_id
        .and_then(|id| {
            buffer::with_buffer(id, |b| {
                let line = *b.selections.get(2).unwrap_or(&1);
                let col = *b.selections.get(3).unwrap_or(&1);
                Ok((line, col))
            })
            .ok()
        })
        .unwrap_or((1, 1))
}

/// Selection anchor as (line, col) — the "other end" from the caret.
pub(crate) fn doc_anchor(dv: &DocView) -> (usize, usize) {
    dv.buffer_id
        .and_then(|id| {
            buffer::with_buffer(id, |b| {
                let line = *b.selections.first().unwrap_or(&1);
                let col = *b.selections.get(1).unwrap_or(&1);
                Ok((line, col))
            })
            .ok()
        })
        .unwrap_or((1, 1))
}

/// Replace the current selection (match) with replacement text. Caller must
/// ensure the selection is the active find match — we trust the find state
/// machine to keep the caret aligned with `find_matches[find_current]`.
pub(crate) fn replace_current_match(dv: &mut DocView, find_query: &str, replacement: &str) {
    if find_query.is_empty() {
        return;
    }
    let Some(buf_id) = dv.buffer_id else { return };
    let _ = buffer::with_buffer_mut(buf_id, |b| {
        if buffer::get_selected_text(b).is_empty() {
            return Ok(());
        }
        buffer::push_undo(b);
        buffer::delete_selection(b);
        let line = b.selections[0];
        let col = b.selections[1];
        if line <= b.lines.len() {
            let l = &mut b.lines[line - 1];
            let byte_pos = char_to_byte(l, col - 1);
            l.insert_str(byte_pos, replacement);
            let new_col = col + replacement.chars().count();
            b.selections = vec![line, new_col, line, new_col];
        }
        Ok(())
    });
}

/// When the cursor sits inside a leading run of spaces, returns the number
/// of characters backspace should delete to align with the previous indent
/// boundary. Returns `None` when the normal single-character backspace
/// should run instead (tab-indented documents, non-whitespace before cursor,
/// or cursor at column 1).
fn smart_backspace_span(
    line_text: &str,
    col: usize,
    indent_type: &str,
    indent_size: usize,
) -> Option<usize> {
    if indent_type != "soft" || col <= 1 || indent_size == 0 {
        return None;
    }
    let leading = col - 1;
    let prefix_is_all_spaces = line_text
        .chars()
        .take(leading)
        .all(|c| c == ' ');
    if !prefix_is_all_spaces {
        return None;
    }
    let remove = if leading % indent_size == 0 {
        indent_size
    } else {
        leading % indent_size
    };
    if remove >= 2 { Some(remove) } else { None }
}

/// Returns true when `prefix` (the line content before the cursor) ends —
/// after trailing whitespace and a line-comment if any — with an
/// open-block character: `:`, `{`, `(`, or `[`. Used to drive smart
/// auto-indent on Enter.
fn smart_indent_opens_block(prefix: &str) -> bool {
    let stripped = strip_trailing_line_comment(prefix);
    matches!(stripped.trim_end().chars().last(), Some(':' | '{' | '(' | '['))
}

/// Drop everything from the first `//`, `--`, or `#` line-comment marker.
/// Tracks simple single/double-quoted strings so that markers inside a
/// string literal are ignored. Heuristic only — does not understand raw
/// strings, escape sequences beyond a single backslash, or nested
/// comments — but it handles the common cases that drive smart-indent.
fn strip_trailing_line_comment(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut quote: Option<u8> = None;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = quote {
            if c == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if c == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        if c == b'"' || c == b'\'' {
            quote = Some(c);
            i += 1;
            continue;
        }
        if (c == b'/' && bytes.get(i + 1) == Some(&b'/'))
            || (c == b'-' && bytes.get(i + 1) == Some(&b'-'))
            || c == b'#'
        {
            return &s[..i];
        }
        i += 1;
    }
    s
}

/// Insert `text` at the active caret, replacing any selection and splitting
/// on '\n' across lines. Records an undo step and leaves the caret at the end
/// of the inserted text. Shared by clipboard paste and middle-click (PRIMARY)
/// paste so both behave identically.
pub(crate) fn insert_text_at_caret(b: &mut crate::editor::buffer::BufferState, text: &str) {
    buffer::push_undo(b);
    buffer::delete_selection(b);
    let line = b.selections[0];
    let col = b.selections[1];
    if line <= b.lines.len() {
        let l = &mut b.lines[line - 1];
        let byte_pos = char_to_byte(l, col - 1);
        let after = l[byte_pos..].to_string();
        l.truncate(byte_pos);
        let paste_lines: Vec<&str> = text.split('\n').collect();
        if paste_lines.len() == 1 {
            l.push_str(text);
            l.push_str(&after);
            let new_col = col + text.chars().count();
            b.selections = vec![line, new_col, line, new_col];
        } else {
            l.push_str(paste_lines[0]);
            l.push('\n');
            let mut cur_line = line;
            for (i, pl) in paste_lines.iter().enumerate().skip(1) {
                cur_line += 1;
                if i == paste_lines.len() - 1 {
                    let new_col = pl.chars().count() + 1;
                    let mut new_line = pl.to_string();
                    new_line.push_str(&after);
                    b.lines.insert(cur_line - 1, new_line);
                    b.selections = vec![cur_line, new_col, cur_line, new_col];
                } else {
                    b.lines.insert(cur_line - 1, format!("{pl}\n"));
                }
            }
        }
    }
}

/// Convert pasted text's leading whitespace to match the document's indent
/// style. Detects whether the clipboard content uses tabs or spaces, then
/// re-indents every line to the target style (preserving relative depth).
/// Apply a list of LSP `TextEdit`s to a buffer as one undoable change.
/// Edits are applied last-position-first so earlier offsets stay valid.
/// LSP positions are 0-based; character is treated as a char column to match
/// the rest of this editor's LSP handling. Returns true if any edit applied.
pub(crate) fn apply_lsp_text_edits(
    state: &mut crate::editor::buffer::BufferState,
    edits: &[serde_json::Value],
) -> bool {
    use crate::editor::buffer::{self, EditRecord};
    let mut parsed: Vec<(usize, usize, usize, usize, String)> = Vec::new();
    for e in edits {
        let range = e.get("range");
        let pos = |key: &str, field: &str| -> usize {
            range
                .and_then(|r| r.get(key))
                .and_then(|p| p.get(field))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as usize
                + 1
        };
        let new_text = e
            .get("newText")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        parsed.push((
            pos("start", "line"),
            pos("start", "character"),
            pos("end", "line"),
            pos("end", "character"),
            new_text,
        ));
    }
    if parsed.is_empty() {
        return false;
    }
    parsed.sort_by_key(|e| std::cmp::Reverse((e.0, e.1)));
    buffer::push_undo(state);
    for (sl, sc, el, ec, nt) in parsed {
        let remove = EditRecord {
            kind: b'r',
            line1: sl,
            col1: sc,
            line2: el,
            col2: ec,
            text: String::new(),
        };
        let _ = buffer::apply_single_edit(&mut state.lines, &mut state.selections, &remove);
        if !nt.is_empty() {
            let insert = EditRecord {
                kind: b'i',
                line1: sl,
                col1: sc,
                line2: sl,
                col2: sc,
                text: nt,
            };
            let _ = buffer::apply_single_edit(&mut state.lines, &mut state.selections, &insert);
        }
    }
    state.total_bytes = state.lines.iter().map(|l| l.len() as u64).sum();
    true
}

/// Apply an LSP `WorkspaceEdit` (`changes` or `documentChanges` form) across
/// the affected files, opening any that are not already tabs, and save each so
/// the edit takes effect on disk. Returns the number of files changed.
pub(crate) fn apply_lsp_workspace_edit(
    edit: &serde_json::Value,
    docs: &mut Vec<crate::editor::open_doc::OpenDoc>,
    use_git: bool,
    atomic: bool,
) -> usize {
    let mut per_file: Vec<(String, Vec<serde_json::Value>)> = Vec::new();
    if let Some(changes) = edit.get("changes").and_then(|c| c.as_object()) {
        for (uri, edits) in changes {
            if let Some(arr) = edits.as_array() {
                per_file.push((uri.clone(), arr.clone()));
            }
        }
    } else if let Some(dc) = edit.get("documentChanges").and_then(|d| d.as_array()) {
        for change in dc {
            let uri = change
                .get("textDocument")
                .and_then(|t| t.get("uri"))
                .and_then(|v| v.as_str());
            let edits = change.get("edits").and_then(|e| e.as_array());
            if let (Some(uri), Some(edits)) = (uri, edits) {
                per_file.push((uri.to_string(), edits.clone()));
            }
        }
    }
    let mut changed = 0;
    for (uri, edits) in per_file {
        let path = crate::editor::lsp_client::uri_to_path(&uri);
        if path.is_empty() {
            continue;
        }
        let idx = match docs.iter().position(|d| d.path == path) {
            Some(i) => i,
            None => {
                if !std::path::Path::new(&path).exists()
                    || !crate::editor::open_doc::open_file_into(&path, docs, use_git)
                {
                    continue;
                }
                docs.len() - 1
            }
        };
        let Some(buf_id) = docs[idx].view.buffer_id else {
            continue;
        };
        let applied = buffer::with_buffer_mut(buf_id, |b| Ok(apply_lsp_text_edits(b, &edits)))
            .unwrap_or(false);
        if applied {
            docs[idx].cached_change_id = -1;
            docs[idx].cached_render = std::sync::Arc::new(Vec::new());
            let p = docs[idx].path.clone();
            if let Ok(Ok(cid)) = buffer::with_buffer(buf_id, |b| {
                Ok(buffer::save_file(b, &p, b.crlf, atomic).map(|()| b.change_id))
            }) {
                docs[idx].saved_change_id = cid;
                docs[idx].saved_signature =
                    buffer::with_buffer(buf_id, |b| Ok(buffer::content_signature(&b.lines)))
                        .unwrap_or(0);
            }
            changed += 1;
        }
    }
    changed
}

pub(crate) fn convert_paste_indent(text: &str, doc_indent_type: &str, doc_indent_size: usize) -> String {
    let size = doc_indent_size.max(1);
    // Detect the paste's dominant indent character: if any non-blank line
    // starts with a tab, treat the paste as tab-indented; otherwise spaces.
    let paste_uses_tabs = text.lines().any(|l| l.starts_with('\t'));
    let paste_uses_spaces = !paste_uses_tabs && text.lines().any(|l| l.starts_with(' '));
    // Detect the paste's space-indent width (smallest leading-space run > 0).
    let paste_space_width = if paste_uses_spaces {
        text.lines()
            .filter(|l| l.starts_with(' '))
            .map(|l| l.chars().take_while(|c| *c == ' ').count())
            .filter(|&n| n > 0)
            .min()
            .unwrap_or(size)
    } else {
        size
    };
    let doc_uses_tabs = doc_indent_type == "hard";
    // No conversion needed if both sides agree.
    if paste_uses_tabs == doc_uses_tabs && (!paste_uses_spaces || paste_space_width == size) {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        // Count the indent level of this line in the paste's style.
        let (indent_level, rest_start) = if paste_uses_tabs {
            let tabs = line.chars().take_while(|c| *c == '\t').count();
            let byte = line
                .char_indices()
                .nth(tabs)
                .map(|(i, _)| i)
                .unwrap_or(line.len());
            (tabs, byte)
        } else {
            let spaces = line.chars().take_while(|c| *c == ' ').count();
            let byte = line
                .char_indices()
                .nth(spaces)
                .map(|(i, _)| i)
                .unwrap_or(line.len());
            (spaces / paste_space_width, byte)
        };
        // Re-indent in the document's style.
        if doc_uses_tabs {
            for _ in 0..indent_level {
                out.push('\t');
            }
        } else {
            for _ in 0..indent_level * size {
                out.push(' ');
            }
        }
        out.push_str(&line[rest_start..]);
    }
    out
}

/// Convert char index to byte index in a string.
/// Returns true when `a` is a strictly greater semver than `b`.
/// Compares major, minor, patch numerically; non-numeric segments fall back to
/// lexicographic order so malformed tags don't panic.
pub(crate) fn semver_gt(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> (u64, u64, u64) {
        let mut parts = s.splitn(4, '.');
        let major = parts.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        let minor = parts.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        let patch = parts.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        (major, minor, patch)
    };
    parse(a) > parse(b)
}

pub(crate) fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

/// Count chars in a string (for column positioning).
pub(crate) fn char_count(s: &str) -> usize {
    s.chars().count()
}

/// Handle a document command (cursor movement, editing).
/// `auto_scroll`: when true, the view scrolls to keep the cursor visible after
/// movement commands. Pass false for commands triggered by mouse clicks or
/// context menus — the user didn't intend to scroll.
/// `line_wrapping`: when true, horizontal auto-scroll is suppressed since the
/// cursor is always reachable by wrap — scrolling right would push content
/// out of view even though nothing extends past the visual right edge.
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_doc_command(
    dv: &mut DocView,
    cmd: &str,
    style: &StyleContext,
    indent_type: &str,
    indent_size: usize,
    comment_marker: Option<&crate::editor::main_loop::CommentMarker>,
    auto_scroll: bool,
    line_wrapping: bool,
) {
    let Some(buf_id) = dv.buffer_id else { return };
    let line_h = style.code_font_height * 1.2;

    match cmd {
        "doc:copy" | "doc:cut" => {
            let text =
                buffer::with_buffer(buf_id, |b| Ok(buffer::get_selected_text(b)))
                    .unwrap_or_default();
            if !text.is_empty() {
                crate::window::set_clipboard_text(&text);
                if cmd == "doc:cut" {
                    let _ = buffer::with_buffer_mut(buf_id, |b| {
                        buffer::push_undo(b);
                        buffer::delete_selection(b);
                        Ok(())
                    });
                }
            }
            return;
        }
        "doc:paste" => {
            if let Some(text) = crate::window::get_clipboard_text() {
                let text = convert_paste_indent(&text, indent_type, indent_size);
                let _ = buffer::with_buffer_mut(buf_id, |b| {
                    buffer::push_undo(b);
                    buffer::delete_selection(b);
                    let line = b.selections[0];
                    let col = b.selections[1];
                    if line <= b.lines.len() {
                        let l = &mut b.lines[line - 1];
                        let byte_pos = char_to_byte(l, col - 1);
                        let after = l[byte_pos..].to_string();
                        l.truncate(byte_pos);
                        let paste_lines: Vec<&str> = text.split('\n').collect();
                        if paste_lines.len() == 1 {
                            l.push_str(&text);
                            l.push_str(&after);
                            let new_col = col + text.chars().count();
                            b.selections = vec![line, new_col, line, new_col];
                        } else {
                            l.push_str(paste_lines[0]);
                            l.push('\n');
                            let mut cur_line = line;
                            for (i, pl) in paste_lines.iter().enumerate().skip(1) {
                                cur_line += 1;
                                if i == paste_lines.len() - 1 {
                                    let new_col = pl.chars().count() + 1;
                                    let mut new_line = pl.to_string();
                                    new_line.push_str(&after);
                                    b.lines.insert(cur_line - 1, new_line);
                                    b.selections = vec![cur_line, new_col, cur_line, new_col];
                                } else {
                                    b.lines.insert(cur_line - 1, format!("{pl}\n"));
                                }
                            }
                        }
                    }
                    Ok(())
                });
            }
            return;
        }
        _ => {}
    }

    let mut prev_cursor_line: usize = 0;
    let _ = buffer::with_buffer_mut(buf_id, |b| {
        let anchor_line = *b.selections.first().unwrap_or(&1);
        let mut anchor_col = *b.selections.get(1).unwrap_or(&1);
        let cursor_line = *b.selections.get(2).unwrap_or(&anchor_line);
        let cursor_col = *b.selections.get(3).unwrap_or(&anchor_col);
        prev_cursor_line = cursor_line;
        let line_count = b.lines.len();

        // Selection: shift variants move cursor but keep anchor.
        let is_select = cmd.starts_with("doc:select-to-");
        let mut preserve_anchor = false;
        let mut handled = true;

        // Movement always operates on the cursor position.
        let mut line = cursor_line;
        let mut col = cursor_col;

        match cmd {
            "doc:select-none"
                if buffer::cursor_count(b) > 1 => {
                    buffer::remove_extra_cursors(b);
                    return Ok(());
                }
                // Collapse selection to cursor.
            "doc:create-cursor-previous-line" => {
                let last_idx = b.selections.len() - 4;
                let last_line = b.selections[last_idx + 2];
                let last_col = b.selections[last_idx + 3];
                if last_line > 1 {
                    let new_line = last_line - 1;
                    let max_col = char_count(b.lines[new_line - 1].trim_end_matches('\n')) + 1;
                    buffer::add_cursor(b, new_line, last_col.min(max_col));
                }
                return Ok(());
            }
            "doc:create-cursor-next-line" => {
                let last_idx = b.selections.len() - 4;
                let last_line = b.selections[last_idx + 2];
                let last_col = b.selections[last_idx + 3];
                if last_line < line_count {
                    let new_line = last_line + 1;
                    let max_col = char_count(b.lines[new_line - 1].trim_end_matches('\n')) + 1;
                    buffer::add_cursor(b, new_line, last_col.min(max_col));
                }
                return Ok(());
            }
            "doc:select-all" => {
                b.selections[0] = 1;
                b.selections[1] = 1;
                let last = b.lines.len();
                let last_col = char_count(b.lines[last - 1].trim_end_matches('\n')) + 1;
                b.selections[2] = last;
                b.selections[3] = last_col;
                return Ok(());
            }
            "doc:move-to-previous-char" | "doc:select-to-previous-char" => {
                if col > 1 {
                    col -= 1;
                } else if line > 1 {
                    line -= 1;
                    col = char_count(b.lines[line - 1].trim_end_matches('\n')) + 1;
                }
            }
            "doc:move-to-next-char" | "doc:select-to-next-char" => {
                let max_col = char_count(b.lines[line - 1].trim_end_matches('\n')) + 1;
                if col < max_col {
                    col += 1;
                } else if line < line_count {
                    line += 1;
                    col = 1;
                }
            }
            "doc:move-to-previous-line" | "doc:select-to-previous-line"
                if line > 1 => {
                    line -= 1;
                    let max_col = char_count(b.lines[line - 1].trim_end_matches('\n')) + 1;
                    col = col.min(max_col);
                }
            "doc:move-to-next-line" | "doc:select-to-next-line"
                if line < line_count => {
                    line += 1;
                    let max_col = char_count(b.lines[line - 1].trim_end_matches('\n')) + 1;
                    col = col.min(max_col);
                }
            "doc:move-to-start-of-indentation" | "doc:select-to-start-of-indentation" => {
                let text = b.lines[line - 1].trim_end_matches('\n');
                let indent = text.len() - text.trim_start().len();
                col = if col == indent + 1 { 1 } else { indent + 1 };
            }
            "doc:move-to-end-of-line" | "doc:select-to-end-of-line" => {
                col = char_count(b.lines[line - 1].trim_end_matches('\n')) + 1;
            }
            "doc:move-to-start-of-doc" | "doc:select-to-start-of-doc" => {
                line = 1;
                col = 1;
            }
            "doc:move-to-end-of-doc" | "doc:select-to-end-of-doc" => {
                line = line_count;
                col = char_count(b.lines[line - 1].trim_end_matches('\n')) + 1;
            }
            "doc:move-to-previous-word-start" | "doc:select-to-previous-word-start" => {
                if col > 1 {
                    let text = b.lines[line - 1].trim_end_matches('\n');
                    let chars: Vec<char> = text.chars().collect();
                    let mut i = (col - 2).min(chars.len().saturating_sub(1));
                    // Skip whitespace backwards.
                    while i > 0 && chars[i].is_whitespace() {
                        i -= 1;
                    }
                    // Skip word chars backwards.
                    while i > 0 && !chars[i - 1].is_whitespace() && chars[i - 1].is_alphanumeric()
                        || chars.get(i.wrapping_sub(1)).is_some_and(|c| *c == '_')
                    {
                        if i == 0 {
                            break;
                        }
                        i -= 1;
                    }
                    col = i + 1;
                } else if line > 1 {
                    line -= 1;
                    col = char_count(b.lines[line - 1].trim_end_matches('\n')) + 1;
                }
            }
            "doc:move-to-next-word-end" | "doc:select-to-next-word-end" => {
                let text = b.lines[line - 1].trim_end_matches('\n');
                let chars: Vec<char> = text.chars().collect();
                let max = chars.len();
                let mut i = col - 1;
                if i < max {
                    // Skip word chars forward.
                    while i < max && (chars[i].is_alphanumeric() || chars[i] == '_') {
                        i += 1;
                    }
                    // Skip whitespace forward.
                    while i < max && chars[i].is_whitespace() {
                        i += 1;
                    }
                    col = i + 1;
                } else if line < line_count {
                    line += 1;
                    col = 1;
                }
            }
            "doc:delete-to-previous-word-start" => {
                buffer::push_undo(b);
                let text = b.lines[line - 1].trim_end_matches('\n').to_string();
                let chars: Vec<char> = text.chars().collect();
                let mut i = (col - 2).min(chars.len().saturating_sub(1));
                while i > 0 && chars[i].is_whitespace() {
                    i -= 1;
                }
                while i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_') {
                    i -= 1;
                }
                let new_col = i + 1;
                let l = &mut b.lines[line - 1];
                let start = char_to_byte(l, new_col - 1);
                let end = char_to_byte(l, col - 1);
                l.drain(start..end);
                col = new_col;
            }
            "doc:delete-to-next-word-end" => {
                buffer::push_undo(b);
                let text = b.lines[line - 1].trim_end_matches('\n').to_string();
                let chars: Vec<char> = text.chars().collect();
                let max = chars.len();
                let mut i = col - 1;
                while i < max && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                while i < max && chars[i].is_whitespace() {
                    i += 1;
                }
                let l = &mut b.lines[line - 1];
                let start = char_to_byte(l, col - 1);
                let end = char_to_byte(l, i);
                l.drain(start..end);
            }
            "doc:duplicate-lines" => {
                buffer::push_undo(b);
                let dup = b.lines[line - 1].clone();
                b.lines.insert(line, dup);
                line += 1;
            }
            "doc:delete-lines" => {
                buffer::push_undo(b);
                if b.lines.len() > 1 {
                    b.lines.remove(line - 1);
                    if line > b.lines.len() {
                        line = b.lines.len();
                    }
                    let max_col = char_count(b.lines[line - 1].trim_end_matches('\n')) + 1;
                    col = col.min(max_col);
                } else {
                    b.lines[0] = "\n".to_string();
                    col = 1;
                }
            }
            "doc:move-lines-up"
                if line > 1 => {
                    buffer::push_undo(b);
                    b.lines.swap(line - 1, line - 2);
                    line -= 1;
                }
            "doc:move-lines-down"
                if line < line_count => {
                    buffer::push_undo(b);
                    b.lines.swap(line - 1, line);
                    line += 1;
                }
            "doc:move-to-previous-page" | "doc:select-to-previous-page" => {
                let page = (dv.rect().h / (style.code_font_height * 1.2)) as usize;
                line = line.saturating_sub(page).max(1);
                let max_col = char_count(b.lines[line - 1].trim_end_matches('\n')) + 1;
                col = col.min(max_col);
            }
            "doc:move-to-next-page" | "doc:select-to-next-page" => {
                let page = (dv.rect().h / (style.code_font_height * 1.2)) as usize;
                line = (line + page).min(line_count);
                let max_col = char_count(b.lines[line - 1].trim_end_matches('\n')) + 1;
                col = col.min(max_col);
            }
            "doc:backspace" | "doc:delete"
                if anchor_line != cursor_line || anchor_col != cursor_col =>
            {
                // Selection active: delete the selected text.
                buffer::push_undo(b);
                buffer::delete_selection(b);
                line = b.selections[0];
                col = b.selections[1];
            }
            "doc:backspace" => {
                buffer::push_undo(b);
                let n = buffer::cursor_count(b);
                if n > 1 {
                    // Multi-cursor backspace: process bottom-to-top.
                    let mut positions: Vec<(usize, usize, usize)> = (0..n)
                        .map(|i| {
                            let base = i * 4;
                            (i, b.selections[base + 2], b.selections[base + 3])
                        })
                        .collect();
                    positions.sort_by(|a, bp| bp.1.cmp(&a.1).then(bp.2.cmp(&a.2)));
                    let mut results: Vec<(usize, usize, usize)> = Vec::new();
                    for &(idx, cline, ccol) in &positions {
                        if cline <= b.lines.len()
                            && let Some(remove) = smart_backspace_span(
                                &b.lines[cline - 1],
                                ccol,
                                indent_type,
                                indent_size,
                            )
                        {
                            let l = &mut b.lines[cline - 1];
                            let bp = char_to_byte(l, ccol - 1 - remove);
                            let ep = char_to_byte(l, ccol - 1);
                            l.drain(bp..ep);
                            results.push((idx, cline, ccol - remove));
                        } else if ccol > 1 && cline <= b.lines.len() {
                            let l = &mut b.lines[cline - 1];
                            let bp = char_to_byte(l, ccol - 2);
                            let ep = char_to_byte(l, ccol - 1);
                            l.drain(bp..ep);
                            results.push((idx, cline, ccol - 1));
                        } else if cline > 1 {
                            let removed = b.lines.remove(cline - 1);
                            let new_line = cline - 1;
                            let prev_len = char_count(b.lines[new_line - 1].trim_end_matches('\n'));
                            let prev = &mut b.lines[new_line - 1];
                            if prev.ends_with('\n') {
                                prev.pop();
                            }
                            prev.push_str(&removed);
                            results.push((idx, new_line, prev_len + 1));
                        } else {
                            results.push((idx, cline, ccol));
                        }
                    }
                    for (idx, rl, rc) in results {
                        let base = idx * 4;
                        b.selections[base] = rl;
                        b.selections[base + 1] = rc;
                        b.selections[base + 2] = rl;
                        b.selections[base + 3] = rc;
                    }
                    return Ok(());
                }
                buffer::delete_selection(b);
                line = b.selections[0];
                col = b.selections[1];
                if let Some(remove) = smart_backspace_span(
                    &b.lines[line - 1],
                    col,
                    indent_type,
                    indent_size,
                ) {
                    let l = &mut b.lines[line - 1];
                    let byte_start = char_to_byte(l, col - 1 - remove);
                    let byte_end = char_to_byte(l, col - 1);
                    l.drain(byte_start..byte_end);
                    col -= remove;
                } else if col > 1 {
                    let l = &mut b.lines[line - 1];
                    let byte_pos = char_to_byte(l, col - 2);
                    let end = char_to_byte(l, col - 1);
                    l.drain(byte_pos..end);
                    col -= 1;
                } else if line > 1 {
                    let removed = b.lines.remove(line - 1);
                    line -= 1;
                    let prev_len = char_count(b.lines[line - 1].trim_end_matches('\n'));
                    let prev = &mut b.lines[line - 1];
                    if prev.ends_with('\n') {
                        prev.pop();
                    }
                    prev.push_str(&removed);
                    col = prev_len + 1;
                }
            }
            "doc:delete" => {
                buffer::push_undo(b);
                let n = buffer::cursor_count(b);
                if n > 1 {
                    // Multi-cursor delete: process bottom-to-top.
                    let mut positions: Vec<(usize, usize, usize)> = (0..n)
                        .map(|i| {
                            let base = i * 4;
                            (i, b.selections[base + 2], b.selections[base + 3])
                        })
                        .collect();
                    positions.sort_by(|a, bp| bp.1.cmp(&a.1).then(bp.2.cmp(&a.2)));
                    for &(_idx, cline, ccol) in &positions {
                        if cline > b.lines.len() {
                            continue;
                        }
                        let max_c = char_count(b.lines[cline - 1].trim_end_matches('\n')) + 1;
                        if ccol < max_c {
                            let l = &mut b.lines[cline - 1];
                            let bp = char_to_byte(l, ccol - 1);
                            let ep = char_to_byte(l, ccol);
                            l.drain(bp..ep);
                        } else if cline < b.lines.len() {
                            let removed = b.lines.remove(cline);
                            let cur = &mut b.lines[cline - 1];
                            if cur.ends_with('\n') {
                                cur.pop();
                            }
                            cur.push_str(&removed);
                        }
                    }
                    return Ok(());
                }
                let max_col = char_count(b.lines[line - 1].trim_end_matches('\n')) + 1;
                if col < max_col {
                    let l = &mut b.lines[line - 1];
                    let byte_pos = char_to_byte(l, col - 1);
                    let end = char_to_byte(l, col);
                    l.drain(byte_pos..end);
                } else if line < b.lines.len() {
                    let removed = b.lines.remove(line);
                    let cur = &mut b.lines[line - 1];
                    if cur.ends_with('\n') {
                        cur.pop();
                    }
                    cur.push_str(&removed);
                }
            }
            "doc:newline" => {
                buffer::push_undo(b);
                buffer::delete_selection(b);
                line = b.selections[0];
                col = b.selections[1];
                let indent: String = b.lines[line - 1]
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect();
                let l = &mut b.lines[line - 1];
                let byte_pos = char_to_byte(l, col - 1);
                let rest = l[byte_pos..].to_string();
                let before_cursor = l[..byte_pos].to_string();
                l.truncate(byte_pos);
                l.push('\n');
                let extra = if smart_indent_opens_block(&before_cursor) {
                    if indent_type == "hard" {
                        "\t".to_string()
                    } else {
                        " ".repeat(indent_size.max(1))
                    }
                } else {
                    String::new()
                };
                let new_line = format!("{indent}{extra}{rest}");
                let new_col = indent.chars().count() + extra.chars().count() + 1;
                b.lines.insert(line, new_line);
                line += 1;
                col = new_col;
            }
            "doc:newline-below" => {
                buffer::push_undo(b);
                let indent: String = b.lines[line - 1]
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect();
                let new_line = format!("{indent}\n");
                let new_col = indent.len() + 1;
                b.lines.insert(line, new_line);
                line += 1;
                col = new_col;
            }
            "doc:newline-above" => {
                buffer::push_undo(b);
                let indent: String = b.lines[line - 1]
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect();
                let new_line = format!("{indent}\n");
                let new_col = indent.len() + 1;
                b.lines.insert(line - 1, new_line);
                col = new_col;
            }
            "doc:indent" => {
                buffer::push_undo(b);
                let indent_str = if indent_type == "hard" {
                    "\t".to_string()
                } else {
                    " ".repeat(indent_size)
                };
                let indent_len = indent_str.chars().count();
                if anchor_line != cursor_line {
                    let (start, end) =
                        (anchor_line.min(cursor_line), anchor_line.max(cursor_line));
                    for i in start..=end {
                        if let Some(l) = b.lines.get_mut(i - 1) {
                            l.insert_str(0, &indent_str);
                        }
                    }
                    col += indent_len;
                    anchor_col += indent_len;
                    preserve_anchor = true;
                } else {
                    let l = &mut b.lines[line - 1];
                    let byte_pos = char_to_byte(l, col - 1);
                    l.insert_str(byte_pos, &indent_str);
                    col += indent_len;
                }
            }
            "core:sort-lines" => {
                buffer::push_undo(b);
                let (start, end) = if anchor_line != cursor_line || anchor_col != cursor_col {
                    // If cursor is at col 1 of the last selected line, exclude it.
                    let raw_end = if cursor_line > anchor_line && cursor_col <= 1 {
                        cursor_line - 1
                    } else {
                        cursor_line
                    };
                    if anchor_line <= raw_end {
                        (anchor_line, raw_end)
                    } else {
                        (raw_end, anchor_line)
                    }
                } else {
                    (1, b.lines.len())
                };
                let slice = &mut b.lines[start - 1..end];
                slice.sort();
                // Place cursor at the start of the sorted range.
                line = start;
                col = 1;
            }
            "doc:upper-case" | "doc:lower-case"
                if (anchor_line != cursor_line || anchor_col != cursor_col) => {
                    buffer::push_undo(b);
                    let (s_line, s_col, e_line, e_col) = if anchor_line < cursor_line
                        || (anchor_line == cursor_line && anchor_col <= cursor_col)
                    {
                        (anchor_line, anchor_col, cursor_line, cursor_col)
                    } else {
                        (cursor_line, cursor_col, anchor_line, anchor_col)
                    };
                    let is_upper = cmd == "doc:upper-case";
                    if s_line == e_line {
                        let l = &mut b.lines[s_line - 1];
                        let start_byte =
                            l.char_indices().nth(s_col - 1).map(|(i, _)| i).unwrap_or(0);
                        let end_byte = l
                            .char_indices()
                            .nth(e_col - 1)
                            .map(|(i, _)| i)
                            .unwrap_or(l.len());
                        let fragment = &l[start_byte..end_byte];
                        let converted = if is_upper {
                            fragment.to_uppercase()
                        } else {
                            fragment.to_lowercase()
                        };
                        l.replace_range(start_byte..end_byte, &converted);
                    } else {
                        for li in s_line..=e_line {
                            let l = &mut b.lines[li - 1];
                            let start = if li == s_line {
                                l.char_indices().nth(s_col - 1).map(|(i, _)| i).unwrap_or(0)
                            } else {
                                0
                            };
                            let end = if li == e_line {
                                l.char_indices()
                                    .nth(e_col - 1)
                                    .map(|(i, _)| i)
                                    .unwrap_or(l.len())
                            } else {
                                l.trim_end_matches('\n').len()
                            };
                            let fragment = &l[start..end];
                            let converted = if is_upper {
                                fragment.to_uppercase()
                            } else {
                                fragment.to_lowercase()
                            };
                            l.replace_range(start..end, &converted);
                        }
                    }
                }
            "doc:toggle-line-comments" => {
                let Some(marker) = comment_marker else {
                    // Language has no defined comment style; do nothing
                    // rather than guessing and corrupting the file.
                    return Ok(());
                };
                buffer::push_undo(b);
                let (start, end) = if anchor_line != cursor_line {
                    (anchor_line.min(cursor_line), anchor_line.max(cursor_line))
                } else {
                    (line, line)
                };
                match marker {
                    crate::editor::main_loop::CommentMarker::Line(prefix) => {
                        let prefix_space = format!("{prefix} ");
                        // All non-blank lines must already start with the
                        // prefix for the toggle to remove rather than add.
                        let all_commented = (start..=end)
                            .filter_map(|i| b.lines.get(i - 1))
                            .filter(|l| !l.trim().is_empty())
                            .all(|l| l.trim_start().starts_with(prefix.as_str()));
                        if all_commented {
                            for i in start..=end {
                                if let Some(l) = b.lines.get_mut(i - 1) {
                                    if let Some(pos) = l.find(&prefix_space) {
                                        l.replace_range(pos..pos + prefix_space.len(), "");
                                    } else if let Some(pos) = l.find(prefix.as_str()) {
                                        l.replace_range(pos..pos + prefix.len(), "");
                                    }
                                }
                            }
                        } else {
                            for i in start..=end {
                                if let Some(l) = b.lines.get_mut(i - 1) {
                                    if l.trim().is_empty() {
                                        continue;
                                    }
                                    let indent_len =
                                        l.chars().take_while(|c| *c == ' ' || *c == '\t').count();
                                    let byte = l
                                        .char_indices()
                                        .nth(indent_len)
                                        .map(|(i, _)| i)
                                        .unwrap_or(0);
                                    l.insert_str(byte, &prefix_space);
                                }
                            }
                        }
                    }
                    crate::editor::main_loop::CommentMarker::Block(open, close) => {
                        // Per-line wrap: open at start (after indent), close at
                        // end (before any trailing whitespace + newline). When
                        // every non-blank line is already wrapped, strip instead.
                        let all_wrapped = (start..=end)
                            .filter_map(|i| b.lines.get(i - 1))
                            .filter(|l| !l.trim().is_empty())
                            .all(|l| {
                                let trimmed = l.trim_end_matches('\n').trim_end();
                                let stripped_left = l.trim_start();
                                stripped_left.starts_with(open.as_str())
                                    && trimmed.ends_with(close.as_str())
                                    && trimmed.len() >= open.len() + close.len()
                            });
                        if all_wrapped {
                            for i in start..=end {
                                if let Some(l) = b.lines.get_mut(i - 1) {
                                    let had_newline = l.ends_with('\n');
                                    let body = l.trim_end_matches('\n').to_string();
                                    let trailing_ws_len = body.len() - body.trim_end().len();
                                    let trailing_ws =
                                        body[body.len() - trailing_ws_len..].to_string();
                                    let core = body[..body.len() - trailing_ws_len].to_string();
                                    // Strip closing marker (with optional preceding space).
                                    let core = if let Some(c) = core.strip_suffix(close.as_str()) {
                                        c.strip_suffix(' ').unwrap_or(c).to_string()
                                    } else {
                                        core
                                    };
                                    // Strip opening marker (with optional trailing space) after indent.
                                    let indent_len = core
                                        .chars()
                                        .take_while(|c| *c == ' ' || *c == '\t')
                                        .count();
                                    let indent_byte = core
                                        .char_indices()
                                        .nth(indent_len)
                                        .map(|(i, _)| i)
                                        .unwrap_or(core.len());
                                    let (indent, rest) = core.split_at(indent_byte);
                                    let rest = rest.strip_prefix(open.as_str()).unwrap_or(rest);
                                    let rest = rest.strip_prefix(' ').unwrap_or(rest);
                                    let mut new_line = format!("{indent}{rest}{trailing_ws}");
                                    if had_newline {
                                        new_line.push('\n');
                                    }
                                    *l = new_line;
                                }
                            }
                        } else {
                            for i in start..=end {
                                if let Some(l) = b.lines.get_mut(i - 1) {
                                    if l.trim().is_empty() {
                                        continue;
                                    }
                                    let had_newline = l.ends_with('\n');
                                    let body = l.trim_end_matches('\n').to_string();
                                    let indent_len = body
                                        .chars()
                                        .take_while(|c| *c == ' ' || *c == '\t')
                                        .count();
                                    let indent_byte = body
                                        .char_indices()
                                        .nth(indent_len)
                                        .map(|(i, _)| i)
                                        .unwrap_or(0);
                                    let (indent, rest) = body.split_at(indent_byte);
                                    let mut new_line =
                                        format!("{indent}{open} {} {close}", rest.trim_end());
                                    // Preserve any trailing whitespace after the close marker.
                                    let trailing_ws_len = rest.len() - rest.trim_end().len();
                                    if trailing_ws_len > 0 {
                                        new_line.push_str(&rest[rest.len() - trailing_ws_len..]);
                                    }
                                    if had_newline {
                                        new_line.push('\n');
                                    }
                                    *l = new_line;
                                }
                            }
                        }
                    }
                }
            }
            "doc:unindent" => {
                buffer::push_undo(b);
                let (start, end) = if anchor_line != cursor_line {
                    (anchor_line.min(cursor_line), anchor_line.max(cursor_line))
                } else {
                    (line, line)
                };
                for i in start..=end {
                    if let Some(l) = b.lines.get_mut(i - 1) {
                        if indent_type == "hard" {
                            if l.starts_with('\t') {
                                l.remove(0);
                            }
                        } else {
                            let remove = l
                                .chars()
                                .take(indent_size)
                                .take_while(|c| *c == ' ')
                                .count();
                            if remove > 0 {
                                l.replace_range(..remove, "");
                            }
                        }
                    }
                }
                col = col.saturating_sub(indent_size).max(1);
                if anchor_line != cursor_line {
                    anchor_col = anchor_col.saturating_sub(indent_size).max(1);
                    preserve_anchor = true;
                }
            }
            "doc:join-lines" => {
                buffer::push_undo(b);
                if line < b.lines.len() {
                    let next = b.lines.remove(line);
                    let trimmed = next.trim_start().trim_end_matches('\n');
                    let l = &mut b.lines[line - 1];
                    if l.ends_with('\n') {
                        l.pop();
                    }
                    if !l.ends_with(' ') && !trimmed.is_empty() {
                        l.push(' ');
                    }
                    col = l.chars().count() + 1;
                    l.push_str(trimmed);
                    l.push('\n');
                }
            }
            _ => {
                handled = false;
            }
        }

        if !handled {
            return Ok(());
        }

        // Collapse to single cursor when a non-create-cursor command runs.
        if buffer::cursor_count(b) > 1 {
            buffer::remove_extra_cursors(b);
        }

        // Update selections: select commands and indent/unindent keep anchor,
        // move commands collapse.
        if is_select || preserve_anchor {
            b.selections[0] = anchor_line;
            b.selections[1] = anchor_col;
        } else {
            b.selections[0] = line;
            b.selections[1] = col;
        }
        b.selections[2] = line;
        b.selections[3] = col;
        Ok(())
    });

    // Auto-scroll to keep cursor visible — only for keyboard-initiated
    // navigation where the cursor's line actually changed. Snap scroll_y
    // to the new target so Enter on the last line doesn't leave the
    // fresh line visibly clipped for the ~6 frames the lerp takes to
    // settle — the old behaviour was only "saved" by the user issuing
    // another command (save, etc.) that triggered an unrelated repaint.
    if auto_scroll {
        let _ = buffer::with_buffer(buf_id, |b| {
            let cursor_line = *b.selections.get(2).unwrap_or(&1);
            if cursor_line == prev_cursor_line {
                return Ok(());
            }
            let cursor_y = (cursor_line as f64 - 1.0) * line_h;
            let view_h = dv.rect().h;
            // One line of margin above and below the cursor so it's never
            // drawn flush with the viewport edge (would otherwise clip
            // the descender, and on the last line the glyph sat at
            // half-height until some later event forced another scroll).
            let margin = line_h;
            let mut new_target = dv.target_scroll_y;
            if cursor_y - margin < new_target {
                new_target = (cursor_y - margin).max(0.0);
            } else if cursor_y + line_h + margin > new_target + view_h {
                new_target = cursor_y + line_h + margin - view_h;
            }
            if new_target != dv.target_scroll_y {
                dv.target_scroll_y = new_target;
                dv.scroll_y = new_target;
            }
            Ok(())
        });
    }

    // Horizontal auto-scroll to keep cursor visible (e.g. End on a long line).
    // Cross-line jumps only scroll LEFT (to reveal a cursor at a small column),
    // never RIGHT (which would push the left-side content of nearby shorter
    // lines off-screen and make the document appear blank). When line
    // wrapping is on, the caret always has a visible visual row, so this
    // whole block would otherwise chase a virtual column that doesn't
    // exist — pin `scroll_x` to 0 instead.
    if line_wrapping {
        if dv.scroll_x != 0.0 || dv.target_scroll_x != 0.0 {
            dv.scroll_x = 0.0;
            dv.target_scroll_x = 0.0;
        }
    } else if dv.code_char_w > 0.0 {
        let _ = buffer::with_buffer(buf_id, |b| {
            let cursor_line_now = *b.selections.get(2).unwrap_or(&1);
            let cursor_col = *b.selections.get(3).unwrap_or(&1);
            let cursor_x = (cursor_col as f64 - 1.0) * dv.code_char_w;
            let text_w =
                (dv.rect().w - dv.gutter_width - style.padding_x * 2.0 - style.scrollbar_size)
                    .max(0.0);
            // Keep one char of trailing padding so the caret isn't flush with the right edge.
            let right_pad = dv.code_char_w;
            let same_line = cursor_line_now == prev_cursor_line;
            if cursor_x < dv.scroll_x {
                dv.scroll_x = cursor_x;
                dv.target_scroll_x = cursor_x;
            } else if same_line && cursor_x + right_pad > dv.scroll_x + text_w {
                dv.scroll_x = (cursor_x + right_pad - text_w).max(0.0);
                dv.target_scroll_x = dv.scroll_x;
            }
            Ok(())
        });
    }

    // Fold/unfold commands operate on dv.folds outside the buffer closure.
    match cmd {
        "doc:fold" => {
            let _ = buffer::with_buffer(buf_id, |b| {
                let cursor_line = *b.selections.get(2).unwrap_or(&1);
                if let Some(end) = crate::editor::picker::get_fold_end(&b.lines, cursor_line) {
                    if !dv.folds.iter().any(|(s, _)| *s == cursor_line) {
                        dv.folds.push((cursor_line, end));
                        dv.folds.sort_by_key(|(s, _)| *s);
                    }
                }
                Ok(())
            });
        }
        "doc:unfold" => {
            let _ = buffer::with_buffer(buf_id, |b| {
                let cursor_line = *b.selections.get(2).unwrap_or(&1);
                dv.folds
                    .retain(|(s, e)| !(cursor_line >= *s && cursor_line <= *e));
                Ok(())
            });
        }
        "doc:unfold-all" => {
            dv.folds.clear();
        }
        "doc:toggle-bookmark" => {
            let _ = buffer::with_buffer(buf_id, |b| {
                let cursor_line = *b.selections.get(2).unwrap_or(&1);
                if let Some(pos) = dv.bookmarks.iter().position(|&l| l == cursor_line) {
                    dv.bookmarks.remove(pos);
                } else {
                    dv.bookmarks.push(cursor_line);
                    dv.bookmarks.sort();
                }
                Ok(())
            });
        }
        "doc:next-bookmark"
            if !dv.bookmarks.is_empty() => {
                let _ = buffer::with_buffer_mut(buf_id, |b| {
                    let cursor_line = *b.selections.get(2).unwrap_or(&1);
                    let target = dv
                        .bookmarks
                        .iter()
                        .find(|&&l| l > cursor_line)
                        .copied()
                        .unwrap_or(dv.bookmarks[0]);
                    b.selections = vec![target, 1, target, 1];
                    Ok(())
                });
                scroll_to_cursor(dv);
            }
        "doc:previous-bookmark"
            if !dv.bookmarks.is_empty() => {
                let _ = buffer::with_buffer_mut(buf_id, |b| {
                    let cursor_line = *b.selections.get(2).unwrap_or(&1);
                    let target = dv
                        .bookmarks
                        .iter()
                        .rev()
                        .find(|&&l| l < cursor_line)
                        .copied()
                        .unwrap_or(*dv.bookmarks.last().unwrap_or(&1));
                    b.selections = vec![target, 1, target, 1];
                    Ok(())
                });
                scroll_to_cursor(dv);
            }
        _ => {}
    }
}

/// Scroll view so the cursor line is visible.
pub(crate) fn scroll_to_cursor(dv: &mut DocView) {
    let Some(buf_id) = dv.buffer_id else { return };
    let _ = buffer::with_buffer(buf_id, |b| {
        let cursor_line = *b.selections.get(2).unwrap_or(&1);
        let line_h = 20.0;
        let cursor_y = (cursor_line as f64 - 1.0) * line_h;
        let view_h = dv.rect().h;
        if cursor_y < dv.target_scroll_y || cursor_y + line_h > dv.target_scroll_y + view_h {
            dv.target_scroll_y = (cursor_y - view_h / 2.0).max(0.0);
        }
        Ok(())
    });
}

/// Parse a hex color string like "#rrggbb" or "#rrggbbaa" or "rgba(r,g,b,a)" into Color.
pub(crate) fn parse_theme_color(s: &str) -> Option<crate::editor::types::Color> {
    use crate::editor::types::Color;
    if let Some(hex) = s.strip_prefix('#') {
        let hex = hex.trim();
        if hex.len() == 6 || hex.len() == 8 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = if hex.len() == 8 {
                u8::from_str_radix(&hex[6..8], 16).ok()?
            } else {
                255
            };
            return Some(Color::new(r, g, b, a));
        }
    }
    if s.starts_with("rgba(") {
        let inner = s.trim_start_matches("rgba(").trim_end_matches(')');
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 4 {
            let r = parts[0].trim().parse::<u8>().ok()?;
            let g = parts[1].trim().parse::<u8>().ok()?;
            let b = parts[2].trim().parse::<u8>().ok()?;
            let a = (parts[3].trim().parse::<f64>().ok()? * 255.0) as u8;
            return Some(Color::new(r, g, b, a));
        }
    }
    None
}

/// Apply a loaded theme palette to a StyleContext.
pub(crate) fn apply_theme_to_style(style: &mut StyleContext, palette: &crate::editor::style::ThemePalette) {
    let set = |field: &mut crate::editor::types::Color, key: &str| {
        if let Some(hex) = palette.colors.get(key) {
            if let Some(c) = parse_theme_color(hex) {
                *field = c;
            }
        }
    };
    set(&mut style.background, "background");
    set(&mut style.background2, "background2");
    set(&mut style.background3, "background3");
    set(&mut style.text, "text");
    set(&mut style.caret, "caret");
    set(&mut style.accent, "accent");
    set(&mut style.dim, "dim");
    set(&mut style.divider, "divider");
    set(&mut style.selection, "selection");
    set(&mut style.line_number, "line_number");
    set(&mut style.line_number2, "line_number2");
    set(&mut style.line_highlight, "line_highlight");
    set(&mut style.scrollbar, "scrollbar");
    set(&mut style.scrollbar2, "scrollbar2");
    set(&mut style.scrollbar_track, "scrollbar_track");
    set(&mut style.nagbar, "nagbar");
    set(&mut style.nagbar_text, "nagbar_text");
    set(&mut style.nagbar_dim, "nagbar_dim");
    set(&mut style.good, "good");
    set(&mut style.warn, "warn");
    set(&mut style.error, "error");

    // Store syntax colors in a thread-local for the tokenizer to use.
    if let Some(syn) = palette.sub_palettes.get("syntax") {
        let mut colors = std::collections::HashMap::new();
        for (k, v) in syn {
            if let Some(c) = parse_theme_color(v) {
                colors.insert(k.clone(), c.to_array());
            }
        }
        SYNTAX_COLORS.with(|s| *s.borrow_mut() = colors);
    }
}


/// Load fonts from NativeConfig into a draw context.
pub(crate) fn load_fonts(
    config: &NativeConfig,
) -> Result<crate::editor::draw_context::NativeDrawContext, String> {
    use crate::renderer::{Antialiasing, FontInner, Hinting};

    let mut ctx = crate::editor::draw_context::NativeDrawContext::new();

    // Display scale: ratio of pixel size to logical window size.
    let scale = crate::window::get_display_scale();

    let load = |spec: &crate::editor::config::FontSpec,
                ctx: &mut crate::editor::draw_context::NativeDrawContext|
     -> Result<u64, String> {
        let aa = spec
            .options
            .antialiasing
            .as_deref()
            .map(|s| match s {
                "none" => Antialiasing::None,
                "grayscale" => Antialiasing::Grayscale,
                _ => Antialiasing::Subpixel,
            })
            .unwrap_or_default();
        let hint = spec
            .options
            .hinting
            .as_deref()
            .map(|s| match s {
                "none" => Hinting::None,
                "full" => Hinting::Full,
                _ => Hinting::Slight,
            })
            .unwrap_or_default();
        let paths: Vec<&str> = if let Some(ref ps) = spec.paths {
            ps.iter().map(String::as_str).collect()
        } else if let Some(ref p) = spec.path {
            vec![p.as_str()]
        } else {
            return Err("font spec has no path".into());
        };
        let mut refs = Vec::new();
        for path in paths {
            let scaled_size = spec.size as f32 * scale as f32;
            let inner = FontInner::load(path, scaled_size, aa, hint)?;
            let arc = std::sync::Arc::new(parking_lot::Mutex::new(inner));
            crate::renderer::font::register_font(&arc);
            refs.push(arc);
        }
        Ok(ctx.add_font(refs))
    };

    let ui = load(&config.fonts.ui, &mut ctx)?;
    let code = load(&config.fonts.code, &mut ctx)?;

    // Load scaled heading fonts from the UI font path. Sizes scale the body
    // font size (`config.fonts.ui.size`) by decreasing factors so h1 > h2 >
    // h3 > h4 (= body). h4-h6 reuse the body slot. Any load failure falls
    // back to the body font so a missing path never blocks startup.
    let load_heading = |mul: f64, ctx: &mut crate::editor::draw_context::NativeDrawContext| {
        let spec = crate::editor::config::FontSpec {
            path: config.fonts.ui.path.clone(),
            size: ((config.fonts.ui.size as f64) * mul).round().max(1.0) as u32,
            options: config.fonts.ui.options.clone(),
            ..Default::default()
        };
        load(&spec, ctx).unwrap_or(ui)
    };
    let h1 = load_heading(1.75, &mut ctx);
    let h2 = load_heading(1.45, &mut ctx);
    let h3 = load_heading(1.2, &mut ctx);

    let (icon, big, icon_big, seti) = if crate::editor::main_loop::is_single_file() {
        (ui, ui, ui, ui)
    } else {
        let icon = load(&config.fonts.icon, &mut ctx)?;
        let big = if config.fonts.big.path.is_some() {
            load(&config.fonts.big, &mut ctx)?
        } else {
            let big_spec = crate::editor::config::FontSpec {
                path: config.fonts.ui.path.clone(),
                size: config.fonts.big.size,
                options: config.fonts.ui.options.clone(),
                ..Default::default()
            };
            load(&big_spec, &mut ctx)?
        };
        let icon_big = {
            let spec = crate::editor::config::FontSpec {
                path: config.fonts.icon.path.clone(),
                size: config.fonts.icon_big.size,
                options: config.fonts.icon.options.clone(),
                ..Default::default()
            };
            load(&spec, &mut ctx)?
        };
        // Load the Seti icon font for file-type icons in the sidebar.
        let seti = {
            let seti_path = config
                .fonts
                .icon
                .path
                .as_deref()
                .map(|p| {
                    let dir = std::path::Path::new(p)
                        .parent()
                        .unwrap_or(std::path::Path::new("."));
                    dir.join("seti.ttf").to_string_lossy().to_string()
                })
                .unwrap_or_default();
            if std::path::Path::new(&seti_path).exists() {
                let spec = crate::editor::config::FontSpec {
                    path: Some(seti_path),
                    // Seti glyphs are designed small; scale to 150% of UI font
                    // to match VS Code's rendering and fill the sidebar row.
                    size: (config.fonts.ui.size as f64 * 1.5) as u32,
                    options: crate::editor::config::FontOptions {
                        antialiasing: Some("grayscale".into()),
                        hinting: Some("full".into()),
                        ..Default::default()
                    },
                    ..Default::default()
                };
                load(&spec, &mut ctx).unwrap_or(icon)
            } else {
                icon
            }
        };
        (icon, big, icon_big, seti)
    };

    FONT_SLOTS.with(|s| *s.borrow_mut() = Some((ui, code, icon, big, icon_big, seti, h1, h2, h3)));

    Ok(ctx)
}

use std::cell::RefCell;

/// (ui, code, icon, big, icon_big, seti, h1, h2, h3) font slot ids.
type FontSlotIds = (u64, u64, u64, u64, u64, u64, u64, u64, u64);

thread_local! {
    static FONT_SLOTS: RefCell<Option<FontSlotIds>> = const { RefCell::new(None) };
}

/// Build a StyleContext from NativeConfig and loaded fonts.
pub(crate) fn build_style(
    config: &NativeConfig,
    ctx: &crate::editor::draw_context::NativeDrawContext,
) -> StyleContext {
    use crate::editor::types::Color;
    use crate::editor::view::DrawContext as _;

    let (ui, code, icon, big, icon_big, seti, h1, h2, h3) =
        FONT_SLOTS.with(|s| s.borrow().unwrap_or((0, 0, 0, 0, 0, 0, 0, 0, 0)));

    StyleContext {
        font: ui,
        code_font: code,
        icon_font: icon,
        icon_big_font: icon_big,
        big_font: big,
        seti_font: seti,
        h1_font: h1,
        h2_font: h2,
        h3_font: h3,
        font_height: ctx.font_height(ui),
        code_font_height: ctx.font_height(code),
        h1_font_height: ctx.font_height(h1),
        h2_font_height: ctx.font_height(h2),
        h3_font_height: ctx.font_height(h3),
        padding_x: config.ui.padding_x as f64,
        padding_y: config.ui.padding_y as f64,
        divider_size: config.ui.divider_size as f64,
        scrollbar_size: config.ui.scrollbar_size as f64,
        caret_width: config.ui.caret_width as f64,
        tab_width: config.ui.tab_width as f64,
        scale: 1.0,
        background: Color::new(40, 42, 54, 255),
        background2: Color::new(34, 36, 46, 255),
        background3: Color::new(48, 50, 62, 255),
        text: Color::new(215, 218, 224, 255),
        caret: Color::new(147, 161, 255, 255),
        accent: Color::new(97, 175, 239, 255),
        dim: Color::new(114, 120, 138, 255),
        divider: Color::new(24, 26, 34, 255),
        selection: Color::new(72, 79, 100, 255),
        line_number: Color::new(82, 88, 106, 255),
        line_number2: Color::new(147, 161, 255, 255),
        line_highlight: Color::new(44, 47, 59, 255),
        scrollbar: Color::new(72, 79, 100, 255),
        scrollbar2: Color::new(97, 175, 239, 255),
        good: Color::new(80, 200, 120, 255),
        warn: Color::new(255, 212, 121, 255),
        error: Color::new(255, 95, 86, 255),
        nagbar: Color::new(64, 64, 64, 255),
        nagbar_text: Color::new(255, 255, 255, 255),
        nagbar_dim: Color::new(0, 0, 0, 115),
        ..Default::default()
    }
}

}
#[cfg(not(feature = "sdl"))]
fn build_style(_config: &NativeConfig, _ctx: &()) -> StyleContext {
    StyleContext::default()
}

#[cfg(not(feature = "sdl"))]
fn load_fonts(_config: &NativeConfig) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod indent_tests {
    use super::{smart_backspace_span, smart_indent_opens_block};

    #[test]
    fn smart_indent_opens_block_python_colon() {
        assert!(smart_indent_opens_block("for a in sys.argv:"));
        assert!(smart_indent_opens_block("if x > 0:"));
        assert!(smart_indent_opens_block("def foo():"));
    }

    #[test]
    fn smart_indent_opens_block_braces_brackets() {
        assert!(smart_indent_opens_block("fn main() {"));
        assert!(smart_indent_opens_block("let xs = ["));
        assert!(smart_indent_opens_block("println!("));
    }

    #[test]
    fn smart_indent_opens_block_trailing_whitespace() {
        assert!(smart_indent_opens_block("fn main() {   "));
    }

    #[test]
    fn smart_indent_opens_block_ignores_line_comment() {
        assert!(smart_indent_opens_block("if x: # comment"));
        assert!(smart_indent_opens_block("fn() { // comment"));
    }

    #[test]
    fn smart_indent_opens_block_negatives() {
        assert!(!smart_indent_opens_block("print(a)"));
        assert!(!smart_indent_opens_block("let x = 1"));
        assert!(!smart_indent_opens_block(""));
        assert!(!smart_indent_opens_block("    "));
    }

    #[test]
    fn smart_backspace_full_indent_unit() {
        assert_eq!(smart_backspace_span("    ", 5, "soft", 4), Some(4));
    }

    #[test]
    fn smart_backspace_aligns_to_boundary() {
        assert_eq!(smart_backspace_span("      ", 7, "soft", 4), Some(2));
    }

    #[test]
    fn smart_backspace_deeper_indent() {
        assert_eq!(smart_backspace_span("        ", 9, "soft", 4), Some(4));
    }

    #[test]
    fn smart_backspace_two_space_doc() {
        assert_eq!(smart_backspace_span("  ", 3, "soft", 2), Some(2));
        assert_eq!(smart_backspace_span("    ", 5, "soft", 2), Some(2));
    }

    #[test]
    fn smart_backspace_skips_when_text_before() {
        assert_eq!(smart_backspace_span("    a", 6, "soft", 4), None);
        assert_eq!(smart_backspace_span("foo", 4, "soft", 4), None);
    }

    #[test]
    fn smart_backspace_skips_for_hard_tabs() {
        assert_eq!(smart_backspace_span("    ", 5, "hard", 4), None);
    }

    #[test]
    fn smart_backspace_skips_single_space() {
        assert_eq!(smart_backspace_span(" ", 2, "soft", 4), None);
    }

    #[test]
    fn smart_backspace_skips_col_one() {
        assert_eq!(smart_backspace_span("    ", 1, "soft", 4), None);
    }
}

#[cfg(test)]
mod clipboard_tests {
    use super::{append_clipboard_line, insert_clipboard_line};

    #[test]
    fn append_clipboard_line_appends_plain_text() {
        let mut buf = String::from("foo");
        append_clipboard_line(&mut buf, "bar");
        assert_eq!(buf, "foobar");
    }

    #[test]
    fn append_clipboard_line_strips_line_breaks() {
        let mut buf = String::new();
        append_clipboard_line(&mut buf, "a\r\nb\nc");
        assert_eq!(buf, "abc");
    }

    #[test]
    fn append_clipboard_line_keeps_tabs_and_unicode() {
        let mut buf = String::new();
        append_clipboard_line(&mut buf, "a\tπ");
        assert_eq!(buf, "a\tπ");
    }

    #[test]
    fn insert_clipboard_line_inserts_at_caret_and_returns_offset() {
        let mut buf = String::from("ac");
        let caret = insert_clipboard_line(&mut buf, 1, "b");
        assert_eq!(buf, "abc");
        assert_eq!(caret, 2);
    }

    #[test]
    fn insert_clipboard_line_advances_caret_past_multibyte() {
        let mut buf = String::from("xy");
        let caret = insert_clipboard_line(&mut buf, 1, "é");
        assert_eq!(buf, "xéy");
        assert_eq!(caret, 3);
    }

    #[test]
    fn insert_clipboard_line_strips_line_breaks_before_inserting() {
        let mut buf = String::from("[]");
        let caret = insert_clipboard_line(&mut buf, 1, "a\nb");
        assert_eq!(buf, "[ab]");
        assert_eq!(caret, 3);
    }
}

#[cfg(test)]
mod paste_insert_tests {
    use super::insert_text_at_caret;
    use crate::editor::buffer::default_buffer_state;

    #[test]
    fn inserts_single_line_at_caret_and_advances() {
        let mut b = default_buffer_state();
        b.lines = vec!["hello\n".to_string()];
        b.selections = vec![1, 3, 1, 3];
        insert_text_at_caret(&mut b, "XY");
        assert_eq!(b.lines, vec!["heXYllo\n".to_string()]);
        assert_eq!(b.selections, vec![1, 5, 1, 5]);
    }

    #[test]
    fn inserts_multi_line_and_splits_the_row() {
        let mut b = default_buffer_state();
        b.lines = vec!["hello\n".to_string()];
        b.selections = vec![1, 3, 1, 3];
        insert_text_at_caret(&mut b, "A\nB");
        assert_eq!(b.lines, vec!["heA\n".to_string(), "Bllo\n".to_string()]);
        assert_eq!(b.selections, vec![2, 2, 2, 2]);
    }

    #[test]
    fn replaces_the_active_selection() {
        let mut b = default_buffer_state();
        b.lines = vec!["hello\n".to_string()];
        b.selections = vec![1, 1, 1, 6];
        insert_text_at_caret(&mut b, "bye");
        assert_eq!(b.lines, vec!["bye\n".to_string()]);
        assert_eq!(b.selections, vec![1, 4, 1, 4]);
    }
}
