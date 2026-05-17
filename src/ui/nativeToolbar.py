import objc
from AppKit import (
    NSApplication,
    NSBezelStyleTexturedRounded,
    NSButton,
    NSImage,
    NSImageOnly,
    NSImageSymbolConfiguration,
    NSMenuItem,
    NSPopUpButton,
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
        self._viewCallback = None
        return self

    @objc.python_method
    def set_callback(self, viewCallback):
        self._viewCallback = viewCallback

    def viewOptionSelected_(self, sender):
        selectedSegment = sender.selectedSegment()
        if self._viewCallback:
            self._viewCallback(selectedSegment)


class RunButtonController(NSObj):

    def init(self):
        self = objc.super(RunButtonController, self).init()
        if self is None:
            return None
        self._runCallback = None
        return self

    @objc.python_method
    def set_callback(self, runCallback):
        self._runCallback = runCallback

    def runAction_(self, sender):
        if self._runCallback:
            self._runCallback()


class PopupButtonController(NSObj):

    def init(self):
        self = objc.super(PopupButtonController, self).init()
        if self is None:
            return None
        self._popupCallback = None
        return self

    @objc.python_method
    def set_callback(self, popupCallback):
        self._popupCallback = popupCallback

    def menuItemSelected_(self, sender):
        if self._popupCallback:
            selectedItem = sender.selectedItem()
            title = selectedItem.title()
            self._popupCallback(title)


class ToolbarController(NSObj):

    def init(self):
        self = objc.super(ToolbarController, self).init()
        if self is None:
            return None
        self._viewOptionsController = ViewOptionsController.alloc().init()
        self._runButtonController = RunButtonController.alloc().init()
        self._popupButtonController = PopupButtonController.alloc().init()
        return self

    @objc.python_method
    def set_view_callback(self, viewCallback):
        self._viewOptionsController.set_callback(viewCallback)

    @objc.python_method
    def set_run_callback(self, runCallback):
        self._runButtonController.set_callback(runCallback)

    @objc.python_method
    def set_popup_callback(self, popupCallback):
        self._popupButtonController.set_callback(popupCallback)

    @objc.python_method
    def get_run_controller(self):
        return self._runButtonController

    @objc.python_method
    def create_project_selector(self):
        projectButton = NSPopUpButton.alloc().initWithFrame_(((0, 0), (120, 28)))
        projectButton.setBezelStyle_(NSBezelStyleTexturedRounded)
        projectButton.addItemWithTitle_("Project 1")
        projectButton.addItemWithTitle_("Project 2")
        projectButton.addItemWithTitle_("Project 3")
        projectButton.selectItemAtIndex_(0)
        projectButton.setTarget_(self._popupButtonController)
        projectButton.setAction_("menuItemSelected:")
        projectButton.setTranslatesAutoresizingMaskIntoConstraints_(False)
        return projectButton



    def create_segmented_control(self):
        viewSegmentedControl = NSSegmentedControl.alloc().initWithFrame_(((0, 0), (72, 28)))
        viewSegmentedControl.setTrackingMode_(NSSegmentSwitchTrackingSelectOne)
        viewSegmentedControl.setSegmentCount_(2)

        codeViewSymbol = NSImage.imageWithSystemSymbolName_accessibilityDescription_(
            "chevron.left.slash.chevron.right", None
        )
        commandViewSymbol = NSImage.imageWithSystemSymbolName_accessibilityDescription_(
            "wand.and.stars", None
        )

        viewSegmentedControl.setImage_forSegment_(codeViewSymbol, 0)
        viewSegmentedControl.setWidth_forSegment_(36, 0)
        viewSegmentedControl.setToolTip_forSegment_("Code View", 0)

        viewSegmentedControl.setImage_forSegment_(commandViewSymbol, 1)
        viewSegmentedControl.setWidth_forSegment_(36, 1)
        viewSegmentedControl.setToolTip_forSegment_("Command View", 1)

        viewSegmentedControl.setControlSize_(NSControlSizeRegular)
        viewSegmentedControl.setSelectedSegment_(0)

        viewSegmentedControl.setTarget_(self._viewOptionsController)
        viewSegmentedControl.setAction_("viewOptionSelected:")

        viewSegmentedControl.setTranslatesAutoresizingMaskIntoConstraints_(False)

        return viewSegmentedControl


class ToolbarDelegate(NSObj):

    def initWithToolbarController_(self, toolbarController):
        self = objc.super(ToolbarDelegate, self).init()
        if self is None:
            return None
        self._toolbarController = toolbarController
        self._toolbarItemIdentifiers = [NSToolbarFlexibleSpaceItemIdentifier, "RunScript"]
        self._runItem = None
        return self

    def toolbarAllowedItemIdentifiers_(self, mainToolbar):
        return self._toolbarItemIdentifiers

    def toolbarDefaultItemIdentifiers_(self, mainToolbar):
        return [NSToolbarFlexibleSpaceItemIdentifier, "RunScript"]

    def toolbarSelectableItemIdentifiers_(self, mainToolbar):
        return []

    def toolbar_itemForItemIdentifier_willBeInsertedIntoToolbar_(self, mainToolbar, itemIdentifier, flag):
        if itemIdentifier == "RunScript":
            if self._runItem is None:
                runButtonImage = NSImage.imageWithSystemSymbolName_accessibilityDescription_(
                    "play.fill", None
                )
                symbolConfiguration = NSImageSymbolConfiguration.configurationWithPointSize_weight_scale_(14, 4, 1)
                runButtonImage = runButtonImage.imageWithSymbolConfiguration_(symbolConfiguration)

                runButton = NSButton.alloc().initWithFrame_(((0, 0), (36, 28)))
                runButton.setBezelStyle_(NSBezelStyleTexturedRounded)
                runButton.setImage_(runButtonImage)
                runButton.setImagePosition_(NSImageOnly)
                runButton.setToolTip_("Run script")
                runButton.setTarget_(self._toolbarController.get_run_controller())
                runButton.setAction_("runAction:")

                self._runItem = NSToolbarItem.alloc().initWithItemIdentifier_(itemIdentifier)
                self._runItem.setLabel_("Run")
                self._runItem.setPaletteLabel_("Run Script")
                self._runItem.setToolTip_("Run the current script")
                self._runItem.setView_(runButton)
                self._runItem.setMinSize_((36, 28))
                self._runItem.setMaxSize_((36, 28))
            return self._runItem
        return None


def attach_native_toolbar(windowId: str, viewCallback=None, runCallback=None, popupCallback=None):
    app = NSApplication.sharedApplication()
    for mainWindow in app.windows():
        if mainWindow.title() == windowId:
            mainWindow.setTitleVisibility_(NSWindowTitleHidden)
            mainWindow.setStyleMask_(
                mainWindow.styleMask() | NSWindowStyleMaskFullSizeContentView
            )

            toolbarController = ToolbarController.alloc().init()
            if viewCallback:
                toolbarController.set_view_callback(viewCallback)
            if runCallback:
                toolbarController.set_run_callback(runCallback)
            if popupCallback:
                toolbarController.set_popup_callback(popupCallback)

            viewSegmentedControl = toolbarController.create_segmented_control()
            projectButton = toolbarController.create_project_selector()

            titlebarAccessoryView = NSView.alloc().initWithFrame_(((0, 0), (224, 40)))
            titlebarAccessoryView.addSubview_(projectButton)
            titlebarAccessoryView.addSubview_(viewSegmentedControl)

            NSLayoutConstraint.activateConstraints_([
                projectButton.leadingAnchor().constraintEqualToAnchor_constant_(titlebarAccessoryView.leadingAnchor(), 12),
                projectButton.centerYAnchor().constraintEqualToAnchor_(titlebarAccessoryView.centerYAnchor()),
                projectButton.widthAnchor().constraintEqualToConstant_(120),
                projectButton.heightAnchor().constraintEqualToConstant_(28),
                viewSegmentedControl.leadingAnchor().constraintEqualToAnchor_constant_(projectButton.trailingAnchor(), 8),
                viewSegmentedControl.centerYAnchor().constraintEqualToAnchor_(titlebarAccessoryView.centerYAnchor()),
                viewSegmentedControl.widthAnchor().constraintEqualToConstant_(72),
                viewSegmentedControl.heightAnchor().constraintEqualToConstant_(28),
            ])

            accessoryViewController = NSTitlebarAccessoryViewController.alloc().init()
            accessoryViewController.setView_(titlebarAccessoryView)
            accessoryViewController.setLayoutAttribute_(1)

            mainWindow.addTitlebarAccessoryViewController_(accessoryViewController)

            mainToolbar = NSToolbar.alloc().initWithIdentifier_("MainToolbar")
            toolbarDelegate = ToolbarDelegate.alloc().initWithToolbarController_(toolbarController)
            mainToolbar.setDelegate_(toolbarDelegate)
            mainToolbar.setDisplayMode_(NSToolbarDisplayModeIconOnly)
            mainToolbar.setAllowsUserCustomization_(True)
            mainWindow.setToolbar_(mainToolbar)

            return toolbarController, viewSegmentedControl
    return None, None
