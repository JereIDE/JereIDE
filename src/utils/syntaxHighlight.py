from PySide6.QtCore import QRegularExpression
from PySide6.QtGui import QFont, QColor, QSyntaxHighlighter, QTextCharFormat
from const.theme import (
    SYNTAX_KEYWORD, SYNTAX_STRING, SYNTAX_NUMBER, SYNTAX_COMMENT,
    SYNTAX_BUILTIN, SYNTAX_DECORATOR, SYNTAX_CLASS_DEF, SYNTAX_FUNCTION_DEF,
    SYNTAX_HIGHLIGHTING_ENABLED, PYTHON_KEYWORDS, PYTHON_BUILTINS,
)


class PythonSyntaxHighlighter(QSyntaxHighlighter):
    """Python syntax highlighter with per-block result caching.

    Caches highlighting results keyed by ``(block_number, prev_state, text_hash)``.
    When a cache hit occurs (block text unchanged), regex matching is skipped and
    previously stored formats are replayed. This eliminates redundant work when
    blocks scroll into view.
    """

    MAX_CACHE_SIZE = 2000

    def __init__(self, parent):
        super().__init__(parent)
        self.syntax_highlighting_enabled = SYNTAX_HIGHLIGHTING_ENABLED
        self.PYTHON_KEYWORDS = PYTHON_KEYWORDS
        self.PYTHON_BUILTINS = PYTHON_BUILTINS

        self._triple_quote_fmt = self._create_format(SYNTAX_STRING)
        self._highlighting_rules = []
        self._build_highlighting_rules()

        # Cache: {(block_number, prev_state): (text_hash, final_state, [(s, l, fmt), ...])}
        self._cache = {}

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
        block = self.currentBlock()
        block_number = block.blockNumber()
        prev_state = self.previousBlockState()
        text_hash = hash(text)
        cache_key = (block_number, prev_state)

        cached = self._cache.get(cache_key)
        if cached is not None and cached[0] == text_hash:
            # Cache hit — replay stored formats, skip all regex
            self.setCurrentBlockState(cached[1])
            for start, length, fmt in cached[2]:
                self.setFormat(start, length, fmt)
            return

        # Cache miss — run full highlighting and record formats
        self.setCurrentBlockState(0)
        formats = []

        for pattern, fmt in self._highlighting_rules:
            iterator = pattern.globalMatch(text)
            while iterator.hasNext():
                match = iterator.next()
                s = match.capturedStart()
                l = match.capturedLength()
                self.setFormat(s, l, fmt)
                formats.append((s, l, fmt))

        self._highlight_multiline_strings(text, formats)

        final_state = self.currentBlockState()
        self._cache[cache_key] = (text_hash, final_state, formats)

        if len(self._cache) > self.MAX_CACHE_SIZE:
            self._cache.clear()

    def _highlight_multiline_strings(self, text, formats):
        state = self.previousBlockState()
        if state == 1:
            self._continue_triple(text, '"""', 1, formats)
            return
        elif state == 2:
            self._continue_triple(text, "'''", 2, formats)
            return
        self._find_triples(text, 0, formats)

    def _continue_triple(self, text, delim, next_state, formats):
        end = text.find(delim)
        if end >= 0:
            self.setFormat(0, end + 3, self._triple_quote_fmt)
            formats.append((0, end + 3, self._triple_quote_fmt))
            self.setCurrentBlockState(0)
            self._find_triples(text[end + 3:], end + 3, formats)
        else:
            self.setFormat(0, len(text), self._triple_quote_fmt)
            formats.append((0, len(text), self._triple_quote_fmt))
            self.setCurrentBlockState(next_state)

    def _find_triples(self, text, offset, formats):
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
                formats.append((offset + idx, close - idx + 3, self._triple_quote_fmt))
                pos = close + 3
            else:
                self.setFormat(offset + idx, len(text) - idx, self._triple_quote_fmt)
                formats.append((offset + idx, len(text) - idx, self._triple_quote_fmt))
                self.setCurrentBlockState(1 if delim == '"""' else 2)
                break
