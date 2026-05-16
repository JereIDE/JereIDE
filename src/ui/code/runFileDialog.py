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
        dialogLayout = QVBoxLayout(self)
        dialogLayout.setContentsMargins(20, 20, 20, 20)
        dialogLayout.setSpacing(10)

        titleLabel = QLabel("Run file")
        dialogLayout.addWidget(titleLabel)

        self.pythonButton = QPushButton("Run Python File")
        self.pythonButton.setFocusPolicy(Qt.NoFocus)
        self.pythonButton.clicked.connect(self._on_python)
        dialogLayout.addWidget(self.pythonButton)

        self.cppButton = QPushButton("Run file with C++", enabled=False)
        self.cppButton.setFocusPolicy(Qt.NoFocus)
        self.cppButton.clicked.connect(self._on_cpp)
        dialogLayout.addWidget(self.cppButton)

        cancelButton = QPushButton("Cancel")
        cancelButton.setFocusPolicy(Qt.NoFocus)
        cancelButton.clicked.connect(self.close)
        dialogLayout.addWidget(cancelButton, alignment=Qt.AlignCenter)

    def paintEvent(self, event):
        dialogPainter = QPainter(self)
        dialogPainter.setRenderHint(QPainter.Antialiasing)

        dialogRect = self.rect().adjusted(1, 1, -1, -1)
        roundedRectPath = QPainterPath()
        roundedRectPath.addRoundedRect(QRectF(dialogRect), 12, 12)

        dialogPainter.fillPath(roundedRectPath, QBrush(QColor(246, 246, 246)))
        dialogPainter.setPen(QColor(180, 180, 180))
        dialogPainter.drawPath(roundedRectPath)

    def _on_python(self):
        self.runRequested.emit("python")
        self.close()

    def _on_cpp(self):
        self.runRequested.emit("cpp")
        self.close()


if __name__ == "__main__":
    app = QApplication(sys.argv)
    runFileDialog = RunFileDialog()
    runFileDialog.runRequested.connect(lambda runtime: print(f"Run requested: {runtime}"))
    runFileDialog.show()
    sys.exit(app.exec())
