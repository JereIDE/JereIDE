from PySide6.QtWidgets import QFrame


class TabDragManager:
    def __init__(self, notebook):
        self._notebook = notebook

        self.source_index = -1
        self._dragged_tab = None
        self._dragged_page = None
        self._dragged_tab_width = 0
        self.ghost_index = -1
        self._spacer = None
        self.completed = False
        self.drop_index = -1

    def on_drag_started(self, source_index: int) -> None:
        nb = self._notebook
        self.source_index = source_index
        self._dragged_tab = nb._tabs[source_index]
        self._dragged_page = nb._stacked_widget.widget(source_index)
        self._dragged_tab_width = self._dragged_tab.width()

        self._dragged_tab.setVisible(False)

        self._spacer = QFrame(nb._tabs_container)
        self._spacer.setFixedWidth(self._dragged_tab_width)
        self._spacer.setVisible(False)

        self.ghost_index = -1
        self.completed = False

        nb._update_container_min_width()

    def update_ghost(self, ghost_index: int) -> None:
        if ghost_index == self.ghost_index:
            return

        nb = self._notebook

        if self._spacer and self._spacer.isVisible():
            nb._tabs_container_layout.removeWidget(self._spacer)
            self._spacer.setVisible(False)

        self.ghost_index = ghost_index

        if ghost_index < 0 or not self._spacer:
            nb._update_container_min_width()
            return

        nb._tabs_container_layout.insertWidget(ghost_index, self._spacer)
        self._spacer.setVisible(True)
        nb._update_container_min_width()

    def on_drop(self, target_index: int) -> None:
        nb = self._notebook
        source = self.source_index

        if target_index < 0:
            self.on_drag_cancelled()
            return

        if target_index == source or target_index == source + 1:
            self._dragged_tab.setVisible(True)
            self._clear_state()
            return

        if self._spacer and self._spacer.isVisible():
            nb._tabs_container_layout.removeWidget(self._spacer)
            self._spacer.setVisible(False)

        tab = nb._tabs.pop(source)
        nb._tabs_container_layout.removeWidget(tab)
        nb._stacked_widget.removeWidget(self._dragged_page)

        adjusted = target_index
        if adjusted > source:
            adjusted -= 1

        nb._tabs.insert(adjusted, tab)
        nb._tabs_container_layout.insertWidget(adjusted, tab)
        nb._stacked_widget.insertWidget(adjusted, self._dragged_page)

        tab.setVisible(True)

        for i, t in enumerate(nb._tabs):
            t.index = i

        nb._current_selection = adjusted
        nb.SelectTab(adjusted)

        self._clear_state()

    def on_drag_cancelled(self) -> None:
        nb = self._notebook

        if self._spacer and self._spacer.isVisible():
            nb._tabs_container_layout.removeWidget(self._spacer)
            self._spacer.setVisible(False)

        if self._dragged_tab:
            self._dragged_tab.setVisible(True)

        self._clear_state()

    def _clear_state(self) -> None:
        if self._spacer:
            self._spacer.deleteLater()
            self._spacer = None

        self.source_index = -1
        self._dragged_tab = None
        self._dragged_page = None
        self._dragged_tab_width = 0
        self.ghost_index = -1
        self.completed = False
        self.drop_index = -1

        nb = self._notebook
        nb._update_container_min_width()
        nb._update_arrow_states()

    def get_drop_index(self, event) -> int:
        x = int(event.position().x())
        tabs = self._notebook._tabs
        count = len(tabs)
        for i in range(count):
            tab = tabs[i]
            if not tab.isVisible():
                continue
            tab_center = tab.x() + tab.width() // 2
            if x < tab_center:
                return i
        return count

    def get_indicator_x(self) -> int:
        di = self.drop_index
        if di < 0:
            return 0
        tabs = self._notebook._tabs
        if di < len(tabs):
            return tabs[di].x()
        for tab in reversed(tabs):
            if tab.isVisible():
                return tab.x() + tab.width()
        return 0
