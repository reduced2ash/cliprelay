//! Theme palettes — Relay, Pitch Black, Full White (DESIGN.md tokens).
#![allow(dead_code)]

use gpui::Hsla;

fn color(hex: &str) -> Hsla {
    Hsla::from(hex_to_rgba(hex))
}

fn hex_to_rgba(hex: &str) -> gpui::Rgba {
    let hex = hex.trim_start_matches('#');
    let value = u32::from_str_radix(hex, 16).unwrap_or(0);
    let r = ((value >> 16) & 0xFF) as f32 / 255.0;
    let g = ((value >> 8) & 0xFF) as f32 / 255.0;
    let b = (value & 0xFF) as f32 / 255.0;
    gpui::Rgba { r, g, b, a: 1.0 }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Relay,
    PitchBlack,
    FullWhite,
}

impl ThemeMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "pitch_black" => Self::PitchBlack,
            "full_white" => Self::FullWhite,
            _ => Self::Relay,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Relay => "relay",
            Self::PitchBlack => "pitch_black",
            Self::FullWhite => "full_white",
        }
    }
}

/// Media area colors are theme-independent.
pub const MEDIA_WELL: &str = "#05070B";
pub const MEDIA_TEXT: &str = "#F4F7FB";
pub const MEDIA_MUTED: &str = "#A9B2C0";

#[derive(Clone, Debug)]
pub struct Theme {
    pub mode: ThemeMode,
    pub is_light: bool,
    pub uses_blue_accent: bool,
    // neutrals
    pub ink: Hsla,
    pub surface: Hsla,
    pub surface_soft: Hsla,
    pub raised: Hsla,
    pub active: Hsla,
    pub hover: Hsla,
    pub text: Hsla,
    pub text_soft: Hsla,
    pub muted: Hsla,
    pub muted_soft: Hsla,
    pub border: Hsla,
    pub border_strong: Hsla,
    // accent
    pub accent: Hsla,
    pub accent_pressed: Hsla,
    pub accent_soft: Hsla,
    pub accent_text: Hsla,
    pub accent_content: Hsla,
    // semantic
    pub success: Hsla,
    pub success_soft: Hsla,
    pub warning: Hsla,
    pub warning_soft: Hsla,
    pub error: Hsla,
    pub error_soft: Hsla,
    // media overlay (letterbox dim)
    pub media_overlay: Hsla,
    // media-area text (theme-independent light-on-dark)
    pub media_text: Hsla,
    pub media_muted: Hsla,
}

impl Theme {
    pub fn relay() -> Self {
        Self::from_palette(
            ThemeMode::Relay,
            "#151416", "#1D1B1E", "#211F22", "#262328", "#312C32", "#2B272D",
            "#F1ECE8", "#D3CBCA", "#ABA3A4", "#827A7E", "#3A343B", "#4B434C",
            "#F07858", "#D96247", "#3C2928", "#F07858", "#151416",
            "#72B985", "#213229", "#D8A758", "#352D20", "#DD6B70", "#392426",
            "#211F22",
        )
    }

    pub fn pitch_black() -> Self {
        Self::from_palette(
            ThemeMode::PitchBlack,
            "#020305", "#07090D", "#0B0E14", "#11151D", "#192235", "#141B2A",
            "#F4F7FB", "#D2DAE6", "#96A0B0", "#697588", "#252C39", "#3A4658",
            "#1F6FEF", "#1658C7", "#0D1B38", "#78A9FF", "#FDFEFF",
            "#68C38A", "#0F251B", "#E3B45F", "#2A210F", "#F07B83", "#2E1418",
            "#151A23",
        )
    }

    pub fn full_white() -> Self {
        Self::from_palette(
            ThemeMode::FullWhite,
            "#F7F9FC", "#FDFEFF", "#FAFBFD", "#F0F3F8", "#E4EAF4", "#EAF0F8",
            "#171A21", "#343A46", "#5E697B", "#7C8799", "#D9DFE8", "#BBC5D3",
            "#1F6FEF", "#1658C7", "#E5EDFF", "#1554C5", "#FDFEFF",
            "#237A4E", "#E4F3EA", "#95620E", "#FFF1D6", "#BA3D49", "#FBE7E9",
            "#151A23",
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_palette(
        mode: ThemeMode,
        ink: &str,
        surface: &str,
        surface_soft: &str,
        raised: &str,
        active: &str,
        hover: &str,
        text: &str,
        text_soft: &str,
        muted: &str,
        muted_soft: &str,
        border: &str,
        border_strong: &str,
        accent: &str,
        accent_pressed: &str,
        accent_soft: &str,
        accent_text: &str,
        accent_content: &str,
        success: &str,
        success_soft: &str,
        warning: &str,
        warning_soft: &str,
        error: &str,
        error_soft: &str,
        media_overlay: &str,
    ) -> Self {
        let is_light = mode == ThemeMode::FullWhite;
        Self {
            mode,
            is_light,
            uses_blue_accent: mode != ThemeMode::Relay,
            ink: color(ink),
            surface: color(surface),
            surface_soft: color(surface_soft),
            raised: color(raised),
            active: color(active),
            hover: color(hover),
            text: color(text),
            text_soft: color(text_soft),
            muted: color(muted),
            muted_soft: color(muted_soft),
            border: color(border),
            border_strong: color(border_strong),
            accent: color(accent),
            accent_pressed: color(accent_pressed),
            accent_soft: color(accent_soft),
            accent_text: color(accent_text),
            accent_content: color(accent_content),
            success: color(success),
            success_soft: color(success_soft),
            warning: color(warning),
            warning_soft: color(warning_soft),
            error: color(error),
            error_soft: color(error_soft),
            media_overlay: color(media_overlay),
            media_text: color(MEDIA_TEXT),
            media_muted: color(MEDIA_MUTED),
        }
    }

    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Relay => Self::relay(),
            ThemeMode::PitchBlack => Self::pitch_black(),
            ThemeMode::FullWhite => Self::full_white(),
        }
    }
}

// Shared layout constants from the design system.
pub const SPACING_XS: f32 = 4.0;
pub const SPACING_SM: f32 = 8.0;
pub const SPACING_MD: f32 = 12.0;
pub const SPACING_LG: f32 = 16.0;
pub const SPACING_XL: f32 = 24.0;
pub const SPACING_XXL: f32 = 32.0;

pub const RADIUS_SM: f32 = 6.0;
pub const RADIUS_MD: f32 = 10.0;
pub const RADIUS_LG: f32 = 14.0;

pub const CONTROL_HEIGHT: f32 = 44.0;
pub const COMPACT_CONTROL_HEIGHT: f32 = 40.0;
pub const WORKBENCH_CONTROL_HEIGHT: f32 = 30.0;

pub const FOCUS_WIDTH: f32 = 2.0;

pub const TILE_MIN_DEFAULT: f32 = 236.0;
pub const TILE_MIN_COMPACT: f32 = 176.0;
pub const TILE_CHROME_DEFAULT: f32 = 58.0;
pub const TILE_CHROME_COMPACT: f32 = 48.0;
pub const TILE_GAP_DEFAULT: f32 = 12.0;
pub const TILE_GAP_COMPACT: f32 = 8.0;
pub const TILE_RADIUS: f32 = 4.0;
pub const PREVIEW_DELAY_MS: u64 = 350;

pub const SIDEBAR_EXPANDED_WIDTH: f32 = 184.0;
pub const SIDEBAR_COLLAPSED_WIDTH: f32 = 64.0;
pub const EXPLORER_WIDTH: f32 = 204.0;
pub const TITLE_BAR_HEIGHT: f32 = 40.0;
pub const WORKSPACE_TAB_HEIGHT: f32 = 34.0;
pub const CONTEXT_TOOLBAR_HEIGHT: f32 = 42.0;
