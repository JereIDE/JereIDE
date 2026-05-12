from PySide6.QtWidgets import QFrame, QHBoxLayout, QPushButton
from PySide6.QtCore import QSize
from icons.sfSymbols import get_sf_qicon
from const.theme import STATUS_BAR_BG, STATUS_BAR_HEIGHT


class StatusBar(QFrame):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setFixedHeight(STATUS_BAR_HEIGHT)
        self.setStyleSheet(f"background-color: {STATUS_BAR_BG}; border-top: 1px solid #ccc;")

        layout = QHBoxLayout(self)
        # Dock button – looks like a label (transparent background) with a rectangle.dock SF Symbol
        self._dock_button = QPushButton()
        self._dock_button.setIcon(get_sf_qicon("rectangle.dock", size=16, weight=1))
        self._dock_button.setIconSize(QSize(16, 16))
        self._dock_button.setFixedHeight(STATUS_BAR_HEIGHT - 4)
        self._dock_button.setStyleSheet(
            "QPushButton { background-color: transparent; border: none; }"
        )
        self._dock_button.clicked.connect(self._dummy_dock_action)
        layout.setContentsMargins(5, 0, 5, 0)
        layout.setSpacing(5)

        self._position_button = QPushButton("1:1")
        self._position_button.setFixedHeight(STATUS_BAR_HEIGHT - 4)
        self._position_button.setStyleSheet(
            "QPushButton { background-color: transparent; border: none; "
            "color: #666; font-size: 12px; padding: 0 5px; text-align: left; }"
            "QPushButton:disabled { color: #666; }"
        )

        layout.addWidget(self._position_button)
        layout.addStretch()
        layout.addWidget(self._dock_button)

    def update_position(self, line: int, column: int):
        self._position_button.setText(f"{line}:{column}")

    def _dummy_dock_action(self):
        # Dummy function triggered by the dock button – does nothing substantial
        print("Dock button clicked (dummy action)")
