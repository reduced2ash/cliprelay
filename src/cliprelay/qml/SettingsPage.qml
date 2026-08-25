import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import "."

Rectangle {
    id: root
    color: Theme.ink
    property var diagnosticState: controller.diagnostics()
    property var performanceState: performanceMonitor.state
    property var frameSamples: []
    property string selectedTheme: String(
        controller.settings.theme_mode || "relay"
    )
    Component.onCompleted: performanceMonitor.setActive(visible)
    Component.onDestruction: performanceMonitor.setActive(false)
    onVisibleChanged: {
        performanceMonitor.setActive(visible)
        if (!visible)
            frameSamples = []
    }

    FrameAnimation {
        running: root.visible
        onTriggered: {
            if (frameTime > 0 && frameTime < 0.1)
                root.frameSamples.push(frameTime * 1000)
            if (root.frameSamples.length >= 30) {
                performanceMonitor.recordFrameBatch(root.frameSamples)
                root.frameSamples = []
            }
        }
    }

    FolderDialog {
        id: libraryDialog
        title: "Choose your video library"
        onAccepted: controller.setSetting("library_root", selectedFolder)
    }
    FolderDialog {
        id: exportDialog
        title: "Choose where generated videos are kept"
        onAccepted: controller.setSetting("export_dir", selectedFolder)
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 82
            Layout.leftMargin: 26; Layout.rightMargin: 24
            ColumnLayout {
                Layout.fillWidth: true; spacing: 2
                Text { text: "Settings"; color: Theme.text; font.pixelSize: Theme.textTitle; font.weight: Font.DemiBold }
                Text { text: "Appearance, folders, connections, and safe file behavior"; color: Theme.muted; font.pixelSize: Theme.textXs }
            }
        }
        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.border }

        ScrollView {
            objectName: "settingsScroll"
            Component.onCompleted: {
                contentItem.objectName = "settingsFlickable"
                contentItem.pixelAligned = true
                contentItem.synchronousDrag = true
                contentItem.maximumFlickVelocity = 4000
            }
            Layout.fillWidth: true
            Layout.fillHeight: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ScrollBar.vertical: AppScrollBar { }
            contentWidth: availableWidth

            ColumnLayout {
                width: Math.min(820, parent.width - 48)
                x: Math.max(24, (parent.width - width) / 2)
                spacing: 0

                Item { Layout.preferredHeight: 28 }
                Text { text: "INTERFACE"; color: Theme.accentText; font.pixelSize: Theme.textXs; font.letterSpacing: 1.3 }
                Text { text: "Appearance and density"; color: Theme.text; font.pixelSize: Theme.textSection; font.weight: Font.DemiBold; Layout.topMargin: 5 }
                Text {
                    text: "Choose a palette for the whole app, then adjust how much of your workspace fits on screen."
                    color: Theme.muted; font.pixelSize: Theme.textSm; wrapMode: Text.Wrap
                    Layout.fillWidth: true; Layout.topMargin: 3; Layout.bottomMargin: 14
                }
                Text {
                    text: "Color theme"
                    color: Theme.text
                    font.pixelSize: Theme.textSm
                    font.weight: Font.Medium
                }
                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 7
                    spacing: 10

                    ThemeChoice {
                        Layout.fillWidth: true
                        title: "Relay"
                        description: "Warm dark"
                        selected: root.selectedTheme === "relay"
                        previewInk: "#151416"
                        previewSurface: "#1D1B1E"
                        previewRaised: "#262328"
                        previewTextColor: "#F1ECE8"
                        previewAccent: "#F07858"
                        previewBorder: "#3A343B"
                        onClicked: controller.setSetting("theme_mode", "relay")
                    }
                    ThemeChoice {
                        Layout.fillWidth: true
                        title: "Pitch black"
                        description: "Blue accent"
                        selected: root.selectedTheme === "pitch_black"
                        previewInk: "#020305"
                        previewSurface: "#07090D"
                        previewRaised: "#11151D"
                        previewTextColor: "#F4F7FB"
                        previewAccent: "#1F6FEF"
                        previewBorder: "#252C39"
                        onClicked: controller.setSetting(
                            "theme_mode", "pitch_black"
                        )
                    }
                    ThemeChoice {
                        Layout.fillWidth: true
                        title: "Full white"
                        description: "Blue accent"
                        selected: root.selectedTheme === "full_white"
                        previewInk: "#F7F9FC"
                        previewSurface: "#FDFEFF"
                        previewRaised: "#F0F3F8"
                        previewTextColor: "#171A21"
                        previewAccent: "#1F6FEF"
                        previewBorder: "#D9DFE8"
                        onClicked: controller.setSetting(
                            "theme_mode", "full_white"
                        )
                    }
                }
                Text {
                    text: "Theme changes apply immediately and are saved for the next launch."
                    color: Theme.muted
                    font.pixelSize: Theme.textXs
                    wrapMode: Text.Wrap
                    Layout.fillWidth: true
                    Layout.topMargin: 5
                }
                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 18
                    Text {
                        text: "Interface scale"
                        color: Theme.text; font.pixelSize: Theme.textSm; font.weight: Font.Medium
                        Layout.fillWidth: true
                    }
                    AppComboBox {
                        id: interfaceScaleBox
                        Layout.preferredWidth: 190
                        model: ["Compact · 80%", "Balanced · 90%", "Standard · 100%"]
                        currentIndex: {
                            var value = Number(controller.settings.ui_scale || 1.0)
                            return value < 0.85 ? 0 : value < 0.95 ? 1 : 2
                        }
                        onActivated: controller.setSetting("ui_scale", [0.8, 0.9, 1.0][currentIndex])
                        Accessible.name: "Interface scale"
                    }
                }
                Text {
                    text: "Standard preserves the current size. Compact provides the widest working view."
                    color: Theme.muted; font.pixelSize: Theme.textXs; wrapMode: Text.Wrap
                    Layout.fillWidth: true; Layout.topMargin: 4
                }
                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 18
                    spacing: 14
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3
                        Text {
                            text: "Library density"
                            color: Theme.text
                            font.pixelSize: Theme.textSm
                            font.weight: Font.Medium
                        }
                        Text {
                            text: "Compact fits more videos without changing the rest of the interface."
                            color: Theme.muted
                            font.pixelSize: Theme.textXs
                            wrapMode: Text.Wrap
                            Layout.fillWidth: true
                        }
                    }
                    AppComboBox {
                        Layout.preferredWidth: 190
                        model: ["Default", "Compact"]
                        currentIndex:
                            controller.settings.library_density === "compact"
                                ? 1 : 0
                        onActivated: controller.setSetting(
                            "library_density",
                            currentIndex === 1 ? "compact" : "default"
                        )
                        Accessible.name: "Library density"
                    }
                }

                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.border; Layout.topMargin: 28; Layout.bottomMargin: 28 }
                Text { text: "PERFORMANCE"; color: Theme.accentText; font.pixelSize: Theme.textXs; font.letterSpacing: 1.3 }
                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 5
                    spacing: 10
                    Text {
                        text: "Rendering and media"
                        color: Theme.text
                        font.pixelSize: Theme.textSection
                        font.weight: Font.DemiBold
                        Layout.fillWidth: true
                    }
                    StatusPill {
                        text: controller.settings.performance_mode === "maximum"
                            ? "MAXIMUM" : "AUTOMATIC"
                        status: controller.settings.performance_mode === "maximum"
                            ? "success" : "neutral"
                    }
                }
                Text {
                    text: "VSync stays enabled. Maximum mode keeps graphics resources resident, preloads adjacent media, raises safe thumbnail concurrency, and prefers hardware export."
                    color: Theme.muted
                    font.pixelSize: Theme.textSm
                    wrapMode: Text.Wrap
                    Layout.fillWidth: true
                    Layout.topMargin: 3
                    Layout.bottomMargin: 14
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 14
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3
                        Text {
                            text: "Performance mode"
                            color: Theme.text
                            font.pixelSize: Theme.textSm
                            font.weight: Font.Medium
                        }
                        Text {
                            text: "Maximum takes full effect after restarting ClipRelay."
                            color: Theme.muted
                            font.pixelSize: Theme.textXs
                        }
                    }
                    AppComboBox {
                        Layout.preferredWidth: 220
                        model: ["Automatic", "Maximum performance"]
                        currentIndex: controller.settings.performance_mode === "maximum" ? 1 : 0
                        onActivated: controller.setSetting(
                            "performance_mode",
                            currentIndex === 1 ? "maximum" : "automatic"
                        )
                        Accessible.name: "Performance mode"
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    Layout.topMargin: 14
                    spacing: 14
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3
                        Text {
                            text: "Export encoder"
                            color: Theme.text
                            font.pixelSize: Theme.textSm
                            font.weight: Font.Medium
                        }
                        Text {
                            text: "Hardware always falls back to software if the device or upload limit requires it."
                            color: Theme.muted
                            font.pixelSize: Theme.textXs
                            wrapMode: Text.Wrap
                            Layout.fillWidth: true
                        }
                    }
                    AppComboBox {
                        Layout.preferredWidth: 220
                        model: [
                            "Automatic",
                            "Prefer hardware",
                            "Software only"
                        ]
                        currentIndex: {
                            var mode = String(
                                controller.settings.export_encoder || "auto"
                            )
                            return mode === "hardware" ? 1
                                : mode === "software" ? 2 : 0
                        }
                        onActivated: controller.setSetting(
                            "export_encoder",
                            ["auto", "hardware", "software"][currentIndex]
                        )
                        Accessible.name: "Export encoder"
                    }
                }

                Text {
                    text: "LIVE DIAGNOSTICS"
                    color: Theme.muted
                    font.pixelSize: Theme.textXs
                    font.letterSpacing: 1.2
                    Layout.topMargin: 24
                    Layout.bottomMargin: 8
                }
                GridLayout {
                    Layout.fillWidth: true
                    columns: 2
                    columnSpacing: 22
                    rowSpacing: 8

                    Text { text: "Renderer"; color: Theme.muted; font.pixelSize: Theme.textSm }
                    Text { text: root.performanceState.renderer || "Detecting"; color: Theme.text; font.pixelSize: Theme.textXs; elide: Text.ElideRight; Layout.fillWidth: true }
                    Text { text: "GPU"; color: Theme.muted; font.pixelSize: Theme.textSm }
                    Text { text: root.performanceState.gpu || "Detecting"; color: Theme.text; font.pixelSize: Theme.textXs; elide: Text.ElideRight; Layout.fillWidth: true }
                    Text { text: "Display"; color: Theme.muted; font.pixelSize: Theme.textSm }
                    Text { text: root.performanceState.display || "Detecting"; color: Theme.text; font.pixelSize: Theme.textXs; elide: Text.ElideRight; Layout.fillWidth: true }
                    Text { text: "Video decoder"; color: Theme.muted; font.pixelSize: Theme.textSm }
                    Text { text: root.performanceState.decoder || "Automatic"; color: Theme.text; font.pixelSize: Theme.textXs; elide: Text.ElideRight; Layout.fillWidth: true }
                    Text { text: "Export"; color: Theme.muted; font.pixelSize: Theme.textSm }
                    Text { text: root.performanceState.exportEncoder || "Detecting"; color: Theme.text; font.pixelSize: Theme.textXs; elide: Text.ElideRight; Layout.fillWidth: true }
                    Text { text: "Frame pacing"; color: Theme.muted; font.pixelSize: Theme.textSm }
                    Text { text: root.performanceState.framePacing || "Sampling"; color: Theme.text; font.pixelSize: Theme.textXs; elide: Text.ElideRight; Layout.fillWidth: true }
                    Text { text: "Frame spikes"; color: Theme.muted; font.pixelSize: Theme.textSm }
                    Text { text: root.performanceState.frameSpikes || "—"; color: Theme.text; font.pixelSize: Theme.textXs; elide: Text.ElideRight; Layout.fillWidth: true }
                    Text { text: "Resources"; color: Theme.muted; font.pixelSize: Theme.textSm }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        Text {
                            text: root.performanceState.resourcePolicy
                                || "Resident on hide / restore"
                            color: Theme.text
                            font.pixelSize: Theme.textXs
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                        AppButton {
                            text: "Refresh"
                            iconName: "refresh"
                            kind: "ghost"
                            compact: true
                            onClicked: performanceMonitor.refresh()
                        }
                    }
                }

                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.border; Layout.topMargin: 28; Layout.bottomMargin: 28 }
                Text { text: "FILES"; color: Theme.accentText; font.pixelSize: Theme.textXs; font.letterSpacing: 1.3 }
                Text { text: "Library and generated media"; color: Theme.text; font.pixelSize: Theme.textSection; font.weight: Font.DemiBold; Layout.topMargin: 5 }
                Text { text: "Your original videos are never moved or modified."; color: Theme.muted; font.pixelSize: Theme.textSm; Layout.topMargin: 3; Layout.bottomMargin: 14 }

                Text { text: "Video library"; color: Theme.text; font.pixelSize: Theme.textSm; font.weight: Font.Medium }
                RowLayout {
                    Layout.fillWidth: true; Layout.topMargin: 6
                    AppField { Layout.fillWidth: true; text: controller.settings.library_root || ""; readOnly: true; placeholderText: "No folder chosen" }
                    AppButton { text: "Choose"; iconName: "folder"; onClicked: libraryDialog.open() }
                }
                Text { text: "ClipRelay searches this folder and every folder inside it."; color: Theme.muted; font.pixelSize: Theme.textXs; Layout.topMargin: 4 }

                Text { text: "Generated video folder"; color: Theme.text; font.pixelSize: Theme.textSm; font.weight: Font.Medium; Layout.topMargin: 18 }
                RowLayout {
                    Layout.fillWidth: true; Layout.topMargin: 6
                    AppField { Layout.fillWidth: true; text: controller.settings.export_dir || ""; readOnly: true }
                    AppButton { text: "Choose"; iconName: "folder"; onClicked: exportDialog.open() }
                    AppButton { text: "Reveal"; iconName: "external"; kind: "ghost"; onClicked: controller.revealPath(controller.settings.export_dir) }
                }

                Text { text: "FAST PICKING"; color: Theme.muted; font.pixelSize: Theme.textXs; font.letterSpacing: 1.2; Layout.topMargin: 24 }
                AppCheckBox {
                    text: "Make Random available before indexing finishes"
                    checked: controller.settings.fast_random
                    onToggled: controller.setSetting("fast_random", checked)
                    Layout.topMargin: 6
                }
                Text {
                    text: "ClipRelay keeps a lightweight filename list and checks only the clip Random chooses. This stays fast without allowing unreadable files into preparation."
                    color: Theme.muted; font.pixelSize: Theme.textXs; wrapMode: Text.Wrap
                    Layout.fillWidth: true; Layout.leftMargin: 30; Layout.bottomMargin: 3
                }
                AppCheckBox {
                    text: "Avoid repeats until every video has been picked"
                    checked: controller.settings.avoid_repeats
                    onToggled: controller.setSetting("avoid_repeats", checked)
                }

                Text { text: "BACKGROUND LIBRARY INDEX"; color: Theme.muted; font.pixelSize: Theme.textXs; font.letterSpacing: 1.2; Layout.topMargin: 22 }
                AppCheckBox {
                    text: "Start indexing automatically after choosing a folder"
                    checked: controller.settings.auto_index
                    onToggled: controller.setSetting("auto_index", checked)
                    Layout.topMargin: 6
                }
                Text {
                    text: "Off by default for large libraries. Rescan always starts it manually; reopening the app only refreshes the lightweight filename list."
                    color: Theme.muted; font.pixelSize: Theme.textXs; wrapMode: Text.Wrap
                    Layout.fillWidth: true; Layout.leftMargin: 30; Layout.bottomMargin: 3
                }
                AppCheckBox {
                    text: "Verify every file and read duration, resolution, and codec details"
                    checked: controller.settings.verify_during_index
                    onToggled: controller.setSetting("verify_during_index", checked)
                }
                AppCheckBox {
                    text: "Inspect files with uncommon or missing video extensions (slowest)"
                    checked: controller.settings.deep_scan
                    enabled: controller.settings.verify_during_index
                    opacity: enabled ? 1 : 0.45
                    onToggled: controller.setSetting("deep_scan", checked)
                }
                AppCheckBox {
                    text: "Generate all missing thumbnails while indexing"
                    checked: controller.settings.thumbnails_during_index
                    onToggled: controller.setSetting("thumbnails_during_index", checked)
                }
                AppCheckBox {
                    text: "Generate and play muted previews when hovering"
                    checked: controller.settings.hover_previews
                    onToggled: controller.setSetting("hover_previews", checked)
                }
                Text {
                    text: controller.settings.verify_during_index
                        ? "Turn off any optional step above to reduce background work. Random remains independent of this index."
                        : "With verification off, the library appears from filenames and sizes. A video is checked only when you select or publish it."
                    color: Theme.muted; font.pixelSize: Theme.textXs; wrapMode: Text.Wrap
                    Layout.fillWidth: true; Layout.leftMargin: 30; Layout.topMargin: 2
                }

                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.border; Layout.topMargin: 28; Layout.bottomMargin: 28 }
                Text { text: "TELEGRAM"; color: Theme.accentText; font.pixelSize: Theme.textXs; font.letterSpacing: 1.3 }
                Text { text: "Bot connection"; color: Theme.text; font.pixelSize: Theme.textSection; font.weight: Font.DemiBold; Layout.topMargin: 5 }
                Text {
                    text: "Best for a channel: simple setup, reliable sending, and no personal session stored. Add the bot as an administrator in the channel."
                    color: Theme.muted; font.pixelSize: Theme.textSm; wrapMode: Text.Wrap; Layout.fillWidth: true; Layout.topMargin: 3; Layout.bottomMargin: 12
                }
                RowLayout {
                    Layout.fillWidth: true
                    AppField { id: botToken; Layout.fillWidth: true; placeholderText: "Bot token from @BotFather"; echoMode: TextInput.Password; Accessible.name: "Telegram bot token" }
                    AppButton {
                        text: controller.settings.botConfigured ? "Replace bot" : "Connect bot"
                        iconName: "send"
                        kind: "primary"
                        onClicked: controller.validateBotToken(botToken.text)
                    }
                }
                RowLayout {
                    Layout.fillWidth: true; Layout.topMargin: 8
                    AppField { id: botDestination; Layout.fillWidth: true; text: controller.settings.telegram_destination || ""; placeholderText: "@channelname or numeric chat ID"; Accessible.name: "Telegram bot destination" }
                    AppButton { text: "Check destination"; iconName: "check"; onClicked: controller.validateBotDestination(botDestination.text) }
                }
                RowLayout {
                    Layout.fillWidth: true; Layout.topMargin: 8
                    StatusPill { text: controller.telegramState.bot; status: controller.settings.botConfigured ? "success" : "warning" }
                    Text { text: controller.telegramState.message || ""; color: Theme.muted; font.pixelSize: Theme.textXs; Layout.fillWidth: true; elide: Text.ElideRight }
                    AppButton { visible: controller.settings.botConfigured; text: "Disconnect"; kind: "danger"; onClicked: controller.disconnectBot() }
                }

                Text { text: "Personal account"; color: Theme.text; font.pixelSize: Theme.textSection; font.weight: Font.DemiBold; Layout.topMargin: 26 }
                Text {
                    text: "Use this when the sender must be your own account. Telegram requires an API ID and hash from my.telegram.org; the resulting session is stored in your OS keychain."
                    color: Theme.muted; font.pixelSize: Theme.textSm; wrapMode: Text.Wrap; Layout.fillWidth: true; Layout.topMargin: 3; Layout.bottomMargin: 12
                }
                GridLayout {
                    Layout.fillWidth: true
                    columns: 2
                    columnSpacing: 10; rowSpacing: 8
                    AppField { id: apiId; Layout.fillWidth: true; text: controller.settings.telegram_api_id || ""; placeholderText: "API ID"; inputMethodHints: Qt.ImhDigitsOnly }
                    AppField { id: apiHash; Layout.fillWidth: true; placeholderText: "API hash"; echoMode: TextInput.Password }
                    AppField { id: phone; Layout.fillWidth: true; text: controller.settings.telegram_phone || ""; placeholderText: "+1 555 123 4567" }
                    AppButton { text: "Send login code"; Layout.fillWidth: true; enabled: !controller.settings.personalConfigured; onClicked: controller.beginPersonalLogin(apiId.text, apiHash.text, phone.text) }
                    AppField { id: loginCode; Layout.fillWidth: true; placeholderText: "Login code"; inputMethodHints: Qt.ImhDigitsOnly }
                    AppField { id: cloudPassword; Layout.fillWidth: true; placeholderText: "2-step password, if requested"; echoMode: TextInput.Password }
                }
                RowLayout {
                    Layout.fillWidth: true; Layout.topMargin: 9
                    AppButton { text: "Finish sign-in"; iconName: "check"; kind: "primary"; enabled: !controller.settings.personalConfigured; onClicked: controller.completePersonalLogin(loginCode.text, cloudPassword.text) }
                    AppButton { visible: controller.settings.personalConfigured; text: "Load chats"; iconName: "refresh"; onClicked: controller.loadTelegramDialogs() }
                    AppButton { visible: controller.settings.personalConfigured; text: "Sign out"; kind: "danger"; onClicked: controller.signOutPersonal() }
                    StatusPill { text: controller.telegramState.personal; status: controller.settings.personalConfigured ? "success" : "warning" }
                    Item { Layout.fillWidth: true }
                }
                AppComboBox {
                    visible: controller.telegramDialogs.length > 0
                    Layout.fillWidth: true; Layout.topMargin: 9
                    model: controller.telegramDialogs
                    textRole: "title"
                    valueRole: "id"
                    onActivated: controller.setSetting("telegram_destination", currentValue)
                    Accessible.name: "Choose Telegram chat"
                }

                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.border; Layout.topMargin: 28; Layout.bottomMargin: 28 }
                Text { text: "X HANDOFF"; color: Theme.accentText; font.pixelSize: Theme.textXs; font.letterSpacing: 1.3 }
                Text { text: "Manual browser posting"; color: Theme.text; font.pixelSize: Theme.textSection; font.weight: Font.DemiBold; Layout.topMargin: 5 }
                Text {
                    text: "ClipRelay opens X’s official composer with your text prefilled, then places the prepared video on the clipboard and keeps drag-to-upload available. You review and press Post yourself. No paid X API is required."
                    color: Theme.muted; font.pixelSize: Theme.textSm; wrapMode: Text.Wrap; Layout.fillWidth: true; Layout.topMargin: 3; Layout.bottomMargin: 12
                }
                RowLayout {
                    Layout.fillWidth: true
                    Text { text: "Default file limit"; color: Theme.text; font.pixelSize: Theme.textSm; Layout.fillWidth: true }
                    AppField {
                        Layout.preferredWidth: 110
                        text: String(controller.settings.x_limit_mb)
                        inputMethodHints: Qt.ImhDigitsOnly
                        onEditingFinished: controller.setSetting("x_limit_mb", Number(text) || 512)
                        Accessible.name: "X file limit megabytes"
                    }
                    Text { text: "MB"; color: Theme.muted; font.pixelSize: Theme.textSm }
                }

                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.border; Layout.topMargin: 28; Layout.bottomMargin: 28 }
                Text { text: "DIAGNOSTICS"; color: Theme.accentText; font.pixelSize: Theme.textXs; font.letterSpacing: 1.3 }
                Text { text: "Local tools"; color: Theme.text; font.pixelSize: Theme.textSection; font.weight: Font.DemiBold; Layout.topMargin: 5; Layout.bottomMargin: 12 }
                GridLayout {
                    Layout.fillWidth: true; columns: 2; columnSpacing: 18; rowSpacing: 8
                    Text { text: "FFmpeg"; color: Theme.muted; font.pixelSize: Theme.textSm }
                    Text { text: root.diagnosticState.ffmpeg; color: Theme.text; font.pixelSize: Theme.textXs; elide: Text.ElideMiddle; Layout.fillWidth: true }
                    Text { text: "FFprobe"; color: Theme.muted; font.pixelSize: Theme.textSm }
                    Text { text: root.diagnosticState.ffprobe; color: Theme.text; font.pixelSize: Theme.textXs; elide: Text.ElideMiddle; Layout.fillWidth: true }
                    Text { text: "Database"; color: Theme.muted; font.pixelSize: Theme.textSm }
                    Text { text: root.diagnosticState.database; color: Theme.text; font.pixelSize: Theme.textXs; elide: Text.ElideMiddle; Layout.fillWidth: true }
                    Text { text: "Secrets"; color: Theme.muted; font.pixelSize: Theme.textSm }
                    Text { text: root.diagnosticState.secretBackend; color: Theme.text; font.pixelSize: Theme.textXs; Layout.fillWidth: true }
                }
                Item { Layout.preferredHeight: 42 }
            }
        }
    }
}
