import QtQuick
import QtQuick.Layouts
import "."

ColumnLayout {
    id: root

    property var editor
    property bool studioMode: false
    readonly property bool cropIsOriginal: editor
        && Math.abs(Number(editor.cropX || 0)) < 0.001
        && Math.abs(Number(editor.cropY || 0)) < 0.001
        && Math.abs(Number(editor.cropWidth || 1) - 1) < 0.001
        && Math.abs(Number(editor.cropHeight || 1) - 1) < 0.001

    signal changed()

    spacing: 12

    function syncFromEditor() {
        cropPreset.currentIndex = root.editor
            && root.editor.cropEnabled ? 0 : 1
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Text {
                text: "FRAME"
                color: Theme.muted
                font.pixelSize: Theme.textXs
                font.weight: Font.DemiBold
                font.letterSpacing: 1.0
            }
            Text {
                text: "Crop the visible frame without changing the source."
                color: Theme.textSoft
                font.pixelSize: Theme.textXs
                wrapMode: Text.Wrap
                Layout.fillWidth: true
            }
        }
        StatusPill {
            visible: root.editor && root.editor.cropEnabled
            status: "accent"
            text: "Crop"
        }
    }

    AppCheckBox {
        text: "Enable crop"
        checked: root.editor && root.editor.cropEnabled
        onToggled: {
            if (!root.editor)
                return
            root.editor.selectedShapeIndex = -1
            root.editor.setCropEnabled(checked)
            if (!checked) {
                root.editor.resetCrop()
                cropPreset.currentIndex = 1
            } else {
                cropPreset.currentIndex = 0
            }
            root.changed()
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8

        AppComboBox {
            id: cropPreset
            Layout.fillWidth: true
            enabled: root.editor && root.editor.cropEnabled
            model: [
                "Free crop", "Original frame", "Square (1:1)",
                "Landscape (16:9)", "Portrait (9:16)"
            ]
            Accessible.name: "Crop aspect"
            onActivated: {
                if (!root.editor)
                    return
                root.editor.selectedShapeIndex = -1
                if (currentIndex === 0) {
                    root.editor.applyCropAspect(0)
                } else if (currentIndex === 1) {
                    root.editor.resetCrop()
                    root.editor.setCropEnabled(false)
                } else if (currentIndex === 2) {
                    root.editor.applyCropAspect(1)
                } else if (currentIndex === 3) {
                    root.editor.applyCropAspect(16 / 9)
                } else {
                    root.editor.applyCropAspect(9 / 16)
                }
                root.changed()
            }
        }
        AppButton {
            visible: root.editor && root.editor.cropEnabled
                && !root.cropIsOriginal
            text: "Reset crop"
            iconName: "refresh"
            kind: "ghost"
            compact: root.width < 430
            toolTipText: text
            onClicked: {
                root.editor.selectedShapeIndex = -1
                root.editor.resetCrop()
                root.editor.setCropEnabled(false)
                cropPreset.currentIndex = 1
                root.changed()
            }
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

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Text {
                text: "BLACK MASKS"
                color: Theme.muted
                font.pixelSize: Theme.textXs
                font.weight: Font.DemiBold
                font.letterSpacing: 1.0
            }
            Text {
                text: "Select a mask here, then position it on the video."
                color: Theme.textSoft
                font.pixelSize: Theme.textXs
                wrapMode: Text.Wrap
                Layout.fillWidth: true
            }
        }
        StatusPill {
            visible: root.editor && root.editor.shapeCount > 0
            status: "neutral"
            text: root.editor ? root.editor.shapeCount + " masks" : ""
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8

        AppButton {
            Layout.fillWidth: true
            text: "Add rectangle"
            iconName: "rectangle"
            onClicked: {
                root.editor.addShape(false)
                root.changed()
            }
        }
        AppButton {
            Layout.fillWidth: true
            text: "Add square"
            iconName: "square"
            onClicked: {
                root.editor.addShape(true)
                root.changed()
            }
        }
    }

    PrepareMaskList {
        Layout.fillWidth: true
        editor: root.editor
        onChanged: root.changed()
    }

    Item {
        Layout.fillWidth: true
        Layout.preferredHeight: 4
    }
}
