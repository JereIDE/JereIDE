class FileManager:
    def __init__(self, window):
        self._window = window

    def open_with_dialog(self):
        from PySide6.QtWidgets import QFileDialog, QMessageBox
        path, _ = QFileDialog.getOpenFileName(self._window, "Open File", "", "All Files (*)")
        if not path:
            return None
        try:
            with open(path, 'r', encoding='utf-8') as f:
                return path, f.read()
        except Exception as e:
            QMessageBox.critical(self._window, "Error", f"Could not open file: {e}")
            return None

    def write_or_save_as(self, file_path, content):
        from PySide6.QtWidgets import QFileDialog, QMessageBox
        if not file_path:
            path, _ = QFileDialog.getSaveFileName(
                self._window, "Save File As", "",
                "Text Files (*.txt);;Python Files (*.py);;All Files (*)"
            )
            if not path:
                return None
            file_path = path
        try:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            return file_path
        except Exception as e:
            QMessageBox.critical(self._window, "Error", f"Could not save file: {e}")
            return None
