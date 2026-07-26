from __future__ import annotations

import asyncio
import json
import os
import platform
import subprocess
import sys
import time
from collections import deque
from typing import Any, Callable

from PySide6.QtCore import QObject, Property, QTimer, Signal, Slot
from PySide6.QtQuick import QQuickWindow

from .media import MediaProcessor


class PerformanceMonitor(QObject):
    """Low-overhead, on-demand diagnostics for the Qt Quick media surface."""

    stateChanged = Signal()

    def __init__(
        self,
        processor: MediaProcessor,
        settings_getter: Callable[[], dict[str, Any]],
    ) -> None:
        super().__init__()
        self._window: QQuickWindow | None = None
        self._processor = processor
        self._settings_getter = settings_getter
        self._active = False
        self._refresh_task: asyncio.Task | None = None
        self._gpu_name = ""
        self._last_frame_at = 0.0
        self._frame_intervals: deque[float] = deque(maxlen=240)
        self._state: dict[str, Any] = {
            "renderer": "Initializing",
            "gpu": "Detecting",
            "display": "Detecting",
            "decoder": self._decoder_label(),
            "exportEncoder": "Detecting",
            "framePacing": "Open Settings to sample",
            "frameSpikes": "—",
            "resourcePolicy": "Resident on hide / restore",
        }
        self._sample_timer = QTimer(self)
        self._sample_timer.setInterval(700)
        self._sample_timer.timeout.connect(self._publish_frame_sample)
        QTimer.singleShot(0, self.refresh)

    def attach_window(self, window: QQuickWindow) -> None:
        if self._window is window:
            return
        self._window = window
        self.setParent(window)
        window.screenChanged.connect(self._screen_changed)
        window.sceneGraphInitialized.connect(
            lambda: QTimer.singleShot(0, self.refresh)
        )
        self._screen_changed(window.screen())
        self.refresh()

    @Property("QVariantMap", notify=stateChanged)
    def state(self) -> dict[str, Any]:
        return dict(self._state)

    @Slot(bool)
    def setActive(self, active: bool) -> None:
        active = bool(active)
        if self._active == active:
            return
        self._active = active
        self._frame_intervals.clear()
        self._last_frame_at = 0.0
        if active:
            self._sample_timer.start()
            self.refresh()
        else:
            self._sample_timer.stop()
            self._update(
                framePacing="Paused while Diagnostics is hidden",
                frameSpikes="—",
            )

    @Slot()
    def settingsChanged(self) -> None:
        self._update(decoder=self._decoder_label())
        self.refresh()

    @Slot()
    def refresh(self) -> None:
        if self._refresh_task and not self._refresh_task.done():
            return
        try:
            loop = asyncio.get_running_loop()
        except RuntimeError:
            QTimer.singleShot(100, self.refresh)
            return
        self._refresh_task = loop.create_task(self._refresh_async())
        self._refresh_task.add_done_callback(self._refresh_done)

    @Slot()
    def shutdown(self) -> None:
        self._sample_timer.stop()
        if self._refresh_task:
            self._refresh_task.cancel()
            self._refresh_task = None

    def _refresh_done(self, task: asyncio.Task) -> None:
        if self._refresh_task is task:
            self._refresh_task = None
        if not task.cancelled():
            task.exception()

    async def _refresh_async(self) -> None:
        renderer = self._renderer_label()
        display = self._display_label()
        gpu, encoder = await asyncio.gather(
            asyncio.to_thread(self._detect_gpu_cached),
            asyncio.to_thread(self._processor.hardware_encoder_info),
        )
        settings = self._settings_getter()
        effective_mode = self._processor.encoder_mode
        preference = str(settings.get("export_encoder", "auto"))
        if effective_mode == "hardware" and encoder["available"]:
            export_encoder = f"{encoder['label']} · preferred"
        elif effective_mode == "hardware":
            export_encoder = "libx264 fallback · no compatible hardware encoder"
        elif preference == "software":
            export_encoder = "libx264 · software only"
        else:
            export_encoder = "libx264 · quality default"
        self._update(
            renderer=renderer,
            gpu=gpu,
            display=display,
            decoder=self._decoder_label(),
            exportEncoder=export_encoder,
            resourcePolicy=(
                "Resident · aggressive preload"
                if settings.get("performance_mode") == "maximum"
                else "Resident on hide / restore"
            ),
        )

    def _update(self, **changes: Any) -> None:
        next_state = {**self._state, **changes}
        if next_state == self._state:
            return
        self._state = next_state
        self.stateChanged.emit()

    def _renderer_label(self) -> str:
        if not self._window:
            return "Waiting for render surface"
        try:
            api = self._window.rendererInterface().graphicsApi()
            api_name = {
                "Metal": "Metal",
                "Direct3D11": "Direct3D 11",
                "Direct3D12": "Direct3D 12",
                "Vulkan": "Vulkan",
                "OpenGL": "OpenGL",
                "Software": "Software",
            }.get(api.name, api.name)
        except Exception:
            api_name = "Qt RHI"
        render_loop = os.environ.get("QSG_RENDER_LOOP", "threaded auto")
        persistence = (
            "resident"
            if (
                self._window.isPersistentGraphics()
                and self._window.isPersistentSceneGraph()
            )
            else "releases on hide"
        )
        return f"{api_name} · {render_loop} · VSync · {persistence}"

    def _display_label(self) -> str:
        if not self._window:
            return "Waiting for active display"
        screen = self._window.screen()
        if not screen:
            return "No active display"
        refresh_rate = max(1.0, float(screen.refreshRate() or 60.0))
        name = str(screen.name() or "").strip()
        prefix = f"{name} · " if name else ""
        return (
            f"{prefix}{refresh_rate:.0f} Hz · "
            f"{1000 / refresh_rate:.2f} ms budget"
        )

    def _decoder_label(self) -> str:
        maximum = (
            self._settings_getter().get("performance_mode") == "maximum"
            if hasattr(self, "_settings_getter")
            else False
        )
        suffix = "preferred · software fallback" if maximum else "automatic"
        if sys.platform == "darwin":
            return f"VideoToolbox · {suffix}"
        if sys.platform == "win32":
            return f"Direct3D hardware decode · {suffix}"
        return f"FFmpeg hardware decode · {suffix}"

    def _detect_gpu_cached(self) -> str:
        if not self._gpu_name:
            self._gpu_name = self._detect_gpu()
        return self._gpu_name

    @staticmethod
    def _detect_gpu() -> str:
        if sys.platform == "darwin":
            try:
                completed = subprocess.run(
                    [
                        "system_profiler",
                        "SPDisplaysDataType",
                        "-json",
                    ],
                    capture_output=True,
                    text=True,
                    timeout=12,
                    check=False,
                )
                payload = json.loads(completed.stdout or "{}")
                displays = payload.get("SPDisplaysDataType") or []
                names = []
                for item in displays:
                    name = (
                        item.get("sppci_model")
                        or item.get("spdisplays_chipset-model")
                        or item.get("_name")
                    )
                    if name and name not in names:
                        names.append(str(name))
                if names:
                    return " + ".join(names)
            except (
                OSError,
                subprocess.TimeoutExpired,
                json.JSONDecodeError,
            ):
                pass
        elif sys.platform == "win32":
            try:
                completed = subprocess.run(
                    [
                        "powershell",
                        "-NoProfile",
                        "-Command",
                        (
                            "Get-CimInstance Win32_VideoController | "
                            "Select-Object -ExpandProperty Name"
                        ),
                    ],
                    capture_output=True,
                    text=True,
                    timeout=12,
                    check=False,
                )
                names = [
                    line.strip()
                    for line in completed.stdout.splitlines()
                    if line.strip()
                ]
                if names:
                    return " + ".join(dict.fromkeys(names))
            except (OSError, subprocess.TimeoutExpired):
                pass
        return f"{platform.machine()} graphics"

    @Slot("QVariantList")
    def recordFrameBatch(self, values: list[Any]) -> None:
        if not self._active:
            return
        for value in values:
            try:
                frame_time = float(value)
            except (TypeError, ValueError):
                continue
            if 0.05 <= frame_time <= 100.0:
                self._frame_intervals.append(frame_time)
        if values:
            self._last_frame_at = time.perf_counter()

    @Slot()
    def _publish_frame_sample(self) -> None:
        if not self._window:
            return
        if (
            not self._last_frame_at
            or time.perf_counter() - self._last_frame_at > 1.2
        ):
            self._update(
                framePacing="Idle · Qt renders on demand",
                frameSpikes="0 while idle",
            )
            return
        if len(self._frame_intervals) < 3:
            self._update(
                framePacing="Sampling active frames",
                frameSpikes="Collecting",
            )
            return
        samples = sorted(self._frame_intervals)
        p95 = samples[min(len(samples) - 1, int(len(samples) * 0.95))]
        screen = self._window.screen()
        refresh_rate = (
            max(1.0, float(screen.refreshRate() or 60.0))
            if screen
            else 60.0
        )
        budget = 1000 / refresh_rate
        spikes = sum(value > budget * 1.55 for value in samples)
        self._update(
            framePacing=(
                f"{p95:.2f} ms p95 · {len(samples)} render frames"
            ),
            frameSpikes=(
                f"{spikes} above {budget * 1.55:.1f} ms · "
                f"{max(samples):.2f} ms worst"
            ),
        )

    @Slot(object)
    def _screen_changed(self, screen: Any) -> None:
        if screen:
            try:
                screen.refreshRateChanged.connect(self._screen_metrics_changed)
            except (AttributeError, RuntimeError):
                pass
        self._screen_metrics_changed()

    @Slot(float)
    def _screen_metrics_changed(self, _refresh_rate: float = 0.0) -> None:
        self._update(display=self._display_label())
