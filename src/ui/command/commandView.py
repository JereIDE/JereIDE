from PySide6.QtCore import Qt
from PySide6.QtWidgets import QWidget, QVBoxLayout, QLabel

from config.theme import WELCOME_TEXT_SECONDARY


class CommandView(QWidget):
    def __init__(self, parent=None):
        super().__init__(parent)
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        placeholder = QLabel("Needs implementation")
        placeholder.setAlignment(Qt.AlignCenter)
        placeholder.setStyleSheet(f"color: {WELCOME_TEXT_SECONDARY}; font-size: 18px;")
        layout.addWidget(placeholder)
