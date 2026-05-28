from PySide6.QtCore import Qt, Signal, QPoint
from PySide6.QtGui import QPainter, QColor, QMouseEvent, QPaintEvent, QPolygon
from PySide6.QtWidgets import QWidget

from const.theme import TAB_UNSELECTED_CLOSE_HOVER_BG, TAB_UNSELECTED_TEXT


class TabScrollArrow(QWidget):
    """A scroll arrow button for the tab bar."""

    clicked = Signal(bool)

    def __init__(self, parent: QWidget, left: bool = True):
        super().__init__(parent)
        self.left = left
        self._is_hovered = False
        self.setFixedWidth(20)
        self.setMouseTracking(True)

    def paintEvent(self, event: QPaintEvent) -> None:
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        width = self.width()
        height = self.height()
        center_y = height // 2

        if self._is_hovered:
            painter.setPen(Qt.PenStyle.NoPen)
            painter.setBrush(QColor(TAB_UNSELECTED_CLOSE_HOVER_BG))
            painter.drawRect(0, 0, width, height)

        painter.setPen(QColor(TAB_UNSELECTED_TEXT))
        painter.setBrush(QColor(TAB_UNSELECTED_TEXT))

        if self.left:
            points = [
                (width - 6, center_y - 4),
                (width - 10, center_y),
                (width - 6, center_y + 4),
            ]
        else:
            points = [
                (6, center_y - 4),
                (10, center_y),
                (6, center_y + 4),
            ]
        polygon = QPolygon([QPoint(x, y) for x, y in points])
        painter.drawPolygon(polygon)
        painter.end()

    def mouseMoveEvent(self, event: QMouseEvent) -> None:
        if not self._is_hovered:
            self._is_hovered = True
            self.update()

    def leaveEvent(self, event) -> None:
        if self._is_hovered:
            self._is_hovered = False
            self.update()

    def mousePressEvent(self, event: QMouseEvent) -> None:
        if event.button() == Qt.MouseButton.LeftButton:
            self.clicked.emit(self.left)
