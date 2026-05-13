from PySide6.QtWidgets import QScrollArea, QWidget, QHBoxLayout, QFrame
from PySide6.QtCore import Qt, Signal, QPropertyAnimation, QEasingCurve


class SlidingPanel(QScrollArea):
    pageChanged = Signal(int)

    def __init__(self, parent=None):
        super().__init__(parent)
        self._pages: list[QWidget] = []
        self._current_index = 0
        self._animation = QPropertyAnimation()

        self.setWidgetResizable(True)
        self.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self.setVerticalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self.setFrameShape(QFrame.Shape.NoFrame)

        self._container = QWidget()
        self._layout = QHBoxLayout(self._container)
        self._layout.setContentsMargins(0, 0, 0, 0)
        self._layout.setSpacing(0)
        self.setWidget(self._container)

        self._animation.setTargetObject(self.horizontalScrollBar())
        self._animation.setPropertyName(b"value")
        self._animation.setDuration(300)
        self._animation.setEasingCurve(QEasingCurve.Type.OutCubic)
        self._animation.finished.connect(self._on_animation_finished)

    def addPage(self, widget: QWidget):
        self._pages.append(widget)
        self._layout.addWidget(widget)
        vp_width = self.viewport().width()
        if vp_width > 0 and len(self._pages) == 1:
            widget.setFixedWidth(vp_width)

    def currentIndex(self) -> int:
        return self._current_index

    def slideTo(self, index: int):
        if index < 0 or index >= len(self._pages):
            return
        if index == self._current_index and self._animation.state() != QPropertyAnimation.State.Running:
            return

        self._current_index = index
        target = index * self.viewport().width()

        self._animation.stop()
        self._animation.setStartValue(self.horizontalScrollBar().value())
        self._animation.setEndValue(target)
        self._animation.start()

    def resizeEvent(self, event):
        super().resizeEvent(event)
        width = event.size().width()
        height = event.size().height()
        page_count = max(len(self._pages), 1)
        for page in self._pages:
            page.setFixedWidth(width)
        self._container.setMinimumWidth(width * page_count)
        self._container.setMinimumHeight(height)
        scroll_bar = self.horizontalScrollBar()
        scroll_bar.setValue(self._current_index * width)

    def _on_animation_finished(self):
        self.pageChanged.emit(self._current_index)
