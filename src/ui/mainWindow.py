from PySide6.QtCore import Qt, QUrl
from PySide6.QtGui import QShortcut, QKeySequence, QDesktopServices
from PySide6.QtWidgets import QMainWindow, QWidget, QVBoxLayout

from const.constants import MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT
from ui.menu import MenuBar
from ui.nativeToolbar import attach_native_toolbar
from ui.slidingPanel import SlidingPanel
from ui.code import CodeView
from ui.command import CommandView
from ui.tasks.taskDialog import TaskDialog, _substitute_variables
from ui.aboutDialog import AboutDialog
from utils.focusManager import FocusManager


class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self._native_id = "JereIDEQ_MainWindow"
        self.resize(800, 600)
        self.setMinimumSize(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT)
        self._nativeSegmentedControl = None
        self.fullScreenEnabled = False

        centralContainer = QWidget()
        centralLayout = QVBoxLayout(centralContainer)
        centralLayout.setContentsMargins(0, 0, 0, 0)
        centralLayout.setSpacing(0)

        self.sliding_panel = SlidingPanel()
        centralLayout.addWidget(self.sliding_panel, 1)

        self.code_view = CodeView()
        self.sliding_panel.addPage(self.code_view)

        self.command_view = CommandView()
        self.sliding_panel.addPage(self.command_view)

        self.setCentralWidget(centralContainer)

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

        QShortcut(QKeySequence("Meta+Tab"), self).activated.connect(
            self._switch_to_next_tab
        )
        QShortcut(QKeySequence("Meta+Shift+Tab"), self).activated.connect(
            self._switch_to_prev_tab
        )

        # Font zoom (Ctrl = Cmd on macOS)
        QShortcut(QKeySequence("Ctrl+="), self).activated.connect(self.zoom_in)
        QShortcut(QKeySequence("Ctrl++"), self).activated.connect(self.zoom_in)
        QShortcut(QKeySequence("Ctrl+-"), self).activated.connect(self.zoom_out)
        QShortcut(QKeySequence("Ctrl+0"), self).activated.connect(self.reset_zoom)

    # --- Native toolbar ---

    def _attach_native_toolbar(self):
        originalWindowTitle = self.windowTitle()
        self.setWindowTitle(self._native_id)
        self._native_toolbar_ctrl, nativeSegmentedControl = attach_native_toolbar(
            self._native_id,
            viewCallback=self._on_view_changed,
            runCallback=self._on_run_requested,
            popupCallback=self._on_project_selected
        )
        self._nativeSegmentedControl = nativeSegmentedControl
        self.setWindowTitle(originalWindowTitle)
        self._update_segmented_state()

    def _on_run_requested(self):
        file_path = self.current_file_path
        dialog = TaskDialog(file_path=file_path, parent=self)
        dialog.runRequested.connect(self._execute_task)
        dialog.setWindowTitle("Tasks")
        dialog.show()
        dialog.raise_()
        self._center_dialog(dialog)

    def _center_dialog(self, dialog):
        dialog.adjustSize()
        dialog.setFixedSize(dialog.size())
        mainRect = self.geometry()
        dialog.move(
            mainRect.center().x() - dialog.width() // 2,
            mainRect.center().y() - dialog.height() // 2
        )

    def _execute_task(self, command, file_path):
        # Handle internal __rebuild__ signal (task dialog needs re-opening)
        if command == "__rebuild__":
            self._on_run_requested()
            return
        self.code_view.show_terminal()
        terminal = self.code_view.terminal
        cmd = _substitute_variables(command, file_path) if file_path else command
        terminal.run_command(cmd)

    def _update_segmented_state(self, _count=None):
        if self._nativeSegmentedControl is None:
            return
        self._nativeSegmentedControl.setEnabled_forSegment_(True, 0)
        self._nativeSegmentedControl.setEnabled_forSegment_(True, 1)

    def _on_project_selected(self, title):
        print(f"Switched to {title}")

    def _on_view_changed(self, index):
        self.sliding_panel.slideTo(index)

    def _switch_page(self, index):
        if self._nativeSegmentedControl:
            self._nativeSegmentedControl.setSelectedSegment_(index)
        self.sliding_panel.slideTo(index)

    # --- Menu ---

    def setup_menu(self):
        self.menu_bar = MenuBar(self)
        self.menu_bar.setup()

    @property
    def current_file_path(self):
        _, data = self.code_view._get_current_tab_data()
        return data["file_path"] if data else None

    @property
    def line_numbers_enabled(self):
        return self.code_view.line_numbers_enabled

    def _switch_to_code_view(self):
        if self.sliding_panel.currentIndex() != 0:
            self._switch_page(0)

    def _switch_to_next_tab(self):
        self._switch_to_code_view()
        nb = self.code_view.notebook
        count = nb.GetPageCount()
        if count < 2:
            return
        current = nb.GetSelection()
        next_idx = 0 if current >= count - 1 else current + 1
        nb.SelectTab(next_idx)

    def _switch_to_prev_tab(self):
        self._switch_to_code_view()
        nb = self.code_view.notebook
        count = nb.GetPageCount()
        if count < 2:
            return
        current = nb.GetSelection()
        prev_idx = count - 1 if current <= 0 else current - 1
        nb.SelectTab(prev_idx)

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

    def toggle_line_numbers(self):
        self.code_view.toggle_line_numbers()

    def open_settings(self):
        self.code_view.open_settings()

    def run_task(self):
        self._on_run_requested()

    # --- Font zoom ---

    def zoom_in(self):
        self.code_view.zoom_in()

    def zoom_out(self):
        self.code_view.zoom_out()

    def reset_zoom(self):
        self.code_view.reset_zoom()

    # --- Panel toggles ---

    def show_about(self):
        dialog = AboutDialog(self)
        self._center_dialog(dialog)
        dialog.exec()

    def open_github(self):
        QDesktopServices.openUrl(QUrl("https://github.com/Jeremy-Qian/JereIDE"))

    def toggle_full_screen(self):
        self.fullScreenEnabled = not self.fullScreenEnabled
        if self.fullScreenEnabled:
            self.showFullScreen()
            self.show()
        else:
            self.showNormal()
            self.show()
