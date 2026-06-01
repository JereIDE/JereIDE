import sys
import os
import pty
import struct
import fcntl
import termios
import signal
from PySide6.QtWidgets import QApplication, QPlainTextEdit
from PySide6.QtCore import QSocketNotifier, Qt, QTimer
from PySide6.QtGui import (
    QColor, QFont, QFontMetrics, QKeyEvent, QResizeEvent,
    QTextCharFormat, QTextCursor,
)
import pyte
from pyte import modes as _pyte_modes
from pyte.screens import HistoryScreen

_orig_sgr = pyte.screens.Screen.select_graphic_rendition
def _sgr_safe(self, *attrs, **kwargs):
    return _orig_sgr(self, *attrs)
pyte.screens.Screen.select_graphic_rendition = _sgr_safe




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
        Qt.Key.Key_Return: b'\r',
        Qt.Key.Key_Backspace: b'\x7f',
        Qt.Key.Key_Tab: b'\t',
        Qt.Key.Key_Up: b'\x1b[A',
        Qt.Key.Key_Down: b'\x1b[B',
        Qt.Key.Key_Right: b'\x1b[C',
        Qt.Key.Key_Left: b'\x1b[D',
        Qt.Key.Key_Home: b'\x1b[H',
        Qt.Key.Key_End: b'\x1b[F',
        Qt.Key.Key_PageUp: b'\x1b[5~',
        Qt.Key.Key_PageDown: b'\x1b[6~',
        Qt.Key.Key_Delete: b'\x1b[3~',
        Qt.Key.Key_Escape: b'\x1b',
        Qt.Key.Key_F1: b'\x1bOP',
        Qt.Key.Key_F2: b'\x1bOQ',
        Qt.Key.Key_F3: b'\x1bOR',
        Qt.Key.Key_F4: b'\x1bOS',
        Qt.Key.Key_F5: b'\x1b[15~',
        Qt.Key.Key_F6: b'\x1b[17~',
        Qt.Key.Key_F7: b'\x1b[18~',
        Qt.Key.Key_F8: b'\x1b[19~',
        Qt.Key.Key_F9: b'\x1b[20~',
        Qt.Key.Key_F10: b'\x1b[21~',
        Qt.Key.Key_F11: b'\x1b[23~',
        Qt.Key.Key_F12: b'\x1b[24~',
        Qt.Key.Key_Insert: b'\x1b[2~',
    }

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setReadOnly(True)
        self.setFrameStyle(0)
        self.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self.setVerticalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOn)

        self.terminal_font = QFont("Menlo", 14)
        self.terminal_font.setStyleHint(QFont.StyleHint.Monospace)
        self.setFont(self.terminal_font)
        self.setLineWrapMode(QPlainTextEdit.LineWrapMode.NoWrap)
        self.verticalScrollBar().valueChanged.connect(self._on_scroll)

        self.columns = 80
        self.lines = 24
        self.term_screen = HistoryScreen(self.columns, self.lines, history=self.MAX_HISTORY)
        self.term_screen.set_mode(_pyte_modes.LNM)
        self.term_stream = pyte.Stream(self.term_screen)

        self._cursor_visible = True
        self._cursor_timer = QTimer(self)
        self._cursor_timer.timeout.connect(self._toggle_cursor)
        self._cursor_timer.start(500)

        self._dirty = False
        self._render_timer = QTimer(self)
        self._render_timer.setSingleShot(True)
        self._render_timer.timeout.connect(self._render)
        self._at_bottom = True
        self._rendered_history_count = 0
        self._history_end_pos = 0

        self._pending_rows = None
        self._pending_cols = None

        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            env = os.environ.copy()
            env['TERM'] = 'xterm-256color'
            shell = os.environ.get('SHELL', '/bin/zsh')
            os.execve(shell, [shell], env)
            os._exit(1)
        else:
            self._update_size()
            self._apply_pending_resize()
            self.notifier = QSocketNotifier(self.fd, QSocketNotifier.Type.Read)
            self.notifier.activated.connect(self._read_data)

    def _read_data(self):
        try:
            data = os.read(self.fd, 65536)
            if not data:
                self.notifier.setEnabled(False)
                return
            self.term_stream.feed(data.decode('utf-8', errors='replace'))
            self._dirty = True
            self._render_timer.start(16)
        except OSError:
            self.notifier.setEnabled(False)

    def run_command(self, command):
        """Run a command in its own terminal session (new PTY + shell)."""
        self.notifier.setEnabled(False)

        # Clean up old shell process
        if self.pid > 0:
            try:
                os.kill(self.pid, signal.SIGHUP)
            except (ProcessLookupError, ChildProcessError):
                pass
            try:
                os.waitpid(self.pid, os.WNOHANG)
            except ChildProcessError:
                pass

        # Close old PTY file descriptor
        try:
            os.close(self.fd)
        except OSError:
            pass

        # Reset pyte screen and history for a clean display
        self.term_screen.reset()
        self._rendered_history_count = 0
        self._history_end_pos = 0
        self._dirty = True
        self._render_timer.start(16)

        # Create a new PTY session for the command
        pid, fd = pty.fork()
        if pid == 0:
            env = os.environ.copy()
            env['TERM'] = 'xterm-256color'
            os.execve('/bin/sh', ['/bin/sh', '-c', command], env)
            os._exit(1)
        else:
            self.pid = pid
            self.fd = fd
            self._update_size()
            self._apply_pending_resize()
            self.notifier = QSocketNotifier(self.fd, QSocketNotifier.Type.Read)
            self.notifier.activated.connect(self._read_data)
            self.notifier.setEnabled(True)

    def _invalidate(self):
        if not self._dirty:
            self._dirty = True
            self._render_timer.start(16)


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
        self.term_screen.resize(rows, cols)
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
            fmt.setFontWeight(QFont.Weight.Bold)

        fg = self._resolve_color(pyte_char.fg)
        fmt.setForeground(fg or QColor(0, 0, 0))

        bg = self._resolve_color(pyte_char.bg)
        fmt.setBackground(bg or QColor(255, 255, 255))

        if pyte_char.underscore:
            fmt.setUnderlineStyle(QTextCharFormat.UnderlineStyle.SingleUnderline)
            fmt.setFontUnderline(True)

        if pyte_char.strikethrough:
            fmt.setFontStrikeOut(True)

        return fmt

    def _render(self):
        if not self._dirty:
            return
        self._dirty = False
        was_at_bottom = self._at_bottom

        history = list(self.term_screen.history.top)
        hist_len = len(history)

        doc = self.document()
        cursor = QTextCursor(doc)
        cursor.beginEditBlock()

        default_fmt = self._make_format()

        # --- Incremental history: only append new lines that scrolled in ---
        if hist_len > self._rendered_history_count:
            cursor.setPosition(self._history_end_pos)

            for i in range(self._rendered_history_count, hist_len):
                cursor.insertText('\n', default_fmt)
                row = history[i]
                for x in range(self.term_screen.columns):
                    char = row.get(x)
                    if char and char.data:
                        cursor.insertText(char.data, self._make_format(char))
                    else:
                        cursor.insertText(' ', QTextCharFormat(default_fmt))

            cursor.insertText('\n', default_fmt)
            self._rendered_history_count = hist_len
            self._history_end_pos = cursor.position()

        elif hist_len < self._rendered_history_count:
            cursor.select(QTextCursor.SelectionType.Document)
            cursor.removeSelectedText()
            self._rendered_history_count = 0

            for i, row in enumerate(history):
                if i > 0:
                    cursor.insertText('\n', default_fmt)
                for x in range(self.term_screen.columns):
                    char = row.get(x)
                    if char and char.data:
                        cursor.insertText(char.data, self._make_format(char))
                    else:
                        cursor.insertText(' ', QTextCharFormat(default_fmt))
            if hist_len > 0:
                cursor.insertText('\n', default_fmt)
            self._rendered_history_count = hist_len
            self._history_end_pos = cursor.position()

        # --- Buffer: always rewrite the visible area (small, ~24 lines) ---
        # Position cursor at the boundary between history and buffer
        if hist_len > 0:
            cursor.setPosition(self._history_end_pos)
        cursor.movePosition(QTextCursor.MoveOperation.End, QTextCursor.MoveMode.KeepAnchor)
        cursor.removeSelectedText()

        buf = self.term_screen.buffer
        cy = self.term_screen.cursor.y
        cx = self.term_screen.cursor.x

        for y in range(self.term_screen.lines):
            if y > 0 or hist_len > 0:
                cursor.insertText('\n', default_fmt)
            for x in range(self.term_screen.columns):
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
            target_line = hist_len + cy
            doc_line_count = self.document().lineCount()
            if target_line >= doc_line_count:
                target_line = max(0, doc_line_count - 1)
            c = self.textCursor()
            c.movePosition(QTextCursor.MoveOperation.Start)
            c.movePosition(QTextCursor.MoveOperation.Down, QTextCursor.MoveMode.MoveAnchor, target_line)
            c.movePosition(QTextCursor.MoveOperation.Right, QTextCursor.MoveMode.MoveAnchor,
                           min(cx, len(self.document().findBlockByLineNumber(target_line).text())))
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

        if mods == (Qt.KeyboardModifier.ControlModifier | Qt.KeyboardModifier.ShiftModifier) and key == Qt.Key.Key_C:
            self.copy()
            return
        if mods == (Qt.KeyboardModifier.ControlModifier | Qt.KeyboardModifier.ShiftModifier) and key == Qt.Key.Key_V:
            self.paste()
            return
        if mods == Qt.KeyboardModifier.ControlModifier and Qt.Key.Key_A <= key <= Qt.Key.Key_Z:
            os.write(self.fd, bytes([key - Qt.Key.Key_A + 1]))
            return
        if mods == Qt.KeyboardModifier.AltModifier:
            text = event.text()
            if text:
                os.write(self.fd, b'\x1b' + text.encode('utf-8'))
            return
        if key in self._KEY_MAP:  # type: ignore[operator]
            os.write(self.fd, self._KEY_MAP[key])  # type: ignore[index]
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
