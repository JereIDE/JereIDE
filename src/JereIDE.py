# Entry Point
import sys
import os
from PySide6.QtWidgets import QApplication
from ui.mainWindow import MainWindow
#from const.paths import STYLESHEET_PATH

# Add src directory to path
SRC_DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, SRC_DIR)




if __name__ == "__main__":
    app = QApplication(sys.argv)
    # QSS file is empty; all styling done in Python
    # with open(STYLESHEET_PATH, "r") as f:
    #     qss = f.read()
    # if qss.strip():
    #     app.setStyleSheet(qss)
    window = MainWindow()
    window.show()
    sys.exit(app.exec())
