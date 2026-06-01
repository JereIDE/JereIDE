import os
import shlex

from PySide6.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QPushButton, QLabel,
    QDialog, QLineEdit, QFormLayout, QDialogButtonBox,
    QListWidget, QListWidgetItem, QMessageBox,
)
from PySide6.QtCore import Signal, Qt, QRectF, QSettings
from PySide6.QtGui import QPainter, QColor, QBrush, QPainterPath

TASKS_KEY = "tasks/list"

DEFAULT_TASKS = [
    {"name": "Run with Python", "command": "python3 $FILE"},
    {"name": "Check syntax", "command": "python3 -m py_compile $FILE"},
    {"name": "Open in Finder", "command": "open -R $FILE"},
]

SUBSTITUTION_HELP = (
    "Variables: $FILE, $DIR, $NAME, $EXT  (leave empty to append file path)"
)


# ---------------------------------------------------------------------------
# Persistence
# ---------------------------------------------------------------------------

def _load_tasks():
    """Read tasks from QSettings, falling back to defaults."""
    settings = QSettings("Jeremy", "JereIDE")
    stored = settings.value(TASKS_KEY)
    if isinstance(stored, list) and len(stored) > 0:
        # Validate structure — discard entries that aren't dicts with keys
        valid = [t for t in stored if isinstance(t, dict) and "name" in t and "command" in t]
        if valid:
            return valid
    return [dict(t) for t in DEFAULT_TASKS]  # return copies


def _save_tasks(tasks):
    """Persist tasks to QSettings."""
    settings = QSettings("Jeremy", "JereIDE")
    settings.setValue(TASKS_KEY, tasks)
    settings.sync()


# ---------------------------------------------------------------------------
# Variable substitution
# ---------------------------------------------------------------------------

def _substitute_variables(command, file_path):
    """Replace $FILE, $DIR, $NAME, $EXT with values from file_path.

    Falls back to appending the escaped file path when no variables are used
    (backward-compatible with the legacy behaviour).
    """
    if not file_path:
        return command

    dir_path = os.path.dirname(file_path)
    base_name = os.path.basename(file_path)
    name, ext = os.path.splitext(base_name)

    variables = {
        "$FILE": shlex.quote(file_path),
        "$DIR": shlex.quote(dir_path),
        "$NAME": shlex.quote(name),
        "$EXT": shlex.quote(ext),
    }

    if any(var in command for var in variables):
        result = command
        for var, val in variables.items():
            result = result.replace(var, val)
        return result

    # Legacy fallback: append escaped path at the end
    escaped_path = shlex.quote(file_path)
    return f"{command} {escaped_path}"


# ---------------------------------------------------------------------------
# Task input sub-dialog (add / edit one task)
# ---------------------------------------------------------------------------

class _TaskInputDialog(QDialog):
    """Small form dialog for entering or editing a single task."""

    def __init__(self, parent=None, name="", command=""):
        super().__init__(parent)
        self.setWindowTitle("Task" if not name else "Edit Task")
        self.setMinimumWidth(400)

        layout = QFormLayout(self)

        self._name_edit = QLineEdit(name)
        self._name_edit.setPlaceholderText("e.g. Run tests")
        layout.addRow("Name:", self._name_edit)

        self._command_edit = QLineEdit(command)
        self._command_edit.setPlaceholderText("e.g. python3 -m pytest $FILE")
        layout.addRow("Command:", self._command_edit)

        hint = QLabel(SUBSTITUTION_HELP)
        hint.setStyleSheet("color: #888; font-size: 10px;")
        layout.addRow(hint)

        buttons = QDialogButtonBox(QDialogButtonBox.Ok | QDialogButtonBox.Cancel)
        buttons.accepted.connect(self.accept)
        buttons.rejected.connect(self.reject)
        layout.addRow(buttons)

    def get_values(self):
        return self._name_edit.text().strip(), self._command_edit.text().strip()


# ---------------------------------------------------------------------------
# Task list editor (full CRUD)
# ---------------------------------------------------------------------------

class TaskEditDialog(QDialog):
    """Dialog for adding, editing and deleting tasks."""

    def __init__(self, tasks, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Manage Tasks")
        self.setMinimumSize(460, 340)
        self._tasks = list(tasks)  # work on a copy
        self._setup_ui()

    def _setup_ui(self):
        layout = QVBoxLayout(self)

        hint = QLabel(SUBSTITUTION_HELP)
        hint.setStyleSheet("color: #888; font-size: 11px;")
        hint.setWordWrap(True)
        layout.addWidget(hint)

        self._list = QListWidget()
        self._list.setAlternatingRowColors(True)
        self._refresh_list()
        layout.addWidget(self._list, 1)

        # Action buttons
        btn_row = QHBoxLayout()
        add_btn = QPushButton("Add")
        edit_btn = QPushButton("Edit")
        delete_btn = QPushButton("Delete")
        for b in (add_btn, edit_btn, delete_btn):
            b.setCursor(Qt.PointingHandCursor)
        btn_row.addWidget(add_btn)
        btn_row.addWidget(edit_btn)
        btn_row.addWidget(delete_btn)
        btn_row.addStretch()
        layout.addLayout(btn_row)

        add_btn.clicked.connect(self._add_task)
        edit_btn.clicked.connect(self._edit_task)
        delete_btn.clicked.connect(self._delete_task)
        self._list.itemDoubleClicked.connect(lambda: self._edit_task())

        # OK / Cancel
        button_box = QDialogButtonBox(QDialogButtonBox.Ok | QDialogButtonBox.Cancel)
        button_box.accepted.connect(self.accept)
        button_box.rejected.connect(self.reject)
        layout.addWidget(button_box)

    def _refresh_list(self):
        self._list.clear()
        for t in self._tasks:
            text = f"{t['name']}  —  {t['command']}"
            item = QListWidgetItem(text)
            item.setData(Qt.UserRole, t)
            self._list.addItem(item)

    def _add_task(self):
        dlg = _TaskInputDialog(self)
        if dlg.exec() == QDialog.Accepted:
            name, command = dlg.get_values()
            if name and command:
                self._tasks.append({"name": name, "command": command})
                self._refresh_list()

    def _edit_task(self):
        current = self._list.currentItem()
        if not current:
            return
        task = current.data(Qt.UserRole)
        dlg = _TaskInputDialog(self, task["name"], task["command"])
        if dlg.exec() == QDialog.Accepted:
            name, command = dlg.get_values()
            if name and command:
                task["name"] = name
                task["command"] = command
                self._refresh_list()

    def _delete_task(self):
        current = self._list.currentItem()
        if not current:
            return
        task = current.data(Qt.UserRole)
        reply = QMessageBox.question(
            self, "Delete Task",
            f'Delete task "{task["name"]}"?',
            QMessageBox.Yes | QMessageBox.No,
        )
        if reply == QMessageBox.Yes:
            self._tasks.remove(task)
            self._refresh_list()

    def get_tasks(self):
        return self._tasks


# ---------------------------------------------------------------------------
# Main task-runner popup
# ---------------------------------------------------------------------------

class TaskDialog(QWidget):
    """Popup dialog showing the list of available tasks to run."""

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

        # Show the current file context
        if self._file_path:
            contextLabel = QLabel(self._file_path)
            contextLabel.setObjectName("taskDialogContext")
            contextLabel.setStyleSheet(
                "color: #999; font-size: 10px; padding-bottom: 4px;"
            )
            contextLabel.setWordWrap(True)
            dialogLayout.addWidget(contextLabel)

        tasks = _load_tasks()
        if not tasks:
            noTasksLabel = QLabel("No tasks configured")
            noTasksLabel.setAlignment(Qt.AlignCenter)
            noTasksLabel.setStyleSheet("color: #aaa; padding: 12px 0;")
            dialogLayout.addWidget(noTasksLabel)
        else:
            for task in tasks:
                btn = QPushButton(task["name"])
                btn.setFocusPolicy(Qt.TabFocus)
                btn.setCursor(Qt.PointingHandCursor)
                btn.setToolTip(task["command"])
                command = task["command"]
                btn.clicked.connect(lambda checked, c=command: self._on_task_selected(c))
                dialogLayout.addWidget(btn)

        dialogLayout.addStretch()

        # Bottom row: Edit tasks + Cancel
        bottomRow = QHBoxLayout()
        editBtn = QPushButton("Edit Tasks...")
        editBtn.setFocusPolicy(Qt.TabFocus)
        editBtn.setCursor(Qt.PointingHandCursor)
        editBtn.clicked.connect(self._open_edit_dialog)
        bottomRow.addWidget(editBtn)

        bottomRow.addStretch()

        cancelButton = QPushButton("Cancel")
        cancelButton.setFocusPolicy(Qt.TabFocus)
        cancelButton.setCursor(Qt.PointingHandCursor)
        cancelButton.clicked.connect(self.close)
        bottomRow.addWidget(cancelButton)
        dialogLayout.addLayout(bottomRow)

    def _on_task_selected(self, command):
        self.runRequested.emit(command, self._file_path or "")
        self.close()

    def _open_edit_dialog(self):
        tasks = _load_tasks()
        dlg = TaskEditDialog(tasks, self.window())
        if dlg.exec() == QDialog.Accepted:
            _save_tasks(dlg.get_tasks())
            # Rebuild the dialog to show updated task list
            self.close()
            # Signal the parent to re-open
            self.runRequested.emit("__rebuild__", "")

    def paintEvent(self, event):
        dialogPainter = QPainter(self)
        dialogPainter.setRenderHint(QPainter.Antialiasing)

        dialogRect = self.rect().adjusted(1, 1, -1, -1)
        roundedRectPath = QPainterPath()
        roundedRectPath.addRoundedRect(QRectF(dialogRect), 12, 12)

        dialogPainter.fillPath(roundedRectPath, QBrush(QColor(255, 255, 255)))
        dialogPainter.setPen(QColor(200, 200, 200))
        dialogPainter.drawPath(roundedRectPath)
