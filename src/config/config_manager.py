import json
import os
from const.paths import ROOT_DIR


class ConfigManager:
    """Manages loading and accessing configuration from JSON files."""

    def __init__(self):
        self.config_dir = os.path.join(ROOT_DIR, "src", "config")
        self._defaults = {}
        self._theme_config = {}
        self._editor_config = {}
        self._load_defaults()
        self._ensure_config_files()
        self._load_configs()

    def _load_defaults(self):
        """Load defaults from defaults.json."""
        defaults_path = os.path.join(self.config_dir, "defaults.json")
        if os.path.exists(defaults_path):
            with open(defaults_path, "r") as f:
                self._defaults = json.load(f)
        else:
            self._defaults = {}

    def _ensure_config_files(self):
        """Create config files from defaults if they don't exist."""
        files_to_create = {
            "theme.json": self._defaults.get("theme", {}),
            "editor.json": self._defaults.get("editor", {}),
        }

        for filename, content in files_to_create.items():
            filepath = os.path.join(self.config_dir, filename)
            if not os.path.exists(filepath):
                with open(filepath, "w") as f:
                    json.dump(content, f, indent=2)

    def _load_configs(self):
        """Load all configuration files."""
        theme_path = os.path.join(self.config_dir, "theme.json")
        if os.path.exists(theme_path):
            with open(theme_path, "r") as f:
                self._theme_config = json.load(f)

        editor_path = os.path.join(self.config_dir, "editor.json")
        if os.path.exists(editor_path):
            with open(editor_path, "r") as f:
                self._editor_config = json.load(f)

    def get_theme_config(self):
        """Get the theme configuration."""
        return self._theme_config

    def get_editor_config(self):
        """Get the editor configuration."""
        return self._editor_config

    def get_config_value(self, config_type, key_path, default=None):
        """
        Get a configuration value using dot notation.
        Falls back to defaults if value not found.

        Args:
            config_type: 'theme', 'editor', or 'defaults'
            key_path: Dot-separated path to the value (e.g., 'editor.background')
            default: Default value if key not found

        Returns:
            The configuration value or default if not found
        """
        if config_type == "theme":
            config = self._theme_config
            fallback = self._defaults.get("theme", {})
        elif config_type == "editor":
            config = self._editor_config
            fallback = self._defaults.get("editor", {})
        elif config_type == "defaults":
            config = {}
            fallback = self._defaults
        else:
            return default

        keys = key_path.split(".")
        value = config

        try:
            for key in keys:
                value = value[key]
            return value
        except (KeyError, TypeError):
            pass

        value = fallback
        try:
            for key in keys:
                value = value[key]
            return value
        except (KeyError, TypeError):
            return default


# Create a singleton instance
config_manager = ConfigManager()