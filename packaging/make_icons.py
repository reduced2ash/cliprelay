from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

from PIL import Image
from PySide6.QtCore import QRectF
from PySide6.QtGui import QColor, QImage, QPainter
from PySide6.QtSvg import QSvgRenderer


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "src" / "cliprelay" / "assets" / "cliprelay.svg"
OUTPUT = ROOT / "packaging" / "icons"


def render(renderer: QSvgRenderer, size: int, target: Path) -> None:
    image = QImage(size, size, QImage.Format.Format_RGBA8888)
    image.fill(QColor(0, 0, 0, 0))
    painter = QPainter(image)
    renderer.render(painter, QRectF(0, 0, size, size))
    painter.end()
    if not image.save(str(target), "PNG"):
        raise RuntimeError(f"Could not write {target}")


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    renderer = QSvgRenderer(str(SOURCE))
    if not renderer.isValid():
        raise RuntimeError(f"Invalid SVG: {SOURCE}")
    master = OUTPUT / "cliprelay-1024.png"
    render(renderer, 1024, master)

    with Image.open(master) as image:
        image.save(
            OUTPUT / "cliprelay.ico",
            format="ICO",
            sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
        )

    if sys.platform == "darwin" and shutil.which("iconutil"):
        iconset = OUTPUT / "ClipRelay.iconset"
        iconset.mkdir(exist_ok=True)
        entries = {
            "icon_16x16.png": 16,
            "icon_16x16@2x.png": 32,
            "icon_32x32.png": 32,
            "icon_32x32@2x.png": 64,
            "icon_128x128.png": 128,
            "icon_128x128@2x.png": 256,
            "icon_256x256.png": 256,
            "icon_256x256@2x.png": 512,
            "icon_512x512.png": 512,
            "icon_512x512@2x.png": 1024,
        }
        for name, size in entries.items():
            render(renderer, size, iconset / name)
        subprocess.run(
            ["iconutil", "-c", "icns", str(iconset), "-o", str(OUTPUT / "cliprelay.icns")],
            check=True,
        )


if __name__ == "__main__":
    main()
