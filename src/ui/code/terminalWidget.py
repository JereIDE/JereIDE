import sys

import pty

import os

import pyte

from PySide6.QtWidgets import QApplication, QTextEdit, QVBoxLayout, QWidget

from PySide6.QtCore import QSocketNotifier, Qt, QTimer

from PySide6.QtGui import QTextCursor



class TerminalWidget(QTextEdit):

    def __init__(self, parent=None):

        super().__init__(parent)

        self.setReadOnly(True)

        self.setStyleSheet("background-color: white; color: black; font-family: 'Menlo'; font-size: 14px;")



        self.screen = pyte.Screen(80, 24)

        self.stream = pyte.Stream(self.screen)



        # --- FLASHING SETUP ---

        self.cursor_visible = True

        self.flash_timer = QTimer(self)

        self.flash_timer.timeout.connect(self.toggle_cursor)

        self.flash_timer.start(500)  # Flash every 500 milliseconds (0.5 seconds)



        self.pid, self.fd = pty.fork()

        if self.pid == 0:

            os.environ['TERM'] = 'linux'

            os.execlp('zsh', 'zsh')

        else:

            self.notifier = QSocketNotifier(self.fd, QSocketNotifier.Read)

            self.notifier.activated.connect(self.read_terminal)



    def toggle_cursor(self):

        # Flip the switch and redraw the screen

        self.cursor_visible = not self.cursor_visible

        self.render_screen()



    def read_terminal(self):

        try:

            data = os.read(self.fd, 1024).decode('utf-8', errors='replace')

            self.stream.feed(data)

            self.render_screen()

        except Exception:

            pass



    def render_screen(self):

        at_bottom = self.is_scrolled_to_bottom()

        lines = self.screen.display.copy()

        while lines and lines[-1].strip() == '':
            lines.pop()



        y = self.screen.cursor.y

        x = self.screen.cursor.x



        if self.cursor_visible and y < len(lines):

            line = lines[y]

            lines[y] = line[:x] + "█" + line[x+1:]



        self.setPlainText("\n".join(lines))

        if at_bottom:
            self.moveCursor(QTextCursor.MoveOperation.End)
        else:
            self.moveCursor(QTextCursor.MoveOperation.Start)

    def is_scrolled_to_bottom(self):

        scroll_bar = self.verticalScrollBar()
        return scroll_bar.value() >= scroll_bar.maximum() - 10



    def keyPressEvent(self, event):

        # Reset cursor to visible immediately when typing

        self.cursor_visible = True



        key = event.key()

        special_keys = {

            Qt.Key_Return: b'\r',

            Qt.Key_Backspace: b'\x7f',

            Qt.Key_Tab: b'\t',

            Qt.Key_Up: b'\x1b[A',

            Qt.Key_Down: b'\x1b[B',

            Qt.Key_Right: b'\x1b[C',

            Qt.Key_Left: b'\x1b[D',

        }



        if key in special_keys:

            os.write(self.fd, special_keys[key])

        else:

            text = event.text()

            if text:

                os.write(self.fd, text.encode('utf-8'))



if __name__ == '__main__':

    app = QApplication(sys.argv)

    term = TerminalWidget()

    term.resize(700, 450)

    term.show()

    sys.exit(app.exec())
