import json
import logging
import os
import threading
from copy import deepcopy

from PySide6.QtCore import QFileSystemWatcher, QObject, Signal

RC_FILE = os.path.expanduser("~/.jereiderc")

logger = logging.getLogger(__name__)

# Schema describing the expected structure of the config file.
# Leaf values are Python types; the validator checks isinstance() against them.
CONFIG_SCHEMA = {
    "theme": {
        "editor": {
            "background": str,
            "font_family": str,
            "font_size": int,
        },
        "line_numbers": {
            "background": str,
            "text": str,
        },
        "current_line": {
            "background": str,
        },
        "status_bar": {
            "background": str,
            "height": int,
        },
        "syntax_highlighting": {
            "keyword": str,
            "string": str,
            "number": str,
            "comment": str,
            "builtin": str,
            "decorator": str,
            "class_def": str,
            "function_def": str,
        },
        "pair_highlighting": {
            "color": str,
        },
        "welcome": {
            "text_primary": str,
            "text_secondary": str,
            "divider": str,
        },
        "tabs": {
            "strip_background": str,
            "selected_background": str,
            "unselected_background": str,
            "border": str,
            "selected_text": str,
            "unselected_text": str,
            "selected_close_hover_background": str,
            "unselected_close_hover_background": str,
            "separator": str,
            "height": int,
        },
    },
    "editor": {
        "font": {
            "tab_size": int,
        },
        "line_numbers": {
            "enabled": bool,
            "minimum_width": int,
        },
        "syntax_highlighting": {
            "enabled": bool,
            "keywords": list,
            "builtins": list,
        },
        "auto_indent": {
            "enabled": bool,
            "pairs": dict,
        },
        "auto_pairing": {
            "enabled": bool,
            "pairs": dict,
        },
    },
    "tasks": {
        "tasks": list,
    },
}


class ConfigManager(QObject):
    config_reloaded = Signal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self._lock = threading.RLock()
        self._config = {}
        self._defaults = {}
        self._watcher = None
        self._loaded = False

    # ------------------------------------------------------------------
    # Lazy initialisation
    # ------------------------------------------------------------------

    def _lazy_load(self):
        if self._loaded:
            return
        with self._lock:
            if self._loaded:
                return
            self._load_defaults()
            self._ensure_rc_file()
            self._load_rc_file()
            self._setup_watcher()
            self._loaded = True

    def _load_defaults(self):
        defaults_path = os.path.join(os.path.dirname(__file__), "defaults.json")
        if os.path.exists(defaults_path):
            with open(defaults_path, "r") as f:
                self._defaults = json.load(f)

    def _ensure_rc_file(self):
        """Create the user config file from defaults if it doesn't exist."""
        if os.path.exists(RC_FILE):
            return
        self._write_rc_file(self._defaults)

    def _write_rc_file(self, config):
        full = {
            "__info__": [
                "# JereIDE Configuration File",
                "#",
                "# Top-level keys:",
                "#   theme  - Colors and fonts for the editor UI",
                "#   editor - Editor behavior (tabs, syntax, auto-indent, etc.)",
                "#   tasks  - Quick-run task definitions",
                "#",
                "# Changes made here while JereIDE is running are picked up automatically.",
                "# Delete a key and restart JereIDE to reset that section to defaults.",
            ]
        }
        full.update(config)
        with open(RC_FILE, "w") as f:
            json.dump(full, f, indent=2)
            f.write("\n")

    # ------------------------------------------------------------------
    # Loading & validation
    # ------------------------------------------------------------------

    def _load_rc_file(self):
        if not os.path.exists(RC_FILE):
            self._config = {}
            return
        try:
            with open(RC_FILE, "r") as f:
                raw = json.load(f)
        except json.JSONDecodeError as e:
            logger.warning("Config: failed to parse ~/.jereiderc (%s). Falling back to defaults.", e)
            self._config = {}
            return

        config = {k: v for k, v in raw.items() if not k.startswith("_")}
        self._validate_and_clean(config, CONFIG_SCHEMA)
        self._config = config

    def _validate_and_clean(self, config, schema, path=""):
        """Recursively validate *config* against *schema*, pruning invalid entries.

        Unknown keys and values of the wrong type are removed from *config*
        so that :meth:`get_config_value` cleanly falls back to defaults.
        Warnings are logged for each issue found.

        Returns True if *config* is valid, False if it should be discarded.
        """
        if isinstance(schema, dict):
            if not isinstance(config, dict):
                logger.warning(
                    "Config: '%s' expected dict, got %s — discarding",
                    path, type(config).__name__,
                )
                config.clear()
                return False
            for key in list(config.keys()):
                full_key = f"{path}.{key}" if path else key
                if key not in schema:
                    logger.warning("Config: unknown key '%s' — removing", full_key)
                    del config[key]
                    continue
                if not self._validate_and_clean(config[key], schema[key], full_key):
                    logger.warning("Config: removing invalid key '%s'", full_key)
                    del config[key]
            # Remove keys whose values became empty dicts after cleaning
            for key in list(config.keys()):
                sub_schema = schema.get(key)
                if isinstance(sub_schema, dict) and config[key] == {}:
                    del config[key]
            return True

        # Leaf: *schema* is a Python type
        if isinstance(config, schema):
            return True
        logger.warning(
            "Config: '%s' expected %s, got %s — falling back to default",
            path, schema.__name__, type(config).__name__,
        )
        return False

    # ------------------------------------------------------------------
    # File watcher
    # ------------------------------------------------------------------

    def _setup_watcher(self):
        self._watcher = QFileSystemWatcher(self)
        if os.path.exists(RC_FILE):
            self._watcher.addPath(RC_FILE)
        self._watcher.fileChanged.connect(self._on_file_changed)

    def _on_file_changed(self, path):
        if not os.path.exists(path):
            return
        with self._lock:
            try:
                self._load_rc_file()
            except Exception as e:
                logger.warning("Config: error reloading %s: %s", path, e)
                return
            finally:
                if path not in self._watcher.files():
                    self._watcher.addPath(path)
            self.config_reloaded.emit()

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def get_config_value(self, config_type, key_path, default=None):
        """Look up a config value, falling back to built-in defaults then *default*."""
        self._lazy_load()
        keys = key_path.split(".")
        with self._lock:
            config = self._config.get(config_type, {})
            fallback = self._defaults.get(config_type, {})
            for source in (config, fallback):
                value = source
                try:
                    for key in keys:
                        value = value[key]
                    return value
                except (KeyError, TypeError):
                    continue
            return default

    def get_section(self, config_type):
        """Return a deep copy of the entire user-config section."""
        self._lazy_load()
        with self._lock:
            return deepcopy(self._config.get(config_type, {}))

    def update_section(self, config_type, data):
        """Replace an entire config section and persist to disk."""
        self._lazy_load()
        with self._lock:
            self._config[config_type] = data
            self._write_rc_file(self._config)
        self.config_reloaded.emit()

    def get_default_value(self, config_type, key_path, default=None):
        """Read a value from the built-in defaults only, ignoring user config.

        This is useful for operations like "reset to default" where you
        want the original fallback value regardless of what the user has set.
        """
        self._lazy_load()
        keys = key_path.split(".")
        with self._lock:
            fallback = self._defaults.get(config_type, {})
            value = fallback
            try:
                for key in keys:
                    value = value[key]
                return value
            except (KeyError, TypeError):
                return default

    def reset_config(self):
        """Reset the configuration to defaults and rewrite the RC file."""
        self._lazy_load()
        with self._lock:
            self._config = {}
            self._write_rc_file(self._config)
        logger.info("Config: reset to defaults.")
        self.config_reloaded.emit()


config_manager = ConfigManager()
