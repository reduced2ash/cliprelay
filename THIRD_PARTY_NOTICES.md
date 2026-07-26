# Third-party notices

ClipRelay source code is licensed under MIT. Packaged application releases
also contain third-party software that remains under its own license. This
document is informational and does not replace those license terms.
Copies of the GNU GPL version 3 and GNU LGPL version 3 are included in the
repository's `LICENSES` directory and in every packaged application.

## Qt for Python

ClipRelay uses PySide6, Shiboken6, and dynamically loaded Qt libraries from
the Qt for Python community distribution.

- Project: [Qt for Python](https://doc.qt.io/qtforpython-6/)
- Source: [qtproject/pyside-pyside-setup](https://code.qt.io/cgit/pyside/pyside-setup.git/)
- License options: LGPL version 3, GPL version 3, or a Qt commercial license
- Community release policy: ClipRelay distributes the community packages
  under the LGPL version 3 terms

Users may replace the dynamically loaded Qt libraries with compatible
modified versions. ClipRelay does not impose terms that prohibit reverse
engineering for the purpose of debugging modifications to LGPL-covered
components.

## FFmpeg and FFprobe

Release packages bundle FFmpeg and FFprobe as separate command-line programs.
ClipRelay invokes them as external processes.

- Project and source: [FFmpeg](https://ffmpeg.org/)
- License details: [FFmpeg legal information](https://ffmpeg.org/legal.html)
- Build license: recorded in the `FFmpeg-build-info-*` file attached to each
  GitHub release

FFmpeg is primarily LGPL version 2.1 or later. Builds that enable GPL
components, including common H.264 encoders, are GPL version 2 or later and
may select GPL version 3 or later through their build configuration. The exact
configuration and license output are published beside every ClipRelay binary.

Redistributors are responsible for providing the complete corresponding
source for the exact FFmpeg build and its enabled external libraries. Stable
ClipRelay releases should not be mirrored without reviewing those obligations.

## Python runtime and direct Python dependencies

Packaged builds include the Python runtime and the runtime dependencies locked
in `uv.lock`. Principal projects and license families include:

| Component | License |
| --- | --- |
| Python | Python Software Foundation License |
| httpx / httpcore | BSD-3-Clause |
| keyring | MIT |
| platformdirs | MIT |
| qasync | BSD-2-Clause |
| send2trash | BSD-3-Clause |
| Telethon | MIT |
| watchdog | Apache-2.0 |

Transitive packages retain their own notices and license terms. The release
workflow publishes a dependency manifest for each release.

## Packaging tools

PyInstaller and Inno Setup are build-time tools. Their licenses do not replace
the licenses of the application or bundled libraries. Refer to:

- [PyInstaller licensing](https://pyinstaller.org/en/stable/license.html)
- [Inno Setup licensing](https://jrsoftware.org/files/is/license.txt)

This file is not legal advice. Anyone redistributing ClipRelay should perform
their own license review for the exact binaries they distribute.
