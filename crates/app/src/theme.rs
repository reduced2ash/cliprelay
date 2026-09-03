//! Theme palettes for Relay, Pitch Black, Full White, and glass materials.
#![allow(dead_code)]

use gpui::{linear_color_stop, linear_gradient, point, px, Background, BoxShadow, Hsla, Rgba};
use std::collections::HashMap;

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

/// Selector prefix persisted in the `theme_mode` setting for user themes
/// (e.g. `"custom:custom-2"`). The id after the prefix keys into the
/// `custom_themes` setting.
pub const CUSTOM_THEME_PREFIX: &str = "custom:";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Relay,
    PitchBlack,
    FullWhite,
    FrostedGlass,
    GraphiteGlass,
    /// A user-created theme; the payload is the custom theme id.
    Custom(String),
}

/// Physical state for the app's deliberately shallow interactive surfaces.
/// Passive panes stay on the ordinary semantic color ladder; this state is
/// reserved for controls and explicit selections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TactileState {
    Rest,
    Hover,
    Pressed,
}

impl ThemeMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "pitch_black" => Self::PitchBlack,
            "full_white" => Self::FullWhite,
            "frosted_glass" => Self::FrostedGlass,
            "graphite_glass" => Self::GraphiteGlass,
            _ if value.starts_with(CUSTOM_THEME_PREFIX) => {
                Self::Custom(value[CUSTOM_THEME_PREFIX.len()..].to_string())
            }
            _ => Self::Relay,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Relay => "relay",
            Self::PitchBlack => "pitch_black",
            Self::FullWhite => "full_white",
            Self::FrostedGlass => "frosted_glass",
            Self::GraphiteGlass => "graphite_glass",
            Self::Custom(id) => id.as_str(),
        }
    }

    /// Persisted selector for the `theme_mode` setting (`"custom:<id>"` for
    /// user themes, the plain mode id otherwise).
    pub fn selection_key(&self) -> String {
        match self {
            Self::Custom(id) => format!("{CUSTOM_THEME_PREFIX}{id}"),
            _ => self.as_str().to_string(),
        }
    }

    /// Display label for the built-in modes, in rail order.
    pub fn builtin_label(mode: &str) -> &'static str {
        match mode {
            "pitch_black" => "Pitch black",
            "full_white" => "Full white",
            "frosted_glass" => "Frosted glass",
            "graphite_glass" => "Graphite glass",
            _ => "Relay",
        }
    }
}

/// Built-in mode ids in rail order, for pickers and duplication flows.
pub const BUILTIN_THEME_MODES: [&str; 5] = [
    "relay",
    "pitch_black",
    "full_white",
    "frosted_glass",
    "graphite_glass",
];

/// A user-created theme: a built-in base palette plus a sparse set of
/// `role -> #RRGGBB` overrides. Overrides carry RGB only; the base color's
/// alpha is preserved on apply so glass bases stay translucent after edits.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CustomTheme {
    pub id: String,
    pub name: String,
    pub base: String,
    #[serde(default)]
    pub colors: HashMap<String, String>,
}

impl CustomTheme {
    pub fn base_theme(&self) -> Theme {
        Theme::for_mode(ThemeMode::parse(&self.base))
    }

    /// The fully resolved palette: base plus overrides.
    pub fn resolve(&self) -> Theme {
        let mut theme = self.base_theme();
        for (key, hex) in &self.colors {
            let Some(role) = THEME_ROLES.iter().find(|role| role.key == key.as_str()) else {
                continue;
            };
            let Some((r, g, b)) = parse_hex_color(hex) else {
                continue;
            };
            let current = (role.get)(&theme);
            let mut next = Hsla::from(Rgba { r, g, b, a: 1.0 });
            next.a = current.a;
            (role.set)(&mut theme, next);
        }
        theme
    }

    /// Effective color for a role: the override when present, else base.
    pub fn effective(&self, role: &ThemeRole) -> Hsla {
        (role.get)(&self.resolve())
    }

    /// Display hex for a role: the stored override, else the base hex.
    pub fn role_hex(&self, role: &ThemeRole) -> String {
        self.colors
            .get(role.key)
            .cloned()
            .unwrap_or_else(|| hsla_to_hex((role.get)(&self.base_theme())))
    }
}

/// One user-editable color role: a stable key plus typed accessors into
/// [`Theme`] so the configurator never duplicates palette wiring.
pub struct ThemeRole {
    pub key: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    pub get: fn(&Theme) -> Hsla,
    pub set: fn(&mut Theme, Hsla),
}

macro_rules! theme_role {
    ($get:ident, $set:ident, $field:ident) => {
        fn $get(theme: &Theme) -> Hsla {
            theme.$field
        }
        fn $set(theme: &mut Theme, value: Hsla) {
            theme.$field = value;
        }
    };
}

theme_role!(role_get_accent, role_set_accent, accent);
theme_role!(
    role_get_accent_pressed,
    role_set_accent_pressed,
    accent_pressed
);
theme_role!(role_get_accent_soft, role_set_accent_soft, accent_soft);
theme_role!(role_get_accent_text, role_set_accent_text, accent_text);
theme_role!(
    role_get_accent_content,
    role_set_accent_content,
    accent_content
);
theme_role!(role_get_ink, role_set_ink, ink);
theme_role!(role_get_surface, role_set_surface, surface);
theme_role!(role_get_surface_soft, role_set_surface_soft, surface_soft);
theme_role!(role_get_raised, role_set_raised, raised);
theme_role!(role_get_active, role_set_active, active);
theme_role!(role_get_hover, role_set_hover, hover);
theme_role!(role_get_text, role_set_text, text);
theme_role!(role_get_text_soft, role_set_text_soft, text_soft);
theme_role!(role_get_muted, role_set_muted, muted);
theme_role!(role_get_muted_soft, role_set_muted_soft, muted_soft);
theme_role!(role_get_border, role_set_border, border);
theme_role!(
    role_get_border_strong,
    role_set_border_strong,
    border_strong
);
theme_role!(role_get_success, role_set_success, success);
theme_role!(role_get_success_soft, role_set_success_soft, success_soft);
theme_role!(role_get_warning, role_set_warning, warning);
theme_role!(role_get_warning_soft, role_set_warning_soft, warning_soft);
theme_role!(role_get_error, role_set_error, error);
theme_role!(role_get_error_soft, role_set_error_soft, error_soft);
theme_role!(
    role_get_workbench_chrome,
    role_set_workbench_chrome,
    workbench_chrome
);
theme_role!(
    role_get_workbench_rail,
    role_set_workbench_rail,
    workbench_rail
);
theme_role!(
    role_get_workbench_explorer,
    role_set_workbench_explorer,
    workbench_explorer
);
theme_role!(
    role_get_workbench_canvas,
    role_set_workbench_canvas,
    workbench_canvas
);
theme_role!(
    role_get_workbench_header,
    role_set_workbench_header,
    workbench_header
);
theme_role!(
    role_get_workbench_border,
    role_set_workbench_border,
    workbench_border
);
theme_role!(
    role_get_workbench_selection,
    role_set_workbench_selection,
    workbench_selection
);
theme_role!(
    role_get_media_overlay,
    role_set_media_overlay,
    media_overlay
);

/// Roles shown without opening Advanced: the handful that restyle the app
/// on their own (backgrounds, text, hairlines, accent).
pub const BASIC_THEME_ROLE_KEYS: [&str; 8] = [
    "accent",
    "ink",
    "surface",
    "raised",
    "text",
    "text_soft",
    "muted",
    "border",
];

/// Every color a custom theme may override, grouped for the configurator.
pub const THEME_ROLES: &[ThemeRole] = &[
    ThemeRole {
        key: "workbench_chrome",
        label: "Top chrome",
        group: "Chrome",
        get: role_get_workbench_chrome,
        set: role_set_workbench_chrome,
    },
    ThemeRole {
        key: "workbench_rail",
        label: "Activity rail",
        group: "Chrome",
        get: role_get_workbench_rail,
        set: role_set_workbench_rail,
    },
    ThemeRole {
        key: "workbench_explorer",
        label: "Explorer",
        group: "Chrome",
        get: role_get_workbench_explorer,
        set: role_set_workbench_explorer,
    },
    ThemeRole {
        key: "workbench_canvas",
        label: "Canvas",
        group: "Chrome",
        get: role_get_workbench_canvas,
        set: role_set_workbench_canvas,
    },
    ThemeRole {
        key: "workbench_header",
        label: "Section headers",
        group: "Chrome",
        get: role_get_workbench_header,
        set: role_set_workbench_header,
    },
    ThemeRole {
        key: "workbench_border",
        label: "Chrome borders",
        group: "Chrome",
        get: role_get_workbench_border,
        set: role_set_workbench_border,
    },
    ThemeRole {
        key: "workbench_selection",
        label: "Chrome selection",
        group: "Chrome",
        get: role_get_workbench_selection,
        set: role_set_workbench_selection,
    },
    ThemeRole {
        key: "ink",
        label: "App background",
        group: "Surfaces",
        get: role_get_ink,
        set: role_set_ink,
    },
    ThemeRole {
        key: "surface",
        label: "Cards",
        group: "Surfaces",
        get: role_get_surface,
        set: role_set_surface,
    },
    ThemeRole {
        key: "surface_soft",
        label: "Popups",
        group: "Surfaces",
        get: role_get_surface_soft,
        set: role_set_surface_soft,
    },
    ThemeRole {
        key: "raised",
        label: "Fields",
        group: "Surfaces",
        get: role_get_raised,
        set: role_set_raised,
    },
    ThemeRole {
        key: "active",
        label: "Selected rows",
        group: "Surfaces",
        get: role_get_active,
        set: role_set_active,
    },
    ThemeRole {
        key: "hover",
        label: "Hover rows",
        group: "Surfaces",
        get: role_get_hover,
        set: role_set_hover,
    },
    ThemeRole {
        key: "text",
        label: "Primary text",
        group: "Text & hairlines",
        get: role_get_text,
        set: role_set_text,
    },
    ThemeRole {
        key: "text_soft",
        label: "Secondary text",
        group: "Text & hairlines",
        get: role_get_text_soft,
        set: role_set_text_soft,
    },
    ThemeRole {
        key: "muted",
        label: "Muted text",
        group: "Text & hairlines",
        get: role_get_muted,
        set: role_set_muted,
    },
    ThemeRole {
        key: "muted_soft",
        label: "Faint text",
        group: "Text & hairlines",
        get: role_get_muted_soft,
        set: role_set_muted_soft,
    },
    ThemeRole {
        key: "border",
        label: "Hairlines",
        group: "Text & hairlines",
        get: role_get_border,
        set: role_set_border,
    },
    ThemeRole {
        key: "border_strong",
        label: "Strong hairlines",
        group: "Text & hairlines",
        get: role_get_border_strong,
        set: role_set_border_strong,
    },
    ThemeRole {
        key: "accent",
        label: "Accent",
        group: "Accent",
        get: role_get_accent,
        set: role_set_accent,
    },
    ThemeRole {
        key: "accent_pressed",
        label: "Accent pressed",
        group: "Accent",
        get: role_get_accent_pressed,
        set: role_set_accent_pressed,
    },
    ThemeRole {
        key: "accent_soft",
        label: "Accent wash",
        group: "Accent",
        get: role_get_accent_soft,
        set: role_set_accent_soft,
    },
    ThemeRole {
        key: "accent_text",
        label: "Accent text",
        group: "Accent",
        get: role_get_accent_text,
        set: role_set_accent_text,
    },
    ThemeRole {
        key: "accent_content",
        label: "Text on accent",
        group: "Accent",
        get: role_get_accent_content,
        set: role_set_accent_content,
    },
    ThemeRole {
        key: "success",
        label: "Success",
        group: "Status",
        get: role_get_success,
        set: role_set_success,
    },
    ThemeRole {
        key: "success_soft",
        label: "Success wash",
        group: "Status",
        get: role_get_success_soft,
        set: role_set_success_soft,
    },
    ThemeRole {
        key: "warning",
        label: "Warning",
        group: "Status",
        get: role_get_warning,
        set: role_set_warning,
    },
    ThemeRole {
        key: "warning_soft",
        label: "Warning wash",
        group: "Status",
        get: role_get_warning_soft,
        set: role_set_warning_soft,
    },
    ThemeRole {
        key: "error",
        label: "Error",
        group: "Status",
        get: role_get_error,
        set: role_set_error,
    },
    ThemeRole {
        key: "error_soft",
        label: "Error wash",
        group: "Status",
        get: role_get_error_soft,
        set: role_set_error_soft,
    },
    ThemeRole {
        key: "media_overlay",
        label: "Media dim",
        group: "Media",
        get: role_get_media_overlay,
        set: role_set_media_overlay,
    },
];

/// Render a color as `#RRGGBB` (alpha is structural and never edited).
pub fn hsla_to_hex(color: Hsla) -> String {
    let rgb = Rgba::from(color);
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02X}{:02X}{:02X}",
        channel(rgb.r),
        channel(rgb.g),
        channel(rgb.b)
    )
}

/// Decode the stored `custom_themes` array. Corrupt entries are dropped so
/// one bad edit can never lock the theme system out.
pub fn decode_custom_themes(value: Option<&serde_json::Value>) -> Vec<CustomTheme> {
    value
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| serde_json::from_value(item.clone()).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Normalize user input to uppercase `#RRGGBB` (`#RGB` expands). `None`
/// means the text is not a hex color at all.
pub fn normalize_hex_color(text: &str) -> Option<String> {
    let hex = text.trim().strip_prefix('#').unwrap_or(text.trim());
    let full = match hex.len() {
        3 => hex
            .as_bytes()
            .iter()
            .map(|byte| format!("{}{}", *byte as char, *byte as char))
            .collect::<String>(),
        6 => hex.to_string(),
        _ => return None,
    };
    if full.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(format!("#{}", full.to_uppercase()))
    } else {
        None
    }
}

/// Parse `#RRGGBB` or `#RGB` (leading `#` optional) into linear channels.
pub fn parse_hex_color(text: &str) -> Option<(f32, f32, f32)> {
    let hex = text.trim().strip_prefix('#').unwrap_or(text.trim());
    let channel = |pair: &str| u8::from_str_radix(pair, 16).ok().map(|v| v as f32 / 255.0);
    match hex.len() {
        3 => {
            let bytes = hex.as_bytes();
            let expand = |i: usize| channel(&format!("{}{}", bytes[i] as char, bytes[i] as char));
            Some((expand(0)?, expand(1)?, expand(2)?))
        }
        6 => Some((
            channel(&hex[0..2])?,
            channel(&hex[2..4])?,
            channel(&hex[4..6])?,
        )),
        _ => None,
    }
}

/// Media area colors are theme-independent.
pub const MEDIA_WELL: &str = "#050509";
pub const MEDIA_TEXT: &str = "#F4F7FB";
pub const MEDIA_MUTED: &str = "#A9B2C0";

#[derive(Clone, Debug, PartialEq)]
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
    fn shifted(color: Hsla, lightness: f32, alpha: f32) -> Hsla {
        Hsla {
            l: (color.l + lightness).clamp(0.0, 1.0),
            a: (color.a * alpha).clamp(0.0, 1.0),
            ..color
        }
    }

    fn tactile_gradient(
        &self,
        base: Hsla,
        state: TactileState,
        glass_bottom: Option<Hsla>,
    ) -> Background {
        if self.is_frosted() {
            let bottom = glass_bottom.unwrap_or(self.raised.opacity(0.30));
            return match state {
                TactileState::Rest => linear_gradient(
                    180.0,
                    linear_color_stop(Self::shifted(base, 0.0, 0.92), 0.0),
                    linear_color_stop(Self::shifted(bottom, 0.0, 0.82), 1.0),
                ),
                TactileState::Hover => linear_gradient(
                    180.0,
                    linear_color_stop(Self::shifted(base, 0.0, 1.18), 0.0),
                    linear_color_stop(Self::shifted(bottom, 0.0, 0.72), 1.0),
                ),
                TactileState::Pressed => linear_gradient(
                    180.0,
                    linear_color_stop(Self::shifted(bottom, 0.0, 1.02), 0.0),
                    linear_color_stop(Self::shifted(base, -0.03, 0.62), 1.0),
                ),
            };
        }

        let (top, bottom) = match state {
            TactileState::Rest => (0.035, -0.018),
            TactileState::Hover => (0.055, -0.004),
            TactileState::Pressed => (-0.030, -0.055),
        };
        linear_gradient(
            180.0,
            linear_color_stop(Self::shifted(base, top, 1.0), 0.0),
            linear_color_stop(Self::shifted(base, bottom, 1.0), 1.0),
        )
    }

    /// Neutral face used by bordered buttons and button-like controls.
    pub fn control_face(&self, state: TactileState) -> Background {
        let base = if self.is_frosted() {
            self.surface_soft
        } else {
            self.raised
        };
        self.tactile_gradient(base, state, None)
    }

    /// Coral/blue commit face. Accent ownership remains unchanged; only the
    /// face-to-edge tonal shaping changes with physical state.
    pub fn accent_control_face(&self, state: TactileState) -> Background {
        let base = match state {
            TactileState::Pressed => self.accent_pressed,
            _ => self.accent,
        };
        self.tactile_gradient(base, state, Some(self.accent_pressed))
    }

    pub fn danger_control_face(&self, state: TactileState) -> Background {
        self.tactile_gradient(self.error_soft, state, Some(self.error_soft))
    }

    /// Raised selection face for navigation and explicit choices. The accent
    /// variant keeps the existing selected hue; the neutral variant keeps
    /// Explorer/workspace selection on the workbench graphite ladder.
    pub fn selection_face(&self, state: TactileState, accented: bool) -> Background {
        let base = if accented {
            self.accent_soft
        } else {
            self.workbench_selection
        };
        self.tactile_gradient(base, state, Some(self.raised.opacity(0.34)))
    }

    pub fn tactile_edge(&self, state: TactileState, accented: bool) -> Hsla {
        if accented {
            return self.accent.opacity(match state {
                TactileState::Rest => 0.58,
                TactileState::Hover => 0.78,
                TactileState::Pressed => 0.46,
            });
        }
        self.border_strong.opacity(match state {
            TactileState::Rest => 0.68,
            TactileState::Hover => 0.88,
            TactileState::Pressed => 0.54,
        })
    }

    /// Directional contact shadow with a faint upper reflection. Its compact
    /// form is shared by toolbars, tabs, and selected navigation items.
    pub fn tactile_shadow(&self, state: TactileState, compact: bool) -> Vec<BoxShadow> {
        let (offset, blur, spread, alpha) = match (state, compact) {
            (TactileState::Rest, false) => (2.0, 5.0, -1.0, 0.34),
            (TactileState::Hover, false) => (3.0, 7.0, -2.0, 0.42),
            (TactileState::Pressed, false) => (1.0, 2.0, -1.0, 0.28),
            (TactileState::Rest, true) => (1.0, 3.0, -1.0, 0.30),
            (TactileState::Hover, true) => (2.0, 5.0, -2.0, 0.38),
            (TactileState::Pressed, true) => (1.0, 1.0, -1.0, 0.24),
        };
        let dark_alpha = if self.is_light { alpha * 0.58 } else { alpha };
        let reflection_alpha = if self.is_light { 0.62 } else { 0.075 };
        vec![
            BoxShadow {
                color: color("#000000").opacity(dark_alpha),
                offset: point(px(0.0), px(offset)),
                blur_radius: px(blur),
                spread_radius: px(spread),
            },
            BoxShadow {
                color: color("#FFFFFF").opacity(reflection_alpha),
                offset: point(px(0.0), px(-1.0)),
                blur_radius: px(if compact { 1.0 } else { 2.0 }),
                spread_radius: px(-1.0),
            },
        ]
    }

    /// Focused Prepare is a dark, color-managed editing environment even when
    /// the surrounding workbench uses a lighter application theme. Keeping the
    /// palette here lets fields, combos, focus states, and media controls share
    /// one semantic set instead of accumulating one-off paint values.
    pub fn prepare_studio() -> Self {
        Self::from_palette(
            ThemeMode::Relay,
            "#0C1014",
            "#14181D",
            "#171C21",
            "#1B2127",
            "#222A32",
            "#1D242B",
            "#EEF1F4",
            "#C7CDD4",
            "#929BA6",
            "#707A85",
            "#2C343D",
            "#414B56",
            "#F06B4E",
            "#DA5B40",
            "#2B1C19",
            "#FF8062",
            "#0B0E11",
            "#36D184",
            "#10271B",
            "#F1B32B",
            "#2B2412",
            "#F27B83",
            "#30191F",
            "#081019",
        )
    }

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

    /// Frosted Glass is ClipRelay's opt-in macOS-style material theme. Every
    /// structural surface is achromatic and translucent so its apparent color
    /// comes exclusively from the native blurred backdrop.
    pub fn frosted_glass() -> Self {
        let mut theme = Self::from_palette(
            ThemeMode::FrostedGlass,
            "#000000",
            "#FFFFFF",
            "#FFFFFF",
            "#000000",
            "#FFFFFF",
            "#FFFFFF",
            "#F4F4F6",
            "#D4D4D8",
            "#A8A8AE",
            "#8E8E93",
            "#FFFFFF",
            "#FFFFFF",
            "#FF7259",
            "#E95742",
            "#572E2C",
            "#FF9B88",
            "#090A0D",
            "#61E4B1",
            "#153B31",
            "#FFD278",
            "#44381D",
            "#FF8799",
            "#44242C",
            "#06080C",
        );

        let light = color("#FFFFFF");
        let dark = color("#000000");
        theme.workbench_chrome = light.opacity(0.055);
        theme.workbench_rail = light.opacity(0.035);
        theme.workbench_explorer = light.opacity(0.050);
        theme.workbench_canvas = dark.opacity(0.080);
        theme.workbench_header = light.opacity(0.055);
        theme.workbench_border = light.opacity(0.14);
        theme.workbench_selection = light.opacity(0.12);
        theme.ink = dark.opacity(0.10);
        theme.surface = light.opacity(0.08);
        theme.surface_soft = light.opacity(0.12);
        theme.raised = dark.opacity(0.48);
        theme.active = light.opacity(0.16);
        theme.hover = light.opacity(0.08);
        theme.border = light.opacity(0.14);
        theme.border_strong = light.opacity(0.32);
        theme.accent_soft = color("#572E2C").opacity(0.56);
        theme.success_soft = color("#153B31").opacity(0.62);
        theme.warning_soft = color("#44381D").opacity(0.64);
        theme.error_soft = color("#44242C").opacity(0.64);
        theme
    }

    /// Graphite Glass copies the colorless Frosted Glass system exactly, then
    /// gives its translucent materials a denser silver/graphite treatment.
    /// The heavier neutral root tint keeps the desktop impression diffuse
    /// while still allowing the native compositor blur to shape the material.
    pub fn graphite_glass() -> Self {
        let mut theme = Self::frosted_glass();
        theme.mode = ThemeMode::GraphiteGlass;

        let silver = color("#D6D6D6");
        let bright_silver = color("#F0F0F0");
        let dark = color("#171717");
        theme.workbench_chrome = silver.opacity(0.16);
        theme.workbench_rail = silver.opacity(0.09);
        theme.workbench_explorer = silver.opacity(0.12);
        theme.workbench_canvas = dark.opacity(0.16);
        theme.workbench_header = bright_silver.opacity(0.14);
        theme.workbench_border = bright_silver.opacity(0.18);
        theme.workbench_selection = bright_silver.opacity(0.18);
        theme.ink = dark.opacity(0.18);
        theme.surface = silver.opacity(0.12);
        theme.surface_soft = bright_silver.opacity(0.16);
        theme.raised = dark.opacity(0.56);
        theme.active = bright_silver.opacity(0.22);
        theme.hover = bright_silver.opacity(0.10);
        theme.border = bright_silver.opacity(0.16);
        theme.border_strong = bright_silver.opacity(0.34);
        theme
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
        let uses_blue_accent = mode != ThemeMode::Relay;
        Self {
            mode,
            is_light,
            uses_blue_accent,
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
            ThemeMode::FrostedGlass => Self::frosted_glass(),
            ThemeMode::GraphiteGlass => Self::graphite_glass(),
            ThemeMode::Custom(_) => Self::relay(),
        }
    }

    /// Resolve a mode against stored user themes. A custom id with no stored
    /// theme falls back to Relay, matching `parse` for unknown built-ins.
    pub fn resolve(mode: &ThemeMode, customs: &[CustomTheme]) -> Self {
        match mode {
            ThemeMode::Custom(id) => customs
                .iter()
                .find(|custom| &custom.id == id)
                .map(|custom| custom.resolve())
                .unwrap_or_else(Self::relay),
            other => Self::for_mode(other.clone()),
        }
    }

    pub fn is_frosted(&self) -> bool {
        matches!(
            self.mode,
            ThemeMode::FrostedGlass | ThemeMode::GraphiteGlass
        )
    }

    pub fn chrome_background(&self) -> Background {
        self.workbench_chrome.into()
    }

    pub fn rail_background(&self) -> Background {
        self.workbench_rail.into()
    }

    pub fn explorer_background(&self) -> Background {
        self.workbench_explorer.into()
    }

    pub fn canvas_background(&self) -> Background {
        self.workbench_canvas.into()
    }

    pub fn section_background(&self) -> Background {
        self.workbench_header.into()
    }

    pub fn overlay_surface(&self) -> Background {
        match self.mode {
            ThemeMode::FrostedGlass => color("#000000").opacity(0.68).into(),
            ThemeMode::GraphiteGlass => color("#303030").opacity(0.90).into(),
            _ => self.surface.into(),
        }
    }

    /// Transient material uses one neutral macOS-style elevation shadow.
    pub fn material_shadow(&self) -> Vec<BoxShadow> {
        if !self.is_frosted() {
            return Vec::new();
        }
        vec![BoxShadow {
            color: color("#000000").opacity(0.42),
            offset: point(px(0.0), px(10.0)),
            blur_radius: px(28.0),
            spread_radius: px(-8.0),
        }]
    }

    /// Glass roots tint the native whole-window blur without adding a hue.
    pub fn application_background(&self) -> Background {
        match self.mode {
            // Preserve the original colorless system-backdrop treatment.
            ThemeMode::FrostedGlass => color("#000000").opacity(0.18).into(),
            // A substantial neutral tint makes the backdrop read as polished
            // graphite rather than a recognizable desktop image.
            ThemeMode::GraphiteGlass => color("#3A3A3A").opacity(0.76).into(),
            _ => self.workbench_chrome.into(),
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
// Menu shells and their rows, highlights, and badges share the workbench's
// near-square geometry rather than introducing a separate rounded surface.
pub const MENU_RADIUS: f32 = RADIUS_SM;

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
// The workspace strip is a persistent part of the workbench chrome. Keep it
// tall enough to read as a deliberate navigation row instead of a compressed
// status bar, while remaining denser than the main toolbar.
pub const WORKSPACE_TAB_HEIGHT: f32 = 38.0;
pub const CONTEXT_TOOLBAR_HEIGHT: f32 = 58.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frosted_glass_round_trips_and_uses_translucent_materials() {
        let mode = ThemeMode::parse("frosted_glass");
        assert_eq!(mode, ThemeMode::FrostedGlass);
        assert_eq!(mode.as_str(), "frosted_glass");

        let theme = Theme::for_mode(mode);
        assert!(theme.is_frosted());
        for material in [
            theme.workbench_chrome,
            theme.workbench_rail,
            theme.workbench_explorer,
            theme.workbench_canvas,
            theme.workbench_header,
            theme.ink,
            theme.surface,
            theme.surface_soft,
            theme.raised,
        ] {
            assert!(material.a < 1.0);
            assert_eq!(material.s, 0.0);
        }
        assert_eq!(theme.text.a, 1.0);
        assert_eq!(theme.workbench_chrome.a, 0.055);
        assert_eq!(theme.workbench_rail.a, 0.035);
        assert_eq!(theme.workbench_explorer.a, 0.050);
        assert_eq!(theme.workbench_canvas.a, 0.080);
        assert_eq!(theme.workbench_header.a, 0.055);
    }

    #[test]
    fn graphite_glass_round_trips_and_keeps_the_glass_structure() {
        let mode = ThemeMode::parse("graphite_glass");
        assert_eq!(mode, ThemeMode::GraphiteGlass);
        assert_eq!(mode.as_str(), "graphite_glass");

        let theme = Theme::for_mode(mode);
        let original = Theme::frosted_glass();
        assert!(theme.is_frosted());
        assert_eq!(theme.is_light, original.is_light);
        assert_eq!(theme.uses_blue_accent, original.uses_blue_accent);
        assert_eq!(theme.accent, original.accent);
        assert_eq!(theme.success, original.success);
        assert_eq!(theme.warning, original.warning);
        assert_eq!(theme.error, original.error);
        for material in [
            theme.workbench_chrome,
            theme.workbench_rail,
            theme.workbench_explorer,
            theme.workbench_canvas,
            theme.workbench_header,
            theme.ink,
            theme.surface,
            theme.surface_soft,
            theme.raised,
        ] {
            assert!(material.a < 1.0);
            assert_eq!(material.s, 0.0);
        }
        assert!(theme.workbench_chrome.a > original.workbench_chrome.a);
        assert!(theme.workbench_canvas.a > original.workbench_canvas.a);
    }

    #[test]
    fn unknown_theme_values_still_fall_back_to_relay() {
        assert_eq!(ThemeMode::parse("unknown"), ThemeMode::Relay);
    }

    #[test]
    fn custom_mode_round_trips_through_its_selection_key() {
        let mode = ThemeMode::parse("custom:custom-2");
        assert_eq!(mode, ThemeMode::Custom("custom-2".to_string()));
        assert_eq!(mode.as_str(), "custom-2");
        assert_eq!(mode.selection_key(), "custom:custom-2");
        assert_eq!(ThemeMode::parse(&mode.selection_key()), mode);
        assert_eq!(ThemeMode::Relay.selection_key(), "relay");
    }

    #[test]
    fn hex_helpers_round_trip_and_reject_garbage() {
        assert_eq!(hsla_to_hex(Theme::relay().accent), "#FF7152");
        let (r, g, b) = parse_hex_color("#ff7152").expect("valid hex");
        assert_eq!(hsla_to_hex(Hsla::from(Rgba { r, g, b, a: 1.0 })), "#FF7152");
        assert!(parse_hex_color("#F72").is_some());
        assert!(parse_hex_color("not a color").is_none());
        assert!(parse_hex_color("#12345").is_none());
        assert!(parse_hex_color("").is_none());
    }

    #[test]
    fn custom_theme_overrides_apply_but_keep_base_alpha() {
        let glass = Theme::frosted_glass();
        assert!(glass.surface.a < 1.0);
        let custom = CustomTheme {
            id: "custom-1".to_string(),
            name: "Test".to_string(),
            base: "frosted_glass".to_string(),
            colors: [("accent".to_string(), "#123456".to_string())]
                .into_iter()
                .collect(),
        };
        let resolved = custom.resolve();
        assert_eq!(hsla_to_hex(resolved.accent), "#123456");
        assert_eq!(resolved.accent.a, glass.accent.a);
        // Untouched roles match the base exactly.
        assert_eq!(resolved.surface, glass.surface);
        // Unknown roles and garbage hex never break resolution.
        let sloppy = CustomTheme {
            colors: [
                ("nope".to_string(), "#123456".to_string()),
                ("text".to_string(), "garbage".to_string()),
            ]
            .into_iter()
            .collect(),
            ..custom.clone()
        };
        assert_eq!(sloppy.resolve().text, glass.text);
    }

    #[test]
    fn resolve_falls_back_when_a_custom_theme_is_missing() {
        let missing = ThemeMode::parse("custom:gone");
        assert_eq!(Theme::resolve(&missing, &[]), Theme::relay());
        let stored = CustomTheme {
            id: "custom-1".to_string(),
            name: "Test".to_string(),
            base: "pitch_black".to_string(),
            colors: HashMap::new(),
        };
        assert_eq!(
            Theme::resolve(&ThemeMode::parse("custom:custom-1"), &[stored]),
            Theme::pitch_black()
        );
    }

    #[test]
    fn basic_roles_name_real_unique_roles() {
        assert!(!BASIC_THEME_ROLE_KEYS.is_empty());
        let mut seen = std::collections::HashSet::new();
        for key in BASIC_THEME_ROLE_KEYS {
            assert!(seen.insert(key), "duplicate basic role {key}");
            assert!(
                THEME_ROLES.iter().any(|role| role.key == key),
                "unknown basic role {key}"
            );
        }
        assert!(BASIC_THEME_ROLE_KEYS.len() < THEME_ROLES.len());
    }

    #[test]
    fn every_theme_role_reads_and_writes_a_distinct_field() {
        let mut seen = std::collections::HashSet::new();
        for role in THEME_ROLES {
            assert!(seen.insert(role.key), "duplicate role key {}", role.key);
            let mut theme = Theme::relay();
            let before = (role.get)(&theme);
            let flipped = Hsla {
                h: (before.h + 0.5) % 1.0,
                s: before.s,
                l: 1.0 - before.l,
                a: before.a,
            };
            (role.set)(&mut theme, flipped);
            assert_eq!((role.get)(&theme), flipped);
        }
        // The configurator covers the full palette surface.
        assert!(THEME_ROLES.len() >= 31);
    }

    #[test]
    fn tactile_controls_keep_directional_depth_and_compress_when_pressed() {
        for mode in [
            ThemeMode::Relay,
            ThemeMode::PitchBlack,
            ThemeMode::FullWhite,
            ThemeMode::FrostedGlass,
            ThemeMode::GraphiteGlass,
        ] {
            let theme = Theme::for_mode(mode);
            assert_ne!(
                theme.control_face(TactileState::Rest),
                theme.control_face(TactileState::Pressed)
            );
            assert_ne!(
                theme.accent_control_face(TactileState::Rest),
                theme.accent_control_face(TactileState::Pressed)
            );

            let rest = theme.tactile_shadow(TactileState::Rest, false);
            let hover = theme.tactile_shadow(TactileState::Hover, false);
            let pressed = theme.tactile_shadow(TactileState::Pressed, false);
            assert_eq!(rest.len(), 2);
            assert_eq!(hover.len(), 2);
            assert_eq!(pressed.len(), 2);
            assert_eq!(rest[0].offset, point(px(0.0), px(2.0)));
            assert_eq!(hover[0].offset, point(px(0.0), px(3.0)));
            assert_eq!(pressed[0].offset, point(px(0.0), px(1.0)));
            assert!(hover[0].blur_radius > rest[0].blur_radius);
            assert!(pressed[0].blur_radius < rest[0].blur_radius);
            assert_eq!(rest[1].offset, point(px(0.0), px(-1.0)));
            assert!(rest[1].blur_radius > px(0.0));
        }
    }
}
