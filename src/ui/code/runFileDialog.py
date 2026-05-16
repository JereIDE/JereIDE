from PySide6.QtWidgets import QWidget, QVBoxLayout, QPushButton, QLabel, QApplication
from PySide6.QtCore import Signal, Qt, QRectF
from PySide6.QtGui import QPainter, QColor, QBrush, QPainterPath
import sys


class RunFileDialog(QWidget):
    """Floating widget that offers options to run the current file."""

    runRequested = Signal(str)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowFlags(Qt.FramelessWindowHint | Qt.Tool)
        self.setAttribute(Qt.WA_TranslucentBackground)
        self.setMinimumSize(300, 180)
        self._setup_ui()

    def _setup_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(20, 20, 20, 20)
        layout.setSpacing(10)

        label = QLabel("Run file")
        layout.addWidget(label)

        self.python_btn = QPushButton("Run file with Python")
        self.python_btn.setFocusPolicy(Qt.NoFocus)
        self.python_btn.clicked.connect(self._on_python)
        layout.addWidget(self.python_btn)

        self.cpp_btn = QPushButton("Run file with C++")
        self.cpp_btn.setFocusPolicy(Qt.NoFocus)
        self.cpp_btn.clicked.connect(self._on_cpp)
        layout.addWidget(self.cpp_btn)

        cancel_btn = QPushButton("Cancel")
        cancel_btn.setFocusPolicy(Qt.NoFocus)
        cancel_btn.clicked.connect(self.close)
        layout.addWidget(cancel_btn, alignment=Qt.AlignCenter)

    def paintEvent(self, event):
        painter = QPainter(self)
        painter.setRenderHint(QPainter.Antialiasing)

        rect = self.rect().adjusted(1, 1, -1, -1)
        path = QPainterPath()
        path.addRoundedRect(QRectF(rect), 12, 12)

        painter.fillPath(path, QBrush(QColor(246, 246, 246)))
        painter.setPen(QColor(180, 180, 180))
        painter.drawPath(path)

    def _on_python(self):
        self.runRequested.emit("python")
        self.close()

    def _on_cpp(self):
        self.runRequested.emit("cpp")
        self.close()


if __name__ == "__main__":
    app = QApplication(sys.argv)
    dialog = RunFileDialog()
    dialog.runRequested.connect(lambda runtime: print(f"Run requested: {runtime}"))
    dialog.show()
    sys.exit(app.exec())
