from PySide6.QtCore import Qt
from PySide6.QtGui import QShortcut, QKeySequence
from PySide6.QtWidgets import QMainWindow, QWidget, QVBoxLayout

from const.constants import MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT
from ui.menu import MenuBar
from ui.nativeToolbar import attach_native_toolbar
from ui.slidingPanel import SlidingPanel
from ui.code import CodeView
from ui.code.runFileDialog import RunFileDialog
from ui.command import CommandView
from utils.focusManager import FocusManager


class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self._native_id = "JereIDEQ_MainWindow"
        self.resize(800, 600)
        self.setMinimumSize(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT)
        self._native_segmented = None
        self.full_screen_enabled = False
        self._run_file_dialog = RunFileDialog(self)
        self._run_file_dialog.runRequested.connect(self._on_run_requested)

        container = QWidget()
        layout = QVBoxLayout(container)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(0)

        self.sliding_panel = SlidingPanel()
        layout.addWidget(self.sliding_panel, 1)

        self.code_view = CodeView()
        self.sliding_panel.addPage(self.code_view)

        self.command_view = CommandView()
        self.sliding_panel.addPage(self.command_view)

        self.setCentralWidget(container)

        self._focus_manager = FocusManager(self)
        self._focus_manager.setup(
            get_current_editor=lambda: self.code_view.current_editor,
            terminal=self.code_view.terminal,
            commandview_focus_target=self.command_view,
            bottom_panel=self.code_view.bottom_panel
        )
        self.sliding_panel.pageChanged.connect(self._focus_manager.on_page_changed)

        self.code_view.tabCountChanged.connect(self._update_segmented_state)
        self.code_view.commandViewRequested.connect(lambda: self._switch_page(1))
        self.code_view.modifiedStateChanged.connect(self.setWindowModified)

        self.setup_menu()

        self.winId()
        self._attach_native_toolbar()

        QShortcut(QKeySequence("Shift+Ctrl+C"), self).activated.connect(
            lambda: self._switch_page(0)
        )
        QShortcut(QKeySequence("Shift+Ctrl+P"), self).activated.connect(
            lambda: self._switch_page(1)
        )

    # --- Native toolbar ---

    def _attach_native_toolbar(self):
        old_title = self.windowTitle()
        self.setWindowTitle(self._native_id)
        self._native_toolbar_ctrl, native_segmented = attach_native_toolbar(
            self._native_id,
            callback=self._on_view_changed,
            run_callback=self._show_run_dialog
        )
        self._native_segmented = native_segmented
        self.setWindowTitle(old_title)
        self._update_segmented_state()

    def _update_segmented_state(self, _count=None):
        if self._native_segmented is None:
            return
        self._native_segmented.setEnabled_forSegment_(True, 0)
        self._native_segmented.setEnabled_forSegment_(True, 1)

    def _on_view_changed(self, index):
        self.sliding_panel.slideTo(index)

    def _switch_page(self, index):
        if self._native_segmented:
            self._native_segmented.setSelectedSegment_(index)
        self.sliding_panel.slideTo(index)

    # --- Menu ---

    def setup_menu(self):
        self.menu_bar = MenuBar(self)
        self.menu_bar.setup()

    @property
    def syntax_highlighting_enabled(self):
        return self.code_view.syntax_highlighting_enabled

    @property
    def auto_indent_enabled(self):
        return self.code_view.auto_indent_enabled

    @property
    def line_numbers_enabled(self):
        return self.code_view.line_numbers_enabled

    @property
    def auto_pairing_enabled(self):
        return self.code_view.auto_pairing_enabled

    @property
    def wrap_enabled(self):
        return self.code_view.wrap_enabled

    def _switch_to_code_view(self):
        if self.sliding_panel.currentIndex() != 0:
            self._switch_page(0)

    def new_file(self):
        self._switch_to_code_view()
        self.code_view.new_file()

    def open_file(self):
        self._switch_to_code_view()
        self.code_view.open_file()

    def save_file(self):
        self._switch_to_code_view()
        self.code_view.save_file()

    def save_as_file(self):
        self._switch_to_code_view()
        self.code_view.save_as_file()

    def save_all(self):
        self._switch_to_code_view()
        self.code_view.save_all()

    def undo(self):
        self.code_view.undo()

    def redo(self):
        self.code_view.redo()

    def cut(self):
        self.code_view.cut()

    def copy(self):
        self.code_view.copy()

    def paste(self):
        self.code_view.paste()

    def select_all(self):
        self.code_view.select_all()

    def find_replace(self):
        self.code_view.find_replace()

    def toggle_syntax_highlighting(self):
        self.code_view.toggle_syntax_highlighting()

    def toggle_auto_indent(self):
        self.code_view.toggle_auto_indent()

    def toggle_line_numbers(self):
        self.code_view.toggle_line_numbers()

    def toggle_auto_pairing(self):
        self.code_view.toggle_auto_pairing()

    def toggle_wrap(self):
        self.code_view.toggle_wrap()

    # --- Panel toggles ---

    def toggle_full_screen(self):
        self.full_screen_enabled = not self.full_screen_enabled
        if self.full_screen_enabled:
            self.showFullScreen()
            self.show()
        else:
            self.showNormal()
            self.show()

    def _show_run_dialog(self):
        self._run_file_dialog.show()
        self._run_file_dialog.raise_()
        self._run_file_dialog.activateWindow()

    def _on_run_requested(self, runtime: str):
        print(f"Run requested: {runtime}")
