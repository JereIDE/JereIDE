from PySide6.QtWidgets import QPlainTextEdit, QTextEdit
from PySide6.QtCore import Qt, QRect
from PySide6.QtGui import QPainter, QFont, QColor, QTextCursor, QTextFormat
from .lineNumber import LineNumberArea
from utils.syntaxHighlight import PythonSyntaxHighlighter
from utils.autoIndent import AutoIndent
from utils.autoPairing import AutoPairingMixin
from config.theme import EDITOR_FONT_FAMILY, EDITOR_FONT_SIZE, LINE_NUMBER_BG, LINE_NUMBER_TEXT, CURRENT_LINE_BG
from config.theme import SYNTAX_KEYWORD, SYNTAX_STRING, SYNTAX_NUMBER, SYNTAX_COMMENT
from config.theme import SYNTAX_BUILTIN, SYNTAX_DECORATOR, SYNTAX_CLASS_DEF, SYNTAX_FUNCTION_DEF
from config.config_manager import config_manager


class QCodeEditor(QPlainTextEdit, AutoPairingMixin):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.line_number_area = LineNumberArea(self)
        self.auto_indent_enabled = True
        self.line_numbers_enabled = config_manager.get_config_value('editor', 'line_numbers.enabled', True)
        self.init_auto_pairing()

        font = QFont(EDITOR_FONT_FAMILY, EDITOR_FONT_SIZE)
        font.setStyleHint(QFont.Monospace)
        self.setFont(font)

        self.setFrameShape(QPlainTextEdit.NoFrame)
        self._tab_size = config_manager.get_config_value('editor', 'font.tab_size', 4)
        self.setTabStopDistance(self._tab_size * self.fontMetrics().horizontalAdvance(' '))
        self._line_numbers_min_width = config_manager.get_config_value('editor', 'line_numbers.minimum_width', 15)
        self.setLineWrapMode(QPlainTextEdit.NoWrap)

        # Apply Python syntax highlighting
        self.syntax_highlighter_enabled = True
        self.syntax_highlighter = PythonSyntaxHighlighter(self.document())

        self._find_highlights: list[QTextEdit.ExtraSelection] = []

        self.blockCountChanged.connect(self.update_line_number_area_width)
        self.updateRequest.connect(self.update_line_number_area)
        self.cursorPositionChanged.connect(self.highlight_current_line)

        self.update_line_number_area_width(0)
        self.highlight_current_line()

    def set_font_size(self, size: int):
        """Update the editor's font size and recalculate dependent metrics."""
        font = self.font()
        font.setPointSize(size)
        self.setFont(font)
        self.setTabStopDistance(self._tab_size * self.fontMetrics().horizontalAdvance(' '))
        self.update_line_number_area_width(0)

    def set_syntax_highlighting_enabled(self, enabled: bool):
        self.syntax_highlighter_enabled = enabled
        self.syntax_highlighter.setDocument(self.document() if enabled else None)

    def keyPressEvent(self, event):
        auto_indent = AutoIndent(self)
        if self.handle_auto_pairing(event):
            return
        if auto_indent.handle_key_press(event):
            return
        super().keyPressEvent(event)

    def set_line_numbers_enabled(self, enabled: bool):
        self.line_numbers_enabled = enabled
        self.line_number_area.setVisible(enabled)
        self.update_line_number_area_width(0)

    def set_word_wrap(self, enabled: bool):
        from PySide6.QtWidgets import QPlainTextEdit
        if enabled:
            self.setLineWrapMode(QPlainTextEdit.WidgetWidth)
        else:
            wrap_enabled = config_manager.get_config_value('editor', 'word_wrap.enabled', False)
            self.setLineWrapMode(QPlainTextEdit.WidgetWidth if wrap_enabled else QPlainTextEdit.NoWrap)

    def line_number_area_width(self):
        digits = 1
        max_blocks = max(1, self.blockCount())
        while max_blocks >= 10:
            max_blocks //= 10
            digits += 1
        space = self._line_numbers_min_width + self.fontMetrics().horizontalAdvance('9') * digits
        return space

    def update_line_number_area_width(self, _):
        if self.line_numbers_enabled:
            self.setViewportMargins(self.line_number_area_width(), 0, 0, 0)
        else:
            self.setViewportMargins(0, 0, 0, 0)

    def update_line_number_area(self, rect, dy):
        if dy:
            self.line_number_area.scroll(0, dy)
        else:
            self.line_number_area.update(0, rect.y(), self.line_number_area.width(), rect.height())

        if rect.contains(self.viewport().rect()):
            self.update_line_number_area_width(0)

    def resizeEvent(self, event):
        super().resizeEvent(event)
        cr = self.contentsRect()
        self.line_number_area.setGeometry(QRect(cr.left(), cr.top(), self.line_number_area_width(), cr.height()))

    def refresh_extra_selections(self):
        selections = list(self._find_highlights)
        if not self.isReadOnly():
            current = QTextEdit.ExtraSelection()
            current.format.setBackground(QColor(CURRENT_LINE_BG))
            current.format.setProperty(QTextFormat.FullWidthSelection, True)
            current.cursor = self.textCursor()
            current.cursor.clearSelection()
            selections.append(current)
            self.apply_pair_highlighting(selections)
        self.setExtraSelections(selections)

    def set_find_highlights(self, selections: list[QTextEdit.ExtraSelection]):
        self._find_highlights = selections
        self.refresh_extra_selections()

    def highlight_current_line(self):
        self.refresh_extra_selections()

    def lineNumberAreaPaintEvent(self, event):
        painter = QPainter(self.line_number_area)
        painter.fillRect(event.rect(), QColor(LINE_NUMBER_BG))
        painter.setFont(self.font())

        block = self.firstVisibleBlock()
        block_number = block.blockNumber()
        top = round(self.blockBoundingGeometry(block).translated(self.contentOffset()).top())
        bottom = top + round(self.blockBoundingRect(block).height())

        while block.isValid() and top <= event.rect().bottom():
            if block.isVisible() and bottom >= event.rect().top():
                number = str(block_number + 1)
                painter.setPen(QColor(LINE_NUMBER_TEXT))
                painter.drawText(0, top, self.line_number_area.width() - 5, self.fontMetrics().height(),
                              Qt.AlignRight, number)

            block = block.next()
            top = bottom
            bottom = top + round(self.blockBoundingRect(block).height())
            block_number += 1
