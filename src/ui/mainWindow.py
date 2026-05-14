import os
from PySide6.QtCore import Qt
from PySide6.QtGui import QShortcut, QKeySequence
from PySide6.QtWidgets import QMainWindow, QWidget, QVBoxLayout, QMessageBox, QLabel
from ui.codeEditor import QCodeEditor
from ui.statusBar import StatusBar
from ui.tabs import JereIDEBook
from ui.menu import MenuBar
from ui.welcomeFrame import WelcomeFrame
from ui.bottomPanel import BottomPanel
from ui.findReplaceDialog import FindReplaceDialog
from utils.findReplace import FindReplace
from ui.nativeToolbar import attach_native_toolbar
from ui.slidingPanel import SlidingPanel
from utils.focusManager import FocusManager
from utils.fileManager import FileManager
from config.theme import WELCOME_TEXT_SECONDARY


class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self._native_id = "JereIDEQ_MainWindow"
        # self.setWindowTitle("JereIDE - untitled")
        # self.setWindowFilePath("")
        self.resize(800, 600)

        self._native_segmented = None

        container = QWidget()
        layout = QVBoxLayout(container)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(0)

        self.sliding_panel = SlidingPanel()
        layout.addWidget(self.sliding_panel, 1)

        codeview = QWidget()
        codeview_layout = QVBoxLayout(codeview)
        codeview_layout.setContentsMargins(0, 0, 0, 0)
        codeview_layout.setSpacing(0)

        self.notebook = JereIDEBook(None)
        codeview_layout.addWidget(self.notebook)
        self.notebook.hide()

        self.welcome_frame = WelcomeFrame()
        codeview_layout.addWidget(self.welcome_frame)

        self.welcome_frame.newFileRequested.connect(self._create_new_tab)
        self.welcome_frame.openFileRequested.connect(self.open_file)

        self.status_bar = StatusBar()
        self.status_bar._dock_button.clicked.connect(self.toggle_bottom_panel)
        codeview_layout.addWidget(self.status_bar)

        self.sliding_panel.addPage(codeview)

        commandview = QWidget()
        commandview_layout = QVBoxLayout(commandview)
        commandview_layout.setContentsMargins(0, 0, 0, 0)
        placeholder = QLabel("Needs implementation")
        placeholder.setAlignment(Qt.AlignCenter)
        placeholder.setStyleSheet(f"color: {WELCOME_TEXT_SECONDARY}; font-size: 18px;")
        commandview_layout.addWidget(placeholder)
        self.sliding_panel.addPage(commandview)

        self.syntax_highlighting_enabled = True
        self.auto_indent_enabled = True
        self.line_numbers_enabled = True
        self.auto_pairing_enabled = True
        self.wrap_enabled = False
        self.full_screen_enabled = False

        self.bottom_panel = BottomPanel()
        layout.addWidget(self.bottom_panel)

        self.setCentralWidget(container)

        self._focus_manager = FocusManager(self)
        self._focus_manager.setup(
            get_current_editor=self._get_current_editor,
            terminal=self.bottom_panel.terminal,
            commandview_focus_target=commandview,
            bottom_panel=self.bottom_panel
        )
        self.sliding_panel.pageChanged.connect(self._focus_manager.on_page_changed)

        self.setup_menu()

        self.notebook.page_changed.connect(self.on_tab_changed)
        self.notebook.page_changed.connect(self._on_page_changed_for_cursor)
        self.notebook.page_close_requested.connect(self.on_tab_close_requested)

        self._tabs_data = []

        self._file_manager = FileManager(self)
        self._find_replace = FindReplace(self)
        self._find_dialog = None

        self._create_new_tab()

        self.winId()
        self._attach_native_toolbar()

        QShortcut(QKeySequence("Shift+Meta+C"), self).activated.connect(
            lambda: self._switch_page(0)
        )
        QShortcut(QKeySequence("Shift+Meta+P"), self).activated.connect(
            lambda: self._switch_page(1)
        )

    def _attach_native_toolbar(self):
        old_title = self.windowTitle()
        self.setWindowTitle(self._native_id)
        self._native_toolbar_ctrl, native_segmented = attach_native_toolbar(self._native_id, self._on_view_changed)
        self._native_segmented = native_segmented
        self.setWindowTitle(old_title)
        self._update_segmented_state()

    def _update_segmented_state(self):
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

    def _create_new_tab(self, title: str = "untitled", file_path: str | None = None, content: str = ""):
        if self.notebook.GetPageCount() == 0:
            self.notebook.show()
            self.welcome_frame.hide()

        editor = QCodeEditor()
        self.notebook.AddPage(editor, title)
        self._update_segmented_state()
        self._tabs_data.append({
            "editor": editor,
            "file_path": file_path,
            "original_content": content
        })
        editor.textChanged.connect(lambda: self.on_text_changed(editor))
        editor.cursorPositionChanged.connect(self._on_cursor_position_changed)
        if content:
            editor.setPlainText(content)

    def _get_current_tab_data(self):
        idx = self.notebook.GetSelection()
        if 0 <= idx < len(self._tabs_data):
            return idx, self._tabs_data[idx]
        return -1, None

    def _get_tab_index_by_editor(self, editor):
        for i, data in enumerate(self._tabs_data):
            if data["editor"] == editor:
                return i
        return -1

    def on_tab_changed(self, index: int):
        if 0 <= index < len(self._tabs_data):
            data = self._tabs_data[index]
            file_path = data["file_path"]
            file_name = os.path.basename(file_path) if file_path else "untitled"
            is_modified = data["editor"].toPlainText() != data["original_content"]
            # title = f"JereIDE - {file_name}{' *' if is_modified else ''}"
            # self.setWindowTitle(title)
            # self.setWindowFilePath(file_path if file_path else "")
            # self.setWindowModified(is_modified)
            self.notebook.SetPageModified(index, is_modified)

    def on_tab_close_requested(self, index: int):
        if 0 <= index < len(self._tabs_data):
            data = self._tabs_data[index]
            is_modified = data["editor"].toPlainText() != data["original_content"]
            if is_modified:
                file_name = os.path.basename(data["file_path"]) if data["file_path"] else "untitled"
                reply = QMessageBox.question(
                    self, "Unsaved Changes",
                    f"Save changes to {file_name}?",
                    QMessageBox.StandardButton.Save |
                    QMessageBox.StandardButton.Discard |
                    QMessageBox.StandardButton.Cancel
                )
                if reply == QMessageBox.StandardButton.Save:
                    self._save_current_tab(index)
                    self._close_tab(index)
                elif reply == QMessageBox.StandardButton.Discard:
                    self._close_tab(index)
            else:
                self._close_tab(index)

    def _close_tab(self, index: int):
        self.notebook.CloseTab(index)
        if 0 <= index < len(self._tabs_data):
            self._tabs_data.pop(index)
        for i in range(len(self._tabs_data)):
            self.notebook.SetPageText(i, self._get_tab_title(i))

        if self.notebook.GetPageCount() == 0:
            self.welcome_frame.show()
            self.notebook.hide()
            self.status_bar.update_position(1, 1)
            # self.setWindowTitle("JereIDE")
            self._update_segmented_state()

    def _get_tab_title(self, index: int):
        if 0 <= index < len(self._tabs_data):
            data = self._tabs_data[index]
            file_path = data["file_path"]
            return os.path.basename(file_path) if file_path else "untitled"
        return "untitled"

    def _save_current_tab(self, index: int):
        if 0 <= index < len(self._tabs_data):
            data = self._tabs_data[index]
            path = self._file_manager.write_or_save_as(data["file_path"], data["editor"].toPlainText())
            if path:
                data["file_path"] = path
                data["original_content"] = data["editor"].toPlainText()
                self.notebook.SetPageText(index, os.path.basename(path))
                self.notebook.SetPageModified(index, False)

    def _save_as_current_tab(self, index: int):
        if 0 <= index < len(self._tabs_data):
            data = self._tabs_data[index]
            old_path = data["file_path"]
            data["file_path"] = None
            self._save_current_tab(index)
            if not data["file_path"]:
                data["file_path"] = old_path

    def setup_menu(self):
        self.menu_bar = MenuBar(self)
        self.menu_bar.setup()

    def new_file(self):
        self._create_new_tab()
        idx = self.notebook.GetSelection()
        self.on_tab_changed(idx)

    def on_text_changed(self, editor):
        index = self._get_tab_index_by_editor(editor)
        if 0 <= index < len(self._tabs_data):
            data = self._tabs_data[index]
            is_modified = data["editor"].toPlainText() != data["original_content"]
            # self.setWindowModified(is_modified)
            file_name = os.path.basename(data["file_path"]) if data["file_path"] else "untitled"
            # title = f"JereIDE - {file_name}{' *' if is_modified else ''}"
            # self.setWindowTitle(title)
            self.notebook.SetPageText(index, file_name)
            self.notebook.SetPageModified(index, is_modified)

    def open_file(self):
        result = self._file_manager.open_with_dialog()
        if not result:
            return
        file_path, content = result
        self._create_new_tab(os.path.basename(file_path), file_path, content)
        self.on_tab_changed(self.notebook.GetSelection())

    def save_file(self):
        idx = self.notebook.GetSelection()
        if 0 <= idx < len(self._tabs_data):
            self._save_current_tab(idx)
            self.on_tab_changed(idx)

    def save_as_file(self):
        idx = self.notebook.GetSelection()
        if 0 <= idx < len(self._tabs_data):
            self._save_as_current_tab(idx)

    def toggle_auto_indent(self):
        self.auto_indent_enabled = self.menu_bar.auto_indent_action.isChecked()
        idx = self.notebook.GetSelection()
        if 0 <= idx < len(self._tabs_data):
            self._tabs_data[idx]["editor"].auto_indent_enabled = self.auto_indent_enabled

    def toggle_line_numbers(self):
        self.line_numbers_enabled = self.menu_bar.line_numbers_action.isChecked()
        idx = self.notebook.GetSelection()
        if 0 <= idx < len(self._tabs_data):
            self._tabs_data[idx]["editor"].set_line_numbers_enabled(self.line_numbers_enabled)

    def toggle_auto_pairing(self):
        self.auto_pairing_enabled = self.menu_bar.auto_pairing_action.isChecked()
        idx = self.notebook.GetSelection()
        if 0 <= idx < len(self._tabs_data):
            self._tabs_data[idx]["editor"].auto_pairing_enabled = self.auto_pairing_enabled
            self.on_tab_changed(idx)

    def toggle_wrap(self):
        self.wrap_enabled = self.menu_bar.wrap_action.isChecked()
        idx = self.notebook.GetSelection()
        if 0 <= idx < len(self._tabs_data):
            self._tabs_data[idx]["editor"].set_word_wrap(self.wrap_enabled)

    def toggle_syntax_highlighting(self):
        self.syntax_highlighting_enabled = self.menu_bar.syntax_highlighting_action.isChecked()
        idx = self.notebook.GetSelection()
        if 0 <= idx < len(self._tabs_data):
            self._tabs_data[idx]["editor"].set_syntax_highlighting_enabled(self.syntax_highlighting_enabled)

    def _get_current_editor(self):
        idx = self.notebook.GetSelection()
        if 0 <= idx < len(self._tabs_data):
            return self._tabs_data[idx]["editor"]
        return None

    def undo(self):
        editor = self._get_current_editor()
        if editor:
            editor.undo()

    def redo(self):
        editor = self._get_current_editor()
        if editor:
            editor.redo()

    def cut(self):
        editor = self._get_current_editor()
        if editor:
            editor.cut()

    def copy(self):
        editor = self._get_current_editor()
        if editor:
            editor.copy()

    def paste(self):
        editor = self._get_current_editor()
        if editor:
            editor.paste()

    def select_all(self):
        editor = self._get_current_editor()
        if editor:
            editor.selectAll()

    def find_replace(self):
        if not self._get_current_editor():
            return

        if self._find_dialog is None:
            self._find_dialog = FindReplaceDialog(self)
            self._find_dialog.findNext.connect(self._find_replace.on_find_next)
            self._find_dialog.replaceOne.connect(self._find_replace.on_replace_one)
            self._find_dialog.replaceAll.connect(self._find_replace.on_replace_all)
            self._find_dialog.finished.connect(self._find_replace.clear_highlights)

        cursor = self._get_current_editor().textCursor()
        if cursor.hasSelection():
            self._find_dialog.set_find_text(cursor.selectedText())

        self._find_dialog.show()
        self._find_dialog.find_input.setFocus()

    def _on_page_changed_for_cursor(self, index: int):
        if 0 <= index < len(self._tabs_data):
            editor = self._tabs_data[index]["editor"]
            self._update_cursor_position(editor)

    def _on_cursor_position_changed(self):
        editor = self.sender()
        if editor:
            self._update_cursor_position(editor)

    def _update_cursor_position(self, editor):
        cursor = editor.textCursor()
        line = cursor.blockNumber() + 1
        col = cursor.columnNumber() + 1
        self.status_bar.update_position(line, col)

    def toggle_bottom_panel(self):
        """Toggle the bottom panel (dock) visibility."""
        self.bottom_panel.toggle()

    def toggle_full_screen(self):
        self.full_screen_enabled = not self.full_screen_enabled

        if self.full_screen_enabled:
            self.showFullScreen()
            self.show()
        else:
            self.showNormal()
            self.show()
