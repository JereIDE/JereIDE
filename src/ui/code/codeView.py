import os
from PySide6.QtCore import Qt, Signal
from PySide6.QtWidgets import QApplication, QWidget, QVBoxLayout, QSplitter, QMessageBox
from .codeEditor import QCodeEditor
from ui.tabs import JereIDEBook
from .statusBar import StatusBar
from .bottomPanel import BottomPanel
from .welcomeFrame import WelcomeFrame
from .findReplaceDialog import FindReplaceDialog
from utils.findReplace import FindReplace
from utils.fileManager import FileManager
from config.config_manager import config_manager


class CodeView(QWidget):
    tabCountChanged = Signal(int)
    dockToggled = Signal()
    commandViewRequested = Signal()
    modifiedStateChanged = Signal(bool)

    def __init__(self, parent=None):
        super().__init__(parent)
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(0)

        self._splitter = QSplitter(Qt.Vertical)
        self._splitter.setHandleWidth(1)
        self._splitter.setChildrenCollapsible(False)
        self._splitter.setStyleSheet("QSplitter::handle { background: transparent; }")
        layout.addWidget(self._splitter, 1)

        top_container = QWidget()
        top_layout = QVBoxLayout(top_container)
        top_layout.setContentsMargins(0, 0, 0, 0)
        top_layout.setSpacing(0)

        self._notebook = JereIDEBook(None)
        top_layout.addWidget(self._notebook)
        self._notebook.hide()

        self._welcome_frame = WelcomeFrame()
        top_layout.addWidget(self._welcome_frame)

        self._welcome_frame.newFileRequested.connect(self._create_new_tab)
        self._welcome_frame.openFileRequested.connect(self.open_file)
        self._welcome_frame.commandViewRequested.connect(self.commandViewRequested.emit)

        self._status_bar = StatusBar()
        self._status_bar._dock_button.clicked.connect(self._on_dock_clicked)
        top_layout.addWidget(self._status_bar)

        self._splitter.addWidget(top_container)

        self._bottom_panel = BottomPanel()
        self._splitter.addWidget(self._bottom_panel)
        self._splitter.setSizes([400, 150])

        self._tabs_data = []

        self._file_manager = FileManager(self)
        self._find_replace = FindReplace(self)
        self._find_dialog = None

        self._notebook.page_changed.connect(self.on_tab_changed)
        self._notebook.page_changed.connect(self._on_page_changed_for_cursor)
        self._notebook.page_close_requested.connect(self.on_tab_close_requested)

        self.syntax_highlighting_enabled = True
        self.auto_indent_enabled = True
        self.line_numbers_enabled = True
        self.auto_pairing_enabled = True
        self.wrap_enabled = False

        self._font_size = config_manager.get_config_value('theme', 'editor.font_size', 11)
        self._status_bar.update_font_size(self._font_size)

        self._create_new_tab()

    @property
    def notebook(self):
        return self._notebook

    @property
    def status_bar(self):
        return self._status_bar

    @property
    def current_editor(self):
        idx = self._notebook.GetSelection()
        if 0 <= idx < len(self._tabs_data):
            return self._tabs_data[idx]["editor"]
        return None

    def _on_dock_clicked(self):
        self._bottom_panel.toggle()

    def show_terminal(self):
        if not self._bottom_panel.isVisible():
            self._bottom_panel.setVisible(True)
            QApplication.processEvents()

    @property
    def bottom_panel(self):
        return self._bottom_panel

    @property
    def terminal(self):
        return self._bottom_panel.terminal

    def _create_new_tab(self, title="untitled", file_path=None, content=""):
        if self._notebook.GetPageCount() == 0:
            self._notebook.show()
            self._welcome_frame.hide()

        editor = QCodeEditor()
        self._notebook.AddPage(editor, title)
        self.tabCountChanged.emit(self._notebook.GetPageCount())
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
        idx = self._notebook.GetSelection()
        if 0 <= idx < len(self._tabs_data):
            return idx, self._tabs_data[idx]
        return -1, None

    def _get_tab_index_by_editor(self, editor):
        for i, data in enumerate(self._tabs_data):
            if data["editor"] == editor:
                return i
        return -1

    def on_tab_changed(self, index):
        if 0 <= index < len(self._tabs_data):
            data = self._tabs_data[index]
            is_modified = data["editor"].toPlainText() != data["original_content"]
            self._notebook.SetPageModified(index, is_modified)
            self.modifiedStateChanged.emit(is_modified)

    def on_tab_close_requested(self, index):
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

    def _close_tab(self, index):
        self._notebook.CloseTab(index)
        if 0 <= index < len(self._tabs_data):
            self._tabs_data.pop(index)
        for i in range(len(self._tabs_data)):
            self._notebook.SetPageText(i, self._get_tab_title(i))

        if self._notebook.GetPageCount() == 0:
            self._welcome_frame.show()
            self._notebook.hide()
            self._status_bar.clear_position()
            self.modifiedStateChanged.emit(False)

    def _get_tab_title(self, index):
        if 0 <= index < len(self._tabs_data):
            data = self._tabs_data[index]
            file_path = data["file_path"]
            return os.path.basename(file_path) if file_path else "untitled"
        return "untitled"

    def _save_current_tab(self, index):
        if 0 <= index < len(self._tabs_data):
            data = self._tabs_data[index]
            path = self._file_manager.write_or_save_as(data["file_path"], data["editor"].toPlainText())
            if path:
                data["file_path"] = path
                data["original_content"] = data["editor"].toPlainText()
                self._notebook.SetPageText(index, os.path.basename(path))
                self._notebook.SetPageModified(index, False)

    def _save_as_current_tab(self, index):
        if 0 <= index < len(self._tabs_data):
            data = self._tabs_data[index]
            old_path = data["file_path"]
            data["file_path"] = None
            self._save_current_tab(index)
            if not data["file_path"]:
                data["file_path"] = old_path

    def new_file(self):
        self._create_new_tab()
        idx = self._notebook.GetSelection()
        self.on_tab_changed(idx)

    def open_file(self):
        result = self._file_manager.open_with_dialog()
        if not result:
            return
        file_path, content = result

        for i, data in enumerate(self._tabs_data):
            if data["file_path"] and os.path.normpath(data["file_path"]) == os.path.normpath(file_path):
                self._notebook.SelectTab(i)
                self.on_tab_changed(i)
                return

        self._create_new_tab(os.path.basename(file_path), file_path, content)
        self.on_tab_changed(self._notebook.GetSelection())

    def save_file(self):
        idx = self._notebook.GetSelection()
        if 0 <= idx < len(self._tabs_data):
            self._save_current_tab(idx)
            self.on_tab_changed(idx)

    def save_as_file(self):
        idx = self._notebook.GetSelection()
        if 0 <= idx < len(self._tabs_data):
            self._save_as_current_tab(idx)

    def save_all(self):
        for i in range(len(self._tabs_data)):
            self._save_current_tab(i)
        if self._tabs_data:
            current = self._notebook.GetSelection()
            self.on_tab_changed(current)

    def on_text_changed(self, editor):
        index = self._get_tab_index_by_editor(editor)
        if 0 <= index < len(self._tabs_data):
            data = self._tabs_data[index]
            is_modified = data["editor"].toPlainText() != data["original_content"]
            file_name = os.path.basename(data["file_path"]) if data["file_path"] else "untitled"
            self._notebook.SetPageText(index, file_name)
            self._notebook.SetPageModified(index, is_modified)
            self.modifiedStateChanged.emit(is_modified)

    def undo(self):
        editor = self.current_editor
        if editor:
            editor.undo()

    def redo(self):
        editor = self.current_editor
        if editor:
            editor.redo()

    def cut(self):
        editor = self.current_editor
        if editor:
            editor.cut()

    def copy(self):
        editor = self.current_editor
        if editor:
            editor.copy()

    def paste(self):
        editor = self.current_editor
        if editor:
            editor.paste()

    def select_all(self):
        editor = self.current_editor
        if editor:
            editor.selectAll()

    def find_replace(self):
        if not self.current_editor:
            return

        if self._find_dialog is None:
            self._find_dialog = FindReplaceDialog(self)
            self._find_dialog.findNext.connect(self._find_replace.on_find_next)
            self._find_dialog.replaceOne.connect(self._find_replace.on_replace_one)
            self._find_dialog.replaceAll.connect(self._find_replace.on_replace_all)
            self._find_dialog.finished.connect(self._find_replace.clear_highlights)

        cursor = self.current_editor.textCursor()
        if cursor.hasSelection():
            self._find_dialog.set_find_text(cursor.selectedText())

        self._find_dialog.show()
        self._find_dialog.find_input.setFocus()

    def toggle_auto_indent(self):
        self.auto_indent_enabled = not self.auto_indent_enabled
        editor = self.current_editor
        if editor:
            editor.auto_indent_enabled = self.auto_indent_enabled

    def toggle_line_numbers(self):
        self.line_numbers_enabled = not self.line_numbers_enabled
        editor = self.current_editor
        if editor:
            editor.set_line_numbers_enabled(self.line_numbers_enabled)

    def toggle_auto_pairing(self):
        self.auto_pairing_enabled = not self.auto_pairing_enabled
        editor = self.current_editor
        if editor:
            editor.auto_pairing_enabled = self.auto_pairing_enabled
            self.on_tab_changed(self._notebook.GetSelection())

    def toggle_wrap(self):
        self.wrap_enabled = not self.wrap_enabled
        editor = self.current_editor
        if editor:
            editor.set_word_wrap(self.wrap_enabled)

    def toggle_syntax_highlighting(self):
        self.syntax_highlighting_enabled = not self.syntax_highlighting_enabled
        editor = self.current_editor
        if editor:
            editor.set_syntax_highlighting_enabled(self.syntax_highlighting_enabled)

    # --- Font zoom ---

    def zoom_in(self):
        self._change_font_size(self._font_size + 1)

    def zoom_out(self):
        self._change_font_size(max(6, self._font_size - 1))

    def reset_zoom(self):
        default_size = config_manager.get_config_value('theme', 'editor.font_size', 11)
        self._change_font_size(default_size)

    def _change_font_size(self, new_size: int):
        if new_size == self._font_size:
            return
        self._font_size = new_size
        for data in self._tabs_data:
            data["editor"].set_font_size(new_size)
        self._status_bar.update_font_size(new_size)
        config_manager.update_section('theme', {
            **config_manager.get_section('theme'),
            'editor.font_size': new_size
        })

    def _on_page_changed_for_cursor(self, index):
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
        self._status_bar.update_position(line, col)
