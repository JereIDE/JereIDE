from PySide6.QtCore import QRegularExpression
from PySide6.QtGui import QFont, QColor, QSyntaxHighlighter, QTextCharFormat
from config.theme import SYNTAX_KEYWORD, SYNTAX_STRING, SYNTAX_NUMBER, SYNTAX_COMMENT
from config.theme import SYNTAX_BUILTIN, SYNTAX_DECORATOR, SYNTAX_CLASS_DEF, SYNTAX_FUNCTION_DEF
from config.config_manager import config_manager


class PythonSyntaxHighlighter(QSyntaxHighlighter):
    def __init__(self, parent):
        super().__init__(parent)
        self.syntax_highlighting_enabled = config_manager.get_config_value('editor', 'syntax_highlighting.enabled', True)
        self.PYTHON_KEYWORDS = config_manager.get_config_value('editor', 'syntax_highlighting.keywords', [
            'False', 'None', 'True', 'and', 'as', 'assert', 'async', 'await',
            'break', 'class', 'continue', 'def', 'del', 'elif', 'else', 'except',
            'finally', 'for', 'from', 'global', 'if', 'import', 'in', 'is',
            'lambda', 'nonlocal', 'not', 'or', 'pass', 'raise', 'return', 'try',
            'while', 'with', 'yield'
        ])

        self.PYTHON_BUILTINS = config_manager.get_config_value('editor', 'syntax_highlighting.builtins', [
            'abs', 'all', 'any', 'bin', 'bool', 'bytes', 'callable', 'chr', 'dict',
            'dir', 'divmod', 'enumerate', 'eval', 'exec', 'filter', 'float', 'format',
            'frozenset', 'getattr', 'globals', 'hasattr', 'hash', 'help', 'hex', 'id',
            'input', 'int', 'isinstance', 'issubclass', 'iter', 'len', 'list', 'locals',
            'map', 'max', 'min', 'next', 'object', 'oct', 'open', 'ord', 'pow',
            'print', 'property', 'range', 'repr', 'reversed', 'round', 'set',
            'setattr', 'slice', 'sorted', 'staticmethod', 'str', 'sum', 'super', 'tuple',
            'type', 'vars', 'zip', '__import__'
        ])

        self._triple_quote_fmt = self._create_format(SYNTAX_STRING)
        self._highlighting_rules = []
        self._build_highlighting_rules()

    def _create_format(self, color, bold=False, italic=False):
        fmt = QTextCharFormat()
        fmt.setForeground(QColor(color))
        if bold:
            fmt.setFontWeight(QFont.Bold)
        if italic:
            fmt.setFontItalic(True)
        return fmt

    def _build_highlighting_rules(self):
        keyword_fmt = self._create_format(SYNTAX_KEYWORD, bold=True)
        for word in self.PYTHON_KEYWORDS:
            pattern = QRegularExpression(r'\b' + word + r'\b')
            self._highlighting_rules.append((pattern, keyword_fmt))

        builtin_fmt = self._create_format(SYNTAX_BUILTIN)
        for word in self.PYTHON_BUILTINS:
            pattern = QRegularExpression(r'\b' + word + r'(?=\s*\()')
            self._highlighting_rules.append((pattern, builtin_fmt))

        decorator_fmt = self._create_format(SYNTAX_DECORATOR)
        pattern = QRegularExpression(r'@\w+')
        self._highlighting_rules.append((pattern, decorator_fmt))

        class_fmt = self._create_format(SYNTAX_CLASS_DEF, bold=True)
        pattern = QRegularExpression(r'\bclass\s+\w+')
        self._highlighting_rules.append((pattern, class_fmt))

        func_fmt = self._create_format(SYNTAX_FUNCTION_DEF)
        pattern = QRegularExpression(r'\bdef\s+\w+')
        self._highlighting_rules.append((pattern, func_fmt))

        comment_fmt = self._create_format(SYNTAX_COMMENT, italic=True)
        pattern = QRegularExpression(r'#.*')
        self._highlighting_rules.append((pattern, comment_fmt))

        string_fmt = self._create_format(SYNTAX_STRING)
        pattern = QRegularExpression(r'"(?:[^"\\]|\\.)*"')
        self._highlighting_rules.append((pattern, string_fmt))

        pattern = QRegularExpression(r"'(?:[^'\\]|\\.)*'")
        self._highlighting_rules.append((pattern, string_fmt))

        number_fmt = self._create_format(SYNTAX_NUMBER)
        pattern = QRegularExpression(r'\b[0-9]+\.?[0-9]*\b')
        self._highlighting_rules.append((pattern, number_fmt))

    def highlightBlock(self, text):
        self.setCurrentBlockState(0)

        for pattern, fmt in self._highlighting_rules:
            iterator = pattern.globalMatch(text)
            while iterator.hasNext():
                match = iterator.next()
                self.setFormat(match.capturedStart(), match.capturedLength(), fmt)

        self._highlight_multiline_strings(text)

    def _highlight_multiline_strings(self, text):
        state = self.previousBlockState()
        if state == 1:
            self._continue_triple(text, '"""', 1)
            return
        elif state == 2:
            self._continue_triple(text, "'''", 2)
            return
        self._find_triples(text, 0)

    def _continue_triple(self, text, delim, next_state):
        end = text.find(delim)
        if end >= 0:
            self.setFormat(0, end + 3, self._triple_quote_fmt)
            self.setCurrentBlockState(0)
            self._find_triples(text[end + 3:], end + 3)
        else:
            self.setFormat(0, len(text), self._triple_quote_fmt)
            self.setCurrentBlockState(next_state)

    def _find_triples(self, text, offset):
        pos = 0
        while True:
            dd = text.find('"""', pos)
            sd = text.find("'''", pos)
            if dd < 0 and sd < 0:
                break
            if dd >= 0 and (sd < 0 or dd < sd):
                delim = '"""'
                idx = dd
            else:
                delim = "'''"
                idx = sd
            close = text.find(delim, idx + 3)
            if close >= 0:
                self.setFormat(offset + idx, close - idx + 3, self._triple_quote_fmt)
                pos = close + 3
            else:
                self.setFormat(offset + idx, len(text) - idx, self._triple_quote_fmt)
                self.setCurrentBlockState(1 if delim == '"""' else 2)
                break
