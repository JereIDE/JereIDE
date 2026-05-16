import subprocess
from PySide6.QtWidgets import QWidget, QVBoxLayout, QPushButton, QLabel, QHBoxLayout
from PySide6.QtCore import Signal, Qt, QRectF
from PySide6.QtGui import QPainter, QColor, QBrush, QPainterPath

from config.tasks import task_manager


class TaskDialog(QWidget):
    runRequested = Signal(str, str)

    def __init__(self, file_path=None, parent=None):
        super().__init__(parent)
        self._file_path = file_path
        self.setWindowFlags(Qt.FramelessWindowHint | Qt.Tool)
        self.setAttribute(Qt.WA_TranslucentBackground)
        self.setMinimumSize(280, 160)
        self._setup_ui()

    def _setup_ui(self):
        dialogLayout = QVBoxLayout(self)
        dialogLayout.setContentsMargins(16, 16, 16, 12)
        dialogLayout.setSpacing(6)

        titleLabel = QLabel("Run Task")
        titleLabel.setStyleSheet("font-weight: bold; font-size: 13px; color: #333;")
        dialogLayout.addWidget(titleLabel)

        tasks = task_manager.get_tasks()
        if not tasks:
            noTasksLabel = QLabel("No tasks configured")
            noTasksLabel.setStyleSheet("color: #888; font-size: 11px;")
            noTasksLabel.setAlignment(Qt.AlignCenter)
            dialogLayout.addWidget(noTasksLabel)
        else:
            for task in tasks:
                btn = QPushButton(task["name"])
                btn.setFocusPolicy(Qt.NoFocus)
                btn.setCursor(Qt.PointingHandCursor)
                btn.setStyleSheet("""
                    QPushButton {
                        text-align: left;
                        padding: 8px 12px;
                        border: 1px solid #ddd;
                        border-radius: 6px;
                        background: #fff;
                        color: #333;
                        font-size: 12px;
                    }
                    QPushButton:hover {
                        background: #e8f0fe;
                        border-color: #2386FB;
                    }
                """)
                command = task["command"]
                btn.clicked.connect(lambda checked, c=command: self._on_task_selected(c))
                dialogLayout.addWidget(btn)

        dialogLayout.addStretch()

        cancelLayout = QHBoxLayout()
        cancelLayout.addStretch()
        cancelButton = QPushButton("Cancel")
        cancelButton.setFocusPolicy(Qt.NoFocus)
        cancelButton.setCursor(Qt.PointingHandCursor)
        cancelButton.setStyleSheet("""
            QPushButton {
                padding: 6px 20px;
                border: 1px solid #ccc;
                border-radius: 6px;
                background: #f5f5f5;
                color: #555;
                font-size: 12px;
            }
            QPushButton:hover {
                background: #e8e8e8;
            }
        """)
        cancelButton.clicked.connect(self.close)
        cancelLayout.addWidget(cancelButton)
        dialogLayout.addLayout(cancelLayout)

    def _on_task_selected(self, command):
        self.runRequested.emit(command, self._file_path or "")
        self.close()

    def paintEvent(self, event):
        dialogPainter = QPainter(self)
        dialogPainter.setRenderHint(QPainter.Antialiasing)

        dialogRect = self.rect().adjusted(1, 1, -1, -1)
        roundedRectPath = QPainterPath()
        roundedRectPath.addRoundedRect(QRectF(dialogRect), 12, 12)

        dialogPainter.fillPath(roundedRectPath, QBrush(QColor(255, 255, 255)))
        dialogPainter.setPen(QColor(200, 200, 200))
        dialogPainter.drawPath(roundedRectPath)
