from PySide6.QtCore import Qt, QSize, Signal
from PySide6.QtWidgets import (
    QFrame, QLabel, QVBoxLayout, QHBoxLayout, QWidget
)
from PySide6.QtGui import QPixmap, QPainter, QFont
from PySide6.QtSvg import QSvgRenderer
from config.theme import (
    EDITOR_BG, WELCOME_TEXT_PRIMARY,
    WELCOME_TEXT_SECONDARY, WELCOME_DIVIDER
)
from const.paths import LOGO_PATH


class WelcomeFrame(QFrame):
    newFileRequested = Signal()
    openFileRequested = Signal()
    commandPaletteRequested = Signal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setStyleSheet(f"background-color: {EDITOR_BG};")

        main_layout = QVBoxLayout(self)
        main_layout.setContentsMargins(0, 0, 0, 0)
        main_layout.setSpacing(0)

        main_layout.addStretch()

        header_widget = QWidget()
        header_layout = QVBoxLayout(header_widget)
        header_layout.setContentsMargins(0, 0, 0, 0)
        header_layout.setSpacing(20)
        header_layout.setAlignment(Qt.AlignCenter)

        logo_layout = QHBoxLayout()
        logo_layout.setSpacing(15)
        logo_layout.setAlignment(Qt.AlignCenter)

        title_tagline_layout = QVBoxLayout()
        title_tagline_layout.setSpacing(5)
        title_tagline_layout.setAlignment(Qt.AlignCenter)


        self._logo = QLabel()
        pixmap = QPixmap(80, 80)
        pixmap.fill(Qt.GlobalColor.transparent)

        renderer = QSvgRenderer(LOGO_PATH)
        if renderer.isValid():
            painter = QPainter(pixmap)
            painter.setRenderHint(QPainter.RenderHint.Antialiasing)
            renderer.render(painter)
            painter.end()

        self._logo.setPixmap(pixmap)
        self._logo.setFixedSize(QSize(80, 80))

        title_font = QFont()
        title_font.setBold(True)
        title_font.setPointSize(24)
        self._title = QLabel("Welcome back to JereIDE")
        self._title.setFont(title_font)
        self._title.setAlignment(Qt.AlignCenter)
        self._title.setStyleSheet(f"color: {WELCOME_TEXT_PRIMARY};")

        tagline_font = QFont()
        tagline_font.setItalic(True)
        tagline_font.setPointSize(14)
        self._tagline = QLabel("the fast simple code editor")
        self._tagline.setFont(tagline_font)
        self._tagline.setAlignment(Qt.AlignCenter)
        self._tagline.setStyleSheet(f"color: {WELCOME_TEXT_SECONDARY};")

        title_tagline_layout.addWidget(self._title)
        title_tagline_layout.addWidget(self._tagline)

        logo_layout.addWidget(self._logo)
        logo_layout.addLayout(title_tagline_layout)

        header_layout.addLayout(logo_layout)

        actions_widget = QWidget()
        actions_layout = QVBoxLayout(actions_widget)
        actions_layout.setContentsMargins(40, 30, 40, 30)
        actions_layout.setSpacing(12)
        actions_layout.setAlignment(Qt.AlignCenter)

        section_label = QLabel("GET STARTED")
        section_font = QFont()
        section_font.setPointSize(11)
        section_label.setFont(section_font)
        section_label.setStyleSheet(f"color: {WELCOME_TEXT_SECONDARY};")
        section_label.setAlignment(Qt.AlignCenter)

        self._actions = []

        self._add_action(
            "New File", "⌘N", "+", self._on_new_file
        )
        self._add_action(
            "Open File", "⌘O", "folder", self._on_open_file
        )
        self._add_action(
            "Open Command Palette\t  ", "⌘⇧P", "cmd", self._on_command_palette, enabled=False
        )

        actions_layout.addWidget(section_label)
        for action in self._actions:
            actions_layout.addWidget(action)

        main_layout.addWidget(header_widget, 0, Qt.AlignHCenter)
        main_layout.addWidget(actions_widget, 0, Qt.AlignHCenter)

        main_layout.addStretch()

    def _add_action(self, text: str, shortcut: str, icon_type: str, callback, enabled: bool = True):
        action_widget = QWidget()
        action_layout = QHBoxLayout(action_widget)
        action_layout.setContentsMargins(0, 0, 0, 0)
        action_layout.setSpacing(0)

        label = QLabel(f"  {text}")
        action_font = QFont()
        action_font.setPointSize(14)
        label.setFont(action_font)
        label.setStyleSheet(f"color: {WELCOME_TEXT_PRIMARY};")

        shortcut_label = QLabel(shortcut)
        shortcut_label.setFont(action_font)
        shortcut_label.setStyleSheet(f"color: {WELCOME_TEXT_SECONDARY};")
        shortcut_label.setAlignment(Qt.AlignRight)

        action_layout.addWidget(label)
        action_layout.addWidget(shortcut_label)

        if enabled:
            action_widget.setStyleSheet("""
                QWidget {{
                    background-color: transparent;
                    padding: 8px 16px;
                    min-width: 200px;
                }}
                QWidget:hover {{
                    background-color: #F0F0F0;
                }}
            """)
            action_widget.setCursor(Qt.CursorShape.PointingHandCursor)
            action_widget.mousePressEvent = lambda event: callback()
        else:
            label.setStyleSheet(f"color: {WELCOME_TEXT_SECONDARY};")
            action_widget.setStyleSheet("""
                QWidget {{
                    background-color: transparent;
                    padding: 8px 16px;
                    min-width: 200px;
                }}
            """)

        self._actions.append(action_widget)

    def _on_new_file(self):
        self.newFileRequested.emit()

    def _on_open_file(self):
        self.openFileRequested.emit()

    def _on_command_palette(self):
        self.commandPaletteRequested.emit()
