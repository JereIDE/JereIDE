import os

SRC_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

ROOT_DIR = os.path.dirname(SRC_DIR)

ICONS_DIR = os.path.join(SRC_DIR, "icons")
LOGO_PATH = os.path.join(ICONS_DIR, "logo.svg")

CONST_DIR = os.path.join(SRC_DIR, "const")
THEME_PATH = os.path.join(SRC_DIR, "config", "theme.py")
STYLESHEET_PATH = os.path.join(CONST_DIR, "styles.qss")
TASKS_PATH = os.path.join(SRC_DIR, "config", "tasks.json")

UI_DIR = os.path.join(SRC_DIR, "ui")
