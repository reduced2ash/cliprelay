from __future__ import annotations

import os
from pathlib import Path

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

from PySide6.QtCore import QUrl
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlComponent, QQmlEngine
from PySide6.QtQuick import QQuickItem

from cliprelay.qt_models import RandomFolderModel


QML_DIR = Path(__file__).parents[1] / "src" / "cliprelay" / "qml"


def test_random_source_tree_creates_a_delegate_for_every_visible_row() -> None:
    app = QGuiApplication.instance() or QGuiApplication([])
    model = RandomFolderModel()
    model.set_rows([
        {"folder": "Events", "count": 2, "direct_count": 0},
        {"folder": "Events/2025", "count": 1, "direct_count": 1},
        {"folder": "Events/2026", "count": 1, "direct_count": 1},
        {"folder": "Personal", "count": 1, "direct_count": 1},
    ])

    engine = QQmlEngine()
    engine.rootContext().setContextProperty("randomFolderModel", model)
    component = QQmlComponent(engine)
    component.setData(
        b"""
import QtQuick
import QtQuick.Window
import "."

Window {
    width: 420
    height: 300
    visible: true

    ListView {
        objectName: "randomSourceList"
        anchors.fill: parent
        model: randomFolderModel
        delegate: RandomSourceTreeRow {
            objectName: "randomSourceRow"
        }
    }
}
""",
        QUrl.fromLocalFile(f"{QML_DIR.resolve()}/"),
    )

    window = component.create()
    assert window is not None, [error.toString() for error in component.errors()]
    for _ in range(20):
        app.processEvents()

    source_list = window.findChild(QQuickItem, "randomSourceList")
    assert source_list is not None
    content_item = source_list.property("contentItem")
    delegates = [
        item
        for item in content_item.childItems()
        if item.objectName() == "randomSourceRow"
    ]

    assert source_list.property("count") == model.visibleCount
    assert len(delegates) == model.visibleCount
    assert {
        item.property("folderPath")
        for item in delegates
    } == {
        "Events",
        "Events/2025",
        "Events/2026",
        "Personal",
    }

    window.close()
    window.deleteLater()
    engine.deleteLater()
    app.processEvents()
