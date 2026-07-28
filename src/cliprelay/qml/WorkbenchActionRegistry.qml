import QtQuick

QtObject {
    id: root

    required property var appController
    required property var libraryPage
    required property var hostWindow
    required property var windowController

    readonly property var actions: [
        {
            "id": "pick_random",
            "label": appController.randomPicking
                ? "Picking random video…" : "Pick random video",
            "detail": "Choose from the active random-source folders",
            "category": "Library",
            "icon": "shuffle",
            "shortcut": "R",
            "keywords": "random shuffle choose video",
            "enabled": !appController.randomPicking
                && appController.hasRandomFolderSelection
                && (appController.settings.fast_random
                    ? Boolean(appController.settings.library_root)
                    : appController.counts.media > 0)
        },
        {
            "id": "reset_shuffle",
            "label": "Reset shuffle history",
            "detail": "Allow every active video to be picked again",
            "category": "Library",
            "icon": "refresh",
            "shortcut": "",
            "keywords": "reset clear shuffle seen random",
            "enabled": Boolean(appController.settings.library_root)
        },
        {
            "id": "navigate_back",
            "label": "Go back",
            "detail": "Restore the previous video or folder",
            "category": "Navigation",
            "icon": "chevronLeft",
            "shortcut": Qt.platform.os === "osx" ? "⌘[" : "Alt+←",
            "keywords": "back undo previous video folder navigation",
            "enabled": appController.canNavigateBack
        },
        {
            "id": "navigate_forward",
            "label": "Go forward",
            "detail": "Reapply the next video or folder",
            "category": "Navigation",
            "icon": "chevronRight",
            "shortcut": Qt.platform.os === "osx" ? "⌘]" : "Alt+→",
            "keywords": "forward redo next video folder navigation",
            "enabled": appController.canNavigateForward
        },
        {
            "id": "toggle_folders",
            "label": libraryPage.showFolders
                ? "Hide folder explorer" : "Show folder explorer",
            "detail": "Toggle the hierarchical folder column",
            "category": "View",
            "icon": "panel",
            "shortcut": "",
            "keywords": "folder explorer sidebar hide show",
            "enabled": Boolean(appController.settings.library_root)
        },
        {
            "id": "choose_folder",
            "label": "Choose library root",
            "detail": "Open a different top-level video folder",
            "category": "Library",
            "icon": "folder",
            "shortcut": "",
            "keywords": "choose open root directory library",
            "enabled": true
        },
        {
            "id": "rescan",
            "label": appController.scanning
                ? "Library scan in progress" : "Rescan library",
            "detail": "Refresh the filename manifest and media index",
            "category": "Library",
            "icon": "refresh",
            "shortcut": "",
            "keywords": "scan refresh index update",
            "enabled": !appController.scanning
                && Boolean(appController.settings.library_root)
        },
        {
            "id": "sort_newest",
            "label": "Sort by newest",
            "detail": "Newest modified files first",
            "category": "Sort",
            "icon": "list",
            "shortcut": "",
            "keywords": "sort date recent newest",
            "enabled": appController.settings.sort_mode !== "newest"
        },
        {
            "id": "sort_oldest",
            "label": "Sort by oldest",
            "detail": "Oldest modified files first",
            "category": "Sort",
            "icon": "list",
            "shortcut": "",
            "keywords": "sort date oldest",
            "enabled": appController.settings.sort_mode !== "oldest"
        },
        {
            "id": "sort_name",
            "label": "Sort by name",
            "detail": "Alphabetical filename order",
            "category": "Sort",
            "icon": "list",
            "shortcut": "",
            "keywords": "sort alphabetical filename name",
            "enabled": appController.settings.sort_mode !== "name"
        },
        {
            "id": "sort_duration",
            "label": "Sort by duration",
            "detail": "Longest videos first",
            "category": "Sort",
            "icon": "list",
            "shortcut": "",
            "keywords": "sort length duration time",
            "enabled": appController.settings.sort_mode !== "duration"
        },
        {
            "id": "sort_size",
            "label": "Sort by size",
            "detail": "Largest files first",
            "category": "Sort",
            "icon": "list",
            "shortcut": "",
            "keywords": "sort file size largest",
            "enabled": appController.settings.sort_mode !== "size"
        },
        {
            "id": "go_library",
            "label": "Go to Library",
            "detail": "Open the video library workspace",
            "category": "Navigation",
            "icon": "library",
            "shortcut": Qt.platform.os === "osx" ? "⌘1" : "Ctrl+1",
            "keywords": "navigate page library",
            "enabled": hostWindow.currentPage !== 0
        },
        {
            "id": "go_history",
            "label": "Go to History",
            "detail": "Review prior relay attempts",
            "category": "Navigation",
            "icon": "history",
            "shortcut": Qt.platform.os === "osx" ? "⌘2" : "Ctrl+2",
            "keywords": "navigate page history posts",
            "enabled": hostWindow.currentPage !== 1
        },
        {
            "id": "go_settings",
            "label": "Go to Settings",
            "detail": "Configure ClipRelay",
            "category": "Navigation",
            "icon": "settings",
            "shortcut": Qt.platform.os === "osx" ? "⌘," : "Ctrl+,",
            "keywords": "navigate page preferences settings",
            "enabled": hostWindow.currentPage !== 2
        },
        {
            "id": "toggle_sidebar",
            "label": hostWindow.navCollapsed
                ? "Expand activity sidebar" : "Collapse activity sidebar",
            "detail": hostWindow.narrowWindow
                ? "Widen the window to expand the sidebar"
                : "Change the activity rail between icons and labels",
            "category": "View",
            "icon": hostWindow.navCollapsed
                ? "chevronRight" : "chevronLeft",
            "shortcut": "",
            "keywords": "sidebar rail collapse expand",
            "enabled": !hostWindow.narrowWindow
        },
        {
            "id": "theme_relay",
            "label": "Use Relay theme",
            "detail": "Warm dark ClipRelay palette",
            "category": "Theme",
            "icon": "settings",
            "shortcut": "",
            "keywords": "appearance theme relay orange",
            "enabled": hostWindow.themeMode !== "relay"
        },
        {
            "id": "theme_pitch_black",
            "label": "Use Pitch Black theme",
            "detail": "Black surfaces with blue actions",
            "category": "Theme",
            "icon": "settings",
            "shortcut": "",
            "keywords": "appearance theme pitch black blue dark",
            "enabled": hostWindow.themeMode !== "pitch_black"
        },
        {
            "id": "theme_full_white",
            "label": "Use Full White theme",
            "detail": "Light surfaces with blue actions",
            "category": "Theme",
            "icon": "settings",
            "shortcut": "",
            "keywords": "appearance theme white blue light",
            "enabled": hostWindow.themeMode !== "full_white"
        }
    ]

    function action(actionId) {
        for (let index = 0; index < actions.length; ++index) {
            if (actions[index].id === actionId)
                return actions[index]
        }
        return {
            "id": actionId,
            "label": actionId,
            "detail": "",
            "category": "",
            "icon": "",
            "shortcut": "",
            "keywords": "",
            "enabled": false
        }
    }

    function triggerAction(actionId) {
        const descriptor = action(actionId)
        if (!descriptor.enabled)
            return false
        if (actionId === "pick_random") {
            hostWindow.currentPage = 0
            appController.pickRandom()
        } else if (actionId === "reset_shuffle") {
            appController.resetShuffle()
        } else if (actionId === "navigate_back") {
            hostWindow.currentPage = 0
            appController.navigateBack()
        } else if (actionId === "navigate_forward") {
            hostWindow.currentPage = 0
            appController.navigateForward()
        } else if (actionId === "toggle_folders") {
            libraryPage.toggleFolders()
        } else if (actionId === "choose_folder") {
            libraryPage.chooseLibraryFolder()
        } else if (actionId === "rescan") {
            appController.scanLibrary()
        } else if (actionId.indexOf("sort_") === 0) {
            libraryPage.setSortMode(actionId.substring(5))
        } else if (actionId === "go_library") {
            hostWindow.currentPage = 0
        } else if (actionId === "go_history") {
            hostWindow.currentPage = 1
        } else if (actionId === "go_settings") {
            hostWindow.currentPage = 2
        } else if (actionId === "toggle_sidebar") {
            appController.setSetting(
                "sidebar_collapsed",
                !Boolean(appController.settings.sidebar_collapsed)
            )
        } else if (actionId === "theme_relay") {
            appController.setSetting("theme_mode", "relay")
        } else if (actionId === "theme_pitch_black") {
            appController.setSetting("theme_mode", "pitch_black")
        } else if (actionId === "theme_full_white") {
            appController.setSetting("theme_mode", "full_white")
        } else {
            return false
        }
        return true
    }
}
