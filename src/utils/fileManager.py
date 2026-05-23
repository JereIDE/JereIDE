class FileManager:
    def __init__(self, window):
        self._window = window

    def open_with_dialog(self):
        from PySide6.QtWidgets import QFileDialog, QMessageBox
        import os
        path, _ = QFileDialog.getOpenFileName(self._window, "Open File", "", "All Files (*)")
        if not path:
            return None
        try:
            size = os.path.getsize(path)
            if size > 200 * 1024 * 1024:
                QMessageBox.critical(
                    self._window, "File Too Large",
                    f"This file is {size / 1024 / 1024:.1f} MB. Files larger than 200 MB cannot be opened."
                )
                return None
            if size > 100 * 1024 * 1024:
                msg_box = QMessageBox(self._window)
                msg_box.setWindowTitle("Large File")
                msg_box.setIcon(QMessageBox.Question)
                msg_box.setText(
                    f"This file is {size / 1024 / 1024:.1f} MB. "
                    "Opening very large files may cause performance issues."
                )
                open_btn = msg_box.addButton("Open Anyway", QMessageBox.DestructiveRole)
                cancel_btn = msg_box.addButton("Cancel", QMessageBox.AcceptRole)
                msg_box.setDefaultButton(cancel_btn)
                msg_box.setEscapeButton(cancel_btn)
                msg_box.exec()
                if msg_box.clickedButton() != open_btn:
                    return None
            with open(path, 'r', encoding='utf-8') as f:
                return path, f.read()
        except Exception as e:
            QMessageBox.critical(self._window, "Error", f"Could not open file: {e}")
            return None

    def write_or_save_as(self, file_path, content):
        from PySide6.QtWidgets import QFileDialog, QMessageBox
        import os
        if not file_path:
            path, _ = QFileDialog.getSaveFileName(
                self._window, "Save File As", "",
                "Text Files (*.txt);;Python Files (*.py);;All Files (*)"
            )
            if not path:
                return None
            file_path = path
        if os.path.exists(file_path):
            writable = os.access(file_path, os.W_OK)
        else:
            writable = os.access(os.path.dirname(os.path.abspath(file_path)), os.W_OK)
        if not writable:
            QMessageBox.critical(
                self._window, "Permission Denied",
                f"Cannot write to \"{file_path}\". Check file/directory permissions."
            )
            return None
        try:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            return file_path
        except Exception as e:
            QMessageBox.critical(self._window, "Error", f"Could not save file: {e}")
            return None
