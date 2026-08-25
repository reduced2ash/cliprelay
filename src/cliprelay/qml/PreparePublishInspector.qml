import QtQuick
import QtQuick.Layouts
import "."

ColumnLayout {
    id: root

    property bool studioMode: false
    property real trimStart: 0
    property real trimEnd: 0
    property bool sameCaption: true
    property bool suppressSignals: false
    readonly property bool telegramConnected: telegramMode.currentIndex === 0
        ? Boolean(controller.settings.botConfigured)
        : Boolean(controller.settings.personalConfigured)
    readonly property bool telegramReady: telegramConnected
        && destinationField.text.trim().length > 0
    readonly property string estimatedSize: controller.estimateOutputSize(
        trimStart,
        trimEnd,
        presetCode(),
        Number(targetField.text) || 0
    )
    readonly property bool xDurationWarning: (trimEnd - trimStart)
        > Number(controller.settings.x_duration_seconds || 140)

    signal changed()

    spacing: 12

    function markChanged() {
        if (!suppressSignals)
            changed()
    }

    function presetCode() {
        return [
            "original", "balanced", "fit_bot", "fit_x",
            "fit_both", "smallest", "custom"
        ][compressionBox.currentIndex]
    }

    function captureState() {
        return {
            sameCaption: root.sameCaption,
            telegramModeIndex: telegramMode.currentIndex,
            destination: destinationField.text,
            caption: captionArea.text,
            xCaption: xCaptionArea.text,
            compressionIndex: compressionBox.currentIndex,
            targetSize: targetField.text,
            cleanupIndex: cleanupBox.currentIndex
        }
    }

    function restoreState(draft) {
        root.suppressSignals = true
        root.sameCaption = draft.sameCaption === undefined
            ? true : Boolean(draft.sameCaption)
        telegramMode.currentIndex = Math.max(
            0,
            Math.min(1, Number(draft.telegramModeIndex || 0))
        )
        destinationField.text = String(
            draft.destination
                || controller.settings.telegram_destination || ""
        )
        captionArea.text = String(draft.caption || "")
        xCaptionArea.text = String(draft.xCaption || "")
        compressionBox.currentIndex = Math.max(
            0,
            Math.min(6, Number(draft.compressionIndex === undefined
                ? 4 : draft.compressionIndex))
        )
        targetField.text = String(draft.targetSize || "")
        cleanupBox.currentIndex = Math.max(
            0,
            Math.min(2, Number(draft.cleanupIndex || 0))
        )
        root.suppressSignals = false
    }

    function deliveryValues() {
        return {
            preset: root.presetCode(),
            targetMb: Number(targetField.text) || 0,
            telegramMode: telegramMode.currentIndex === 0
                ? "bot" : "personal",
            telegramDestination: destinationField.text.trim(),
            telegramCaption: captionArea.text,
            xCaption: root.sameCaption
                ? captionArea.text : xCaptionArea.text,
            cleanupPolicy: [
                "keep", "after_complete", "after_telegram"
            ][cleanupBox.currentIndex]
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Text {
                text: "OUTPUT"
                color: Theme.muted
                font.pixelSize: Theme.textXs
                font.weight: Font.DemiBold
                font.letterSpacing: 1.0
            }
            Text {
                text: "Choose a destination-aware generated copy."
                color: Theme.textSoft
                font.pixelSize: Theme.textXs
                wrapMode: Text.Wrap
                Layout.fillWidth: true
            }
        }
        StatusPill {
            status: "neutral"
            text: root.estimatedSize
        }
    }

    AppComboBox {
        id: compressionBox
        Layout.fillWidth: true
        model: [
            "Original when possible", "Balanced", "Fit Telegram bot",
            "Fit X", "Fit both", "Smallest practical", "Custom size"
        ]
        currentIndex: 4
        Accessible.name: "Compression preset"
        onActivated: root.markChanged()
    }

    RowLayout {
        visible: compressionBox.currentIndex === 6
        Layout.fillWidth: true
        spacing: 8

        Text {
            Layout.fillWidth: true
            text: "Maximum generated size"
            color: Theme.textSoft
            font.pixelSize: Theme.textXs
        }
        AppField {
            id: targetField
            Layout.preferredWidth: 112
            placeholderText: "MB"
            inputMethodHints: Qt.ImhFormattedNumbersOnly
            Accessible.name: "Target megabytes"
            onTextChanged: root.markChanged()
        }
    }

    Text {
        Layout.fillWidth: true
        text: compressionBox.currentIndex === 0
            ? "Uses the source when it already fits."
            : compressionBox.currentIndex === 5
                ? "Prioritizes the smallest practical file."
                : "The estimate follows the current cut."
        color: Theme.muted
        font.pixelSize: Theme.textXs
        wrapMode: Text.Wrap
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: 1
        color: Theme.border
    }

    Text {
        text: "DESTINATIONS"
        color: Theme.muted
        font.pixelSize: Theme.textXs
        font.weight: Font.DemiBold
        font.letterSpacing: 1.0
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8

        AppIcon {
            Layout.preferredWidth: 16
            Layout.preferredHeight: 16
            name: "send"
            iconColor: root.telegramReady ? Theme.success : Theme.muted
        }
        Text {
            Layout.fillWidth: true
            text: "Telegram"
            color: Theme.text
            font.pixelSize: Theme.textSm
            font.weight: Font.DemiBold
        }
        StatusPill {
            status: root.telegramReady ? "success" : "warning"
            text: root.telegramReady ? "Ready" : "Needs setup"
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8

        AppComboBox {
            id: telegramMode
            Layout.preferredWidth: 126
            model: ["Bot", "Personal"]
            currentIndex: controller.settings.telegram_mode === "personal"
                ? 1 : 0
            Accessible.name: "Telegram account type"
            onActivated: {
                controller.setSetting(
                    "telegram_mode",
                    currentIndex === 0 ? "bot" : "personal"
                )
                root.markChanged()
            }
        }
        AppField {
            id: destinationField
            Layout.fillWidth: true
            text: controller.settings.telegram_destination || ""
            placeholderText: telegramMode.currentIndex === 0
                ? "@channel or chat ID" : "Username or chat ID"
            Accessible.name: "Telegram destination"
            onTextChanged: root.markChanged()
        }
    }

    Text {
        Layout.fillWidth: true
        text: !root.telegramConnected
            ? (telegramMode.currentIndex === 0
                ? "Connect a bot in Settings before sending."
                : "Sign in under Settings before sending.")
            : destinationField.text.trim().length === 0
                ? "Enter a destination before sending."
                : telegramMode.currentIndex === 0
                    ? "Bot connected to the selected destination."
                    : "Personal account connected to the selected destination."
        color: root.telegramReady ? Theme.success : Theme.warning
        font.pixelSize: Theme.textXs
        wrapMode: Text.Wrap
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8

        AppIcon {
            Layout.preferredWidth: 16
            Layout.preferredHeight: 16
            name: "x"
            iconColor: root.xDurationWarning ? Theme.warning : Theme.textSoft
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1
            Text {
                text: "X"
                color: Theme.text
                font.pixelSize: Theme.textSm
                font.weight: Font.DemiBold
            }
            Text {
                text: root.xDurationWarning
                    ? "Current cut exceeds the configured duration limit."
                    : "Manual browser handoff"
                color: root.xDurationWarning ? Theme.warning : Theme.muted
                font.pixelSize: Theme.textXs
                wrapMode: Text.Wrap
                Layout.fillWidth: true
            }
        }
        StatusPill {
            status: root.xDurationWarning ? "warning" : "neutral"
            text: root.xDurationWarning ? "Check cut" : "Manual"
        }
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: 1
        color: Theme.border
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8

        Text {
            Layout.fillWidth: true
            text: "CAPTIONS"
            color: Theme.muted
            font.pixelSize: Theme.textXs
            font.weight: Font.DemiBold
            font.letterSpacing: 1.0
        }
        AppCheckBox {
            text: "Shared caption"
            checked: root.sameCaption
            onToggled: {
                root.sameCaption = checked
                root.markChanged()
            }
        }
    }

    Text {
        text: root.sameCaption ? "Telegram and X" : "Telegram"
        color: Theme.textSoft
        font.pixelSize: Theme.textXs
        font.weight: Font.DemiBold
    }
    AppTextArea {
        id: captionArea
        Layout.fillWidth: true
        Layout.preferredHeight: root.studioMode ? 106 : 92
        placeholderText: root.sameCaption
            ? "Caption for Telegram and X" : "Telegram message"
        onTextChanged: root.markChanged()
    }
    Text {
        Layout.alignment: Qt.AlignRight
        text: "Telegram " + captionArea.length + " / "
            + (telegramMode.currentIndex === 0 ? "1,024" : "4,096")
            + (root.sameCaption
                ? "  ·  X " + captionArea.length + " / 280" : "")
        color: captionArea.length
            > (telegramMode.currentIndex === 0 ? 1024 : 4096)
            || (root.sameCaption && captionArea.length > 280)
            ? Theme.warning : Theme.muted
        font.pixelSize: Theme.textXs
        font.features: { "tnum": 1 }
    }

    Text {
        visible: !root.sameCaption
        text: "X"
        color: Theme.textSoft
        font.pixelSize: Theme.textXs
        font.weight: Font.DemiBold
    }
    AppTextArea {
        id: xCaptionArea
        visible: !root.sameCaption
        Layout.fillWidth: true
        Layout.preferredHeight: 92
        placeholderText: "X post text"
        onTextChanged: root.markChanged()
    }
    Text {
        visible: !root.sameCaption
        Layout.alignment: Qt.AlignRight
        text: "X " + xCaptionArea.length + " / 280"
        color: xCaptionArea.length > 280 ? Theme.warning : Theme.muted
        font.pixelSize: Theme.textXs
        font.features: { "tnum": 1 }
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: 1
        color: Theme.border
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1
            Text {
                text: "Generated copy"
                color: Theme.textSoft
                font.pixelSize: Theme.textSm
                font.weight: Font.DemiBold
            }
            Text {
                text: "Cleanup never applies to the source."
                color: Theme.muted
                font.pixelSize: Theme.textXs
            }
        }
        AppComboBox {
            id: cleanupBox
            Layout.preferredWidth: root.width < 430 ? 158 : 184
            model: [
                "Keep", "Trash when complete", "Trash after Telegram"
            ]
            currentIndex: controller.settings.cleanup_policy
                === "after_complete"
                ? 1
                : controller.settings.cleanup_policy === "after_telegram"
                    ? 2 : 0
            Accessible.name: "Generated copy cleanup"
            onActivated: {
                controller.setSetting(
                    "cleanup_policy",
                    [
                        "keep", "after_complete", "after_telegram"
                    ][currentIndex]
                )
                root.markChanged()
            }
        }
    }

    Item {
        Layout.fillWidth: true
        Layout.preferredHeight: 4
    }
}
