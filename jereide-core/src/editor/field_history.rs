//! Per-field undo/redo history for the editor's dialog text inputs (the
//! find/replace bar, the file picker, project search/replace, the command
//! palette, the sidebar search, and the inline new-file input).
//! Each input owns a `FieldHistory` so the undo/redo shortcuts edit the
//! focused field rather than the document buffer, matching VS Code's input
//! boxes.

use crate::editor::buffer::UNDO_MERGE_TIMEOUT;

/// Number of undo snapshots retained per field. Dialog inputs are short, so a
/// modest cap keeps memory bounded without ever reaching the limit in practice.
const MAX_FIELD_UNDOS: usize = 500;

/// How a field edit groups for undo. Consecutive mergeable edits of the same
/// kind within the merge window collapse into a single undo step.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FieldEdit {
    /// Typed character(s) inserted at the caret.
    Insert,
    /// Backspace or forward-delete of existing text.
    Delete,
    /// Paste, cut, clear, or accepting a suggestion -- always its own step.
    Replace,
}

impl FieldEdit {
    /// Edits that coalesce with an adjacent edit of the same kind.
    fn mergeable(self) -> bool {
        matches!(self, FieldEdit::Insert | FieldEdit::Delete)
    }
}

/// Undo/redo stacks for one single-line dialog text field. Each snapshot pairs
/// the field text with its caret byte offset; fields that have no caret pass
/// the text length and ignore the restored offset.
#[derive(Default)]
pub(crate) struct FieldHistory {
    undo: Vec<(String, usize)>,
    redo: Vec<(String, usize)>,
    last_kind: Option<FieldEdit>,
    last_time: f64,
}

impl FieldHistory {
    /// Record the field's pre-edit state. Call immediately *before* mutating
    /// the field, mirroring the buffer's `push_undo`. Consecutive mergeable
    /// edits of the same kind within `UNDO_MERGE_TIMEOUT` seconds coalesce into
    /// the existing undo step rather than pushing a new snapshot, so a run of
    /// keystrokes is undone in one stroke. `now` is monotonic seconds (pass
    /// `buffer::now_secs()`).
    pub(crate) fn record(&mut self, text: &str, cursor: usize, kind: FieldEdit, now: f64) {
        if kind.mergeable()
            && self.last_kind == Some(kind)
            && (now - self.last_time) < UNDO_MERGE_TIMEOUT
        {
            self.last_time = now;
            return;
        }
        self.undo.push((text.to_string(), cursor));
        self.redo.clear();
        self.last_kind = Some(kind);
        self.last_time = now;
        if self.undo.len() > MAX_FIELD_UNDOS {
            self.undo.remove(0);
        }
    }

    /// Restore the previous snapshot, returning the `(text, cursor)` the caller
    /// should apply to the field; the current `(text, cursor)` moves onto the
    /// redo stack. Returns `None` when there is nothing to undo.
    pub(crate) fn undo(&mut self, text: &str, cursor: usize) -> Option<(String, usize)> {
        let prev = self.undo.pop()?;
        self.redo.push((text.to_string(), cursor));
        self.last_kind = None;
        Some(prev)
    }

    /// Restore the most recently undone snapshot, returning the `(text, cursor)`
    /// the caller should apply. Returns `None` when there is nothing to redo.
    pub(crate) fn redo(&mut self, text: &str, cursor: usize) -> Option<(String, usize)> {
        let next = self.redo.pop()?;
        self.undo.push((text.to_string(), cursor));
        self.last_kind = None;
        Some(next)
    }

    /// Discard all history so the next dialog session starts with empty stacks.
    pub(crate) fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.last_kind = None;
    }
}

#[cfg(test)]
mod tests {
    use super::{FieldEdit, FieldHistory};

    #[test]
    fn undo_at_empty_history_is_noop() {
        let mut h = FieldHistory::default();
        assert_eq!(h.undo("abc", 3), None);
        assert_eq!(h.redo("abc", 3), None);
    }

    #[test]
    fn single_edit_undo_restores_pre_edit_state() {
        let mut h = FieldHistory::default();
        h.record("", 0, FieldEdit::Insert, 0.0);
        // Field is now "a" after the caller inserted.
        assert_eq!(h.undo("a", 1), Some((String::new(), 0)));
    }

    #[test]
    fn undo_then_redo_round_trips() {
        let mut h = FieldHistory::default();
        h.record("", 0, FieldEdit::Insert, 0.0);
        let (t, c) = h.undo("abc", 3).unwrap();
        assert_eq!((t.as_str(), c), ("", 0));
        assert_eq!(h.redo("", 0), Some(("abc".to_string(), 3)));
    }

    #[test]
    fn consecutive_inserts_within_window_merge_into_one_step() {
        let mut h = FieldHistory::default();
        h.record("", 0, FieldEdit::Insert, 0.0);
        h.record("a", 1, FieldEdit::Insert, 0.1);
        h.record("ab", 2, FieldEdit::Insert, 0.2);
        // One undo wipes the whole run.
        assert_eq!(h.undo("abc", 3), Some((String::new(), 0)));
        assert_eq!(h.undo("", 0), None);
    }

    #[test]
    fn insert_after_pause_starts_new_undo_step() {
        let mut h = FieldHistory::default();
        h.record("", 0, FieldEdit::Insert, 0.0);
        h.record("a", 1, FieldEdit::Insert, 0.1);
        // Gap exceeds UNDO_MERGE_TIMEOUT (1.0s).
        h.record("ab", 2, FieldEdit::Insert, 5.0);
        assert_eq!(h.undo("abc", 3), Some(("ab".to_string(), 2)));
        assert_eq!(h.undo("ab", 2), Some((String::new(), 0)));
    }

    #[test]
    fn different_kinds_do_not_merge() {
        let mut h = FieldHistory::default();
        h.record("", 0, FieldEdit::Insert, 0.0);
        h.record("ab", 2, FieldEdit::Delete, 0.1);
        assert_eq!(h.undo("a", 1), Some(("ab".to_string(), 2)));
        assert_eq!(h.undo("ab", 2), Some((String::new(), 0)));
    }

    #[test]
    fn replace_edits_never_merge() {
        let mut h = FieldHistory::default();
        h.record("a", 1, FieldEdit::Replace, 0.0);
        h.record("ab", 2, FieldEdit::Replace, 0.0);
        assert_eq!(h.undo("abc", 3), Some(("ab".to_string(), 2)));
        assert_eq!(h.undo("ab", 2), Some(("a".to_string(), 1)));
    }

    #[test]
    fn new_edit_after_undo_clears_redo() {
        let mut h = FieldHistory::default();
        h.record("", 0, FieldEdit::Insert, 0.0);
        h.undo("a", 1);
        // A fresh edit must invalidate the redo branch.
        h.record("", 0, FieldEdit::Insert, 5.0);
        assert_eq!(h.redo("x", 1), None);
    }

    #[test]
    fn clear_discards_all_history() {
        let mut h = FieldHistory::default();
        h.record("", 0, FieldEdit::Insert, 0.0);
        h.clear();
        assert_eq!(h.undo("a", 1), None);
    }
}
