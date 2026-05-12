import sys
import objc
from PySide6.QtCore import QTimer, Qt

if sys.platform == "darwin":
    from AppKit import (
        NSApplication,
        NSWindow,
        NSToolbar,
        NSToolbarItem,
        NSImage,
        NSObject,
        NSSegmentedControl,
        NSTitlebarAccessoryViewController,
        NSView,
        NSLayoutConstraint,
        NSSegmentSwitchTrackingSelectOne,
        NSSegmentStyleSeparated,
        NSControlSizeRegular,
        NSWindowStyleMaskTitled,
        NSWindowStyleMaskClosable,
        NSWindowStyleMaskMiniaturizable,
        NSWindowStyleMaskResizable,
        NSWindowStyleMaskFullSizeContentView,
        NSBackingStoreBuffered,
        NSToolbarDisplayModeIconOnly,
        NSWindowTitleHidden,
    )
    from Foundation import NSObject as NSObj, NSRect


class ViewOptionsController(NSObj):

    def init(self):
        self = objc.super(ViewOptionsController, self).init()
        if self is None:
            return None
        self._callback = None
        return self

    @objc.python_method
    def set_callback(self, func):
        self._callback = func

    def viewOptionSelected_(self, sender):
        selected = sender.selectedSegment()
        if self._callback:
            self._callback(selected)


class ToolbarController(NSObj):

    def init(self):
        self = objc.super(ToolbarController, self).init()
        if self is None:
            return None
        self._view_options_controller = ViewOptionsController.alloc().init()
        return self

    @objc.python_method
    def set_view_callback(self, func):
        self._view_options_controller.set_callback(func)

    def create_segmented_control(self):
        segmented = NSSegmentedControl.alloc().initWithFrame_(((0, 0), (72, 28)))
        segmented.setTrackingMode_(NSSegmentSwitchTrackingSelectOne)
        segmented.setSegmentCount_(2)

        grid_symbol = NSImage.imageWithSystemSymbolName_accessibilityDescription_(
            "square.grid.2x2", None
        )
        list_symbol = NSImage.imageWithSystemSymbolName_accessibilityDescription_(
            "list.bullet", None
        )

        segmented.setImage_forSegment_(grid_symbol, 0)
        segmented.setWidth_forSegment_(36, 0)
        segmented.setToolTip_forSegment_("Gallery View", 0)

        segmented.setImage_forSegment_(list_symbol, 1)
        segmented.setWidth_forSegment_(36, 1)
        segmented.setToolTip_forSegment_("List View", 1)

        segmented.setSegmentStyle_(NSSegmentStyleSeparated)
        segmented.setControlSize_(NSControlSizeRegular)
        segmented.setSelectedSegment_(0)

        segmented.setTarget_(self._view_options_controller)
        segmented.setAction_("viewOptionSelected:")

        segmented.setTranslatesAutoresizingMaskIntoConstraints_(False)

        return segmented


class ToolbarDelegate(NSObj):

    def init(self):
        self = objc.super(ToolbarDelegate, self).init()
        if self is None:
            return None
        self._toolbar_identifiers = ["FlexibleSpace"]
        return self

    def toolbarAllowedItemIdentifiers_(self, toolbar):
        return self._toolbar_identifiers

    def toolbarDefaultItemIdentifiers_(self, toolbar):
        return self._toolbar_identifiers

    def toolbarSelectableItemIdentifiers_(self, toolbar):
        return self._toolbar_identifiers

    def toolbar_itemForItemIdentifier_willBeInsertedIntoToolbar_(self, toolbar, item_identifier, flag):
        if item_identifier == "FlexibleSpace":
            space_item = NSToolbarItem.alloc().initWithItemIdentifier_(item_identifier)
            return space_item
        return None


def attach_native_toolbar(window_id: str, callback=None):
    if sys.platform != "darwin":
        return

    app = NSApplication.sharedApplication()
    for window in app.windows():
        if window.title() == window_id:
            window.setTitleVisibility_(NSWindowTitleHidden)
            window.setStyleMask_(
                window.styleMask() | NSWindowStyleMaskFullSizeContentView
            )

            toolbar_controller = ToolbarController.alloc().init()
            if callback:
                toolbar_controller.set_view_callback(callback)

            segmented = toolbar_controller.create_segmented_control()

            accessory_view = NSView.alloc().initWithFrame_(((0, 0), (84, 40)))
            accessory_view.addSubview_(segmented)

            NSLayoutConstraint.activateConstraints_([
                segmented.centerXAnchor().constraintEqualToAnchor_(accessory_view.centerXAnchor()),
                segmented.centerYAnchor().constraintEqualToAnchor_(accessory_view.centerYAnchor()),
                segmented.widthAnchor().constraintEqualToConstant_(72),
                segmented.heightAnchor().constraintEqualToConstant_(28),
            ])

            accessory_controller = NSTitlebarAccessoryViewController.alloc().init()
            accessory_controller.setView_(accessory_view)
            accessory_controller.setLayoutAttribute_(1)

            window.addTitlebarAccessoryViewController_(accessory_controller)

            toolbar = NSToolbar.alloc().initWithIdentifier_("MainToolbar")
            delegate = ToolbarDelegate.alloc().init()
            toolbar.setDelegate_(delegate)
            toolbar.setDisplayMode_(NSToolbarDisplayModeIconOnly)
            window.setToolbar_(toolbar)

            return toolbar_controller
    return None
