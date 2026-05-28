from __future__ import annotations

from PySide6.QtCore import Qt, Signal, QTimer
from PySide6.QtWidgets import (
    QHBoxLayout,
    QStackedWidget,
    QVBoxLayout,
    QWidget,
    QFrame,
    QScrollArea,
)

from const.theme import TAB_STRIP_BG, TAB_BORDER
from .tabScrollArrow import TabScrollArrow
from .tabWidget import JereIDETab
from .tabDropContainer import TabDropContainer
from .tabDragManager import TabDragManager


class JereIDEBook(QWidget):
    """A notebook widget that manages multiple tabs with closeable tab headers."""

    page_changed = Signal(int)
    page_close_requested = Signal(int)

    def __init__(self, parent: QWidget):
        super().__init__(parent)
        self._tabs: list[JereIDETab] = []
        self._current_selection = -1

        self._drag_mgr = TabDragManager(self)

        main_layout = QVBoxLayout(self)
        main_layout.setContentsMargins(0, 0, 0, 0)
        main_layout.setSpacing(0)

        self._tab_bar_widget = QFrame()
        self._tab_bar_widget.setFixedHeight(30)
        self._tab_bar_widget.setStyleSheet(
            f"QFrame {{ background-color: {TAB_STRIP_BG}; border-bottom: 1px solid {TAB_BORDER}; }}"
        )
        self._tab_bar_layout = QHBoxLayout(self._tab_bar_widget)
        self._tab_bar_layout.setContentsMargins(0, 0, 0, 0)
        self._tab_bar_layout.setSpacing(0)

        self._left_arrow = TabScrollArrow(self._tab_bar_widget, True)
        self._left_arrow.clicked.connect(self._on_scroll_arrow_clicked)
        self._tab_bar_layout.addWidget(self._left_arrow)

        self._right_arrow = TabScrollArrow(self._tab_bar_widget, False)
        self._right_arrow.clicked.connect(self._on_scroll_arrow_clicked)
        self._tab_bar_layout.addWidget(self._right_arrow)

        self._tabs_container = TabDropContainer(self._drag_mgr)
        self._tabs_container_layout = QHBoxLayout(self._tabs_container)
        self._tabs_container_layout.setContentsMargins(0, 0, 0, 0)
        self._tabs_container_layout.setSpacing(0)
        self._tabs_container_layout.addStretch()

        self._scroll_area = QScrollArea()
        self._scroll_area.setWidget(self._tabs_container)
        self._scroll_area.setWidgetResizable(True)
        self._scroll_area.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self._scroll_area.setVerticalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self._scroll_area.setStyleSheet("QScrollArea { border: none; background: transparent; }")
        self._scroll_area.horizontalScrollBar().valueChanged.connect(self._update_arrow_states)

        self._tab_bar_layout.addWidget(self._scroll_area, 1)

        self._stacked_widget = QStackedWidget()
        self._stacked_widget.setStyleSheet("QStackedWidget { border: none; background-color: white; }")

        main_layout.addWidget(self._tab_bar_widget)
        main_layout.addWidget(self._stacked_widget, 1)

    def GetSelection(self) -> int:
        """Return the currently selected page index, or -1."""
        return self._current_selection

    def GetPage(self, index: int) -> QWidget | None:
        """Return the page widget at the given index."""
        if 0 <= index < self._stacked_widget.count():
            return self._stacked_widget.widget(index)
        return None

    def AddPage(self, page_widget: QWidget, title: str, select: bool = True) -> bool:
        """Add a new page to the notebook."""
        index = len(self._tabs)
        tab = JereIDETab(self._tabs_container, title, index, self)
        tab.clicked.connect(self._on_tab_clicked)
        tab.close_clicked.connect(self._on_tab_close_clicked)

        self._tabs.append(tab)
        insert_position = self._tabs_container_layout.count() - 1
        self._tabs_container_layout.insertWidget(insert_position, tab)

        self._stacked_widget.addWidget(page_widget)

        if self._current_selection == -1 or select:
            self.SelectTab(index)
        else:
            self._update_container_min_width()
            self._scroll_to_tab(index)

        self._update_arrow_states()
        self._tab_bar_widget.show()
        return True

    def SetPageText(self, index: int, title: str) -> bool:
        """Set the title of the tab at the given index."""
        if 0 <= index < len(self._tabs):
            self._tabs[index].set_label(title)
            return True
        return False

    def SetPageModified(self, index: int, modified: bool) -> bool:
        """Set the modified state of the tab at the given index."""
        if 0 <= index < len(self._tabs):
            self._tabs[index].set_modified(modified)
            return True
        return False

    def GetPageIndex(self, page: QWidget) -> int:
        """Return the index of the given page, or -1."""
        index = self._stacked_widget.indexOf(page)
        return index if index >= 0 else -1

    def SetSelection(self, index: int) -> int:
        """Set the selection to the page at the given index. Returns previous selection."""
        old_selection = self._current_selection
        self.SelectTab(index)
        return old_selection

    def GetPageCount(self) -> int:
        """Return the number of pages."""
        return len(self._tabs)

    def DeletePage(self, index: int) -> None:
        """Delete the page at the given index."""
        if 0 <= index < len(self._tabs):
            self.page_close_requested.emit(index)

    def _scroll_to_tab(self, index: int) -> None:
        """Scroll the tab at the given index into view."""
        if 0 <= index < len(self._tabs):
            tab = self._tabs[index]
            self._update_container_min_width()
            QTimer.singleShot(50, lambda t=tab: self._do_scroll_to_tab(t))

    def _update_container_min_width(self) -> None:
        """Update the container's minimum width to allow scrolling."""
        total_width = sum(t.width() for t in self._tabs) if self._tabs else 0
        spacer = self._drag_mgr._spacer
        if spacer and spacer.isVisible():
            total_width += spacer.width()
        self._tabs_container.setMinimumWidth(total_width)

    def _do_scroll_to_tab(self, tab: JereIDETab) -> None:
        scroll_bar = self._scroll_area.horizontalScrollBar()
        viewport_width = self._scroll_area.viewport().width()
        x = tab.x()
        width = tab.width()
        current_value = scroll_bar.value()
        max_value = scroll_bar.maximum()

        if x < current_value:
            scroll_bar.setValue(max(0, x))
        elif x + width > current_value + viewport_width:
            scroll_bar.setValue(min(max_value, x + width - viewport_width))

    def SelectTab(self, index: int) -> None:
        """Select the tab at the given index."""
        for i, tab in enumerate(self._tabs):
            tab.is_selected = (i == index)
            tab.update()

        if 0 <= index < self._stacked_widget.count():
            self._stacked_widget.setCurrentIndex(index)
            self._current_selection = index
            self.page_changed.emit(index)

        self._scroll_to_tab(index)
        self._update_arrow_states()

    def CloseTab(self, index: int) -> None:
        """Close and remove the tab at the given index."""
        if index < 0 or index >= len(self._tabs):
            return

        tab = self._tabs.pop(index)
        page = self._stacked_widget.widget(index)

        self._tabs_container_layout.removeWidget(tab)
        tab.deleteLater()
        self._stacked_widget.removeWidget(page)
        page.deleteLater()

        for i, remaining_tab in enumerate(self._tabs):
            remaining_tab.index = i

        if self._current_selection >= len(self._tabs):
            self._current_selection = len(self._tabs) - 1

        if self._tabs:
            self.SelectTab(self._current_selection)
        else:
            self._current_selection = -1
            self._tab_bar_widget.hide()

        self._update_arrow_states()

    def _on_tab_clicked(self, index: int) -> None:
        """Handle tab click events."""
        self.SelectTab(index)

    def _on_tab_close_clicked(self, index: int) -> None:
        """Handle tab close button click events."""
        self.page_close_requested.emit(index)

    def _on_scroll_arrow_clicked(self, left: bool) -> None:
        """Handle scroll arrow click events - switch to adjacent tab."""
        if not self._tabs:
            return

        current = self._current_selection

        if left and current > 0:
            self.SelectTab(current - 1)
        elif not left and current < len(self._tabs) - 1:
            self.SelectTab(current + 1)

    def _update_arrow_states(self) -> None:
        """Update enabled state of scroll arrows based on current tab position."""
        has_tabs = bool(self._tabs)
        current = self._current_selection

        self._left_arrow.setEnabled(has_tabs and current > 0)
        self._right_arrow.setEnabled(has_tabs and current < len(self._tabs) - 1)

    def _on_drag_started(self, source_index: int) -> None:
        self._drag_mgr.on_drag_started(source_index)

    @property
    def _drag_completed(self) -> bool:
        return self._drag_mgr.completed

    @_drag_completed.setter
    def _drag_completed(self, value: bool) -> None:
        self._drag_mgr.completed = value

    def _on_drag_cancelled(self) -> None:
        self._drag_mgr.on_drag_cancelled()
