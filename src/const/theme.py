# Hardcoded theme and editor constants — no config file needed.

# Editor theme
EDITOR_BG = "#FFFFFF"
EDITOR_FONT_FAMILY = "Monaco"
EDITOR_FONT_SIZE = 11

# Line numbers
LINE_NUMBER_BG = "#dcdcdc"
LINE_NUMBER_TEXT = "#000000"

# Current line highlighting
CURRENT_LINE_BG = "#ffffd0"

# Status bar
STATUS_BAR_BG = "#f5f5f5"
STATUS_BAR_HEIGHT = 24

# Syntax highlighting colors
SYNTAX_KEYWORD = "#0000FF"
SYNTAX_STRING = "#A315AD"
SYNTAX_NUMBER = "#098658"
SYNTAX_COMMENT = "#008000"
SYNTAX_BUILTIN = "#795E26"
SYNTAX_DECORATOR = "#800000"
SYNTAX_CLASS_DEF = "#267F99"
SYNTAX_FUNCTION_DEF = "#267F99"

# Pair highlighting
PAIR_HIGHLIGHT = "#FFFD38"

# Welcome frame colors
WELCOME_TEXT_PRIMARY = "#000000"
WELCOME_TEXT_SECONDARY = "#888888"
WELCOME_DIVIDER = "#E0E0E0"

# Tab colors
TAB_STRIP_BG = "#FFFFFF"
TAB_SELECTED_BG = "#CEE6FC"
TAB_UNSELECTED_BG = "#FFFFFF"
TAB_BORDER = "#D2D2D2"
TAB_SELECTED_TEXT = "#2386FB"
TAB_UNSELECTED_TEXT = "#000000"
TAB_SELECTED_CLOSE_HOVER_BG = "#BBDCFB"
TAB_UNSELECTED_CLOSE_HOVER_BG = "#F0F0F0"
TAB_SEPARATOR = "#D2D2D2"
TAB_HEIGHT = 30

# Editor behaviour defaults
EDITOR_TAB_SIZE = 4
LINE_NUMBERS_ENABLED = True
LINE_NUMBERS_MIN_WIDTH = 15
SYNTAX_HIGHLIGHTING_ENABLED = True
AUTO_INDENT_ENABLED = True
AUTO_PAIRING_ENABLED = True
WORD_WRAP_ENABLED = False

# Python syntax data
PYTHON_KEYWORDS = [
    "False", "None", "True", "and", "as", "assert", "async", "await",
    "break", "class", "continue", "def", "del", "elif", "else", "except",
    "finally", "for", "from", "global", "if", "import", "in", "is",
    "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try",
    "while", "with", "yield",
]

PYTHON_BUILTINS = [
    "abs", "all", "any", "bin", "bool", "bytes", "callable", "chr", "dict",
    "dir", "divmod", "enumerate", "eval", "exec", "filter", "float", "format",
    "frozenset", "getattr", "globals", "hasattr", "hash", "help", "hex", "id",
    "input", "int", "isinstance", "issubclass", "iter", "len", "list", "locals",
    "map", "max", "min", "next", "object", "oct", "open", "ord", "pow",
    "print", "property", "range", "repr", "reversed", "round", "set",
    "setattr", "slice", "sorted", "staticmethod", "str", "sum", "super", "tuple",
    "type", "vars", "zip", "__import__",
]

AUTO_PAIR_PAIRS = {
    "(": ")",
    "[": "]",
    "{": "}",
    '"': '"',
    "'": "'",
}
