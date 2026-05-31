from PySide6.QtWidgets import (
    QDialog, QVBoxLayout, QHBoxLayout, QLabel,
    QCheckBox, QSpinBox, QPushButton, QGroupBox,
    QFormLayout,
)
from PySide6.QtCore import Qt


class SettingsDialog(QDialog):
    """Settings dialog for editor preferences."""

    def __init__(self, current: dict, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Settings")
        self.setFixedSize(400, 320)
        self._current = current
        self._setup_ui()

    def _setup_ui(self):
        layout = QVBoxLayout(self)
        layout.setSpacing(12)

        # ── Editor section ──
        editor_group = QGroupBox("Editor")
        editor_layout = QFormLayout(editor_group)
        editor_layout.setSpacing(8)

        self._font_spin = QSpinBox()
        self._font_spin.setRange(6, 48)
        self._font_spin.setValue(self._current.get("default_font_size", 11))
        editor_layout.addRow("Default Font Size:", self._font_spin)

        self._syntax_cb = QCheckBox()
        self._syntax_cb.setChecked(self._current.get("syntax_highlighting", True))
        editor_layout.addRow("Syntax Highlighting:", self._syntax_cb)

        self._wrap_cb = QCheckBox()
        self._wrap_cb.setChecked(self._current.get("word_wrap", False))
        editor_layout.addRow("Word Wrap:", self._wrap_cb)

        layout.addWidget(editor_group)

        # ── Typing section ──
        typing_group = QGroupBox("Typing")
        typing_layout = QFormLayout(typing_group)
        typing_layout.setSpacing(8)

        self._auto_indent_cb = QCheckBox()
        self._auto_indent_cb.setChecked(self._current.get("auto_indent", True))
        typing_layout.addRow("Auto Indent:", self._auto_indent_cb)

        self._auto_pair_cb = QCheckBox()
        self._auto_pair_cb.setChecked(self._current.get("auto_pairing", True))
        typing_layout.addRow("Auto Pairing:", self._auto_pair_cb)

        layout.addWidget(typing_group)

        layout.addStretch()

        # ── Buttons ──
        btn_layout = QHBoxLayout()
        btn_layout.addStretch()

        cancel_btn = QPushButton("Cancel")
        cancel_btn.clicked.connect(self.reject)
        btn_layout.addWidget(cancel_btn)

        save_btn = QPushButton("Save")
        save_btn.setDefault(True)
        save_btn.clicked.connect(self.accept)
        btn_layout.addWidget(save_btn)

        layout.addLayout(btn_layout)

    def get_settings(self) -> dict:
        """Return the settings values from the dialog."""
        return {
            "default_font_size": self._font_spin.value(),
            "syntax_highlighting": self._syntax_cb.isChecked(),
            "word_wrap": self._wrap_cb.isChecked(),
            "auto_indent": self._auto_indent_cb.isChecked(),
            "auto_pairing": self._auto_pair_cb.isChecked(),
        }
