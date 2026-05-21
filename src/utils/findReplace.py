import re
from PySide6.QtWidgets import QMessageBox
from PySide6.QtGui import QTextCursor, QTextDocument


class FindReplace:
    """Find/Replace logic for text editors."""

    def __init__(self, parent_window):
        self.parent = parent_window

    def get_editor(self):
        """Get the currently active editor."""
        idx = self.parent.notebook.GetSelection()
        if 0 <= idx < len(self.parent._tabs_data):
            return self.parent._tabs_data[idx]["editor"]
        return None

    def _build_pattern(self, text: str, flags: dict) -> str:
        """Build regex pattern from search text."""
        if flags.get("regex"):
            return text
        pattern = re.escape(text)
        if flags.get("whole_words"):
            pattern = r"\b" + pattern + r"\b"
        return pattern

    def _find_next(self, text: str, case_sensitive: bool = True, wrap: bool = True) -> tuple:
        """Find next occurrence. Returns (position, length) or (-1, 0)."""
        editor = self.get_editor()
        if not editor or not text:
            return -1, 0

        doc = editor.document()
        cursor = editor.textCursor()
        start = max(cursor.anchor(), cursor.position()) if cursor.hasSelection() else cursor.position()

        find_flags = QTextDocument.FindCaseSensitively if case_sensitive else QTextDocument.FindFlag(0)

        search_cursor = QTextCursor(doc)
        search_cursor.setPosition(start)
        result = doc.find(text, search_cursor, find_flags)

        if result.isNull() and wrap:
            search_cursor = QTextCursor(doc)
            search_cursor.movePosition(QTextCursor.Start)
            result = doc.find(text, search_cursor, find_flags)

        if not result.isNull():
            return result.selectionStart(), len(text)
        return -1, 0

    def on_find_next(self, text: str, case_sensitive: bool):
        """Handle Find Next action."""
        editor = self.get_editor()
        if not editor or not text:
            return

        pos, length = self._find_next(text, case_sensitive, wrap=True)
        if pos != -1:
            cursor = editor.textCursor()
            cursor.setPosition(pos)
            cursor.movePosition(QTextCursor.MoveOperation.Right, QTextCursor.MoveMode.KeepAnchor, length)
            editor.setTextCursor(cursor)
        else:
            QMessageBox.information(self.parent, "Find", f"Cannot find '{text}'")

    def on_replace_one(self, find_text: str, replace_text: str, case_sensitive: bool):
        """Handle Replace one action."""
        editor = self.get_editor()
        if not editor or not find_text:
            return

        cursor = editor.textCursor()
        if cursor.hasSelection():
            selected = cursor.selectedText()
            if not case_sensitive:
                match = selected.lower() == find_text.lower()
            else:
                match = selected == find_text

            if match:
                cursor.insertText(replace_text)
                editor.setTextCursor(cursor)

    def on_replace_all(self, find_text: str, replace_text: str, case_sensitive: bool):
        """Handle Replace All action."""
        editor = self.get_editor()
        if not editor or not find_text:
            return

        cursor = editor.textCursor()
        cursor.movePosition(QTextCursor.Start)
        editor.setTextCursor(cursor)

        find_flags = QTextDocument.FindCaseSensitively if case_sensitive else QTextDocument.FindFlag(0)

        count = 0
        while editor.find(find_text, find_flags):
            cursor = editor.textCursor()
            cursor.insertText(replace_text)
            count += 1

        if count > 0:
            QMessageBox.information(self.parent, "Replace", f"Replaced {count} occurrence(s).")
        else:
            QMessageBox.information(self.parent, "Replace", f"Cannot find '{find_text}'")

    def clear_highlights(self):
        """Clear any find highlights."""
        editor = self.get_editor()
        if editor:
            editor.set_find_highlights([])