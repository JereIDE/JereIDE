import re
from PySide6.QtWidgets import QMessageBox
from PySide6.QtGui import QTextCursor


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

        content = editor.toPlainText()
        cursor = editor.textCursor()
        start = cursor.position() + 1 if cursor.hasSelection() else cursor.position()

        if case_sensitive:
            pos = content.find(text, start)
            if pos == -1 and wrap:
                pos = content.find(text, 0)
        else:
            lower_content = content.lower()
            lower_text = text.lower()
            pos = lower_content.find(lower_text, start)
            if pos == -1 and wrap:
                pos = lower_content.find(lower_text, 0)

        if pos != -1:
            return pos, len(text)
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

        content = editor.toPlainText()
        flags = 0 if case_sensitive else re.IGNORECASE
        pattern = self._build_pattern(find_text, {"regex": False, "whole_words": False})

        new_content, count = re.subn(pattern, replace_text, content, flags=flags)
        if count > 0:
            editor.setPlainText(new_content)
            QMessageBox.information(self.parent, "Replace", f"Replaced {count} occurrence(s).")
        else:
            QMessageBox.information(self.parent, "Replace", f"Cannot find '{find_text}'")

    def clear_highlights(self):
        """Clear any find highlights."""
        editor = self.get_editor()
        if editor:
            editor.set_find_highlights([])