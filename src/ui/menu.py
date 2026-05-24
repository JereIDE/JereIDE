from PySide6.QtWidgets import QMenuBar


class MenuBar:
    def __init__(self, window):
        self.window = window
        self.auto_indent_action = None
        self.line_numbers_action = None
        self.auto_pairing_action = None
        self.wrap_action = None
        self.syntax_highlighting_action = None
        self.toggle_full_screen_action = None

    def setup(self):
        menu_bar = self.window.menuBar()
        self._setup_file_menu(menu_bar)
        self._setup_edit_menu(menu_bar)
        self._setup_options_menu(menu_bar)
        self._setup_view_menu(menu_bar)
        self._setup_help_menu(menu_bar)

    def _setup_edit_menu(self, menu_bar):
        edit_menu = menu_bar.addMenu("&Edit")

        undo_action = edit_menu.addAction("&Undo")
        undo_action.setShortcut("Ctrl+Z")
        undo_action.triggered.connect(self.window.undo)

        redo_action = edit_menu.addAction("&Redo")
        redo_action.setShortcut("Ctrl+Y")
        redo_action.triggered.connect(self.window.redo)

        edit_menu.addSeparator()

        cut_action = edit_menu.addAction("Cu&t")
        cut_action.setShortcut("Ctrl+X")
        cut_action.triggered.connect(self.window.cut)

        copy_action = edit_menu.addAction("&Copy")
        copy_action.setShortcut("Ctrl+C")
        copy_action.triggered.connect(self.window.copy)

        paste_action = edit_menu.addAction("&Paste")
        paste_action.setShortcut("Ctrl+V")
        paste_action.triggered.connect(self.window.paste)

        edit_menu.addSeparator()

        select_all_action = edit_menu.addAction("Select &All")
        select_all_action.setShortcut("Ctrl+A")
        select_all_action.triggered.connect(self.window.select_all)

        edit_menu.addSeparator()

        find_action = edit_menu.addAction("&Find/Replace...")
        find_action.setShortcut("Ctrl+F")
        find_action.triggered.connect(self.window.find_replace)

    def _setup_file_menu(self, menu_bar):
        file_menu = menu_bar.addMenu("&File")

        new_action = file_menu.addAction("&New")
        new_action.setShortcut("Ctrl+N")
        new_action.triggered.connect(self.window.new_file)

        open_action = file_menu.addAction("&Open")
        open_action.setShortcut("Ctrl+O")
        open_action.triggered.connect(self.window.open_file)

        save_action = file_menu.addAction("&Save")
        save_action.setShortcut("Ctrl+S")
        save_action.triggered.connect(self.window.save_file)

        save_as_action = file_menu.addAction("Save &As...")
        save_as_action.setShortcut("Shift+Ctrl+S")
        save_as_action.triggered.connect(self.window.save_as_file)

        save_all_action = file_menu.addAction("Save A&ll")
        save_all_action.setShortcut("Alt+Ctrl+S")
        save_all_action.triggered.connect(self.window.save_all)

        file_menu.addSeparator()

        exit_action = file_menu.addAction("E&xit")
        exit_action.triggered.connect(self.window.close)

    def _setup_options_menu(self, menu_bar):
        options_menu = menu_bar.addMenu("&Options")

        self.syntax_highlighting_action = options_menu.addAction("&Syntax Highlighting")
        self.syntax_highlighting_action.setCheckable(True)
        self.syntax_highlighting_action.setChecked(self.window.syntax_highlighting_enabled)
        self.syntax_highlighting_action.triggered.connect(self.window.toggle_syntax_highlighting)

        self.auto_indent_action = options_menu.addAction("Auto &Indent")
        self.auto_indent_action.setCheckable(True)
        self.auto_indent_action.setChecked(self.window.auto_indent_enabled)
        self.auto_indent_action.triggered.connect(self.window.toggle_auto_indent)

        self.line_numbers_action = options_menu.addAction("&Line Numbers")
        self.line_numbers_action.setCheckable(True)
        self.line_numbers_action.setChecked(self.window.line_numbers_enabled)
        self.line_numbers_action.triggered.connect(self.window.toggle_line_numbers)

        self.auto_pairing_action = options_menu.addAction("Auto &Pairing")
        self.auto_pairing_action.setCheckable(True)
        self.auto_pairing_action.setChecked(self.window.auto_pairing_enabled)
        self.auto_pairing_action.triggered.connect(self.window.toggle_auto_pairing)

        self.wrap_action = options_menu.addAction("&Word Wrap")
        self.wrap_action.setCheckable(True)
        self.wrap_action.setChecked(self.window.wrap_enabled)
        self.wrap_action.triggered.connect(self.window.toggle_wrap)

    def _setup_view_menu(self, menu_bar):
        view_menu = menu_bar.addMenu("&View")

        zoom_in_action = view_menu.addAction("Zoom &In")
        zoom_in_action.setShortcut("Ctrl+=")  # Cmd+= on macOS
        zoom_in_action.triggered.connect(self.window.zoom_in)

        zoom_out_action = view_menu.addAction("Zoom &Out")
        zoom_out_action.setShortcut("Ctrl+-")  # Cmd+- on macOS
        zoom_out_action.triggered.connect(self.window.zoom_out)

        reset_zoom_action = view_menu.addAction("&Reset Zoom")
        reset_zoom_action.setShortcut("Ctrl+0")  # Cmd+0 on macOS
        reset_zoom_action.triggered.connect(self.window.reset_zoom)

        view_menu.addSeparator()

        self.toggle_full_screen_action = view_menu.addAction("Toggle Full Screen")
        self.toggle_full_screen_action.setShortcut("Meta+F")  # ^F (Control+F) on macOS
        self.toggle_full_screen_action.triggered.connect(self.window.toggle_full_screen)

    def _setup_help_menu(self, menu_bar):
        help_menu = menu_bar.addMenu("&Help")
        about_action = help_menu.addAction("&About JereIDE")
        about_action.triggered.connect(self.window.show_about)
        help_menu.addSeparator()
        star_action = help_menu.addAction("Star on &GitHub")
        star_action.triggered.connect(self.window.open_github)
