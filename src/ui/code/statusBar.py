from PySide6.QtWidgets import QFrame, QHBoxLayout, QPushButton, QLabel, QProgressBar
from PySide6.QtCore import QSize
from utils.sfSymbols import get_sf_qicon
from const.theme import STATUS_BAR_BG, STATUS_BAR_HEIGHT


class StatusBar(QFrame):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setFixedHeight(STATUS_BAR_HEIGHT)
        self.setStyleSheet(f"background-color: {STATUS_BAR_BG}; border-top: 1px solid #ccc;")

        layout = QHBoxLayout(self)
        layout.setContentsMargins(5, 0, 5, 0)
        layout.setSpacing(5)

        # --- Left: position indicator ---
        self._position_button = QPushButton("1:1")
        self._position_button.setFixedHeight(STATUS_BAR_HEIGHT - 4)
        self._position_button.setStyleSheet(
            "QPushButton { background-color: transparent; border: none; "
            "color: #666; font-size: 12px; padding: 0 5px; text-align: left; }"
            "QPushButton:disabled { color: #666; }"
        )
        layout.addWidget(self._position_button)

        # --- Center: save-all progress indicator ---
        self._save_progress_label = QLabel()
        self._save_progress_label.setFixedHeight(STATUS_BAR_HEIGHT - 4)
        self._save_progress_label.setStyleSheet(
            "QLabel { background-color: transparent; border: none; "
            "color: #666; font-size: 12px; padding: 0 5px; }"
        )

        self._save_progress_bar = QProgressBar()
        self._save_progress_bar.setFixedHeight(10)
        self._save_progress_bar.setFixedWidth(80)
        self._save_progress_bar.setTextVisible(False)
        self._save_progress_bar.setStyleSheet(
            "QProgressBar { background-color: #e0e0e0; border: none; border-radius: 3px; }"
            "QProgressBar::chunk { background-color: #666; border-radius: 3px; }"
        )

        # Hidden by default
        self._save_progress_label.hide()
        self._save_progress_bar.hide()

        layout.addStretch()            # left stretch pushes group towards center
        layout.addWidget(self._save_progress_label)
        layout.addWidget(self._save_progress_bar)
        layout.addStretch()            # right stretch pushes dock to far right

        # --- Right: dock button ---
        self._dock_button = QPushButton()
        self._dock_button.setIcon(get_sf_qicon("rectangle.dock", size=16, weight=0))
        self._dock_button.setIconSize(QSize(16, 16))
        self._dock_button.setFixedHeight(STATUS_BAR_HEIGHT - 4)
        self._dock_button.setStyleSheet(
            "QPushButton { background-color: transparent; border: none; }"
        )
        layout.addWidget(self._dock_button)

    def show_save_progress(self, current: int, total: int):
        """Show or update the save-all progress indicator."""
        self._save_progress_label.setText(f"Saving {current}/{total}")
        self._save_progress_bar.setMaximum(total)
        self._save_progress_bar.setValue(current)
        self._save_progress_label.show()
        self._save_progress_bar.show()

    def hide_save_progress(self):
        """Hide the save-all progress indicator."""
        self._save_progress_label.hide()
        self._save_progress_bar.hide()

    def update_position(self, line: int, column: int):
        self._position_button.setText(f"{line}:{column}")

    def clear_position(self):
        self._position_button.setText("--:--")
