//! Application shortcut catalog and application-level routing policy.
//!
//! The catalog is deliberately data-first: the application fallback and the
//! sidebar guide read the same definitions, keeping discoverability from
//! drifting away from behavior. Focused controls and transient surfaces get
//! first refusal; this module decides only whether an otherwise-unhandled key
//! is eligible for global dispatch.

/// Bind control activation and traversal once. Application commands are the
/// post-dispatch fallback, not competing bindings on every panel.
pub fn bind_control_keys(app: &mut gpui::App) {
    app.bind_keys([
        gpui::KeyBinding::new("tab", crate::FocusNext, Some("cliprelay")),
        gpui::KeyBinding::new("shift-tab", crate::FocusPrevious, Some("cliprelay")),
        gpui::KeyBinding::new("enter", crate::Activate, Some("cliprelay")),
        gpui::KeyBinding::new("space", crate::ActivateSpace, Some("cliprelay")),
    ]);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobalShortcut {
    PreviousVideo,
    NextVideo,
    TogglePlayback,
    PickRandomVideo,
    OpenStudio,
    MarkIn,
    MarkOut,
    FocusSearch,
    OpenCommandPalette,
    ToggleFullscreen,
    MinimizeWindow,
    Quit,
    OpenLibrary,
    OpenHistory,
    OpenSettings,
    NewWorkspace,
    CloseWorkspace,
    ReopenClosedWorkspace,
    CycleWorkspaceForward,
    CycleWorkspaceBackward,
    NavigateBack,
    NavigateForward,
}

impl GlobalShortcut {
    pub fn is_single_key(self) -> bool {
        matches!(
            self,
            Self::PreviousVideo
                | Self::NextVideo
                | Self::TogglePlayback
                | Self::PickRandomVideo
                | Self::OpenStudio
                | Self::MarkIn
                | Self::MarkOut
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShortcutModifiers {
    /// Command on macOS and Control on Linux/Windows.
    pub shortcut: bool,
    /// The physical Control modifier. This remains distinct on macOS.
    pub control: bool,
    pub shift: bool,
    pub alt: bool,
    pub platform: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShortcutContext {
    /// An editable field or platform IME session currently owns text input.
    pub text_entry_active: bool,
    pub composing: bool,
    /// A menu, dialog, popover, key capture, or other transient owns the key stream.
    pub transient_surface_active: bool,
    /// Explorer owns arrows and activation, but not unrelated letter commands.
    pub explorer_active: bool,
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
        action: Some(GlobalShortcut::MarkIn),
        group: "Workspace & Studio",
        label: "Set cut In at playhead",
        mac_keys: "I",
        other_keys: "I",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::MarkOut),
        group: "Workspace & Studio",
        label: "Set cut Out at playhead",
        mac_keys: "O",
        other_keys: "O",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::FocusSearch),
        group: "Search & general",
        label: "Search ClipRelay",
        mac_keys: "⌘ F / ⌘ K",
        other_keys: "Ctrl F / Ctrl K",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::OpenCommandPalette),
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
        action: Some(GlobalShortcut::ToggleFullscreen),
        group: "Search & general",
        label: "Toggle full screen",
        mac_keys: "⌃ ⌘ F / F11",
        other_keys: "F11",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::MinimizeWindow),
        group: "Search & general",
        label: "Minimize ClipRelay",
        mac_keys: "⌘ M",
        other_keys: "Ctrl M",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::Quit),
        group: "Search & general",
        label: "Quit ClipRelay",
        mac_keys: "⌘ ⇧ W",
        other_keys: "Ctrl Shift W",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::OpenLibrary),
        group: "Pages & workspaces",
        label: "Open Library",
        mac_keys: "⌘ 1",
        other_keys: "Ctrl 1",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::OpenHistory),
        group: "Pages & workspaces",
        label: "Open History",
        mac_keys: "⌘ 2",
        other_keys: "Ctrl 2",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::OpenSettings),
        group: "Pages & workspaces",
        label: "Open Settings",
        mac_keys: "⌘ ,",
        other_keys: "Ctrl ,",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::NewWorkspace),
        group: "Pages & workspaces",
        label: "New workspace",
        mac_keys: "⌘ T",
        other_keys: "Ctrl T",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::CloseWorkspace),
        group: "Pages & workspaces",
        label: "Close workspace",
        mac_keys: "⌘ W",
        other_keys: "Ctrl W",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::ReopenClosedWorkspace),
        group: "Pages & workspaces",
        label: "Reopen closed workspace",
        mac_keys: "⌘ ⇧ T",
        other_keys: "Ctrl Shift T",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::CycleWorkspaceForward),
        group: "Pages & workspaces",
        label: "Cycle workspaces",
        mac_keys: "⌃ Tab",
        other_keys: "Ctrl Tab",
    },
    ShortcutDefinition {
        action: Some(GlobalShortcut::NavigateBack),
        group: "Pages & workspaces",
        label: "Go back or forward",
        mac_keys: "⌘ [ / ⌘ ]",
        other_keys: "Ctrl [ / Ctrl ]",
    },
];

pub fn global_shortcut_for_key(
    key: &str,
    modifiers: ShortcutModifiers,
    context: ShortcutContext,
) -> Option<GlobalShortcut> {
    global_shortcut_for_key_on_platform(key, modifiers, context, cfg!(target_os = "macos"))
}

pub(crate) fn global_shortcut_for_key_on_platform(
    key: &str,
    modifiers: ShortcutModifiers,
    context: ShortcutContext,
    is_macos: bool,
) -> Option<GlobalShortcut> {
    if context.transient_surface_active || context.composing {
        return None;
    }

    let no_modifiers = !modifiers.shortcut
        && !modifiers.control
        && !modifiers.shift
        && !modifiers.alt
        && !modifiers.platform;
    let shortcut = if no_modifiers {
        match key {
            "left" => Some(GlobalShortcut::PreviousVideo),
            "right" => Some(GlobalShortcut::NextVideo),
            " " | "space" => Some(GlobalShortcut::TogglePlayback),
            "r" => Some(GlobalShortcut::PickRandomVideo),
            "s" => Some(GlobalShortcut::OpenStudio),
            "i" => Some(GlobalShortcut::MarkIn),
            "o" => Some(GlobalShortcut::MarkOut),
            "f11" => Some(GlobalShortcut::ToggleFullscreen),
            _ => None,
        }
    } else if modifiers.control && !modifiers.alt && !modifiers.platform && key == "tab" {
        Some(if modifiers.shift {
            GlobalShortcut::CycleWorkspaceBackward
        } else {
            GlobalShortcut::CycleWorkspaceForward
        })
    } else if is_macos
        && modifiers.shortcut
        && modifiers.control
        && !modifiers.shift
        && !modifiers.alt
        && key == "f"
    {
        Some(GlobalShortcut::ToggleFullscreen)
    } else if modifiers.shortcut && !modifiers.alt {
        match (key, modifiers.shift) {
            ("f" | "k", false) => Some(GlobalShortcut::FocusSearch),
            ("p", true) => Some(GlobalShortcut::OpenCommandPalette),
            ("1", false) => Some(GlobalShortcut::OpenLibrary),
            ("2", false) => Some(GlobalShortcut::OpenHistory),
            (",", false) => Some(GlobalShortcut::OpenSettings),
            ("m", false) => Some(GlobalShortcut::MinimizeWindow),
            ("w", true) => Some(GlobalShortcut::Quit),
            ("w", false) => Some(GlobalShortcut::CloseWorkspace),
            ("t", true) => Some(GlobalShortcut::ReopenClosedWorkspace),
            ("t", false) => Some(GlobalShortcut::NewWorkspace),
            ("[", false) => Some(GlobalShortcut::NavigateBack),
            ("]", false) => Some(GlobalShortcut::NavigateForward),
            ("tab", false) if modifiers.control => Some(GlobalShortcut::CycleWorkspaceForward),
            ("tab", true) if modifiers.control => Some(GlobalShortcut::CycleWorkspaceBackward),
            _ => None,
        }
    } else if !is_macos && modifiers.alt && !modifiers.shift && !modifiers.shortcut {
        match key {
            "left" => Some(GlobalShortcut::NavigateBack),
            "right" => Some(GlobalShortcut::NavigateForward),
            _ => None,
        }
    } else {
        None
    }?;

    if shortcut.is_single_key() && context.text_entry_active {
        return None;
    }
    if context.explorer_active
        && matches!(
            shortcut,
            GlobalShortcut::PreviousVideo
                | GlobalShortcut::NextVideo
                | GlobalShortcut::TogglePlayback
        )
    {
        return None;
    }
    Some(shortcut)
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
    use gpui::{
        div, AppContext as _, Context, FocusHandle, InteractiveElement, IntoElement, Keystroke,
        ParentElement, Render, Subscription, TestAppContext, Window,
    };

    struct RoutingProbe {
        ordinary_focus: FocusHandle,
        local_focus: FocusHandle,
        text_entry_active: bool,
        transient_surface_active: bool,
        routed: Vec<GlobalShortcut>,
        local_space_claimed: bool,
        _subscription: Subscription,
    }

    impl RoutingProbe {
        fn new(cx: &mut Context<Self>) -> Self {
            let subscription = cx.observe_keystrokes(|probe, event, _window, _cx| {
                if event.action.is_some() {
                    return;
                }
                if let Some(shortcut) = global_shortcut_for_key(
                    event.keystroke.key.as_str(),
                    ShortcutModifiers::default(),
                    ShortcutContext {
                        text_entry_active: probe.text_entry_active,
                        composing: false,
                        transient_surface_active: probe.transient_surface_active,
                        explorer_active: false,
                    },
                ) {
                    probe.routed.push(shortcut);
                }
            });
            Self {
                ordinary_focus: cx.focus_handle(),
                local_focus: cx.focus_handle(),
                text_entry_active: false,
                transient_surface_active: false,
                routed: Vec::new(),
                local_space_claimed: false,
                _subscription: subscription,
            }
        }
    }

    impl Render for RoutingProbe {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .key_context("cliprelay")
                // Mirrors the root's explicit pass-through: in this pinned
                // revision an empty action listener otherwise eats Space.
                .on_action(
                    cx.listener(|_probe, _: &crate::ActivateSpace, _window, cx| {
                        cx.propagate();
                    }),
                )
                .child(
                    div()
                        .id("ordinary-shortcut-focus")
                        .track_focus(&self.ordinary_focus),
                )
                .child(
                    div()
                        .id("local-shortcut-focus")
                        .key_context("shortcut-test-local")
                        .track_focus(&self.local_focus)
                        .on_action(cx.listener(|probe, _: &crate::ActivateSpace, _window, cx| {
                            probe.local_space_claimed = true;
                            cx.stop_propagation();
                        })),
                )
        }
    }

    #[test]
    fn every_global_binding_has_a_guide_entry() {
        for action in [
            GlobalShortcut::PreviousVideo,
            GlobalShortcut::NextVideo,
            GlobalShortcut::TogglePlayback,
            GlobalShortcut::PickRandomVideo,
            GlobalShortcut::OpenStudio,
            GlobalShortcut::MarkIn,
            GlobalShortcut::MarkOut,
            GlobalShortcut::FocusSearch,
            GlobalShortcut::OpenCommandPalette,
            GlobalShortcut::ToggleFullscreen,
            GlobalShortcut::MinimizeWindow,
            GlobalShortcut::Quit,
            GlobalShortcut::OpenLibrary,
            GlobalShortcut::OpenHistory,
            GlobalShortcut::OpenSettings,
            GlobalShortcut::NewWorkspace,
            GlobalShortcut::CloseWorkspace,
            GlobalShortcut::ReopenClosedWorkspace,
            GlobalShortcut::CycleWorkspaceForward,
            GlobalShortcut::NavigateBack,
        ] {
            assert!(SHORTCUTS
                .iter()
                .any(|definition| definition.action == Some(action)));
        }
    }

    #[test]
    fn media_bindings_reject_unrelated_modifiers() {
        let context = ShortcutContext::default();
        assert_eq!(
            global_shortcut_for_key("left", ShortcutModifiers::default(), context),
            Some(GlobalShortcut::PreviousVideo)
        );
        assert_eq!(
            global_shortcut_for_key("s", ShortcutModifiers::default(), context),
            Some(GlobalShortcut::OpenStudio)
        );
        assert_eq!(
            global_shortcut_for_key(
                "r",
                ShortcutModifiers {
                    shortcut: true,
                    control: !cfg!(target_os = "macos"),
                    platform: cfg!(target_os = "macos"),
                    ..Default::default()
                },
                context,
            ),
            None
        );
        assert_eq!(
            global_shortcut_for_key(
                "space",
                ShortcutModifiers {
                    alt: true,
                    ..Default::default()
                },
                context,
            ),
            None
        );
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
                global_shortcut_for_key(
                    key,
                    ShortcutModifiers::default(),
                    ShortcutContext::default(),
                ),
                Some(expected)
            );
            assert!(SHORTCUTS
                .iter()
                .any(|definition| definition.action == Some(expected)));
        }
    }

    #[test]
    fn routing_matrix_respects_text_transient_and_explorer_owners() {
        let modifiers = ShortcutModifiers::default();
        for key in ["left", "right", "space", "r", "s", "i", "o"] {
            assert_eq!(
                global_shortcut_for_key(
                    key,
                    modifiers,
                    ShortcutContext {
                        text_entry_active: true,
                        ..Default::default()
                    },
                ),
                None,
                "text entry must retain {key}"
            );
            assert_eq!(
                global_shortcut_for_key(
                    key,
                    modifiers,
                    ShortcutContext {
                        transient_surface_active: true,
                        ..Default::default()
                    },
                ),
                None,
                "transient surface must retain {key}"
            );
        }

        let explorer = ShortcutContext {
            explorer_active: true,
            ..Default::default()
        };
        assert_eq!(global_shortcut_for_key("left", modifiers, explorer), None);
        assert_eq!(global_shortcut_for_key("space", modifiers, explorer), None);
        assert_eq!(
            global_shortcut_for_key("r", modifiers, explorer),
            Some(GlobalShortcut::PickRandomVideo)
        );
        assert_eq!(
            global_shortcut_for_key("s", modifiers, explorer),
            Some(GlobalShortcut::OpenStudio)
        );
    }

    #[test]
    fn application_bindings_match_each_platform_without_stealing_control_f() {
        let mac_command = ShortcutModifiers {
            shortcut: true,
            platform: true,
            ..Default::default()
        };
        assert_eq!(
            global_shortcut_for_key_on_platform("f", mac_command, ShortcutContext::default(), true,),
            Some(GlobalShortcut::FocusSearch)
        );
        assert_eq!(
            global_shortcut_for_key_on_platform(
                "f",
                ShortcutModifiers {
                    control: true,
                    ..mac_command
                },
                ShortcutContext::default(),
                true,
            ),
            Some(GlobalShortcut::ToggleFullscreen)
        );

        let other_control = ShortcutModifiers {
            shortcut: true,
            control: true,
            ..Default::default()
        };
        assert_eq!(
            global_shortcut_for_key_on_platform(
                "f",
                other_control,
                ShortcutContext::default(),
                false,
            ),
            Some(GlobalShortcut::FocusSearch)
        );
        assert_eq!(
            global_shortcut_for_key_on_platform(
                "left",
                ShortcutModifiers {
                    alt: true,
                    ..Default::default()
                },
                ShortcutContext::default(),
                false,
            ),
            Some(GlobalShortcut::NavigateBack)
        );
    }

    #[gpui::test]
    fn application_fallback_needs_no_special_focus_and_yields_to_local_owners(
        cx: &mut TestAppContext,
    ) {
        let window = cx.update(|cx| {
            bind_control_keys(cx);
            cx.open_window(Default::default(), |_window, cx| cx.new(RoutingProbe::new))
                .unwrap()
        });

        window
            .update(cx, |_probe, window, _cx| window.blur())
            .unwrap();
        cx.dispatch_keystroke(*window, Keystroke::parse("r").unwrap());
        window
            .update(cx, |probe, _window, _cx| {
                assert_eq!(probe.routed, vec![GlobalShortcut::PickRandomVideo]);
            })
            .unwrap();

        window
            .update(cx, |probe, window, _cx| window.focus(&probe.local_focus))
            .unwrap();
        cx.dispatch_keystroke(*window, Keystroke::parse("space").unwrap());
        window
            .update(cx, |probe, _window, _cx| {
                assert!(probe.local_space_claimed);
                assert_eq!(probe.routed.len(), 1, "local Space must not double-fire");
                probe.text_entry_active = true;
            })
            .unwrap();
        cx.dispatch_keystroke(*window, Keystroke::parse("s").unwrap());
        window
            .update(cx, |probe, _window, _cx| {
                assert_eq!(probe.routed.len(), 1, "text entry must retain S");
                probe.text_entry_active = false;
                probe.transient_surface_active = true;
            })
            .unwrap();
        cx.dispatch_keystroke(*window, Keystroke::parse("left").unwrap());
        window
            .update(cx, |probe, _window, _cx| {
                assert_eq!(probe.routed.len(), 1, "transient must retain Left");
                probe.transient_surface_active = false;
            })
            .unwrap();
        cx.dispatch_keystroke(*window, Keystroke::parse("s").unwrap());
        window
            .update(cx, |probe, _window, _cx| {
                assert_eq!(
                    probe.routed,
                    vec![GlobalShortcut::PickRandomVideo, GlobalShortcut::OpenStudio],
                    "global routing must resume immediately after dismissal"
                );
            })
            .unwrap();

        window
            .update(cx, |probe, window, _cx| window.focus(&probe.ordinary_focus))
            .unwrap();
        cx.dispatch_keystroke(*window, Keystroke::parse("space").unwrap());
        window
            .update(cx, |probe, _window, _cx| {
                assert_eq!(probe.routed.last(), Some(&GlobalShortcut::TogglePlayback));
                assert_eq!(
                    probe.routed.len(),
                    3,
                    "root Space must fall through exactly once"
                );
            })
            .unwrap();
    }

    #[test]
    fn documented_application_catalog_routes_on_both_platforms() {
        for is_macos in [false, true] {
            for (label, key, shift, action) in [
                ("Search ClipRelay", "f", false, GlobalShortcut::FocusSearch),
                (
                    "Open command palette",
                    "p",
                    true,
                    GlobalShortcut::OpenCommandPalette,
                ),
                ("Open Library", "1", false, GlobalShortcut::OpenLibrary),
                ("Open History", "2", false, GlobalShortcut::OpenHistory),
                ("Open Settings", ",", false, GlobalShortcut::OpenSettings),
                (
                    "Minimize ClipRelay",
                    "m",
                    false,
                    GlobalShortcut::MinimizeWindow,
                ),
                ("Quit ClipRelay", "w", true, GlobalShortcut::Quit),
                ("New workspace", "t", false, GlobalShortcut::NewWorkspace),
                (
                    "Close workspace",
                    "w",
                    false,
                    GlobalShortcut::CloseWorkspace,
                ),
                (
                    "Reopen closed workspace",
                    "t",
                    true,
                    GlobalShortcut::ReopenClosedWorkspace,
                ),
                (
                    "Go back or forward",
                    "[",
                    false,
                    GlobalShortcut::NavigateBack,
                ),
            ] {
                let modifiers = ShortcutModifiers {
                    shortcut: true,
                    control: !is_macos,
                    platform: is_macos,
                    shift,
                    ..Default::default()
                };
                assert_eq!(
                    global_shortcut_for_key_on_platform(
                        key,
                        modifiers,
                        ShortcutContext::default(),
                        is_macos
                    ),
                    Some(action),
                    "{label} must route on macos={is_macos}"
                );
                assert_eq!(
                    SHORTCUTS
                        .iter()
                        .find(|entry| entry.label == label)
                        .unwrap()
                        .action,
                    Some(action)
                );
                for context in [
                    ShortcutContext {
                        composing: true,
                        text_entry_active: true,
                        ..Default::default()
                    },
                    ShortcutContext {
                        transient_surface_active: true,
                        ..Default::default()
                    },
                ] {
                    assert_eq!(
                        global_shortcut_for_key_on_platform(key, modifiers, context, is_macos),
                        None
                    );
                }
            }
            for (shift, action) in [
                (false, GlobalShortcut::CycleWorkspaceForward),
                (true, GlobalShortcut::CycleWorkspaceBackward),
            ] {
                assert_eq!(
                    global_shortcut_for_key_on_platform(
                        "tab",
                        ShortcutModifiers {
                            control: true,
                            shortcut: !is_macos,
                            shift,
                            ..Default::default()
                        },
                        ShortcutContext::default(),
                        is_macos
                    ),
                    Some(action)
                );
            }
        }
    }
}
