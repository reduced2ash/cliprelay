pragma Singleton
import QtQuick

QtObject {
    property string mode: "relay"

    readonly property bool isPitchBlack: mode === "pitch_black"
    readonly property bool isFullWhite: mode === "full_white"
    readonly property bool isLight: isFullWhite
    readonly property bool usesBlueAccent: isPitchBlack || isFullWhite

    readonly property color ink: isFullWhite
        ? "#F7F9FC" : isPitchBlack ? "#020305" : "#151416"
    readonly property color surface: isFullWhite
        ? "#FDFEFF" : isPitchBlack ? "#07090D" : "#1D1B1E"
    readonly property color surfaceSoft: isFullWhite
        ? "#FAFBFD" : isPitchBlack ? "#0B0E14" : "#211F22"
    readonly property color raised: isFullWhite
        ? "#F0F3F8" : isPitchBlack ? "#11151D" : "#262328"
    readonly property color active: isFullWhite
        ? "#E4EAF4" : isPitchBlack ? "#192235" : "#312C32"
    readonly property color hover: isFullWhite
        ? "#EAF0F8" : isPitchBlack ? "#141B2A" : "#2B272D"
    readonly property color text: isFullWhite
        ? "#171A21" : isPitchBlack ? "#F4F7FB" : "#F1ECE8"
    readonly property color textSoft: isFullWhite
        ? "#343A46" : isPitchBlack ? "#D2DAE6" : "#D3CBCA"
    readonly property color muted: isFullWhite
        ? "#5E697B" : isPitchBlack ? "#96A0B0" : "#ABA3A4"
    readonly property color mutedSoft: isFullWhite
        ? "#7C8799" : isPitchBlack ? "#697588" : "#827A7E"
    readonly property color border: isFullWhite
        ? "#D9DFE8" : isPitchBlack ? "#252C39" : "#3A343B"
    readonly property color borderStrong: isFullWhite
        ? "#BBC5D3" : isPitchBlack ? "#3A4658" : "#4B434C"

    // The two alternate palettes deliberately share one core blue.
    readonly property color accent: usesBlueAccent ? "#1F6FEF" : "#F07858"
    readonly property color accentPressed: usesBlueAccent ? "#1658C7" : "#D96247"
    readonly property color accentSoft: isFullWhite
        ? "#E5EDFF" : isPitchBlack ? "#0D1B38" : "#3C2928"
    readonly property color accentText: isFullWhite
        ? "#1554C5" : isPitchBlack ? "#78A9FF" : "#F07858"
    readonly property color accentContent: usesBlueAccent ? "#FDFEFF" : "#151416"

    readonly property color success: isFullWhite
        ? "#237A4E" : isPitchBlack ? "#68C38A" : "#72B985"
    readonly property color successSoft: isFullWhite
        ? "#E4F3EA" : isPitchBlack ? "#0F251B" : "#213229"
    readonly property color warning: isFullWhite
        ? "#95620E" : isPitchBlack ? "#E3B45F" : "#D8A758"
    readonly property color warningSoft: isFullWhite
        ? "#FFF1D6" : isPitchBlack ? "#2A210F" : "#352D20"
    readonly property color error: isFullWhite
        ? "#BA3D49" : isPitchBlack ? "#F07B83" : "#DD6B70"
    readonly property color errorSoft: isFullWhite
        ? "#FBE7E9" : isPitchBlack ? "#2E1418" : "#392426"

    // Media remains in a dark well in every theme so letterboxing and masks
    // never turn white around a video.
    readonly property color mediaWell: "#05070B"
    readonly property color mediaText: "#F4F7FB"
    readonly property color mediaMuted: "#A9B2C0"
    readonly property color overlay: usesBlueAccent ? "#151A23" : "#211F22"

    readonly property int spaceXs: 4
    readonly property int spaceSm: 8
    readonly property int spaceMd: 12
    readonly property int spaceLg: 16
    readonly property int spaceXl: 24
    readonly property int spaceXxl: 32

    readonly property int radiusSm: 6
    readonly property int radiusMd: 10
    readonly property int radiusLg: 14

    readonly property int textXs: 12
    readonly property int textSm: 13
    readonly property int textBase: 15
    readonly property int textSection: 16
    readonly property int textTitle: 20
    readonly property int textDisplay: 26
    readonly property int textWorkbench: 12

    readonly property int controlHeight: 44
    readonly property int compactControl: 40
    readonly property int workbenchTitleHeight: 40
    readonly property int workbenchContextHeight: 42
    readonly property int workbenchControlHeight: 30
    readonly property int workspaceFooterHeight: 40
    readonly property int radiusWorkbench: 4
    readonly property int focusWidth: 2

    readonly property string monoFamily: Qt.platform.os === "windows"
        ? "Cascadia Mono" : Qt.platform.os === "osx"
            ? "Menlo" : "monospace"

    readonly property int quickMotion: 90
    readonly property int fastMotion: 140
    readonly property int stateMotion: 210
}
