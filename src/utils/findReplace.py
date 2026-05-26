import re
from PySide6.QtWidgets import QMessageBox
from PySide6.QtGui import QTextCursor, QTextDocument


class FindReplace:
    """Find/Replace logic for text editors."""

    def __init__(self, parent_window):
        self.parent = parent_window

    def get_editor(self):
        """Get the currently active editor."""
        return self.parent.current_editor

    def _build_pattern(self, text: str, flags: dict) -> str:
        """Build regex or plain pattern from search text."""
        if flags.get("regex"):
            return text
        pattern = re.escape(text)
        if flags.get("whole_words"):
            pattern = r"\b" + pattern + r"\b"
        return pattern

    # ── find next ──────────────────────────────────────────────

    def on_find_next(self, text: str, flags: dict):
        """Handle Find Next action."""
        editor = self.get_editor()
        if not editor or not text:
            return

        pos, length = self._find_next(text, flags)
        if pos != -1:
            cursor = editor.textCursor()
            cursor.setPosition(pos)
            cursor.movePosition(QTextCursor.MoveOperation.Right, QTextCursor.MoveMode.KeepAnchor, length)
            editor.setTextCursor(cursor)
        else:
            QMessageBox.information(self.parent, "Find", f"Cannot find '{text}'")

    def _find_next(self, text: str, flags: dict) -> tuple:
        """Find next occurrence. Returns (position, length) or (-1, 0)."""
        editor = self.get_editor()
        if not editor or not text:
            return -1, 0

        if flags.get("regex"):
            return self._find_next_regex(text, flags)

        return self._find_next_plain(text, flags)

    def _find_next_regex(self, pattern: str, flags: dict) -> tuple:
        """Find next occurrence using Python regex."""
        editor = self.get_editor()
        cursor = editor.textCursor()
        doc = editor.document()

        re_flags = re.NOFLAG
        if not flags.get("case_sensitive"):
            re_flags |= re.IGNORECASE

        try:
            compiled = re.compile(pattern, re_flags)
        except re.error:
            return -1, 0

        full_text = doc.toPlainText()
        start_pos = max(cursor.anchor(), cursor.position()) if cursor.hasSelection() else cursor.position()

        for match in compiled.finditer(full_text, start_pos):
            return match.start(), match.end() - match.start()

        if flags.get("wrap", True):
            for match in compiled.finditer(full_text, 0, start_pos):
                return match.start(), match.end() - match.start()

        return -1, 0

    def _find_next_plain(self, text: str, flags: dict) -> tuple:
        """Find next occurrence using plain text search."""
        editor = self.get_editor()
        cursor = editor.textCursor()
        doc = editor.document()

        start = max(cursor.anchor(), cursor.position()) if cursor.hasSelection() else cursor.position()

        find_flags = QTextDocument.FindFlag(0)
        if flags.get("case_sensitive"):
            find_flags |= QTextDocument.FindFlag.FindCaseSensitively
        if flags.get("whole_words"):
            find_flags |= QTextDocument.FindFlag.FindWholeWords

        search_cursor = QTextCursor(doc)
        search_cursor.setPosition(start)
        result = doc.find(text, search_cursor, find_flags)

        if result.isNull() and flags.get("wrap", True):
            search_cursor = QTextCursor(doc)
            search_cursor.movePosition(QTextCursor.MoveOperation.Start)
            result = doc.find(text, search_cursor, find_flags)

        if not result.isNull():
            return result.selectionStart(), len(text)
        return -1, 0

    # ── replace one ────────────────────────────────────────────

    def on_replace_one(self, find_text: str, replace_text: str, flags: dict):
        """Handle Replace one action."""
        editor = self.get_editor()
        if not editor or not find_text:
            return

        cursor = editor.textCursor()
        if not cursor.hasSelection():
            return

        selected = cursor.selectedText()

        if flags.get("regex"):
            re_flags = re.NOFLAG
            if not flags.get("case_sensitive"):
                re_flags |= re.IGNORECASE
            try:
                if re.fullmatch(find_text, selected, re_flags):
                    replacement = re.sub(find_text, replace_text, selected, count=1, flags=re_flags)
                    cursor.insertText(replacement)
                    editor.setTextCursor(cursor)
            except re.error:
                return
        else:
            if flags.get("case_sensitive"):
                match = selected == find_text
            else:
                match = selected.lower() == find_text.lower()

            if match:
                cursor.insertText(replace_text)
                editor.setTextCursor(cursor)

    # ── replace all ────────────────────────────────────────────

    def on_replace_all(self, find_text: str, replace_text: str, flags: dict):
        """Handle Replace All action."""
        editor = self.get_editor()
        if not editor or not find_text:
            return

        if flags.get("regex"):
            self._replace_all_regex(find_text, replace_text, flags)
        else:
            self._replace_all_plain(find_text, replace_text, flags)

    def _replace_all_regex(self, pattern: str, replace_text: str, flags: dict):
        """Replace all matches using regex."""
        editor = self.get_editor()
        doc = editor.document()

        re_flags = re.NOFLAG
        if not flags.get("case_sensitive"):
            re_flags |= re.IGNORECASE

        try:
            compiled = re.compile(pattern, re_flags)
        except re.error:
            return

        full_text = doc.toPlainText()

        matches = list(compiled.finditer(full_text))
        if not matches:
            QMessageBox.information(self.parent, "Replace", f"Cannot find '{pattern}'")
            return

        cursor = editor.textCursor()
        cursor.beginEditBlock()
        for match in reversed(matches):
            cursor.setPosition(match.start())
            cursor.movePosition(QTextCursor.MoveOperation.Right, QTextCursor.MoveMode.KeepAnchor,
                                match.end() - match.start())
            replacement = match.expand(replace_text)
            cursor.insertText(replacement)
        cursor.endEditBlock()

        QMessageBox.information(self.parent, "Replace", f"Replaced {len(matches)} occurrence(s).")

    def _replace_all_plain(self, find_text: str, replace_text: str, flags: dict):
        """Replace all occurrences via plain text search."""
        editor = self.get_editor()
        if not editor or not find_text:
            return

        cursor = editor.textCursor()
        cursor.movePosition(QTextCursor.MoveOperation.Start)
        editor.setTextCursor(cursor)

        find_flags = QTextDocument.FindFlag(0)
        if flags.get("case_sensitive"):
            find_flags |= QTextDocument.FindFlag.FindCaseSensitively
        if flags.get("whole_words"):
            find_flags |= QTextDocument.FindFlag.FindWholeWords

        count = 0
        while editor.find(find_text, find_flags):
            cursor = editor.textCursor()
            cursor.insertText(replace_text)
            count += 1

        if count > 0:
            QMessageBox.information(self.parent, "Replace", f"Replaced {count} occurrence(s).")
        else:
            QMessageBox.information(self.parent, "Replace", f"Cannot find '{find_text}'")

    # ── highlights ─────────────────────────────────────────────

    def clear_highlights(self):
        """Clear any find highlights."""
        editor = self.get_editor()
        if editor:
            editor.set_find_highlights([])
