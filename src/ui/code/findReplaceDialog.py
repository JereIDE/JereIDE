from PySide6.QtWidgets import QWidget, QVBoxLayout, QHBoxLayout, QLabel, QLineEdit, QPushButton, QCheckBox, QGroupBox, QDialog
from PySide6.QtCore import Qt, Signal
from PySide6.QtGui import QTextCursor


class FindReplaceDialog(QDialog):
    """Native PySide6 Find/Replace dialog with Find Next, Replace, Replace All."""

    findNext = Signal(str, bool)  # text, case_sensitive
    replaceOne = Signal(str, str, bool)  # find_text, replace_text, case_sensitive
    replaceAll = Signal(str, str, bool)  # find_text, replace_text, case_sensitive

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Find / Replace")
        self.setMinimumWidth(400)
        self.setModal(False)
        self._setup_ui()

    def _setup_ui(self):
        layout = QVBoxLayout(self)

        # --- Find Section ---
        find_layout = QHBoxLayout()
        find_label = QLabel("Find:")
        self.find_input = QLineEdit()
        self.find_input.setPlaceholderText("Search text...")
        self.find_input.textChanged.connect(self._on_find_text_changed)
        find_layout.addWidget(find_label)
        find_layout.addWidget(self.find_input, 1)

        layout.addLayout(find_layout)

        # --- Replace Section ---
        replace_layout = QHBoxLayout()
        replace_label = QLabel("Replace:")
        self.replace_input = QLineEdit()
        self.replace_input.setPlaceholderText("Replace with...")
        replace_layout.addWidget(replace_label)
        replace_layout.addWidget(self.replace_input, 1)

        layout.addLayout(replace_layout)

        # --- Options ---
        options_layout = QHBoxLayout()
        self.case_sensitive_cb = QCheckBox("Match Case")
        self.whole_words_cb = QCheckBox("Whole Words")
        self.whole_words_cb.setEnabled(False)
        self.regex_cb = QCheckBox("Regex")
        self.regex_cb.setEnabled(False)
        self.wrap_cb = QCheckBox("Wrap Around")
        self.wrap_cb.setEnabled(False)
        self.wrap_cb.setChecked(True)
        options_layout.addWidget(self.case_sensitive_cb)
        options_layout.addWidget(self.whole_words_cb)
        options_layout.addWidget(self.regex_cb)
        options_layout.addWidget(self.wrap_cb)

        layout.addLayout(options_layout)

        # --- Buttons ---
        button_layout = QHBoxLayout()

        self.find_next_btn = QPushButton("Find Next")
        self.find_next_btn.setDefault(True)
        self.find_next_btn.clicked.connect(self._on_find_next)
        button_layout.addWidget(self.find_next_btn)

        self.replace_one_btn = QPushButton("Replace")
        self.replace_one_btn.clicked.connect(self._on_replace_one)
        button_layout.addWidget(self.replace_one_btn)

        self.replace_all_btn = QPushButton("Replace All")
        self.replace_all_btn.clicked.connect(self._on_replace_all)
        button_layout.addWidget(self.replace_all_btn)

        button_layout.addStretch()

        close_btn = QPushButton("Close")
        close_btn.clicked.connect(self.close)
        button_layout.addWidget(close_btn)

        layout.addLayout(button_layout)

    def _on_find_text_changed(self, text):
        self.find_next_btn.setEnabled(bool(text))

    def _on_find_next(self):
        self.findNext.emit(self.find_input.text(), self.case_sensitive_cb.isChecked())

    def _on_replace_one(self):
        self.replaceOne.emit(
            self.find_input.text(),
            self.replace_input.text(),
            self.case_sensitive_cb.isChecked()
        )

    def _on_replace_all(self):
        self.replaceAll.emit(
            self.find_input.text(),
            self.replace_input.text(),
            self.case_sensitive_cb.isChecked()
        )

    def set_find_text(self, text: str):
        self.find_input.setText(text)
        self.find_input.selectAll()
        self.find_input.setFocus()

    def _get_flags(self) -> dict:
        return {
            "case_sensitive": self.case_sensitive_cb.isChecked(),
            "whole_words": self.whole_words_cb.isChecked(),
            "regex": self.regex_cb.isChecked(),
            "wrap": self.wrap_cb.isChecked(),
        }
