# -*- mode: python ; coding: utf-8 -*-
from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path

from PyInstaller.utils.hooks import collect_submodules


root = Path(SPECPATH).parent
icon_dir = root / "packaging" / "icons"
qml_dir = root / "src" / "cliprelay" / "qml"
asset_dir = root / "src" / "cliprelay" / "assets"
release_version = os.environ.get("CLIPRELAY_VERSION", "0.1.0").removeprefix("v")
codesign_identity = os.environ.get("CLIPRELAY_CODESIGN_IDENTITY") or None
entitlements_path = root / "packaging" / "macos-entitlements.plist"
entitlements_file = (
    str(entitlements_path)
    if sys.platform == "darwin" and entitlements_path.is_file()
    else None
)

binaries = []
binary_root = os.environ.get("CLIPRELAY_FFMPEG_DIR", "")
for name in ("ffmpeg", "ffprobe"):
    executable = f"{name}.exe" if sys.platform == "win32" else name
    source = Path(binary_root) / executable if binary_root else Path(shutil.which(name) or "")
    if source.is_file():
        binaries.append((str(source), "bin"))

hiddenimports = collect_submodules("keyring.backends") + ["PySide6.QtMultimedia"]

a = Analysis(
    [str(root / "packaging" / "entrypoint.py")],
    pathex=[str(root / "src")],
    binaries=binaries,
    datas=[
        (str(qml_dir), "cliprelay/qml"),
        (str(asset_dir), "cliprelay/assets"),
        (str(root / "LICENSE"), "licenses"),
        (str(root / "LICENSES" / "GPL-3.0.txt"), "licenses"),
        (str(root / "LICENSES" / "LGPL-3.0.txt"), "licenses"),
        (str(root / "THIRD_PARTY_NOTICES.md"), "licenses"),
    ],
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=["tkinter"],
    noarchive=False,
)
pyz = PYZ(a.pure)

platform_icon = icon_dir / ("cliprelay.ico" if sys.platform == "win32" else "cliprelay.icns")
icon = str(platform_icon) if platform_icon.is_file() else None

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name="ClipRelay",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    console=False,
    icon=icon,
    codesign_identity=codesign_identity,
    entitlements_file=entitlements_file,
)
collection = COLLECT(
    exe,
    a.binaries,
    a.datas,
    strip=False,
    upx=False,
    name="ClipRelay",
)

if sys.platform == "darwin":
    app = BUNDLE(
        collection,
        name="ClipRelay.app",
        icon=icon,
        version=release_version,
        bundle_identifier="app.cliprelay.desktop",
        info_plist={
            "NSHighResolutionCapable": True,
            "NSRequiresAquaSystemAppearance": False,
            "LSMinimumSystemVersion": "12.0",
            "NSHumanReadableCopyright": "Copyright © 2026 ClipRelay contributors",
        },
    )
