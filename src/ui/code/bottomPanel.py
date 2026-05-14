from PySide6.QtWidgets import QWidget, QHBoxLayout
from PySide6.QtCore import Qt
from config.theme import EDITOR_BG, STATUS_BAR_BG
from .terminalWidget import TerminalWidget


class BottomPanel(QWidget):
    """Collapsible bottom panel (dock) that can be toggled via the status bar button."""
    
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setFixedHeight(150)
        self.setStyleSheet(
            f"QWidget {{ background-color: {EDITOR_BG}; border-top: 1px solid #ccc; }}"
        )
        self.setVisible(False)
        
        layout = QHBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        
        self.terminal = TerminalWidget()
        layout.addWidget(self.terminal)
    
    def toggle(self):
        """Toggle the visibility of the bottom panel."""
        self.setVisible(not self.isVisible())
