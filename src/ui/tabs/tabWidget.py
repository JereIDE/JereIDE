from __future__ import annotations

from PySide6.QtCore import Qt, Signal, QRect, QMimeData
from PySide6.QtGui import QPainter, QColor, QMouseEvent, QPaintEvent, QFontMetrics, QDrag
from PySide6.QtWidgets import QWidget

from config.theme import (
    TAB_SELECTED_BG,
    TAB_UNSELECTED_BG,
    TAB_SELECTED_TEXT,
    TAB_UNSELECTED_TEXT,
    TAB_SELECTED_CLOSE_HOVER_BG,
    TAB_UNSELECTED_CLOSE_HOVER_BG,
    TAB_SEPARATOR,
)
from config.config_manager import config_manager


class JereIDETab(QWidget):
    """A single tab widget with a close button."""

    clicked = Signal(int)
    close_clicked = Signal(int)

    def __init__(self, parent: QWidget, label: str, index: int, notebook: "JereIDEBook" = None):
        super().__init__(parent)
        self.label = label
        self.index = index
        self.notebook = notebook
        self.is_selected = False
        self.is_modified = False
        self._is_close_hovered = False
        self._is_tab_hovered = False
        self._text_right = 0

        tab_height = config_manager.get_config_value('theme', 'tabs.height', 30)
        self.setFixedHeight(tab_height)
        self.setMouseTracking(True)
        self._drag_start_pos = None
        self._update_width()

    def _update_width(self):
        fm = QFontMetrics(self.font())
        text_width = fm.horizontalAdvance(self.label)
        self.setMinimumWidth(text_width + 50)

    def set_label(self, label: str, is_modified: bool = False):
        self.label = label
        self.is_modified = is_modified
        self._update_width()
        self.update()

    def set_modified(self, modified: bool):
        self.is_modified = modified
        self.update()

    def _start_drag(self):
        drag = QDrag(self)
        mime = QMimeData()
        mime.setData("application/x-jereide-tab", str(self.index).encode())
        drag.setMimeData(mime)
        pixmap = self.grab()
        drag.setPixmap(pixmap)
        drag.setHotSpot(self._drag_start_pos)
        self._drag_start_pos = None
        drag.exec(Qt.MoveAction)

    @property
    def _close_button_rect(self):
        height = self.height()
        close_y = (height // 2) - 4
        gap = (self.width() - self._text_right) // 2
        return QRect(self._text_right + gap, close_y, 8, 8)

    @property
    def _close_hover_rect(self):
        """Calculate the hover-sensitive area for the close button."""
        rect = self._close_button_rect
        return QRect(rect.x() - 3, rect.y() - 3, rect.width() + 6, rect.height() + 6)

    def paintEvent(self, event: QPaintEvent) -> None:
        painter = QPainter(self)
        width = self.width()
        height = self.height()

        background_color = QColor(TAB_SELECTED_BG) if self.is_selected else QColor(TAB_UNSELECTED_BG)
        painter.fillRect(0, 0, width, height - 1, background_color)

        fm = QFontMetrics(self.font())
        text_width = fm.horizontalAdvance(self.label)
        min_left_padding = 21
        text_x = min_left_padding

        font = self.font()
        if self.is_modified:
            font.setItalic(True)
            painter.setFont(font)
        text_color = QColor(TAB_SELECTED_TEXT) if self.is_selected else QColor(TAB_UNSELECTED_TEXT)
        painter.setPen(text_color)
        display_label = f"{self.label}*" if self.is_modified else self.label
        painter.drawText(text_x, (height // 2) + 4, display_label)

        self._text_right = text_x + text_width

        if self._is_close_hovered:
            hover_rect = self._close_hover_rect
            close_hover_color = TAB_SELECTED_CLOSE_HOVER_BG if self.is_selected else TAB_UNSELECTED_CLOSE_HOVER_BG
            hover_bg = QColor(close_hover_color)
            painter.setBrush(hover_bg)
            painter.setPen(Qt.PenStyle.NoPen)
            painter.drawRoundedRect(hover_rect, 3, 3)

        if self._is_tab_hovered:
            close_rect = self._close_button_rect
            inset = 2
            painter.setPen(text_color)
            painter.drawLine(
                close_rect.x() + inset, close_rect.y() + inset,
                close_rect.x() + close_rect.width() - inset, close_rect.y() + close_rect.height() - inset
            )
            painter.drawLine(
                close_rect.x() + close_rect.width() - inset, close_rect.y() + inset,
                close_rect.x() + inset, close_rect.y() + close_rect.height() - inset
            )

        next_tab = self.notebook._tabs[self.index + 1] if self.notebook and self.index + 1 < len(self.notebook._tabs) else None
        if not self.is_selected:
            painter.setPen(QColor(TAB_SEPARATOR))
            if next_tab and not next_tab.is_selected:
                painter.drawLine(width - 1, 10, width - 1, height - 10)
            elif not next_tab:
                painter.drawLine(width - 1, 10, width - 1, height - 10)

        painter.end()

    def mousePressEvent(self, event: QMouseEvent) -> None:
        if event.button() == Qt.MouseButton.LeftButton:
            if self._close_hover_rect.contains(event.pos()):
                self.close_clicked.emit(self.index)
            else:
                self._drag_start_pos = event.pos()

    def mouseMoveEvent(self, event: QMouseEvent) -> None:
        if event.buttons() & Qt.MouseButton.LeftButton and self._drag_start_pos is not None:
            if (event.pos() - self._drag_start_pos).manhattanLength() >= 5:
                self._start_drag()
                return
        was_close_hovered = self._is_close_hovered
        was_tab_hovered = self._is_tab_hovered
        self._is_close_hovered = self._close_hover_rect.contains(event.pos())
        self._is_tab_hovered = True
        if was_close_hovered != self._is_close_hovered or was_tab_hovered != self._is_tab_hovered:
            self.update()

    def mouseReleaseEvent(self, event: QMouseEvent) -> None:
        if event.button() == Qt.MouseButton.LeftButton and self._drag_start_pos is not None:
            self.clicked.emit(self.index)
            self._drag_start_pos = None
        super().mouseReleaseEvent(event)

    def leaveEvent(self, event) -> None:
        self._is_close_hovered = False
        self._is_tab_hovered = False
        self.update()
