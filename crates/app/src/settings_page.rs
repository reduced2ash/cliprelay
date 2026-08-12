//! Settings page: interface, performance, files, Telegram, X handoff,
//! diagnostics — with the exact labels from the QML spec.

use crate::settings_import::*;
use crate::state::*;
use crate::theme::*;
use crate::widgets::*;
use gpui::*;
use serde_json::json;

#[derive(Clone, Debug, Default)]
pub struct SettingsUiState {
    pub bot_token: String,
    pub bot_destination: String,
    pub personal_api_id: String,
    pub personal_api_hash: String,
    pub personal_phone: String,
    pub login_code: String,
    pub login_password: String,
    pub diagnostics_requested: bool,
}

impl crate::App {
    pub fn render_settings(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        // Request diagnostics when the page first opens.
        if self.diagnostics.ffmpeg.is_empty() {
            self.command(Command::Diagnostics);
        }
        let mut page = div()
            .id("settings")
            .flex_1()
            .flex()
            .flex_col()
            .bg(theme.ink);

        page = page.child(
            div()
                .w_full()
                .px(px(26.0))
                .py(px(24.0))
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .child("Settings")
                        .text_size(px(20.0))
                        .text_color(theme.text)
                        .font_weight(FontWeight::BOLD),
                )
                .child(
                    div()
                        .child("Appearance, folders, connections, and safe file behavior")
                        .text_size(px(12.0))
                        .text_color(theme.muted),
                ),
        );

        // Content width matches the original: min(820, page − 48).
        let sidebar = if self.sidebar_collapsed {
            SIDEBAR_COLLAPSED_WIDTH
        } else {
            SIDEBAR_EXPANDED_WIDTH
        };
        let content_width = (self.window_size.0 - sidebar - 48.0).clamp(300.0, 820.0);

        let mut content = div()
            .id("settings-scroll")
            .flex_1()
            .overflow_scroll()
            .scrollbar_width(px(10.0))
            .child(
                div()
                    .w(px(content_width))
                    .mx(px(24.0))
                    .pb(px(40.0))
                    .flex()
                    .flex_col(),
            );

        // The inner column is built piece by piece.
        let mut inner = div()
            .w(px(content_width))
            .mx(px(24.0))
            .pb(px(40.0))
            .flex()
            .flex_col();

        // INTERFACE
        inner = inner
            .child(section_label(&theme, "INTERFACE"))
            .child(group_title(&theme, "Appearance and density"))
            .child(
                div()
                    .w_full()
                    .child("Color theme")
                    .text_size(px(13.0))
                    .text_color(theme.text)
                    .font_weight(FontWeight::MEDIUM),
            )
            .child(theme_choices(self, cx, &theme))
            .child(help_text(&theme, "Choose a palette for the whole app, then adjust how much of your workspace fits on screen."))
            .child(help_text(&theme, "Theme changes apply immediately and are saved for the next launch."))
            .child(setting_row(
                cx,
                &theme,
                "Interface scale",
                &["Compact · 80%", "Balanced · 90%", "Standard · 100%"],
                self.scale_index(),
                "scale-select",
                self.open_combos.contains("scale-select"),
                move |app, cx, index| {
                    let value = match index {
                        0 => 0.8,
                        1 => 0.9,
                        _ => 1.0,
                    };
                    app.set_setting(UI_SCALE, json!(value), cx);
                },
            ))
            .child(help_text(&theme, "Standard preserves the current size. Compact provides the widest working view."))
            .child(setting_row(
                cx,
                &theme,
                "Library density",
                &["Default", "Compact"],
                if self.density == "compact" { 1 } else { 0 },
                "density-select",
                self.open_combos.contains("density-select"),
                move |app, cx, index| {
                    app.set_setting(LIBRARY_DENSITY, json!(if index == 1 { "compact" } else { "default" }), cx);
                },
            ))
            .child(help_text(&theme, "Compact fits more videos without changing the rest of the interface."));

        // PERFORMANCE
        inner = inner
            .child(section_label(&theme, "PERFORMANCE"))
            .child(group_title(&theme, "Rendering and media"))
            .child(help_text(&theme, "VSync stays enabled. Maximum mode keeps graphics resources resident, preloads adjacent media, raises safe thumbnail concurrency, and prefers hardware export."))
            .child(setting_row(
                cx,
                &theme,
                "Performance mode",
                &["Automatic", "Maximum performance"],
                if self.settings_value(PERFORMANCE_MODE) == "maximum" { 1 } else { 0 },
                "performance-select",
                self.open_combos.contains("performance-select"),
                move |app, cx, index| {
                    app.set_setting(PERFORMANCE_MODE, json!(if index == 1 { "maximum" } else { "automatic" }), cx);
                },
            ))
            .child(help_text(&theme, "Maximum takes full effect after restarting ClipRelay."))
            .child(setting_row(
                cx,
                &theme,
                "Export encoder",
                &["Automatic", "Prefer hardware", "Software only"],
                match self.settings_value(EXPORT_ENCODER).as_str() {
                    "hardware" => 1,
                    "software" => 2,
                    _ => 0,
                },
                "encoder-select",
                self.open_combos.contains("encoder-select"),
                move |app, cx, index| {
                    let value = match index {
                        1 => "hardware",
                        2 => "software",
                        _ => "auto",
                    };
                    app.set_setting(EXPORT_ENCODER, json!(value), cx);
                },
            ))
            .child(help_text(&theme, "Hardware always falls back to software if the device or upload limit requires it."))
            .child(
                div()
                    .w_full()
                    .mt(px(16.0))
                    .mb(px(6.0))
                    .child("LIVE DIAGNOSTICS")
                    .text_size(px(11.0))
                    .text_color(theme.muted)
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(diagnostic_row(&theme, "Renderer", "GPUI"))
                    .child(diagnostic_row(&theme, "GPU", "—"))
                    .child(diagnostic_row(&theme, "Display", "—"))
                    .child(diagnostic_row(&theme, "Video decoder", "FFmpeg"))
                    .child(diagnostic_row(
                        &theme,
                        "Export",
                        &if cliprelay_core::media::hardware_encoder_info().0 {
                            cliprelay_core::media::hardware_encoder_info().1
                        } else {
                            "libx264".to_string()
                        },
                    ))
                    .child(diagnostic_row(&theme, "Frame pacing", "—"))
                    .child(diagnostic_row(&theme, "Frame spikes", "—"))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex_1()
                                    .child("Resources")
                                    .text_size(px(13.0))
                                    .text_color(theme.muted),
                            )
                            .child(
                                div()
                                    .child("Rendered on demand")
                                    .text_size(px(12.0))
                                    .text_color(theme.text)
                                    .text_right(),
                            )
                            .child(button(
                                "diag-refresh",
                                "Refresh",
                                ButtonKind::Ghost,
                                Some("↻"),
                                true,
                                cx,
                                move |_app, cx| {
                                    // Re-sample the renderer/encoder lines.
                                    cx.notify();
                                },
                            )),
                    ),
            );

        // FILES
        inner = inner
            .child(section_label(&theme, "FILES"))
            .child(group_title(&theme, "Library and generated media"))
            .child(help_text(&theme, "Your original videos are never moved or modified."))
            .child(sub_label(&theme, "Video library"))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
                    .child(static_field("static-library-root", &theme, "No folder chosen", self.settings_value(LIBRARY_ROOT)))
                    .child(button(
                        "choose-library",
                        "Choose",
                        ButtonKind::Secondary,
                        Some("▸"),
                        true,
                        cx,
                        |app, cx| {
                            app.choose_library_folder(cx);
                        },
                    )),
            )
            .child(help_text(&theme, "ClipRelay searches this folder and every folder inside it."))
            .child(sub_label(&theme, "Generated video folder"))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
                    .child(static_field("static-export-dir", &theme, "No folder chosen", self.settings_value(EXPORT_DIR)))
                    .child(button(
                        "choose-export",
                        "Choose",
                        ButtonKind::Secondary,
                        Some("▸"),
                        true,
                        cx,
                        |app, cx| {
                            app.choose_export_folder(cx);
                        },
                    ))
                    .child(button(
                        "reveal-export",
                        "Reveal",
                        ButtonKind::Ghost,
                        Some("↗"),
                        true,
                        cx,
                        |app, cx| {
                            let export_dir = app.settings_value(EXPORT_DIR);
                            if !export_dir.is_empty() {
                                let _ = cliprelay_core::x::XAssistant::reveal(std::path::Path::new(&export_dir));
                            }
                            cx.notify();
                        },
                    )),
            )
            .child(sub_label(&theme, "FAST PICKING"))
            .child(checkbox(
                "fast-random",
                "Make Random available before indexing finishes",
                self.settings_bool(FAST_RANDOM),
                true,
                cx,
                |app, cx, value| {
                    app.set_setting(FAST_RANDOM, json!(value), cx);
                },
            ))
            .child(help_text(&theme, "ClipRelay keeps a lightweight filename list and checks only the clip Random chooses. This stays fast without allowing unreadable files into preparation."))
            .child(checkbox(
                "avoid-repeats",
                "Avoid repeats until every video has been picked",
                self.settings_bool(AVOID_REPEATS),
                true,
                cx,
                |app, cx, value| {
                    app.set_setting(AVOID_REPEATS, json!(value), cx);
                },
            ))
            .child(sub_label(&theme, "BACKGROUND LIBRARY INDEX"))
            .child(checkbox(
                "auto-index",
                "Start indexing automatically after choosing a folder",
                self.settings_bool(AUTO_INDEX),
                true,
                cx,
                |app, cx, value| {
                    app.set_setting(AUTO_INDEX, json!(value), cx);
                },
            ))
            .child(help_text(&theme, "Off by default for large libraries. Rescan always starts it manually; reopening the app only refreshes the lightweight filename list."))
            .child(checkbox(
                "verify-index",
                "Verify every file and read duration, resolution, and codec details",
                self.settings_bool(VERIFY_DURING_INDEX),
                true,
                cx,
                |app, cx, value| {
                    app.set_setting(VERIFY_DURING_INDEX, json!(value), cx);
                },
            ))
            .child(checkbox(
                "deep-scan",
                "Inspect files with uncommon or missing video extensions (slowest)",
                self.settings_bool(DEEP_SCAN),
                self.settings_bool(VERIFY_DURING_INDEX),
                cx,
                |app, cx, value| {
                    app.set_setting(DEEP_SCAN, json!(value), cx);
                },
            ))
            .child(checkbox(
                "thumbs-index",
                "Generate all missing thumbnails while indexing",
                self.settings_bool(THUMBNAILS_DURING_INDEX),
                true,
                cx,
                |app, cx, value| {
                    app.set_setting(THUMBNAILS_DURING_INDEX, json!(value), cx);
                },
            ))
            .child(checkbox(
                "hover-previews",
                "Generate and play muted previews when hovering",
                self.settings_bool(HOVER_PREVIEWS),
                true,
                cx,
                |app, cx, value| {
                    app.set_setting(HOVER_PREVIEWS, json!(value), cx);
                },
            ))
            .child(help_text(
                &theme,
                if self.settings_bool(VERIFY_DURING_INDEX) {
                    "Turn off any optional step above to reduce background work. Random remains independent of this index."
                } else {
                    "With verification off, the library appears from filenames and sizes. A video is checked only when you select or publish it."
                },
            ));

        // TELEGRAM
        inner = inner
            .child(section_label(&theme, "TELEGRAM"))
            .child(group_title(&theme, "Bot connection"))
            .child(help_text(&theme, "Best for a channel: simple setup, reliable sending, and no personal session stored. Add the bot as an administrator in the channel."))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
                    .child(field("tg-bot-token", "Bot token from @BotFather",
                            self.fields.get("tg-bot-token").unwrap_or(&FieldState::default()),
                            self.focused_field.as_deref() == Some("tg-bot-token"), true, true, cx))
                    .child(button(
                        "tg-connect",
                        if self.telegram.bot.starts_with('@') { "Replace bot" } else { "Connect bot" },
                        ButtonKind::Primary,
                        Some("➤"),
                        true,
                        cx,
                        |app, cx| {
                            let token = app.field_text("tg-bot-token");
                            app.command(Command::ValidateBotToken(token));
                            cx.notify();
                        },
                    )),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
                    .child(field("tg-destination", "@channelname or numeric chat ID",
                            self.fields.get("tg-destination").unwrap_or(&FieldState::default()),
                            self.focused_field.as_deref() == Some("tg-destination"), true, false, cx))
                    .child(button(
                        "tg-check-dest",
                        "Check destination",
                        ButtonKind::Secondary,
                        Some("✓"),
                        true,
                        cx,
                        |app, cx| {
                            let destination = app.field_text("tg-destination");
                            app.command(Command::ValidateBotDestination(destination));
                            cx.notify();
                        },
                    )),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(12.0))
                    .child(status_pill(
                        "tg-status-pill",
                        if self.bot_connected() { "configured" } else { "not configured" },
                        if self.bot_connected() { PillState::Success } else { PillState::Warning },
                    ))
                    .child(
                        div()
                            .flex_1()
                            .child(self.telegram.message.clone())
                            .text_size(px(12.0))
                            .text_color(theme.muted)
                            .text_ellipsis(),
                    )
                    .child(if self.bot_connected() {
                        button(
                            "tg-disconnect",
                            "Disconnect",
                            ButtonKind::Danger,
                            Some("✕"),
                            true,
                            cx,
                            |app, cx| {
                                app.command(Command::DisconnectBot);
                                cx.notify();
                            },
                        )
                        .into_any()
                    } else {
                        div().into_any()
                    }),
            )
            .child(group_title(&theme, "Personal account"))
            .child(help_text(&theme, "Use this when the sender must be your own account. Telegram requires an API ID and hash from my.telegram.org; the resulting session is stored in your OS keychain."))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .gap(px(10.0))
                    .child(field("tg-api-id", "API ID",
                        self.fields.get("tg-api-id").unwrap_or(&FieldState::default()),
                        self.focused_field.as_deref() == Some("tg-api-id"), true, false, cx).flex_1())
                    .child(field("tg-api-hash", "API hash",
                        self.fields.get("tg-api-hash").unwrap_or(&FieldState::default()),
                        self.focused_field.as_deref() == Some("tg-api-hash"), true, true, cx).flex_1())
                    .child(field("tg-phone", "+1 555 123 4567",
                        self.fields.get("tg-phone").unwrap_or(&FieldState::default()),
                        self.focused_field.as_deref() == Some("tg-phone"), true, false, cx).flex_1()),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .gap(px(10.0))
                    .child(button(
                        "tg-send-code",
                        "Send login code",
                        ButtonKind::Secondary,
                        Some("➤"),
                        !self.personal_configured(),
                        cx,
                        |app, cx| {
                            let api_id: i32 = app.field_text("tg-api-id").parse().unwrap_or(0);
                            let api_hash = app.field_text("tg-api-hash");
                            let phone = app.field_text("tg-phone");
                            app.command(Command::BeginPersonalLogin(api_id, api_hash, phone));
                            cx.notify();
                        },
                    ))
                    .child(field("tg-code", "Login code",
                        self.fields.get("tg-code").unwrap_or(&FieldState::default()),
                        self.focused_field.as_deref() == Some("tg-code"), true, false, cx).flex_1())
                    .child(field("tg-password", "2-step password, if requested",
                        self.fields.get("tg-password").unwrap_or(&FieldState::default()),
                        self.focused_field.as_deref() == Some("tg-password"), true, true, cx).flex_1()),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
                    .child(button(
                        "tg-finish",
                        "Finish sign-in",
                        ButtonKind::Primary,
                        Some("✓"),
                        !self.personal_configured(),
                        cx,
                        |app, cx| {
                            let code = app.field_text("tg-code");
                            let password = app.field_text("tg-password");
                            app.command(Command::CompletePersonalLogin(code, password));
                            cx.notify();
                        },
                    ))
                    .child(button(
                        "tg-load-chats",
                        "Load chats",
                        ButtonKind::Ghost,
                        Some("↻"),
                        self.personal_configured(),
                        cx,
                        |app, cx| {
                            app.command(Command::LoadTelegramDialogs);
                            cx.notify();
                        },
                    ))
                    .child(button(
                        "tg-sign-out",
                        "Sign out",
                        ButtonKind::Danger,
                        Some("✕"),
                        self.personal_configured(),
                        cx,
                        |app, cx| {
                            app.command(Command::SignOutPersonal);
                            cx.notify();
                        },
                    ))
                    .child(status_pill(
                        "tg-personal-pill",
                        if self.personal_configured() { "signed in" } else { "not signed in" },
                        if self.personal_configured() { PillState::Success } else { PillState::Warning },
                    )),
            );

        // Chat picker.
        if !self.dialogs.is_empty() {
            inner = inner.child(chat_picker(self, cx, &theme));
        }

        // X HANDOFF
        inner = inner
            .child(section_label(&theme, "X HANDOFF"))
            .child(group_title(&theme, "Manual browser posting"))
            .child(help_text(&theme, "ClipRelay opens X’s official composer with your text prefilled, then places the prepared video on the clipboard and keeps drag-to-upload available. You review and press Post yourself. No paid X API is required."))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .child("Default file limit")
                            .text_size(px(13.0))
                            .text_color(theme.text_soft)
                            .font_weight(FontWeight::MEDIUM),
                    )
                    .child(
                        div()
                            .w(px(110.0))
                            .h(px(CONTROL_HEIGHT))
                            .child(field("x-limit", "512",
                                    self.fields.get("x-limit").unwrap_or(&FieldState::default()),
                                    self.focused_field.as_deref() == Some("x-limit"), true, false, cx)),
                    )
                    .child(
                        div()
                            .child("MB")
                            .text_size(px(13.0))
                            .text_color(theme.muted),
                    ),
            );

        // DIAGNOSTICS
        inner = inner
            .child(section_label(&theme, "LIVE DIAGNOSTICS"))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .child("Local tools")
                            .text_size(px(16.0))
                            .text_color(theme.text)
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(div().flex_1())
                    .child(button(
                        "diagnostics-refresh",
                        "Refresh",
                        ButtonKind::Ghost,
                        Some("↻"),
                        true,
                        cx,
                        |app, cx| {
                            app.command(Command::Diagnostics);
                            cx.notify();
                        },
                    )),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .gap(px(16.0))
                    .child(diagnostic_cell(&theme, "FFmpeg", &self.diagnostics.ffmpeg))
                    .child(diagnostic_cell(&theme, "FFprobe", &self.diagnostics.ffprobe))
                    .child(diagnostic_cell(&theme, "Database", &self.diagnostics.database))
                    .child(diagnostic_cell(&theme, "Secrets", &self.diagnostics.secret_backend)),
            );

        content = content.child(inner);
        let _ = &mut page;
        page = page.child(content);
        page
    }

    pub fn bot_connected(&self) -> bool {
        self.telegram.bot.starts_with('@') || self.telegram.bot == "configured"
    }

    pub fn personal_configured(&self) -> bool {
        // After a live sign-in the state holds the account display name
        // (e.g. "Roman Fertig"); only the explicit empty state is
        // unconfigured.
        !self.telegram.personal.is_empty() && self.telegram.personal != "not signed in"
    }

    pub fn scale_index(&self) -> usize {
        if self.ui_scale < 0.85 {
            0
        } else if self.ui_scale < 0.95 {
            1
        } else {
            2
        }
    }
}

fn section_label(theme: &crate::theme::Theme, label: &str) -> Div {
    div()
        .mt(px(SPACING_XXL))
        .mb(px(SPACING_SM))
        .child(label.to_string())
        .text_size(px(12.0))
        .text_color(theme.accent_text)
        .font_weight(FontWeight::BOLD)
}

fn group_title(theme: &crate::theme::Theme, title: &str) -> Div {
    div()
        .child(title.to_string())
        .text_size(px(16.0))
        .text_color(theme.text)
        .font_weight(FontWeight::BOLD)
}

fn help_text(theme: &crate::theme::Theme, text: &str) -> Div {
    div()
        .mt(px(4.0))
        .mb(px(8.0))
        .child(text.to_string())
        .text_size(px(13.0))
        .text_color(theme.muted)
        .max_w(px(760.0))
}

fn sub_label(theme: &crate::theme::Theme, label: &str) -> Div {
    div()
        .mt(px(SPACING_LG))
        .mb(px(4.0))
        .child(label.to_string())
        .text_size(px(12.0))
        .text_color(theme.muted_soft)
        .font_weight(FontWeight::BOLD)
}

fn static_field(id: &'static str, theme: &crate::theme::Theme, placeholder: &str, value: String) -> Stateful<Div> {
    let empty = value.is_empty();
    div()
        .id(id)
        .flex_1()
        .h(px(CONTROL_HEIGHT))
        .px(px(13.0))
        .rounded(px(RADIUS_SM))
        .bg(theme.raised)
        .border_1()
        .border_color(theme.border)
        .flex()
        .items_center()
        .child(if empty { placeholder.to_string() } else { value })
        .text_size(px(13.0))
        .text_color(if empty { theme.muted } else { theme.text })
        .text_ellipsis()
}

fn theme_choices(
    app: &mut crate::App,
    cx: &mut Context<crate::App>,
    theme: &crate::theme::Theme,
) -> impl Element {
    let choices = [
        ("relay", "Relay", "Warm dark"),
        ("pitch_black", "Pitch black", "Blue accent"),
        ("full_white", "Full white", "Blue accent"),
    ];
    let mut row = div().w_full().flex().flex_row().gap(px(12.0)).mt(px(12.0));
    for (mode, title, subtitle) in choices {
        let selected = app.theme_mode.as_str() == mode;
        let palette = crate::theme::Theme::for_mode(crate::theme::ThemeMode::parse(mode));
        let mode = mode.to_string();
        let mut card = div()
            .id(SharedString::from(format!("theme-{mode}")))
            .w(px(230.0))
            .h(px(92.0))
            .rounded(px(10.0))
            .border_1()
            .border_color(if selected { theme.accent } else { theme.border })
            .bg(if selected { theme.accent_soft } else { theme.surface })
            .cursor_pointer()
            .p(px(10.0))
            .flex()
            .flex_row()
            .gap(px(10.0))
            .on_click(cx.listener(move |app, _event, _window, cx| {
                app.set_setting(THEME_MODE, json!(mode), cx);
            }));
        // Mini preview.
        card = card.child(
            div()
                .w(px(62.0))
                .h(px(58.0))
                .rounded(px(8.0))
                .bg(palette.surface)
                .border_1()
                .border_color(palette.border)
                .flex()
                .flex_col()
                .gap(px(4.0))
                .p(px(6.0))
                .child(div().w_full().h(px(8.0)).rounded(px(2.0)).bg(palette.raised))
                .child(div().w(px(30.0)).h(px(6.0)).rounded(px(2.0)).bg(palette.accent))
                .child(div().w_full().h(px(4.0)).rounded(px(2.0)).bg(palette.border)),
        );
        card = card.child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .child(title)
                        .text_size(px(13.0))
                        .text_color(theme.text)
                        .font_weight(FontWeight::BOLD),
                )
                .child(
                    div()
                        .child(subtitle)
                        .text_size(px(12.0))
                        .text_color(theme.muted),
                ),
        );
        if selected {
            card = card.child(
                div()
                    .flex_none()
                    .w(px(20.0))
                    .h(px(20.0))
                    .rounded(px(6.0))
                    .bg(theme.accent)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon("✓", 12.0, theme.accent_content)),
            );
        }
        row = row.child(card);
    }
    row
}

fn setting_row(
    cx: &mut Context<crate::App>,
    theme: &crate::theme::Theme,
    label: &'static str,
    options: &'static [&'static str],
    selected: usize,
    id: &'static str,
    open: bool,
    on_change: impl Fn(&mut crate::App, &mut Context<crate::App>, usize) + 'static,
) -> impl Element {
    div()
        .w_full()
        .h(px(CONTROL_HEIGHT + 8.0))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .child(
            div()
                .flex_1()
                .child(label)
                .text_size(px(13.0))
                .text_color(theme.text_soft)
                .font_weight(FontWeight::MEDIUM),
        )
        .child(combo(cx, id, options, selected, 220.0, open, on_change))
}

fn combo(
    cx: &mut Context<crate::App>,
    id: &'static str,
    options: &'static [&'static str],
    selected: usize,
    width: f32,
    open: bool,
    on_change: impl Fn(&mut crate::App, &mut Context<crate::App>, usize) + 'static,
) -> impl Element {
    let theme = current_theme();
    let on_change = std::sync::Arc::new(on_change);
    let selected_label = options.get(selected).copied().unwrap_or("");
    let trigger = div()
        .id(SharedString::from(format!("{id}-trigger")))
        .w(px(width))
        .h(px(CONTROL_HEIGHT))
        .px(px(12.0))
        .rounded(px(RADIUS_SM))
        .bg(theme.raised)
        .border_1()
        .border_color(if open { theme.accent } else { theme.border })
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_between()
        .on_click(cx.listener(move |app, _event, _window, cx| {
            app.toggle_combo(id, cx);
        }))
        .child(
            div()
                .child(selected_label)
                .text_size(px(13.0))
                .text_color(theme.text),
        )
        .child(icon(if open { "▴" } else { "▾" }, 12.0, theme.muted));
    if !open {
        return trigger;
    }
    let mut menu = div()
        .id(id)
        .w(px(width))
        .rounded(px(10.0))
        .bg(theme.surface_soft)
        .border_1()
        .border_color(theme.border_strong)
        .py(px(4.0))
        .flex()
        .flex_col();
    for (index, option) in options.iter().enumerate() {
        let option = *option;
        let on_change = std::sync::Arc::clone(&on_change);
        menu = menu.child(
            div()
                .id(SharedString::from(format!("{id}-item-{index}")))
                .h(px(40.0))
                .px(px(10.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .cursor_pointer()
                .bg(if index == selected { theme.active } else { theme.transparent() })
                .child(
                    div()
                        .w(px(13.0))
                        .child(if index == selected { "✓" } else { "" })
                        .text_size(px(13.0))
                        .text_color(theme.accent_text),
                )
                .child(
                    div()
                        .child(option)
                        .text_size(px(13.0))
                        .text_color(if index == selected { theme.text } else { theme.text_soft })
                        .font_weight(if index == selected { FontWeight::BOLD } else { FontWeight::MEDIUM }),
                )
                .on_click(cx.listener(move |app, _event, _window, cx| {
                    app.close_combo(id, cx);
                    on_change(app, cx, index);
                })),
        );
    }
    menu
}

fn chat_picker(
    app: &mut crate::App,
    cx: &mut Context<crate::App>,
    theme: &crate::theme::Theme,
) -> impl Element {
    let dialogs = app.dialogs.clone();
    let selected = app.settings_value(TELEGRAM_DESTINATION);
    let menu = div()
        .id("tg-chat-picker")
        .w_full()
        .mt(px(8.0))
        .h(px(CONTROL_HEIGHT))
        .px(px(12.0))
        .rounded(px(RADIUS_SM))
        .bg(theme.raised)
        .border_1()
        .border_color(theme.border)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .child("Choose Telegram chat")
                .text_size(px(12.0))
                .text_color(theme.muted_soft)
                .font_weight(FontWeight::BOLD),
        )
        .child(
            div()
                .flex_1()
                .child(if selected.is_empty() {
                    "No chat selected".to_string()
                } else {
                    selected.clone()
                })
                .text_size(px(13.0))
                .text_color(theme.text)
                .text_ellipsis(),
        )
        .child(icon("▾", 12.0, theme.muted));
    let mut options = vec![menu];
    for dialog in dialogs {
        let id = dialog.id.clone();
        let title = dialog.title.clone();
        options.push(
            div()
                .id(SharedString::from(format!("tg-chat-{id}")))
                .w_full()
                .h(px(40.0))
                .px(px(12.0))
                .cursor_pointer()
                .flex()
                .items_center()
                .child(title)
                .text_size(px(13.0))
                .text_color(theme.text_soft)
                .text_ellipsis()
                .on_click(cx.listener(move |app, _event, _window, cx| {
                    app.set_setting(TELEGRAM_DESTINATION, json!(id), cx);
                    cx.notify();
                })),
        );
    }
    div().w_full().flex().flex_col().children(options)
}

fn diagnostic_row(theme: &crate::theme::Theme, label: &str, value: &str) -> Div {
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(22.0))
        .child(
            div()
                .w(px(140.0))
                .child(label.to_string())
                .text_size(px(13.0))
                .text_color(theme.muted),
        )
        .child(
            div()
                .flex_1()
                .child(value.to_string())
                .text_size(px(12.0))
                .text_color(theme.text)
                .text_ellipsis(),
        )
}

fn diagnostic_cell(theme: &crate::theme::Theme, label: &str, value: &str) -> Div {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .child(label.to_string())
                .text_size(px(13.0))
                .text_color(theme.muted),
        )
        .child(
            div()
                .child(value.to_string())
                .text_size(px(12.0))
                .text_color(theme.text)
                .text_ellipsis(),
        )
}
