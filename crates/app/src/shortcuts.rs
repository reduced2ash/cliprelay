//! Application shortcut catalog and the small set of truly global actions.
//!
//! The catalog is deliberately data-first: the root key dispatcher and the
//! sidebar guide both read the same definitions, keeping discoverability from
//! drifting away from behavior.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobalShortcut {
    PreviousVideo,
    NextVideo,
    TogglePlayback,
    PickRandomVideo,
    OpenStudio,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShortcutDefinition {
    pub action: Option<GlobalShortcut>,
    pub group: &'static str,
    pub label: &'static str,
    pub mac_keys: &'static str,
    pub other_keys: &'static str,
}

pub const SHORTCUTS: &[ShortcutDefinition] = &[
    ShortcutDefinition {
        action: Some(GlobalShortcut::PreviousVideo),
        group: "Navigation & playback",
        label: "Previous video",
        mac_keys: "←",
        other_keys: "←",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::NextVideo),
        group: "Navigation & playback",
        label: "Next video",
        mac_keys: "→",
        other_keys: "→",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::TogglePlayback),
        group: "Navigation & playback",
        label: "Play or pause",
        mac_keys: "Space",
        other_keys: "Space",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::PickRandomVideo),
        group: "Navigation & playback",
        label: "Pick random video",
        mac_keys: "R",
        other_keys: "R",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::OpenStudio),
        group: "Workspace & Studio",
        label: "Open selected video in Studio",
        mac_keys: "S",
        other_keys: "S",
    },
    ShortcutDefinition {
        action: None,
        group: "Workspace & Studio",
        label: "Set cut In at playhead",
        mac_keys: "I",
        other_keys: "I",
    },
    ShortcutDefinition {
        action: None,
        group: "Workspace & Studio",
        label: "Set cut Out at playhead",
        mac_keys: "O",
        other_keys: "O",
    },
    ShortcutDefinition {
        action: None,
        group: "Search & general",
        label: "Search ClipRelay",
        mac_keys: "⌘ F / ⌘ K",
        other_keys: "Ctrl F / Ctrl K",
    },
    ShortcutDefinition {
        action: None,
        group: "Search & general",
        label: "Open command palette",
        mac_keys: "⌘ ⇧ P",
        other_keys: "Ctrl Shift P",
    },
    ShortcutDefinition {
        action: None,
        group: "Search & general",
        label: "Move keyboard focus",
        mac_keys: "Tab / ⇧ Tab",
        other_keys: "Tab / Shift Tab",
    },
    ShortcutDefinition {
        action: None,
        group: "Search & general",
        label: "Dismiss the active menu or surface",
        mac_keys: "Esc",
        other_keys: "Esc",
    },
    ShortcutDefinition {
        action: None,
        group: "Search & general",
        label: "Toggle full screen",
        mac_keys: "⌃ ⌘ F / F11",
        other_keys: "F11",
    },
    ShortcutDefinition {
        action: None,
        group: "Search & general",
        label: "Minimize ClipRelay",
        mac_keys: "⌘ M",
        other_keys: "Ctrl M",
    },
    ShortcutDefinition {
        action: None,
        group: "Search & general",
        label: "Quit ClipRelay",
        mac_keys: "⌘ ⇧ W",
        other_keys: "Ctrl Shift W",
    },
    ShortcutDefinition {
        action: None,
        group: "Pages & workspaces",
        label: "Open Library",
        mac_keys: "⌘ 1",
        other_keys: "Ctrl 1",
    },
    ShortcutDefinition {
        action: None,
        group: "Pages & workspaces",
        label: "Open History",
        mac_keys: "⌘ 2",
        other_keys: "Ctrl 2",
    },
    ShortcutDefinition {
        action: None,
        group: "Pages & workspaces",
        label: "Open Settings",
        mac_keys: "⌘ ,",
        other_keys: "Ctrl ,",
    },
    ShortcutDefinition {
        action: None,
        group: "Pages & workspaces",
        label: "New workspace",
        mac_keys: "⌘ T",
        other_keys: "Ctrl T",
    },
    ShortcutDefinition {
        action: None,
        group: "Pages & workspaces",
        label: "Close workspace",
        mac_keys: "⌘ W",
        other_keys: "Ctrl W",
    },
    ShortcutDefinition {
        action: None,
        group: "Pages & workspaces",
        label: "Reopen closed workspace",
        mac_keys: "⌘ ⇧ T",
        other_keys: "Ctrl Shift T",
    },
    ShortcutDefinition {
        action: None,
        group: "Pages & workspaces",
        label: "Cycle workspaces",
        mac_keys: "⌃ Tab",
        other_keys: "Ctrl Tab",
    },
    ShortcutDefinition {
        action: None,
        group: "Pages & workspaces",
        label: "Go back or forward",
        mac_keys: "⌘ [ / ⌘ ]",
        other_keys: "Ctrl [ / Ctrl ]",
    },
];

pub fn global_shortcut_for_key(
    key: &str,
    has_command: bool,
    has_shift: bool,
    has_alt: bool,
) -> Option<GlobalShortcut> {
    if has_command || has_shift || has_alt {
        return None;
    }
    match key {
        "left" => Some(GlobalShortcut::PreviousVideo),
        "right" => Some(GlobalShortcut::NextVideo),
        " " | "space" => Some(GlobalShortcut::TogglePlayback),
        "r" => Some(GlobalShortcut::PickRandomVideo),
        "s" => Some(GlobalShortcut::OpenStudio),
        _ => None,
    }
}

pub fn shortcut_keys(definition: ShortcutDefinition) -> &'static str {
    if cfg!(target_os = "macos") {
        definition.mac_keys
    } else {
        definition.other_keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_global_binding_has_a_guide_entry() {
        for action in [
            GlobalShortcut::PreviousVideo,
            GlobalShortcut::NextVideo,
            GlobalShortcut::TogglePlayback,
            GlobalShortcut::PickRandomVideo,
            GlobalShortcut::OpenStudio,
        ] {
            assert!(SHORTCUTS
                .iter()
                .any(|definition| definition.action == Some(action)));
        }
    }

    #[test]
    fn global_bindings_reject_modified_keys() {
        assert_eq!(
            global_shortcut_for_key("left", false, false, false),
            Some(GlobalShortcut::PreviousVideo)
        );
        assert_eq!(
            global_shortcut_for_key("s", false, false, false),
            Some(GlobalShortcut::OpenStudio)
        );
        assert_eq!(global_shortcut_for_key("r", true, false, false), None);
        assert_eq!(global_shortcut_for_key("space", false, false, true), None);
    }

    #[test]
    fn every_global_action_has_a_stable_key_route() {
        let routes = [
            ("left", GlobalShortcut::PreviousVideo),
            ("right", GlobalShortcut::NextVideo),
            ("space", GlobalShortcut::TogglePlayback),
            ("r", GlobalShortcut::PickRandomVideo),
            ("s", GlobalShortcut::OpenStudio),
        ];
        for (key, expected) in routes {
            assert_eq!(
                global_shortcut_for_key(key, false, false, false),
                Some(expected)
            );
            assert!(SHORTCUTS
                .iter()
                .any(|definition| definition.action == Some(expected)));
        }
    }
}
