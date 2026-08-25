import QtQuick
import QtQuick.Controls
import "."

TextArea {
    id: control
    leftPadding: 13
    rightPadding: 13
    topPadding: 11
    bottomPadding: 11
    color: Theme.text
    placeholderTextColor: Theme.muted
    selectionColor: Theme.accent
    selectedTextColor: Theme.accentContent
    font.pixelSize: Theme.textSm
    wrapMode: TextEdit.Wrap
    focusPolicy: Qt.StrongFocus

    background: Rectangle {
        radius: Theme.radiusSm
        color: Theme.raised
        border.width: control.activeFocus ? Theme.focusWidth : 1
        border.color: control.activeFocus
            ? Theme.accent
            : control.hovered ? Theme.borderStrong : Theme.border
        Behavior on border.color { ColorAnimation { duration: Theme.fastMotion } }
    }
}
