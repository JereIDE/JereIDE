from PySide6.QtCore import Qt
from const.theme import EDITOR_TAB_SIZE


class AutoIndent:
    def __init__(self, editor):
        self.editor = editor

    def handle_key_press(self, event):
        """Handle auto-indent when Enter/Return is pressed."""
        if self.editor.auto_indent_enabled and (event.key() == Qt.Key_Return or event.key() == Qt.Key_Enter):
            cursor = self.editor.textCursor()
            block = cursor.block()
            current_line_text = block.text()

            leading_whitespace = ''
            for char in current_line_text:
                if char in ' \t':
                    leading_whitespace += char
                else:
                    break

            stripped = current_line_text.strip()
            if stripped.endswith(':'):
                indent_size = EDITOR_TAB_SIZE
                indent_char = '\t' if '\t' in leading_whitespace else ' '
                leading_whitespace += indent_char * indent_size

            cursor.insertText('\n' + leading_whitespace)
            self.editor.setTextCursor(cursor)
            return True
        return False
