import objc
from AppKit import (
    NSApplication,
    NSBezelStyleTexturedRounded,
    NSButton,
    NSImage,
    NSImageOnly,
    NSImageSymbolConfiguration,
    NSToolbar,
    NSToolbarFlexibleSpaceItemIdentifier,
    NSToolbarItem,
    NSSegmentedControl,
    NSTitlebarAccessoryViewController,
    NSView,
    NSLayoutConstraint,
    NSSegmentSwitchTrackingSelectOne,
    NSControlSizeRegular,
    NSWindowStyleMaskFullSizeContentView,
    NSToolbarDisplayModeIconOnly,
    NSWindowTitleHidden,
)
from Foundation import NSObject as NSObj


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


class RunButtonController(NSObj):

    def init(self):
        self = objc.super(RunButtonController, self).init()
        if self is None:
            return None
        return self

    def runAction_(self, sender):
        print("Run button clicked (dummy action)")


class ToolbarController(NSObj):

    def init(self):
        self = objc.super(ToolbarController, self).init()
        if self is None:
            return None
        self._view_options_controller = ViewOptionsController.alloc().init()
        self._run_controller = RunButtonController.alloc().init()
        return self

    @objc.python_method
    def set_view_callback(self, func):
        self._view_options_controller.set_callback(func)

    @objc.python_method
    def get_run_controller(self):
        return self._run_controller



    def create_segmented_control(self):
        segmented = NSSegmentedControl.alloc().initWithFrame_(((0, 0), (72, 28)))
        segmented.setTrackingMode_(NSSegmentSwitchTrackingSelectOne)
        segmented.setSegmentCount_(2)

        code_symbol = NSImage.imageWithSystemSymbolName_accessibilityDescription_(
            "chevron.left.slash.chevron.right", None
        )
        command_symbol = NSImage.imageWithSystemSymbolName_accessibilityDescription_(
            "wand.and.stars", None
        )

        segmented.setImage_forSegment_(code_symbol, 0)
        segmented.setWidth_forSegment_(36, 0)
        segmented.setToolTip_forSegment_("Code View", 0)

        segmented.setImage_forSegment_(command_symbol, 1)
        segmented.setWidth_forSegment_(36, 1)
        segmented.setToolTip_forSegment_("Command View", 1)

        #segmented.setSegmentStyle_(NSSegmentStyleSeparated)
        segmented.setControlSize_(NSControlSizeRegular)
        segmented.setSelectedSegment_(0)

        segmented.setTarget_(self._view_options_controller)
        segmented.setAction_("viewOptionSelected:")

        segmented.setTranslatesAutoresizingMaskIntoConstraints_(False)

        return segmented


class ToolbarDelegate(NSObj):

    def initWithToolbarController_(self, controller):
        self = objc.super(ToolbarDelegate, self).init()
        if self is None:
            return None
        self._toolbar_controller = controller
        self._identifiers = [NSToolbarFlexibleSpaceItemIdentifier, "RunScript"]
        self._run_item = None
        return self

    def toolbarAllowedItemIdentifiers_(self, toolbar):
        return self._identifiers

    def toolbarDefaultItemIdentifiers_(self, toolbar):
        return [NSToolbarFlexibleSpaceItemIdentifier, "RunScript"]

    def toolbarSelectableItemIdentifiers_(self, toolbar):
        return []

    def toolbar_itemForItemIdentifier_willBeInsertedIntoToolbar_(self, toolbar, item_identifier, flag):
        if item_identifier == "RunScript":
            if self._run_item is None:
                run_image = NSImage.imageWithSystemSymbolName_accessibilityDescription_(
                    "play.fill", None
                )
                config = NSImageSymbolConfiguration.configurationWithPointSize_weight_scale_(14, 4, 1)
                run_image = run_image.imageWithSymbolConfiguration_(config)

                button = NSButton.alloc().initWithFrame_(((0, 0), (36, 28)))
                button.setBezelStyle_(NSBezelStyleTexturedRounded)
                button.setImage_(run_image)
                button.setImagePosition_(NSImageOnly)
                button.setToolTip_("Run script")
                button.setTarget_(self._toolbar_controller.get_run_controller())
                button.setAction_("runAction:")

                self._run_item = NSToolbarItem.alloc().initWithItemIdentifier_(item_identifier)
                self._run_item.setLabel_("Run")
                self._run_item.setPaletteLabel_("Run Script")
                self._run_item.setToolTip_("Run the current script")
                self._run_item.setView_(button)
                self._run_item.setMinSize_((36, 28))
                self._run_item.setMaxSize_((36, 28))
            return self._run_item
        return None


def attach_native_toolbar(window_id: str, callback=None):
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

            accessory_view = NSView.alloc().initWithFrame_(((0, 0), (96, 40)))
            accessory_view.addSubview_(segmented)

            NSLayoutConstraint.activateConstraints_([
                segmented.leadingAnchor().constraintEqualToAnchor_constant_(accessory_view.leadingAnchor(), 12),
                segmented.centerYAnchor().constraintEqualToAnchor_(accessory_view.centerYAnchor()),
                segmented.widthAnchor().constraintEqualToConstant_(72),
                segmented.heightAnchor().constraintEqualToConstant_(28),
            ])

            accessory_controller = NSTitlebarAccessoryViewController.alloc().init()
            accessory_controller.setView_(accessory_view)
            accessory_controller.setLayoutAttribute_(1)

            window.addTitlebarAccessoryViewController_(accessory_controller)

            toolbar = NSToolbar.alloc().initWithIdentifier_("MainToolbar")
            delegate = ToolbarDelegate.alloc().initWithToolbarController_(toolbar_controller)
            toolbar.setDelegate_(delegate)
            toolbar.setDisplayMode_(NSToolbarDisplayModeIconOnly)
            toolbar.setAllowsUserCustomization_(True)
            window.setToolbar_(toolbar)

            return toolbar_controller, segmented
    return None, None
