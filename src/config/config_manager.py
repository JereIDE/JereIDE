import json
import os
import threading
from copy import deepcopy

from PySide6.QtCore import QFileSystemWatcher, QObject, Signal

from const.paths import ROOT_DIR

RC_FILE = os.path.expanduser("~/.jereiderc")
OLD_CONFIG_DIR = os.path.join(ROOT_DIR, "src", "config")


class ConfigManager(QObject):
    config_reloaded = Signal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self._lock = threading.Lock()
        self._config = {}
        self._defaults = {}
        self._watcher = None
        self._load_defaults()
        self._ensure_rc_file()
        self._load_rc_file()
        self._setup_watcher()

    def _load_defaults(self):
        defaults_path = os.path.join(OLD_CONFIG_DIR, "defaults.json")
        if os.path.exists(defaults_path):
            with open(defaults_path, "r") as f:
                self._defaults = json.load(f)

    def _ensure_rc_file(self):
        if os.path.exists(RC_FILE):
            return
        config = {}
        for key in ("theme", "editor", "tasks"):
            old_path = os.path.join(OLD_CONFIG_DIR, f"{key}.json")
            if os.path.exists(old_path):
                with open(old_path, "r") as f:
                    config[key] = json.load(f)
        self._write_rc_file(config)

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

    def _load_rc_file(self):
        if not os.path.exists(RC_FILE):
            with self._lock:
                self._config = {}
            return
        with open(RC_FILE, "r") as f:
            raw = json.load(f)
        with self._lock:
            self._config = {k: v for k, v in raw.items() if not k.startswith("_")}

    def _setup_watcher(self):
        self._watcher = QFileSystemWatcher(self)
        if os.path.exists(RC_FILE):
            self._watcher.addPath(RC_FILE)
        self._watcher.fileChanged.connect(self._on_file_changed)

    def _on_file_changed(self, path):
        if os.path.exists(path):
            try:
                self._load_rc_file()
            except Exception:
                return
            finally:
                if path not in self._watcher.files():
                    self._watcher.addPath(path)
            self.config_reloaded.emit()

    def get_config_value(self, config_type, key_path, default=None):
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
        with self._lock:
            return deepcopy(self._config.get(config_type, {}))

    def update_section(self, config_type, data):
        with self._lock:
            self._config[config_type] = data
        self._write_rc_file(self._config)
        self.config_reloaded.emit()


config_manager = ConfigManager()
