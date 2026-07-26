from __future__ import annotations

from cliprelay import app as app_module


class FakeStyleHints:
    def __init__(self, wheel_lines: int) -> None:
        self.wheel_lines = wheel_lines
        self.set_calls: list[int] = []

    def wheelScrollLines(self) -> int:
        return self.wheel_lines

    def setWheelScrollLines(self, value: int) -> None:
        self.wheel_lines = value
        self.set_calls.append(value)


class FakeApplication:
    def __init__(self, wheel_lines: int) -> None:
        self.hints = FakeStyleHints(wheel_lines)

    def styleHints(self) -> FakeStyleHints:
        return self.hints


def test_macos_mouse_wheel_scrolling_gets_a_useful_minimum(
    monkeypatch,
) -> None:
    monkeypatch.setattr(app_module.sys, "platform", "darwin")
    application = FakeApplication(3)

    app_module._configure_mouse_wheel_scrolling(application)

    assert application.hints.wheel_lines == 8
    assert application.hints.set_calls == [8]


def test_mouse_wheel_configuration_preserves_faster_or_non_macos_values(
    monkeypatch,
) -> None:
    monkeypatch.setattr(app_module.sys, "platform", "darwin")
    fast_application = FakeApplication(12)
    app_module._configure_mouse_wheel_scrolling(fast_application)
    assert fast_application.hints.set_calls == []

    monkeypatch.setattr(app_module.sys, "platform", "win32")
    windows_application = FakeApplication(3)
    app_module._configure_mouse_wheel_scrolling(windows_application)
    assert windows_application.hints.set_calls == []
