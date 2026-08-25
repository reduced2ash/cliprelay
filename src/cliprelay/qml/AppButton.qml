import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

Button {
    id: control
    property string kind: "secondary"
    property string iconName: ""
    property string leadingText: ""
    property bool compact: false
    property string toolTipText: text
    property bool iconOnly: compact
    readonly property real resolvedVerticalPadding: Math.max(
        4,
        Math.min(10, Math.floor((height - 20) / 2))
    )
    readonly property color resolvedIconColor: kind === "primary"
        ? Theme.accentContent
        : kind === "danger"
            ? Theme.error
            : hovered ? Theme.text : Theme.textSoft

    implicitHeight: Theme.controlHeight
    implicitWidth: Math.max(
        iconOnly ? Theme.controlHeight : 104,
        contentFrame.implicitWidth + (iconOnly ? 20 : 30)
    )
    leftPadding: iconOnly ? 10 : 15
    rightPadding: iconOnly ? 10 : 15
    topPadding: resolvedVerticalPadding
    bottomPadding: resolvedVerticalPadding
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    opacity: enabled ? 1 : 0.46
    scale: down ? 0.985 : 1

    Accessible.role: Accessible.Button
    Accessible.name: text

    contentItem: Item {
        id: contentFrame
        clip: true

        implicitWidth: control.iconOnly ? 20 : contentRow.implicitWidth
        implicitHeight: control.iconOnly ? 20 : contentRow.implicitHeight

        AppIcon {
            anchors.centerIn: parent
            width: 20
            height: 20
            visible: control.iconOnly && control.iconName.length > 0
            name: control.iconName
            strokeWidth: 1.9
            iconColor: control.resolvedIconColor
        }

        RowLayout {
            id: contentRow
            anchors.fill: parent
            visible: !control.iconOnly
            spacing: 7

            AppIcon {
                visible: control.iconName.length > 0
                Layout.preferredWidth: 17
                Layout.preferredHeight: 17
                name: control.iconName
                strokeWidth: 1.75
                iconColor: control.resolvedIconColor
                Layout.alignment: Qt.AlignVCenter
            }
            Text {
                visible: control.iconName.length === 0
                    && control.leadingText.length > 0
                text: control.leadingText
                color: control.kind === "primary"
                    ? Theme.accentContent : Theme.textSoft
                font.pixelSize: 17
                Layout.alignment: Qt.AlignVCenter
            }
            Text {
                text: control.text
                color: control.kind === "primary"
                    ? Theme.accentContent
                    : control.kind === "danger" ? Theme.error : Theme.text
                font.pixelSize: Theme.textSm
                font.weight: Font.DemiBold
                elide: Text.ElideRight
                clip: true
                Layout.fillWidth: true
                Layout.minimumWidth: 0
                Layout.alignment: Qt.AlignVCenter
                horizontalAlignment: control.iconName.length > 0
                    ? Text.AlignLeft : Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }
    }

    background: Rectangle {
        radius: control.iconOnly ? Theme.radiusMd : Theme.radiusSm
        color: {
            if (!control.enabled) return Theme.raised
            if (control.kind === "primary") return control.down ? Theme.accentPressed : Theme.accent
            if (control.kind === "danger") return control.hovered || control.down ? Theme.errorSoft : "transparent"
            if (control.kind === "ghost") return control.hovered || control.down ? Theme.active : "transparent"
            return control.hovered || control.down ? Theme.active : Theme.raised
        }
        border.width: control.visualFocus
            ? Theme.focusWidth
            : (control.kind === "secondary" ? 1 : 0)
        border.color: control.visualFocus ? Theme.accent : Theme.border
        Behavior on color { ColorAnimation { duration: Theme.fastMotion } }
    }

    ToolTip.visible: control.hovered
        && (control.iconOnly || control.toolTipText !== control.text)
    ToolTip.text: control.toolTipText
    ToolTip.delay: 450
    Behavior on scale { NumberAnimation { duration: Theme.quickMotion; easing.type: Easing.OutQuart } }
}
