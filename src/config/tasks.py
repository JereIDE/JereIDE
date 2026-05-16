import json
import os
from const.paths import TASKS_PATH


class TaskManager:
    def __init__(self):
        self._tasks = []
        self._load()

    def _load(self):
        if os.path.exists(TASKS_PATH):
            with open(TASKS_PATH, "r") as f:
                data = json.load(f)
                self._tasks = data.get("tasks", [])
        else:
            self._tasks = []

    def save(self):
        os.makedirs(os.path.dirname(TASKS_PATH), exist_ok=True)
        with open(TASKS_PATH, "w") as f:
            json.dump({"tasks": self._tasks}, f, indent=2)

    def get_tasks(self):
        return list(self._tasks)

    def add_task(self, name, command, icon="play.fill"):
        self._tasks.append({"name": name, "command": command, "icon": icon})
        self.save()

    def remove_task(self, index):
        if 0 <= index < len(self._tasks):
            del self._tasks[index]
            self.save()

    def update_task(self, index, name=None, command=None, icon=None):
        if 0 <= index < len(self._tasks):
            if name is not None:
                self._tasks[index]["name"] = name
            if command is not None:
                self._tasks[index]["command"] = command
            if icon is not None:
                self._tasks[index]["icon"] = icon
            self.save()


task_manager = TaskManager()
