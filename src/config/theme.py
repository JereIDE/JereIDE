# Theme constants loaded from JSON configuration
import sys
import os
# Add parent directory to path for imports
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from config.config_manager import config_manager

# Editor theme
theme_config = config_manager.get_theme_config()

EDITOR_BG = config_manager.get_config_value('theme', 'editor.background', '#FFFFFF')
EDITOR_FONT_FAMILY = config_manager.get_config_value('theme', 'editor.font_family', 'Monaco')
EDITOR_FONT_SIZE = config_manager.get_config_value('theme', 'editor.font_size', 11)

# Line numbers
LINE_NUMBER_BG = config_manager.get_config_value('theme', 'line_numbers.background', '#dcdcdc')
LINE_NUMBER_TEXT = config_manager.get_config_value('theme', 'line_numbers.text', '#000000')

# Current line highlighting
CURRENT_LINE_BG = config_manager.get_config_value('theme', 'current_line.background', '#ffffd0')

# Status bar
STATUS_BAR_BG = config_manager.get_config_value('theme', 'status_bar.background', '#f5f5f5')
STATUS_BAR_HEIGHT = config_manager.get_config_value('theme', 'status_bar.height', 24)

# Syntax highlighting colors
SYNTAX_KEYWORD = config_manager.get_config_value('theme', 'syntax_highlighting.keyword', '#0000FF')
SYNTAX_STRING = config_manager.get_config_value('theme', 'syntax_highlighting.string', '#A315AD')
SYNTAX_NUMBER = config_manager.get_config_value('theme', 'syntax_highlighting.number', '#098658')
SYNTAX_COMMENT = config_manager.get_config_value('theme', 'syntax_highlighting.comment', '#008000')
SYNTAX_BUILTIN = config_manager.get_config_value('theme', 'syntax_highlighting.builtin', '#795E26')
SYNTAX_DECORATOR = config_manager.get_config_value('theme', 'syntax_highlighting.decorator', '#800000')
SYNTAX_CLASS_DEF = config_manager.get_config_value('theme', 'syntax_highlighting.class_def', '#267F99')
SYNTAX_FUNCTION_DEF = config_manager.get_config_value('theme', 'syntax_highlighting.function_def', '#267F99')

# Pair highlighting
PAIR_HIGHLIGHT = config_manager.get_config_value('theme', 'pair_highlighting.color', '#FFFD38')

# Welcome frame colors
WELCOME_TEXT_PRIMARY = config_manager.get_config_value('theme', 'welcome.text_primary', '#000000')
WELCOME_TEXT_SECONDARY = config_manager.get_config_value('theme', 'welcome.text_secondary', '#888888')
WELCOME_DIVIDER = config_manager.get_config_value('theme', 'welcome.divider', '#E0E0E0')

# Tab colors
TAB_STRIP_BG = config_manager.get_config_value('theme', 'tabs.strip_background', '#FFFFFF')
TAB_SELECTED_BG = config_manager.get_config_value('theme', 'tabs.selected_background', '#CEE6FC')
TAB_UNSELECTED_BG = config_manager.get_config_value('theme', 'tabs.unselected_background', '#FFFFFF')
TAB_BORDER = config_manager.get_config_value('theme', 'tabs.border', '#D2D2D2')
TAB_SELECTED_TEXT = config_manager.get_config_value('theme', 'tabs.selected_text', '#2386FB')
TAB_UNSELECTED_TEXT = config_manager.get_config_value('theme', 'tabs.unselected_text', '#000000')
TAB_SELECTED_CLOSE_HOVER_BG = config_manager.get_config_value('theme', 'tabs.selected_close_hover_background', '#BBDCFB')
TAB_UNSELECTED_CLOSE_HOVER_BG = config_manager.get_config_value('theme', 'tabs.unselected_close_hover_background', '#F0F0F0')
TAB_SEPARATOR = config_manager.get_config_value('theme', 'tabs.separator', '#D2D2D2')
