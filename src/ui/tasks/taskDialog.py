from PySide6.QtWidgets import QWidget, QVBoxLayout, QPushButton, QLabel, QHBoxLayout
from PySide6.QtCore import Signal, Qt, QRectF, QSettings
from PySide6.QtGui import QPainter, QColor, QBrush, QPainterPath

def _load_tasks():
    """Read tasks from QSettings, falling back to a default list."""
    settings = QSettings("Jeremy", "JereIDE")
    stored = settings.value("tasks/list")
    if stored is not None:
        return stored
    return [{"name": "Run with Python", "command": "python3"}]


class TaskDialog(QWidget):
    runRequested = Signal(str, str)

    def __init__(self, file_path=None, parent=None):
        super().__init__(parent)
        self._file_path = file_path
        self.setWindowFlags(Qt.Popup)
        self.setAttribute(Qt.WA_TranslucentBackground)
        self.setMinimumSize(280, 160)
        self._setup_ui()

    def _setup_ui(self):
        dialogLayout = QVBoxLayout(self)
        dialogLayout.setContentsMargins(16, 16, 16, 12)
        dialogLayout.setSpacing(6)

        titleLabel = QLabel("Run Task")
        titleLabel.setObjectName("taskDialogTitle")
        dialogLayout.addWidget(titleLabel)

        tasks = _load_tasks()
        if not tasks:
            noTasksLabel = QLabel("No tasks configured")
            noTasksLabel.setAlignment(Qt.AlignCenter)
            dialogLayout.addWidget(noTasksLabel)
        else:
            for task in tasks:
                btn = QPushButton(task["name"])
                btn.setFocusPolicy(Qt.NoFocus)
                btn.setCursor(Qt.PointingHandCursor)
                command = task["command"]
                btn.clicked.connect(lambda checked, c=command: self._on_task_selected(c))
                dialogLayout.addWidget(btn)

        dialogLayout.addStretch()

        cancelLayout = QHBoxLayout()
        cancelLayout.addStretch()
        cancelButton = QPushButton("Cancel")
        cancelButton.setFocusPolicy(Qt.NoFocus)
        cancelButton.setCursor(Qt.PointingHandCursor)
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
