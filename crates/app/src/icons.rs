//! Embedded monochrome icon assets for the GPUI shell.
//!
//! The vocabulary follows gpui-component's Lucide-based icon conventions,
//! while remaining compatible with the published GPUI 0.2.2 used here.

use anyhow::Result;
use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

pub struct ClipRelayAssets;

macro_rules! icon_svg {
    ($body:literal) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">"#,
            $body,
            "</svg>"
        )
        .as_bytes()
    };
}

impl AssetSource for ClipRelayAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let bytes: Option<&'static [u8]> = match path {
            "brand/cliprelay-mark-color.svg" => {
                Some(include_bytes!("../assets/cliprelay-mark-color.svg"))
            }
            "icons/activity.svg" => Some(icon_svg!(r#"<path d="M3 12h4l2-5 3 10 3-7 2 2h4"/>"#)),
            "icons/arrow-down.svg" => Some(icon_svg!(
                r#"<path d="M12 4v16"/><path d="m6 14 6 6 6-6"/>"#
            )),
            "icons/arrow-left.svg" => Some(icon_svg!(
                r#"<path d="M20 12H4"/><path d="m10 6-6 6 6 6"/>"#
            )),
            "icons/arrow-right.svg" => Some(icon_svg!(
                r#"<path d="M4 12h16"/><path d="m14 6 6 6-6 6"/>"#
            )),
            "icons/arrow-up.svg" => Some(icon_svg!(
                r#"<path d="M12 20V4"/><path d="m6 10 6-6 6 6"/>"#
            )),
            "icons/check.svg" => Some(icon_svg!(r#"<path d="m5 12 4 4L19 6"/>"#)),
            "icons/chevron-down.svg" => Some(icon_svg!(r#"<path d="m6 9 6 6 6-6"/>"#)),
            "icons/chevron-left.svg" => Some(icon_svg!(r#"<path d="m15 18-6-6 6-6"/>"#)),
            "icons/chevron-right.svg" => Some(icon_svg!(r#"<path d="m9 18 6-6-6-6"/>"#)),
            "icons/chevron-up.svg" => Some(icon_svg!(r#"<path d="m18 15-6-6-6 6"/>"#)),
            "icons/copy.svg" => Some(icon_svg!(
                r#"<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/>"#
            )),
            "icons/crop.svg" => Some(icon_svg!(
                r#"<path d="M6 2v14a2 2 0 0 0 2 2h14"/><path d="M18 22V8a2 2 0 0 0-2-2H2"/>"#
            )),
            "icons/delivery-info.svg" => Some(icon_svg!(
                r#"<circle cx="12" cy="12" r="8"/><path d="M12 1.5V4M12 20v2.5M1.5 12H4M20 12h2.5M8 10h8"/>"#
            )),
            "icons/expand-horizontal.svg" => Some(icon_svg!(
                r#"<path d="M8 3 3 8l5 5"/><path d="M3 8h7"/><path d="m16 11 5 5-5 5"/><path d="M21 16h-7"/>"#
            )),
            "icons/ellipsis.svg" => Some(icon_svg!(
                r#"<circle cx="5" cy="12" r="1" fill="currentColor" stroke="none"/><circle cx="12" cy="12" r="1" fill="currentColor" stroke="none"/><circle cx="19" cy="12" r="1" fill="currentColor" stroke="none"/>"#
            )),
            "icons/external-link.svg" => Some(icon_svg!(
                r#"<path d="M15 3h6v6"/><path d="m10 14 11-11"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>"#
            )),
            "icons/folder.svg" => Some(icon_svg!(
                r#"<path stroke-width="1.65" d="M3.5 7.25A1.75 1.75 0 0 1 5.25 5.5h4l2 2h7.5a1.75 1.75 0 0 1 1.75 1.75v7.5a1.75 1.75 0 0 1-1.75 1.75H5.25a1.75 1.75 0 0 1-1.75-1.75z"/><path stroke-width="1.65" d="M3.75 9h16.5"/>"#
            )),
            "icons/folders.svg" => Some(icon_svg!(
                r#"<path d="M4 8V6.5A2.5 2.5 0 0 1 6.5 4H10l2 2h5.5A2.5 2.5 0 0 1 20 8.5V10"/><path d="M3 10.5A2.5 2.5 0 0 1 5.5 8H9l2 2h7.5A2.5 2.5 0 0 1 21 12.5v4A2.5 2.5 0 0 1 18.5 19h-13A2.5 2.5 0 0 1 3 16.5z"/>"#
            )),
            "icons/grid.svg" => Some(icon_svg!(
                r#"<rect x="4" y="4" width="6" height="6" rx="1"/><rect x="14" y="4" width="6" height="6" rx="1"/><rect x="4" y="14" width="6" height="6" rx="1"/><rect x="14" y="14" width="6" height="6" rx="1"/>"#
            )),
            "icons/history.svg" => Some(icon_svg!(
                r#"<path d="M3 12a9 9 0 1 0 3-6.7L3 8"/><path d="M3 3v5h5"/><path d="M12 7v5l3 2"/>"#
            )),
            "icons/info.svg" => Some(icon_svg!(
                r#"<circle cx="12" cy="12" r="9"/><path d="M12 11v5"/><path d="M12 8h.01"/>"#
            )),
            "icons/keyboard.svg" => Some(icon_svg!(
                r#"<rect x="3" y="6" width="18" height="12" rx="2"/><path d="M6 10h.01M9 10h.01M12 10h.01M15 10h.01M18 10h.01M7.5 14h9"/>"#
            )),
            "icons/maximize.svg" => Some(icon_svg!(
                r#"<path d="M8 3H3v5"/><path d="m3 3 6 6"/><path d="M16 3h5v5"/><path d="m21 3-6 6"/><path d="M8 21H3v-5"/><path d="m3 21 6-6"/><path d="M16 21h5v-5"/><path d="m21 21-6-6"/>"#
            )),
            "icons/mark-in.svg" => {
                Some(icon_svg!(r#"<path d="M6 4v16"/><path d="m18 7-5 5 5 5"/>"#))
            }
            "icons/mark-out.svg" => {
                Some(icon_svg!(r#"<path d="M18 4v16"/><path d="m6 7 5 5-5 5"/>"#))
            }
            "icons/panel-left.svg" => Some(icon_svg!(
                r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/>"#
            )),
            "icons/pause.svg" => Some(icon_svg!(r#"<path d="M8 5v14"/><path d="M16 5v14"/>"#)),
            "icons/pencil.svg" => Some(icon_svg!(
                r#"<path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L8 18l-4 1 1-4z"/>"#
            )),
            "icons/play.svg" => Some(icon_svg!(r#"<path d="m8 5 11 7-11 7z"/>"#)),
            "icons/plus.svg" => Some(icon_svg!(r#"<path d="M12 5v14"/><path d="M5 12h14"/>"#)),
            "icons/refresh.svg" => Some(icon_svg!(
                r#"<path d="M20 11a8 8 0 1 0-2.3 5.7"/><path d="M20 4v7h-7"/>"#
            )),
            "icons/search.svg" => Some(icon_svg!(
                r#"<circle cx="11" cy="11" r="7"/><path d="m20 20-4-4"/>"#
            )),
            "icons/send.svg" => Some(icon_svg!(
                r#"<path d="m22 2-7 20-4-9-9-4z"/><path d="M22 2 11 13"/>"#
            )),
            "icons/settings.svg" => Some(icon_svg!(
                r#"<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6v.2h-4V21a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1L4.2 17l.1-.1a1.7 1.7 0 0 0 .3-1.9A1.7 1.7 0 0 0 3 14H2.8v-4H3a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9L4.2 7 7 4.2l.1.1A1.7 1.7 0 0 0 9 4.6 1.7 1.7 0 0 0 10 3V2.8h4V3a1.7 1.7 0 0 0 1 1.6 1.7 1.7 0 0 0 1.9-.3l.1-.1L19.8 7l-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.6 1h.2v4H21a1.7 1.7 0 0 0-1.6 1Z"/>"#
            )),
            "icons/shuffle.svg" => Some(icon_svg!(
                r#"<path d="M3 7h3c4 0 5 10 9 10h6"/><path d="m18 14 3 3-3 3"/><path d="M3 17h3c4 0 5-10 9-10h6"/><path d="m18 4 3 3-3 3"/>"#
            )),
            "icons/skip-back.svg" => Some(icon_svg!(
                r#"<path d="M19 20 9 12l10-8z"/><path d="M5 19V5"/>"#
            )),
            "icons/skip-forward.svg" => Some(icon_svg!(
                r#"<path d="m5 4 10 8-10 8z"/><path d="M19 5v14"/>"#
            )),
            "icons/square.svg" => Some(icon_svg!(
                r#"<rect x="5" y="5" width="14" height="14" rx="2"/>"#
            )),
            "icons/target.svg" => Some(icon_svg!(
                r#"<circle cx="12" cy="12" r="8"/><circle cx="12" cy="12" r="3"/><path d="M12 2v3"/><path d="M12 19v3"/><path d="M2 12h3"/><path d="M19 12h3"/>"#
            )),
            "icons/terminal.svg" => {
                Some(icon_svg!(r#"<path d="m5 7 5 5-5 5"/><path d="M13 17h6"/>"#))
            }
            "icons/trash.svg" => Some(icon_svg!(
                r#"<path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="m19 6-1 15H6L5 6"/><path d="M10 11v5"/><path d="M14 11v5"/>"#
            )),
            "icons/warning.svg" => Some(icon_svg!(
                r#"<path d="m12 3 10 18H2z"/><path d="M12 9v5"/><path d="M12 17h.01"/>"#
            )),
            "icons/x-brand.svg" => Some(icon_svg!(
                r#"<path d="M4 3h4.8l11.2 18h-4.8z" fill="currentColor" stroke="none"/><path d="M20 3h-3.1L4 21h3.1z" fill="currentColor" stroke="none"/>"#
            )),
            "icons/x.svg" => Some(icon_svg!(
                r#"<path d="m6 6 12 12"/><path d="m18 6-12 12"/>"#
            )),
            _ => None,
        };
        Ok(bytes.map(Cow::Borrowed))
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

pub fn icon_path(name: &str) -> Option<&'static str> {
    Some(match name {
        "activity" | "⚡" => "icons/activity.svg",
        "arrow-down" => "icons/arrow-down.svg",
        "arrow-left" => "icons/arrow-left.svg",
        "arrow-right" => "icons/arrow-right.svg",
        "arrow-up" => "icons/arrow-up.svg",
        "check" | "✓" => "icons/check.svg",
        "chevron-down" | "▾" => "icons/chevron-down.svg",
        "chevron-left" | "←" | "◂" => "icons/chevron-left.svg",
        "chevron-right" | "→" | "▸" => "icons/chevron-right.svg",
        "chevron-up" | "▴" => "icons/chevron-up.svg",
        "contract" => "icons/expand-horizontal.svg",
        "copy" => "icons/copy.svg",
        "crop" => "icons/crop.svg",
        "delivery-info" => "icons/delivery-info.svg",
        "edit" => "icons/pencil.svg",
        "ellipsis" | "⋯" => "icons/ellipsis.svg",
        "expand" => "icons/expand-horizontal.svg",
        "expand-horizontal" => "icons/expand-horizontal.svg",
        "external" => "icons/external-link.svg",
        "external-link" | "↗" => "icons/external-link.svg",
        "folder" | "🗀" => "icons/folder.svg",
        "folders" => "icons/folders.svg",
        "grid" | "library" | "▤" => "icons/grid.svg",
        "history" | "◷" => "icons/history.svg",
        "info" | "i" => "icons/info.svg",
        "keyboard" => "icons/keyboard.svg",
        "maximize" | "⛶" | "⇔" => "icons/maximize.svg",
        "mark-in" => "icons/mark-in.svg",
        "mark-out" => "icons/mark-out.svg",
        "panel" => "icons/panel-left.svg",
        "panel-left" => "icons/panel-left.svg",
        "pause" | "⏸" => "icons/pause.svg",
        "pencil" | "✎" => "icons/pencil.svg",
        "play" | "▶" | "▷" => "icons/play.svg",
        "plus" | "+" => "icons/plus.svg",
        "refresh" | "↺" | "↻" => "icons/refresh.svg",
        "search" | "⌕" => "icons/search.svg",
        "send" | "➤" => "icons/send.svg",
        "settings" | "⚙" => "icons/settings.svg",
        "shuffle" | "⇄" => "icons/shuffle.svg",
        "skip-back" | "⏮" => "icons/skip-back.svg",
        "skip-forward" | "⏭" => "icons/skip-forward.svg",
        "square" | "■" | "▣" | "□" | "▭" => "icons/square.svg",
        "target" | "◎" => "icons/target.svg",
        "terminal" | ">_" => "icons/terminal.svg",
        "trash" | "🗑" => "icons/trash.svg",
        "warning" | "!" | "⚠" => "icons/warning.svg",
        "x-brand" => "icons/x-brand.svg",
        "x" | "close" | "✕" | "×" => "icons/x.svg",
        _ => return None,
    })
}
