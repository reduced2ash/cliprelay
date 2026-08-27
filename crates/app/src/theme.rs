//! Theme palettes for Relay, Pitch Black, and Full White.
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
pub const MEDIA_WELL: &str = "#050509";
pub const MEDIA_TEXT: &str = "#F4F7FB";
pub const MEDIA_MUTED: &str = "#A9B2C0";

#[derive(Clone, Debug)]
pub struct Theme {
    pub mode: ThemeMode,
    pub is_light: bool,
    pub uses_blue_accent: bool,
    // Workbench chrome. These roles keep the title bar, activity rail,
    // Explorer, and content canvas distinct without turning them into cards.
    pub workbench_chrome: Hsla,
    pub workbench_rail: Hsla,
    pub workbench_explorer: Hsla,
    pub workbench_canvas: Hsla,
    pub workbench_header: Hsla,
    pub workbench_border: Hsla,
    pub workbench_selection: Hsla,
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
            "#0D0E13",
            "#13151C",
            "#171923",
            "#1C1F2A",
            "#262A38",
            "#20232F",
            "#F3F0EC",
            "#CBC8C5",
            "#92929A",
            "#6F717B",
            "#2B2E3A",
            "#414655",
            "#FF7152",
            "#E85C3E",
            "#2B1A1A",
            "#FF8A70",
            "#100E10",
            "#70C98D",
            "#12251A",
            "#E4AE5D",
            "#2C2415",
            "#EE7680",
            "#30191F",
            "#111219",
        )
    }

    pub fn pitch_black() -> Self {
        Self::from_palette(
            ThemeMode::PitchBlack,
            "#020305",
            "#07090D",
            "#0B0E14",
            "#11151D",
            "#192235",
            "#141B2A",
            "#F4F7FB",
            "#D2DAE6",
            "#96A0B0",
            "#7B879A",
            "#252C39",
            "#3A4658",
            "#1F6FEF",
            "#1658C7",
            "#0D1B38",
            "#78A9FF",
            "#FDFEFF",
            "#68C38A",
            "#0F251B",
            "#E3B45F",
            "#2A210F",
            "#F07B83",
            "#2E1418",
            "#151A23",
        )
    }

    pub fn full_white() -> Self {
        Self::from_palette(
            ThemeMode::FullWhite,
            "#F7F9FC",
            "#FDFEFF",
            "#FAFBFD",
            "#F0F3F8",
            "#E4EAF4",
            "#EAF0F8",
            "#171A21",
            "#343A46",
            "#5E697B",
            "#647083",
            "#D9DFE8",
            "#BBC5D3",
            "#1F6FEF",
            "#1658C7",
            "#E5EDFF",
            "#1554C5",
            "#FDFEFF",
            "#237A4E",
            "#E4F3EA",
            "#95620E",
            "#FFF1D6",
            "#BA3D49",
            "#FBE7E9",
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
        // Relay uses a smoked graphite hierarchy with a restrained violet
        // undertone. The warm coral accent stays rare and decisive while the
        // chrome, Explorer, and canvas remain distinct at a glance.
        let (
            workbench_chrome,
            workbench_rail,
            workbench_explorer,
            workbench_canvas,
            workbench_header,
            workbench_border,
            workbench_selection,
        ) = if mode == ThemeMode::Relay {
            (
                color("#151720"),
                color("#0A0B10"),
                color("#12141B"),
                color("#0D0E13"),
                color("#11131A"),
                color("#2A2D38"),
                color("#1D202A"),
            )
        } else {
            (
                color(ink),
                color(ink),
                color(surface),
                color(ink),
                color(surface),
                color(border),
                color(raised),
            )
        };
        Self {
            mode,
            is_light,
            uses_blue_accent: mode != ThemeMode::Relay,
            workbench_chrome,
            workbench_rail,
            workbench_explorer,
            workbench_canvas,
            workbench_header,
            workbench_border,
            workbench_selection,
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

// Action controls use Zed-like near-square geometry. The two-pixel radius is
// enough to soften corners at normal and high-DPI scales without becoming a
// rounded-rectangle style.
pub const RADIUS_SM: f32 = 2.0;
pub const RADIUS_MD: f32 = 5.0;
pub const RADIUS_LG: f32 = 7.0;

pub const CONTROL_HEIGHT: f32 = 36.0;
pub const COMPACT_CONTROL_HEIGHT: f32 = 32.0;
pub const WORKBENCH_CONTROL_HEIGHT: f32 = 28.0;

pub const FOCUS_WIDTH: f32 = 2.0;

pub const TILE_MIN_DEFAULT: f32 = 236.0;
pub const TILE_MIN_COMPACT: f32 = 176.0;
pub const TILE_CHROME_DEFAULT: f32 = 58.0;
pub const TILE_CHROME_COMPACT: f32 = 48.0;
pub const TILE_GAP_DEFAULT: f32 = 12.0;
pub const TILE_GAP_COMPACT: f32 = 8.0;
pub const TILE_RADIUS: f32 = 4.0;
pub const PREVIEW_DELAY_MS: u64 = 350;

// The workbench shell follows a narrow activity rail + useful explorer split.
// This keeps navigation present without spending a full text sidebar beside a
// second folder column, matching the approved desktop composition.
pub const SIDEBAR_EXPANDED_WIDTH: f32 = 196.0;
pub const SIDEBAR_COLLAPSED_WIDTH: f32 = 62.0;
// Includes the Explorer's owned left and right one-pixel seams.
pub const EXPLORER_WIDTH: f32 = 245.0;
pub const TITLE_BAR_HEIGHT: f32 = 56.0;
pub const WORKSPACE_TAB_HEIGHT: f32 = 32.0;
pub const CONTEXT_TOOLBAR_HEIGHT: f32 = 58.0;
