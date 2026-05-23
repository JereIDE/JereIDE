import sys
import os
import pty
import struct
import fcntl
import termios
import signal

import pyte
from pyte.screens import HistoryScreen

_orig_sgr = pyte.screens.Screen.select_graphic_rendition
def _sgr_safe(self, *attrs, **kwargs):
    return _orig_sgr(self, *attrs)
pyte.screens.Screen.select_graphic_rendition = _sgr_safe



from PySide6.QtWidgets import QApplication, QPlainTextEdit
from PySide6.QtCore import QSocketNotifier, Qt, QTimer
from PySide6.QtGui import (
    QColor, QFont, QFontMetrics, QKeyEvent, QResizeEvent,
    QTextCharFormat, QTextCursor,
)

NAMED_COLORS = {
    'black': QColor(0, 0, 0),
    'red': QColor(170, 0, 0),
    'green': QColor(0, 170, 0),
    'brown': QColor(170, 85, 0),
    'blue': QColor(0, 0, 170),
    'magenta': QColor(170, 0, 170),
    'cyan': QColor(0, 170, 170),
    'white': QColor(170, 170, 170),
    'brightblack': QColor(85, 85, 85),
    'brightred': QColor(255, 85, 85),
    'brightgreen': QColor(85, 255, 85),
    'brightbrown': QColor(255, 255, 85),
    'brightblue': QColor(85, 85, 255),
    'brightmagenta': QColor(255, 85, 255),
    'brightcyan': QColor(85, 255, 255),
    'brightwhite': QColor(255, 255, 255),
}


class TerminalWidget(QPlainTextEdit):
    MAX_HISTORY = 5000

    _KEY_MAP = {
        Qt.Key_Return: b'\r',
        Qt.Key_Backspace: b'\x7f',
        Qt.Key_Tab: b'\t',
        Qt.Key_Up: b'\x1b[A',
        Qt.Key_Down: b'\x1b[B',
        Qt.Key_Right: b'\x1b[C',
        Qt.Key_Left: b'\x1b[D',
        Qt.Key_Home: b'\x1b[H',
        Qt.Key_End: b'\x1b[F',
        Qt.Key_PageUp: b'\x1b[5~',
        Qt.Key_PageDown: b'\x1b[6~',
        Qt.Key_Delete: b'\x1b[3~',
        Qt.Key_Escape: b'\x1b',
        Qt.Key_F1: b'\x1bOP',
        Qt.Key_F2: b'\x1bOQ',
        Qt.Key_F3: b'\x1bOR',
        Qt.Key_F4: b'\x1bOS',
        Qt.Key_F5: b'\x1b[15~',
        Qt.Key_F6: b'\x1b[17~',
        Qt.Key_F7: b'\x1b[18~',
        Qt.Key_F8: b'\x1b[19~',
        Qt.Key_F9: b'\x1b[20~',
        Qt.Key_F10: b'\x1b[21~',
        Qt.Key_F11: b'\x1b[23~',
        Qt.Key_F12: b'\x1b[24~',
        Qt.Key_Insert: b'\x1b[2~',
    }

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setReadOnly(True)
        self.setFrameStyle(0)
        self.setHorizontalScrollBarPolicy(Qt.ScrollBarAlwaysOff)
        self.setVerticalScrollBarPolicy(Qt.ScrollBarAlwaysOn)

        self.terminal_font = QFont("Menlo", 14)
        self.terminal_font.setStyleHint(QFont.Monospace)
        self.setFont(self.terminal_font)
        self.setLineWrapMode(QPlainTextEdit.NoWrap)
        self.verticalScrollBar().valueChanged.connect(self._on_scroll)

        self.columns = 80
        self.lines = 24
        self.screen = HistoryScreen(self.columns, self.lines, history=self.MAX_HISTORY)
        self.screen.set_mode(pyte.modes.LNM)
        self.stream = pyte.Stream(self.screen)

        self._cursor_visible = True
        self._cursor_timer = QTimer(self)
        self._cursor_timer.timeout.connect(self._toggle_cursor)
        self._cursor_timer.start(500)

        self._dirty = False
        self._render_timer = QTimer(self)
        self._render_timer.setSingleShot(True)
        self._render_timer.timeout.connect(self._render)
        self._at_bottom = True

        self._pending_rows = None
        self._pending_cols = None

        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            os.close(self.fd)
            os.environ['TERM'] = 'xterm-256color'
            try:
                os.execve('/bin/zsh', ['/bin/zsh'], os.environ)
            except FileNotFoundError:
                os.execve('/bin/bash', ['/bin/bash'], os.environ)
            os._exit(1)
        else:
            self._update_size()
            self._apply_pending_resize()
            self.notifier = QSocketNotifier(self.fd, QSocketNotifier.Read)
            self.notifier.activated.connect(self._read_data)

    def _read_data(self):
        try:
            data = os.read(self.fd, 65536)
            if not data:
                self.notifier.setEnabled(False)
                return
            self.stream.feed(data.decode('utf-8', errors='replace'))
            self._dirty = True
            self._render_timer.start(0)
        except OSError:
            self.notifier.setEnabled(False)

    def _invalidate(self):
        if not self._dirty:
            self._dirty = True
            self._render_timer.start(0)


    def _toggle_cursor(self):
        self._cursor_visible = not self._cursor_visible
        self._invalidate()

    def _on_scroll(self):
        sb = self.verticalScrollBar()
        self._at_bottom = sb.value() >= sb.maximum() - 2

    def _update_size(self):
        vr = self.viewport().rect()
        if vr.width() < 2 or vr.height() < 2:
            return

        fm = QFontMetrics(self.terminal_font)
        cw = max(1, fm.horizontalAdvance('W'))
        ch = max(1, fm.height())
        cols = max(1, vr.width() // cw)
        rows = max(1, vr.height() // ch)

        if cols != self.columns or rows != self.lines:
            self._pending_cols = cols
            self._pending_rows = rows

    def _apply_pending_resize(self):
        if self._pending_rows is None:
            return
        rows = self._pending_rows
        cols = self._pending_cols
        self.columns = cols
        self.lines = rows
        self.screen.resize(rows, cols)
        self._invalidate()
        buf = struct.pack('HHHH', rows, cols, 0, 0)
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, buf)
        self._pending_rows = None
        self._pending_cols = None

    def _resolve_color(self, color_str):
        if not color_str or color_str == 'default':
            return None
        if color_str in NAMED_COLORS:
            return NAMED_COLORS[color_str]
        if len(color_str) == 6:
            try:
                return QColor(
                    int(color_str[0:2], 16),
                    int(color_str[2:4], 16),
                    int(color_str[4:6], 16),
                )
            except ValueError:
                pass
        return None

    def _make_format(self, pyte_char=None):
        fmt = QTextCharFormat()
        fmt.setFont(self.terminal_font)

        if pyte_char is None:
            fmt.setForeground(QColor(0, 0, 0))
            fmt.setBackground(QColor(255, 255, 255))
            return fmt

        if pyte_char.bold:
            fmt.setFontWeight(QFont.Bold)

        fg = self._resolve_color(pyte_char.fg)
        fmt.setForeground(fg or QColor(0, 0, 0))

        bg = self._resolve_color(pyte_char.bg)
        fmt.setBackground(bg or QColor(255, 255, 255))

        if pyte_char.underscore:
            fmt.setUnderlineStyle(QTextCharFormat.SingleUnderline)
            fmt.setFontUnderline(True)

        if pyte_char.strikethrough:
            fmt.setFontStrikeOut(True)

        return fmt

    def _render(self):
        if not self._dirty:
            return
        self._dirty = False
        was_at_bottom = self._at_bottom

        doc = self.document()
        cursor = QTextCursor(doc)
        cursor.beginEditBlock()
        cursor.select(QTextCursor.Document)
        cursor.removeSelectedText()

        default_fmt = self._make_format()

        history = list(self.screen.history.top)

        for row_idx, row in enumerate(history):
            if row_idx > 0:
                cursor.insertText('\n', default_fmt)
            for x in range(self.screen.columns):
                char = row.get(x)
                if char and char.data:
                    data = char.data
                    fmt = self._make_format(char)
                else:
                    data = ' '
                    fmt = QTextCharFormat(default_fmt)
                cursor.insertText(data, fmt)

        if history:
            cursor.insertText('\n', default_fmt)

        buf = self.screen.buffer
        cy = self.screen.cursor.y
        cx = self.screen.cursor.x

        for y in range(self.screen.lines):
            if y > 0:
                cursor.insertText('\n', default_fmt)
            for x in range(self.screen.columns):
                char = buf[y][x]
                if char and char.data:
                    data = char.data
                    fmt = self._make_format(char)
                else:
                    data = ' '
                    fmt = QTextCharFormat(default_fmt)

                if self._cursor_visible and y == cy and x == cx:
                    fg = fmt.foreground().color()
                    bg = fmt.background().color()
                    cf = QTextCharFormat(fmt)
                    cf.setForeground(bg)
                    cf.setBackground(fg)
                    cursor.insertText(data, cf)
                else:
                    cursor.insertText(data, fmt)

        cursor.endEditBlock()

        if was_at_bottom:
            hist_len = len(history)
            target_line = hist_len + cy
            doc_line_count = self.document().lineCount()
            if target_line >= doc_line_count:
                target_line = max(0, doc_line_count - 1)
            c = self.textCursor()
            c.movePosition(QTextCursor.Start)
            c.movePosition(QTextCursor.Down, QTextCursor.MoveAnchor, target_line)
            c.movePosition(QTextCursor.Right, QTextCursor.MoveAnchor, min(cx, len(self.document().findBlockByLineNumber(target_line).text())))
            self.setTextCursor(c)
            self.ensureCursorVisible()

    def resizeEvent(self, event: QResizeEvent):
        super().resizeEvent(event)
        self._update_size()

    def keyPressEvent(self, event: QKeyEvent):
        self._apply_pending_resize()

        self._cursor_visible = True
        self._cursor_timer.start(500)

        key = event.key()
        mods = event.modifiers()

        if mods == (Qt.ControlModifier | Qt.ShiftModifier) and key == Qt.Key_C:
            self.copy()
            return
        if mods == (Qt.ControlModifier | Qt.ShiftModifier) and key == Qt.Key_V:
            self.paste()
            return
        if mods == Qt.ControlModifier and Qt.Key_A <= key <= Qt.Key_Z:
            os.write(self.fd, bytes([key - Qt.Key_A + 1]))
            return
        if mods == Qt.AltModifier:
            text = event.text()
            if text:
                os.write(self.fd, b'\x1b' + text.encode('utf-8'))
            return
        if key in self._KEY_MAP:
            os.write(self.fd, self._KEY_MAP[key])
            return
        text = event.text()
        if text:
            os.write(self.fd, text.encode('utf-8'))

    def closeEvent(self, event):
        self._cursor_timer.stop()
        self._render_timer.stop()
        self.notifier.setEnabled(False)
        if self.pid > 0:
            try:
                os.kill(self.pid, signal.SIGHUP)
                os.waitpid(self.pid, 0)
            except (ProcessLookupError, ChildProcessError):
                pass
        super().closeEvent(event)


if __name__ == '__main__':
    app = QApplication(sys.argv)
    term = TerminalWidget()
    term.setWindowTitle("Terminal")
    term.resize(700, 500)
    term.show()
    sys.exit(app.exec())
