import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

ApplicationWindow {
    id: window
    width: 1460
    height: 900
    minimumWidth: 940
    minimumHeight: 660
    visible: true
    title: "ClipRelay"
    flags: Qt.Window
        | Qt.FramelessWindowHint
    color: "transparent"
    palette.window: Theme.surface
    palette.windowText: Theme.text
    palette.base: Theme.raised
    palette.alternateBase: Theme.active
    palette.text: Theme.text
    palette.button: Theme.raised
    palette.buttonText: Theme.text
    palette.highlight: Theme.accent
    palette.highlightedText: Theme.accentContent
    palette.placeholderText: Theme.muted
    palette.mid: Theme.border
    palette.dark: Theme.isLight ? Theme.borderStrong : Theme.ink
    palette.light: Theme.isLight ? Theme.surface : Theme.muted
    palette.link: Theme.accent

    property int currentPage: 0
    property string requestedThemeMode: String(
        controller.settings.theme_mode || "relay"
    )
    property string themeMode: requestedThemeMode === "pitch_black"
        || requestedThemeMode === "full_white" ? requestedThemeMode : "relay"
    property real uiScale: Math.max(
        0.8,
        Math.min(1.0, Number(controller.settings.ui_scale || 1.0))
    )
    property bool narrowWindow: width / uiScale < 1080
    property bool navCollapsed: narrowWindow || Boolean(controller.settings.sidebar_collapsed)
    readonly property int titleBarHeight: nativeWindow.fullscreen
        ? 0 : Theme.workbenchTitleHeight

    Binding {
        target: Theme
        property: "mode"
        value: window.themeMode
    }

    Shortcut {
        sequence: Qt.platform.os === "osx" ? "Meta+1" : "Ctrl+1"
        onActivated: window.currentPage = 0
    }
    Shortcut {
        sequence: Qt.platform.os === "osx" ? "Meta+2" : "Ctrl+2"
        onActivated: window.currentPage = 1
    }
    Shortcut {
        sequence: Qt.platform.os === "osx" ? "Meta+," : "Ctrl+,"
        onActivated: window.currentPage = 2
    }
    Shortcut {
        sequences: [StandardKey.Find]
        onActivated: windowTitleBar.focusSearch()
    }
    Shortcut {
        sequence: Qt.platform.os === "osx" ? "Meta+K" : "Ctrl+K"
        onActivated: windowTitleBar.focusSearch()
    }
    Shortcut {
        sequence: Qt.platform.os === "osx"
            ? "Meta+Shift+P" : "Ctrl+Shift+P"
        onActivated: windowTitleBar.focusCommands()
    }
    Shortcut {
        sequence: Qt.platform.os === "osx" ? "Meta+M" : "Ctrl+M"
        onActivated: nativeWindow.minimizeWindow()
    }
    Shortcut {
        sequence: Qt.platform.os === "osx" ? "Ctrl+Meta+F" : "F11"
        onActivated: nativeWindow.toggleFullscreen()
    }
    Shortcut {
        sequence: "Escape"
        enabled: nativeWindow.fullscreen
        onActivated: nativeWindow.toggleFullscreen()
    }
    Shortcut {
        sequences: [StandardKey.Close]
        onActivated: nativeWindow.closeWindow()
    }

    Connections {
        target: controller
        function onNavigationRequested(page) {
            window.currentPage = page === "history" ? 1 : page === "settings" ? 2 : 0
        }
        function onToast(kind, message) {
            toast.kind = kind
            toast.message = message
            toast.shown = true
            toastTimer.restart()
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: nativeWindow.resizable ? Theme.radiusMd : 0
        color: Theme.ink
    }

    WorkbenchActionRegistry {
        id: workbenchActions
        appController: controller
        libraryPage: libraryPage
        hostWindow: window
        windowController: nativeWindow
    }

    WindowTitleBar {
        id: windowTitleBar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: window.titleBarHeight
        visible: height > 0
        hostWindow: window
        windowController: nativeWindow
        appController: controller
        actionRegistry: workbenchActions
        libraryPage: libraryPage
        z: 9000
    }

    Item {
        id: contentViewport
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: windowTitleBar.bottom
        anchors.bottom: parent.bottom
        clip: true

        Item {
            id: scaledWorkspace
            focus: true
            anchors.left: parent.left
            anchors.top: parent.top
            width: parent.width / window.uiScale
            height: parent.height / window.uiScale
            scale: window.uiScale
            transformOrigin: Item.TopLeft
            Keys.priority: Keys.AfterItem
            Keys.onPressed: function(event) {
                if (
                    window.currentPage !== 0
                    || !libraryPage.commandShortcutsEnabled
                    || event.isAutoRepeat
                ) {
                    return
                }
                const noModifier = event.modifiers === Qt.NoModifier
                const shiftOnly = event.modifiers === Qt.ShiftModifier
                if (event.key === Qt.Key_Space && noModifier) {
                    libraryPage.togglePlayback()
                    event.accepted = true
                } else if (event.key === Qt.Key_Left && noModifier) {
                    libraryPage.navigateSelection(-1)
                    event.accepted = true
                } else if (event.key === Qt.Key_Right && noModifier) {
                    libraryPage.navigateSelection(1)
                    event.accepted = true
                } else if (event.key === Qt.Key_R && noModifier) {
                    controller.pickRandom()
                    event.accepted = true
                } else if (event.key === Qt.Key_R && shiftOnly) {
                    controller.pickPreviousRandom()
                    event.accepted = true
                }
            }

            ColumnLayout {
                anchors.fill: parent
                spacing: 0

                LibraryContextToolbar {
                    id: libraryContextToolbar
                    Layout.fillWidth: true
                    Layout.preferredHeight: Theme.workbenchContextHeight
                    activityWidth: window.navCollapsed ? 68 : 204
                    explorerWidth: 204
                    currentPage: window.currentPage
                    activityCollapsed: window.navCollapsed
                    showFolders: libraryPage.showFolders
                    currentFolder: libraryPage.currentFolder
                    libraryRoot: String(
                        controller.settings.library_root || ""
                    )
                    actionRegistry: workbenchActions
                    appController: controller
                    folderTreeModel: folderModel
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: 0

                    Rectangle {
                        id: sidebarSurface
                        Layout.fillHeight: true
                        Layout.preferredWidth:
                            window.navCollapsed ? 68 : 204
                        color: Theme.surface
                        border.width: 0

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.leftMargin:
                                window.navCollapsed ? 10 : 14
                            anchors.rightMargin:
                                window.navCollapsed ? 10 : 14
                            anchors.topMargin: 8
                            anchors.bottomMargin: 12
                            spacing: 5

                            NavButton {
                                Layout.fillWidth: true
                                text: "Library"
                                iconName: "library"
                                collapsed: window.navCollapsed
                                selected: window.currentPage === 0
                                onClicked: window.currentPage = 0
                            }
                            NavButton {
                                Layout.fillWidth: true
                                text: "History"
                                iconName: "history"
                                collapsed: window.navCollapsed
                                selected: window.currentPage === 1
                                onClicked: window.currentPage = 1
                            }
                            NavButton {
                                Layout.fillWidth: true
                                text: "Settings"
                                iconName: "settings"
                                collapsed: window.navCollapsed
                                selected: window.currentPage === 2
                                onClicked: window.currentPage = 2
                            }
                            Item { Layout.fillHeight: true }

                            NavButton {
                                Layout.fillWidth: true
                                text: window.navCollapsed
                                    ? "Expand sidebar"
                                    : "Collapse sidebar"
                                iconName: window.navCollapsed
                                    ? "chevronRight"
                                    : "chevronLeft"
                                collapsed: window.navCollapsed
                                enabled: !window.narrowWindow
                                toolTipText: window.narrowWindow
                                    ? "Widen the window to expand the sidebar"
                                    : text
                                onClicked:
                                    workbenchActions.triggerAction(
                                        "toggle_sidebar"
                                    )
                            }

                            Rectangle {
                                Layout.fillWidth: true
                                height: 1
                                color: Theme.border
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 3
                                Text {
                                    visible: !window.navCollapsed
                                    text: controller.counts.media
                                        + (controller.counts.media === 1
                                            ? " video"
                                            : " videos")
                                    color: Theme.text
                                    font.pixelSize: Theme.textSm
                                }
                                Text {
                                    visible: !window.navCollapsed
                                    text: controller.counts.posts
                                        + (controller.counts.posts === 1
                                            ? " relay"
                                            : " relays")
                                        + " recorded"
                                    color: Theme.muted
                                    font.pixelSize: Theme.textXs
                                }
                                Item {
                                    visible: window.navCollapsed
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 36

                                    Row {
                                        anchors.centerIn: parent
                                        spacing: 5
                                        AppIcon {
                                            anchors.verticalCenter:
                                                parent.verticalCenter
                                            width: 15
                                            height: 15
                                            name: "media"
                                            strokeWidth: 1.7
                                            iconColor: Theme.muted
                                        }
                                        Text {
                                            anchors.verticalCenter:
                                                parent.verticalCenter
                                            text: String(
                                                controller.counts.media
                                            )
                                            color: Theme.text
                                            font.pixelSize: 10
                                            font.family: Theme.monoFamily
                                            font.weight: Font.Medium
                                        }
                                    }
                                    HoverHandler {
                                        id: videoCountHover
                                    }
                                    ToolTip.visible:
                                        videoCountHover.hovered
                                    ToolTip.text:
                                        controller.counts.media
                                        + (controller.counts.media === 1
                                            ? " video"
                                            : " videos")
                                        + " in library"
                                    ToolTip.delay: 450
                                }
                            }
                        }
                    }

                    Rectangle {
                        Layout.fillHeight: true
                        Layout.preferredWidth: 1
                        color: Theme.border
                    }

                    StackLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        currentIndex: window.currentPage
                        LibraryPage {
                            id: libraryPage
                            fullscreenHost: scaledWorkspace
                            randomSourceTrigger:
                                windowTitleBar.randomSourceButtonItem
                            onSearchFocusRequested:
                                windowTitleBar.focusSearch()
                        }
                        HistoryPage { }
                        SettingsPage { }
                    }
                }
            }

        Rectangle {
            id: toast
            property string kind: "info"
            property string message: ""
            property bool shown: false
            visible: shown || opacity > 0
            z: 100
            width: Math.min(460, parent.width - 48)
            height: Math.max(52, toastText.implicitHeight + 24)
            radius: Theme.radiusMd
            color: kind === "error" ? Theme.errorSoft
                : kind === "warning" ? Theme.warningSoft
                : kind === "success" ? Theme.successSoft : Theme.surfaceSoft
            border.width: 1
            border.color: kind === "error" ? Theme.error : kind === "warning" ? Theme.warning
                : kind === "success" ? Theme.success : Theme.accent
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 24
            opacity: shown ? 1 : 0

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 8
                spacing: 10
                AppIcon {
                    Layout.preferredWidth: 18
                    Layout.preferredHeight: 18
                    name: toast.kind === "error" || toast.kind === "warning"
                        ? "warning" : toast.kind === "success" ? "check" : "info"
                    iconColor: toast.kind === "error" ? Theme.error
                        : toast.kind === "warning" ? Theme.warning
                        : toast.kind === "success" ? Theme.success : Theme.accent
                }
                Text {
                    id: toastText
                    Layout.fillWidth: true
                    text: toast.message
                    color: Theme.text
                    wrapMode: Text.Wrap
                    font.pixelSize: Theme.textSm
                    verticalAlignment: Text.AlignVCenter
                }
                AppButton {
                    text: "Dismiss"
                    iconName: "close"
                    compact: true
                    kind: "ghost"
                    onClicked: toast.shown = false
                }
            }
            Behavior on opacity {
                NumberAnimation {
                    duration: Theme.stateMotion
                    easing.type: Easing.OutQuint
                }
            }
        }
        Timer { id: toastTimer; interval: 5200; onTriggered: toast.shown = false }
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: nativeWindow.resizable ? Theme.radiusMd : 0
        color: "transparent"
        border.width: nativeWindow.resizable ? 1 : 0
        border.color: Theme.border
        z: 9999
    }

    WindowResizeFrame {
        anchors.fill: parent
        windowController: nativeWindow
    }
}
