from PySide6.QtCore import Qt
from PySide6.QtGui import QTextCursor, QColor
from PySide6.QtWidgets import QTextEdit

from const.theme import PAIR_HIGHLIGHT
from config.config_manager import config_manager


class AutoPairingMixin:
    """Mixin class to add auto-pairing functionality to QPlainTextEdit."""

    def __init__(self):
        """Initialize auto-pairing with configuration."""
        # Load pairs from configuration
        self.PAIRS = config_manager.get_config_value('editor', 'auto_pairing.pairs', {
            '(': ')',
            '[': ']',
            '{': '}',
            '"': '"',
            "'": "'",
        })

    def init_auto_pairing(self):
        """Initialize auto-pairing state. Call from __init__ of the mixin class."""
        # Check if auto-pairing is enabled in configuration
        self.auto_pairing_enabled = config_manager.get_config_value('editor', 'auto_pairing.enabled', True)
        self._highlighted_pair = None
        self.cursorPositionChanged.connect(self._on_pair_cursor_moved)

    def _on_pair_cursor_moved(self):
        if not self.auto_pairing_enabled:
            self._highlighted_pair = None
            return
        self._highlight_pair_at_cursor()

    def handle_auto_pairing(self, event) -> bool:
        """Handle auto-pairing key press. Returns True if handled, False otherwise."""
        key = event.text()
        if self.auto_pairing_enabled and key in self.PAIRS:
            cursor = self.textCursor()
            cursor.insertText(key + self.PAIRS[key])
            cursor.movePosition(QTextCursor.PreviousCharacter, QTextCursor.MoveAnchor, 1)
            self.setTextCursor(cursor)
            self._highlight_pair_at_cursor()
            return True
        return False

    def _highlight_pair_at_cursor(self):
        self._highlighted_pair = None
        cursor = self.textCursor()
        pos = cursor.position()
        text = self.toPlainText()

        if pos < len(text):
            char = text[pos]
            if char in self.PAIRS.values():
                self._find_and_highlight_pair(pos, char)
            elif char in self.PAIRS:
                self._find_opening_pair(pos, char)
        self.highlight_current_line()

    def _find_opening_pair(self, pos, opening_char):
        closing_char = self.PAIRS[opening_char]
        text = self.toPlainText()
        close_pos = text.find(closing_char, pos + 1)

        while close_pos >= 0:
            if self._is_unnested(text, pos, close_pos, closing_char, opening_char):
                break
            close_pos = text.find(closing_char, close_pos + 1)

        if close_pos >= 0:
            self._highlighted_pair = (pos, close_pos)

    def _find_and_highlight_pair(self, pos, closing_char):
        opening_char = None
        for o, c in self.PAIRS.items():
            if c == closing_char:
                opening_char = o
                break

        if not opening_char:
            return

        text = self.toPlainText()
        open_pos = text.rfind(opening_char, 0, pos)

        while open_pos >= 0:
            if self._is_unnested(text, open_pos, pos, closing_char, opening_char):
                break
            open_pos = text.rfind(opening_char, 0, open_pos)

        if open_pos >= 0:
            self._highlighted_pair = (open_pos, pos)

    def _is_unnested(self, text, open_pos, close_pos, closing_char, opening_char):
        stack = []
        for i in range(open_pos + 1, close_pos):
            ch = text[i]
            if ch in self.PAIRS:
                stack.append(ch)
            elif ch in self.PAIRS.values():
                if stack and self.PAIRS.get(stack[-1]) == ch:
                    stack.pop()
        return opening_char not in stack

    def apply_pair_highlighting(self, extra_selections):
        """Apply pair highlighting to extra selections list."""
        if self._highlighted_pair:
            open_pos, close_pos = self._highlighted_pair
            for p in [open_pos, close_pos]:
                cursor = QTextCursor(self.document())
                cursor.setPosition(p)
                cursor.movePosition(QTextCursor.NextCharacter, QTextCursor.KeepAnchor, 1)
                selection = QTextEdit.ExtraSelection()
                selection.format.setBackground(QColor(PAIR_HIGHLIGHT))
                selection.cursor = cursor
                extra_selections.append(selection)
