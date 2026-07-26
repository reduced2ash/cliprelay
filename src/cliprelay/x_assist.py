from __future__ import annotations

import subprocess
import sys
from pathlib import Path
from urllib.parse import quote

from PySide6.QtCore import QMimeData, QUrl, Qt
from PySide6.QtGui import QDesktopServices, QDrag, QGuiApplication


class XAssistant:
    def intent_url(self, caption: str) -> QUrl:
        return QUrl(f"https://x.com/intent/tweet?text={quote(caption, safe='')}")

    def copy_file(self, path: str | Path) -> None:
        target = Path(path).resolve()
        mime = QMimeData()
        mime.setUrls([QUrl.fromLocalFile(str(target))])
        QGuiApplication.clipboard().setMimeData(mime)

    def prepare(self, path: str | Path, caption: str) -> None:
        self.copy_file(path)
        QDesktopServices.openUrl(self.intent_url(caption))

    def reveal(self, path: str | Path) -> None:
        target = Path(path).resolve()
        if sys.platform == "darwin":
            subprocess.Popen(["open", "-R", str(target)])
        elif sys.platform == "win32":
            subprocess.Popen(["explorer", f"/select,{target}"])
        else:
            QDesktopServices.openUrl(QUrl.fromLocalFile(str(target.parent)))

    def start_drag(self, source: object, path: str | Path) -> None:
        target = Path(path).resolve()
        mime = QMimeData()
        mime.setUrls([QUrl.fromLocalFile(str(target))])
        drag = QDrag(source)
        drag.setMimeData(mime)
        drag.exec(Qt.DropAction.CopyAction)
