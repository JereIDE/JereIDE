from PySide6.QtCore import QObject, QEvent


class FocusManager(QObject):
    def __init__(self, parent=None):
        super().__init__(parent)
        self._get_current_editor = None
        self._terminal = None
        self._commandview_focus_target = None
        self._bottom_panel = None

    def setup(self, get_current_editor, terminal, commandview_focus_target, bottom_panel):
        self._get_current_editor = get_current_editor
        self._terminal = terminal
        self._commandview_focus_target = commandview_focus_target
        self._bottom_panel = bottom_panel
        bottom_panel.installEventFilter(self)

    def on_page_changed(self, index):
        if index == 0:
            self._focus_code()
        elif index == 1:
            if self._commandview_focus_target:
                self._commandview_focus_target.setFocus()

    def eventFilter(self, obj, event):
        if obj == self._bottom_panel:
            if event.type() == QEvent.Type.Show:
                if self._terminal:
                    self._terminal.setFocus()
            elif event.type() == QEvent.Type.Hide:
                self._focus_code()
        return super().eventFilter(obj, event)

    def _focus_code(self):
        editor = self._get_current_editor() if self._get_current_editor else None
        if editor:
            editor.setFocus()
