from config.config_manager import config_manager


class TaskManager:
    def __init__(self):
        self._tasks = []
        self._load()

    def _load(self):
        tasks_section = config_manager.get_section("tasks")
        self._tasks = tasks_section.get("tasks", [])

    def save(self):
        config_manager.update_section("tasks", {"tasks": self._tasks})

    def get_tasks(self):
        return list(self._tasks)

    def add_task(self, name, command):
        self._tasks.append({"name": name, "command": command})
        self.save()

    def remove_task(self, index):
        if 0 <= index < len(self._tasks):
            del self._tasks[index]
            self.save()

    def update_task(self, index, name=None, command=None):
        if 0 <= index < len(self._tasks):
            if name is not None:
                self._tasks[index]["name"] = name
            if command is not None:
                self._tasks[index]["command"] = command
            self.save()


task_manager = TaskManager()
