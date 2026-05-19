from PySide6.QtGui import QPainter, QColor
from PySide6.QtWidgets import QFrame

from config.theme import TAB_SELECTED_TEXT


class TabDropContainer(QFrame):
    def __init__(self, drag_mgr):
        super().__init__()
        self._drag_mgr = drag_mgr
        self.setAcceptDrops(True)

    def dragEnterEvent(self, event):
        if event.mimeData().hasFormat("application/x-jereide-tab"):
            event.acceptProposedAction()
            self._drag_mgr.drop_index = self._drag_mgr.get_drop_index(event)
            self._drag_mgr.update_ghost(self._drag_mgr.drop_index)
            self.update()

    def dragMoveEvent(self, event):
        if event.mimeData().hasFormat("application/x-jereide-tab"):
            event.acceptProposedAction()
            new_index = self._drag_mgr.get_drop_index(event)
            if new_index != self._drag_mgr.drop_index:
                self._drag_mgr.drop_index = new_index
                self._drag_mgr.update_ghost(self._drag_mgr.drop_index)
                self.update()

    def dragLeaveEvent(self, event):
        self._drag_mgr.drop_index = -1
        self._drag_mgr.update_ghost(-1)
        self.update()

    def dropEvent(self, event):
        if event.mimeData().hasFormat("application/x-jereide-tab"):
            self._drag_mgr.completed = True
            self._drag_mgr.on_drop(self._drag_mgr.drop_index)
            event.acceptProposedAction()
        self._drag_mgr.drop_index = -1
        self.update()

    def paintEvent(self, event):
        super().paintEvent(event)
        x = self._drag_mgr.get_indicator_x()
        if x <= 0:
            return
        painter = QPainter(self)
        painter.setRenderHint(QPainter.Antialiasing)
        painter.setPen(QColor(TAB_SELECTED_TEXT))
        painter.drawLine(x, 6, x, self.height() - 6)
        painter.end()
