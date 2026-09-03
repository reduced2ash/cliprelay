//! Settings page: interface, performance, files, Telegram, X handoff,
//! diagnostics — with the exact labels from the QML spec.

use crate::settings_import::*;
use crate::state::*;
use crate::theme::*;
use crate::widgets::*;
use gpui::*;
use serde_json::json;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct SettingsUiState {
    pub bot_token: String,
    pub bot_destination: String,
    pub personal_api_id: String,
    pub personal_api_hash: String,
    pub personal_phone: String,
    pub login_code: String,
    pub login_password: String,
    /// Section nav filter (`None` shows every section).
    pub active_section: Option<String>,
    /// Last theme edit rejection, shown under the color editor.
    pub theme_error: Option<String>,
}

/// Pretty labels for [`BUILTIN_THEME_MODES`], used by the rebase picker.
const THEME_BASE_LABELS: [&str; 5] = [
    "Relay",
    "Pitch black",
    "Full white",
    "Frosted glass",
    "Graphite glass",
];

impl crate::App {
    pub fn render_settings(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let mut page = div()
            .id("settings")
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .bg(theme.ink);

        // Master-detail: a section rail on the left, one roomy section on
        // the right. Searching switches to flat results across sections.
        let narrow = self.window_size.0 < 820.0;
        let gutter = if narrow { 16.0 } else { 24.0 };
        let sidebar = if self.sidebar_collapsed || self.window_size.0 < 1080.0 {
            SIDEBAR_COLLAPSED_WIDTH
        } else {
            SIDEBAR_EXPANDED_WIDTH
        };
        let page_width = (self.window_size.0 - sidebar).max(1.0);
        let side_pad = ((page_width - SETTINGS_MAX_W) / 2.0).max(gutter);
        let searching = !settings_query(self).is_empty();
        let active = self
            .settings_page
            .active_section
            .clone()
            .unwrap_or_else(|| "interface".to_string());
        let empty_field = FieldState::default();
        let search_field = field_with_icon(
            "settings-search",
            "Search settings",
            self.fields.get("settings-search").unwrap_or(&empty_field),
            self.focused_field.as_deref() == Some("settings-search"),
            true,
            false,
            Some("⌕"),
            cx,
        );
        // The shared field widget sets flex-basis: 0, which overrides
        // width, so pin the basis too for a true fixed width.
        let search_field = if narrow {
            search_field.w_full()
        } else {
            search_field.w(px(300.0)).flex_none().flex_basis(px(300.0))
        };
        let mut header_row = div()
            .w_full()
            .flex()
            .gap(px(if narrow { 12.0 } else { 16.0 }));
        if narrow {
            header_row = header_row.flex_col();
        } else {
            header_row = header_row.flex_row().items_end();
        }
        page = page.child(
            div()
                .w_full()
                .px(px(gutter))
                .pt(px(if narrow { 16.0 } else { 24.0 }))
                .pb(px(16.0))
                .flex()
                .flex_col()
                .child(
                    header_row
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .child("Settings")
                                .text_size(px(24.0))
                                .text_color(theme.text)
                                .font_weight(FontWeight::SEMIBOLD),
                        )
                        .child(search_field),
                ),
        );

        // Search + section nav narrow the visible blocks below. Every block
        // keeps its exact settings keys, persistence, and actions.
        let mut any_visible = false;
        let query_display = self
            .fields
            .get("settings-search")
            .map(|field| field.text.trim().to_string())
            .unwrap_or_default();

        // Sections append here; the assembly below decides flat results
        // versus the rail + detail split, so this column stays unsized.
        let stacked = page_width < 900.0;
        let mut inner = div().w_full().flex().flex_col().gap(px(20.0));

        // INTERFACE
        if section_open(
            self,
            "interface",
            &[
                "interface",
                "appearance",
                "theme",
                "color",
                "palette",
                "relay",
                "pitch black",
                "full white",
                "frosted",
                "graphite",
                "density",
                "scale",
                "compact",
                "balanced",
                "standard",
                "library thumbnails",
                "fit",
                "custom",
                "duplicate",
                "copy",
                "colors",
                "colour",
                "background",
                "swatch",
                "rename",
                "delete",
                "editor",
                "overrides",
                "based",
                "hex",
            ],
        ) {
            let mut group = seamed_group(&theme);
            group = group.child(theme_choices(self, cx, &theme));
            inner = inner
                .child(section_head(&theme, "Interface", searching))
                .child(group);
            if let Some(customs) = theme_custom_group(self, cx, &theme) {
                inner = inner.child(customs);
            }
            inner = inner.child(theme_color_editor(self, cx, &theme));
            let mut scale_group = seamed_group(&theme);
            scale_group = scale_group
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
                .child(setting_row(
                    cx,
                    &theme,
                    "Library density",
                    &["Default", "Compact"],
                    if self.density == "compact" { 1 } else { 0 },
                    "density-select",
                    self.open_combos.contains("density-select"),
                    move |app, cx, index| {
                        app.set_setting(
                            LIBRARY_DENSITY,
                            json!(if index == 1 { "compact" } else { "default" }),
                            cx,
                        );
                    },
                ))
                .child(checkbox(
                    "fit-library-thumbnails",
                    "Fit the whole video inside Library thumbnails",
                    self.settings_bool(FIT_LIBRARY_THUMBNAILS),
                    true,
                    cx,
                    |app, cx, value| {
                        app.set_setting(FIT_LIBRARY_THUMBNAILS, json!(value), cx);
                    },
                ));
            inner = inner.child(scale_group);
            any_visible = true;
        }

        // PERFORMANCE
        if section_open(
            self,
            "performance",
            &[
                "performance",
                "rendering",
                "media",
                "vsync",
                "maximum",
                "automatic",
                "export encoder",
                "hardware",
                "software",
                "diagnostics",
                "renderer",
                "gpu",
                "display",
                "gstreamer",
                "video playback",
                "frame pacing",
                "frame spikes",
                "resources",
                "refresh",
            ],
        ) {
            let mut group = seamed_group(&theme);
            group = group
                .child(setting_row(
                    cx,
                    &theme,
                    "Performance mode",
                    &["Automatic", "Maximum performance"],
                    if self.settings_value(PERFORMANCE_MODE) == "maximum" {
                        1
                    } else {
                        0
                    },
                    "performance-select",
                    self.open_combos.contains("performance-select"),
                    move |app, cx, index| {
                        app.set_setting(
                            PERFORMANCE_MODE,
                            json!(if index == 1 { "maximum" } else { "automatic" }),
                            cx,
                        );
                    },
                ))
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
                .child(sub_label(&theme, "LIVE DIAGNOSTICS"))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(diagnostic_row(&theme, "Renderer", "GPUI"))
                        .child(diagnostic_row(&theme, "GPU", "—"))
                        .child(diagnostic_row(&theme, "Display", "—"))
                        .child(diagnostic_row(
                            &theme,
                            "Video playback",
                            if self.diagnostics.gstreamer.is_empty() {
                                "GStreamer"
                            } else {
                                &self.diagnostics.gstreamer
                            },
                        ))
                        .child(diagnostic_row(
                            &theme,
                            "Export",
                            if self.diagnostics.export_encoder.is_empty() {
                                "Not sampled"
                            } else {
                                &self.diagnostics.export_encoder
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
            inner = inner
                .child(section_head(&theme, "Performance", searching))
                .child(group);
            any_visible = true;
        }

        // FILES
        if section_open(
            self,
            "files",
            &[
                "files",
                "library",
                "folder",
                "export",
                "generated",
                "reveal",
                "choose",
                "random",
                "repeats",
                "index",
                "verify",
                "deep scan",
                "thumbnails",
                "hover previews",
                "background",
                "automatic",
            ],
        ) {
            let mut group = seamed_group(&theme);
            group = group
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(8.0))
                        .child(static_field(
                            "static-library-root",
                            &theme,
                            "No folder chosen",
                            self.settings_value(LIBRARY_ROOT),
                        )),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(8.0))
                        .child(static_field(
                            "static-export-dir",
                            &theme,
                            "No folder chosen",
                            self.settings_value(EXPORT_DIR),
                        ))
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
                                    let _ = cliprelay_core::x::XAssistant::reveal(
                                        std::path::Path::new(&export_dir),
                                    );
                                }
                                cx.notify();
                            },
                        )),
                )
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
                ));
            inner = inner
                .child(section_head(&theme, "Files", searching))
                .child(group);
            any_visible = true;
        }

        // TELEGRAM
        let telegram_bot = section_open(
            self,
            "telegram",
            &[
                "telegram",
                "bot",
                "token",
                "botfather",
                "channel",
                "destination",
                "chat id",
                "connect",
                "disconnect",
                "connection",
                "check destination",
                "configured",
            ],
        );
        let telegram_personal = section_open(
            self,
            "telegram",
            &[
                "telegram",
                "personal",
                "account",
                "api id",
                "api hash",
                "phone",
                "login code",
                "password",
                "sign in",
                "sign out",
                "chats",
                "dialogs",
                "session",
                "keychain",
            ],
        );
        if telegram_bot || telegram_personal {
            inner = inner.child(section_head(&theme, "Telegram", searching));
            any_visible = true;
        }
        if telegram_bot {
            let mut group = seamed_group(&theme);
            group = group
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(12.0))
                        .child(group_title(&theme, "Bot"))
                        .child(div().flex_1())
                        .child(status_pill(
                            "tg-status-pill",
                            if self.bot_connected() {
                                "configured"
                            } else {
                                "not configured"
                            },
                            if self.bot_connected() {
                                PillState::Success
                            } else {
                                PillState::Warning
                            },
                        ))
                        .child(if self.bot_connected() {
                            button(
                                "tg-disconnect",
                                "Disconnect",
                                ButtonKind::Ghost,
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
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(8.0))
                        .child(
                            field(
                                "tg-bot-token",
                                "Bot token from @BotFather",
                                self.fields
                                    .get("tg-bot-token")
                                    .unwrap_or(&FieldState::default()),
                                self.focused_field.as_deref() == Some("tg-bot-token"),
                                true,
                                true,
                                cx,
                            )
                            .flex_1()
                            .min_w(px(200.0)),
                        )
                        .child(button(
                            "tg-connect",
                            if self.telegram.bot.starts_with('@') {
                                "Replace bot"
                            } else {
                                "Connect bot"
                            },
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
                        .flex_wrap()
                        .gap(px(8.0))
                        .child(
                            field(
                                "tg-destination",
                                "@channelname or numeric chat ID",
                                self.fields
                                    .get("tg-destination")
                                    .unwrap_or(&FieldState::default()),
                                self.focused_field.as_deref() == Some("tg-destination"),
                                true,
                                false,
                                cx,
                            )
                            .flex_1()
                            .min_w(px(200.0)),
                        )
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
                );
            if !self.telegram.message.is_empty() {
                group = group.child(
                    div()
                        .child(self.telegram.message.clone())
                        .text_size(px(13.0))
                        .text_color(theme.muted),
                );
            }
            inner = inner.child(group);
        }
        if telegram_personal {
            let mut group = seamed_group(&theme);
            group = group
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(12.0))
                        .child(group_title(&theme, "Personal account"))
                        .child(div().flex_1())
                        .child(status_pill(
                            "tg-personal-pill",
                            if self.personal_configured() {
                                "signed in"
                            } else {
                                "not signed in"
                            },
                            if self.personal_configured() {
                                PillState::Success
                            } else {
                                PillState::Warning
                            },
                        ))
                        .child(if self.personal_configured() {
                            button(
                                "tg-sign-out",
                                "Sign out",
                                ButtonKind::Ghost,
                                Some("✕"),
                                true,
                                cx,
                                |app, cx| {
                                    app.command(Command::SignOutPersonal);
                                    cx.notify();
                                },
                            )
                            .into_any()
                        } else {
                            div().into_any()
                        }),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(10.0))
                        .child(
                            field(
                                "tg-api-id",
                                "API ID",
                                self.fields
                                    .get("tg-api-id")
                                    .unwrap_or(&FieldState::default()),
                                self.focused_field.as_deref() == Some("tg-api-id"),
                                true,
                                false,
                                cx,
                            )
                            .flex_1()
                            .min_w(px(160.0)),
                        )
                        .child(
                            field(
                                "tg-api-hash",
                                "API hash",
                                self.fields
                                    .get("tg-api-hash")
                                    .unwrap_or(&FieldState::default()),
                                self.focused_field.as_deref() == Some("tg-api-hash"),
                                true,
                                true,
                                cx,
                            )
                            .flex_1()
                            .min_w(px(160.0)),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(10.0))
                        .child(
                            field(
                                "tg-phone",
                                "+1 555 123 4567",
                                self.fields
                                    .get("tg-phone")
                                    .unwrap_or(&FieldState::default()),
                                self.focused_field.as_deref() == Some("tg-phone"),
                                true,
                                false,
                                cx,
                            )
                            .flex_1()
                            .min_w(px(200.0)),
                        )
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
                        )),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(10.0))
                        .child(
                            field(
                                "tg-code",
                                "Login code",
                                self.fields.get("tg-code").unwrap_or(&FieldState::default()),
                                self.focused_field.as_deref() == Some("tg-code"),
                                true,
                                false,
                                cx,
                            )
                            .flex_1()
                            .min_w(px(160.0)),
                        )
                        .child(
                            field(
                                "tg-password",
                                "2-step password, if requested",
                                self.fields
                                    .get("tg-password")
                                    .unwrap_or(&FieldState::default()),
                                self.focused_field.as_deref() == Some("tg-password"),
                                true,
                                true,
                                cx,
                            )
                            .flex_1()
                            .min_w(px(160.0)),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .flex_wrap()
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
                        )),
                );
            // Chat picker stays with the personal account block.
            if !self.dialogs.is_empty() {
                group = group.child(chat_picker(self, cx, &theme));
            }
            inner = inner.child(group);
        }

        // X HANDOFF
        if section_open(
            self,
            "x",
            &[
                "x",
                "handoff",
                "twitter",
                "post",
                "browser",
                "composer",
                "clipboard",
                "file limit",
                "mb",
                "512",
                "compress",
                "manual",
                "upload",
            ],
        ) {
            let mut group = seamed_group(&theme);
            group = group
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
                                .text_size(px(15.0))
                                .text_color(theme.text_soft)
                                .font_weight(FontWeight::MEDIUM),
                        )
                        .child(div().w(px(110.0)).h(px(CONTROL_HEIGHT)).child(field(
                            "x-limit",
                            "512",
                            self.fields.get("x-limit").unwrap_or(&FieldState::default()),
                            self.focused_field.as_deref() == Some("x-limit"),
                            true,
                            false,
                            cx,
                        )))
                        .child(
                            div()
                                .child("MB")
                                .text_size(px(13.0))
                                .text_color(theme.muted),
                        ),
                )
                .child(x_limit_feedback(self, &theme));
            inner = inner
                .child(section_head(&theme, "X handoff", searching))
                .child(group);
            any_visible = true;
        }

        // DIAGNOSTICS
        if section_open(
            self,
            "diagnostics",
            &[
                "diagnostics",
                "local tools",
                "ffmpeg",
                "ffprobe",
                "database",
                "secrets",
                "keychain",
                "refresh",
            ],
        ) {
            let mut group = seamed_group(&theme);
            group = group.child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(16.0))
                    .child(diagnostic_cell(&theme, "FFmpeg", &self.diagnostics.ffmpeg))
                    .child(diagnostic_cell(
                        &theme,
                        "FFprobe",
                        &self.diagnostics.ffprobe,
                    ))
                    .child(diagnostic_cell(
                        &theme,
                        "Database",
                        &self.diagnostics.database,
                    ))
                    .child(diagnostic_cell(
                        &theme,
                        "Secrets",
                        &self.diagnostics.secret_backend,
                    )),
            );
            inner = inner
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(12.0))
                        .child(section_head(&theme, "Diagnostics", searching))
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
                .child(group);
            any_visible = true;
        }

        if !any_visible {
            inner = inner.child(
                div()
                    .w_full()
                    .mt(px(SPACING_XL))
                    .rounded(px(RADIUS_SM))
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .p(px(32.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .child(format!("No settings match “{query_display}”"))
                            .text_size(px(15.0))
                            .text_color(theme.text)
                            .font_weight(FontWeight::MEDIUM),
                    )
                    .child(
                        div()
                            .child("Try another term, or clear the search to browse every section.")
                            .text_size(px(13.0))
                            .text_color(theme.muted),
                    )
                    .child(button(
                        "settings-clear-search",
                        "Clear search",
                        ButtonKind::Secondary,
                        None,
                        true,
                        cx,
                        |app, cx| {
                            if let Some(field) = app.fields.get_mut("settings-search") {
                                field.text.clear();
                                field.caret = 0;
                            }
                            app.settings_page.active_section = None;
                            cx.notify();
                        },
                    )),
            );
        }

        if searching {
            // Flat results: one exact column, same scroll-range guarantee
            // as the detail pane below.
            page = page.child(
                div()
                    .id("settings-scroll")
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_scroll()
                    .scrollbar_width(px(10.0))
                    .track_scroll(&self.settings_scroll)
                    .child(
                        div()
                            .w_full()
                            .px(px(side_pad))
                            .pt(px(4.0))
                            .pb(px(48.0))
                            .flex()
                            .flex_col()
                            .child(inner),
                    ),
            );
        } else {
            let rail = section_rail(self, cx, &theme, &active, stacked, gutter);
            let mut body = div().w_full().flex().flex_1().min_h(px(0.0));
            if stacked {
                body = body.flex_col().child(rail);
            } else {
                body = body.flex_row().child(rail);
            }
            page = page.child(
                body.child(
                    div()
                        .id("settings-detail")
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_scroll()
                        .scrollbar_width(px(10.0))
                        .track_scroll(&self.settings_scroll)
                        .child(
                            div()
                                .w_full()
                                .max_w(px(SETTINGS_MAX_W))
                                .px(px(if stacked { gutter } else { 32.0 }))
                                .pt(px(4.0))
                                .pb(px(48.0))
                                .flex()
                                .flex_col()
                                .child(inner),
                        ),
                ),
            );
        }
        // Persistent filter status while searching, so a query that hides
        // sections always names itself and offers the way out.
        if !query_display.is_empty() {
            let status = format!("Filtering by “{query_display}”");
            page = page.child(
                div()
                    .w_full()
                    .border_t_1()
                    .border_color(theme.border)
                    .px(px(gutter))
                    .py(px(8.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(status)
                            .text_size(px(12.0))
                            .text_color(theme.muted)
                            .text_ellipsis(),
                    )
                    .child(button(
                        "settings-show-all",
                        "Show all",
                        ButtonKind::Ghost,
                        None,
                        true,
                        cx,
                        |app, cx| {
                            if let Some(field) = app.fields.get_mut("settings-search") {
                                field.text.clear();
                                field.caret = 0;
                            }
                            app.settings_page.active_section = None;
                            cx.notify();
                        },
                    )),
            );
        }
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

    /// Stored user themes, oldest first. Corrupt entries are dropped on read
    /// so one bad edit can never lock the theme system out.
    pub fn custom_theme_list(&self) -> Vec<CustomTheme> {
        decode_custom_themes(self.settings.get(CUSTOM_THEMES))
    }

    fn save_custom_themes(&mut self, themes: Vec<CustomTheme>, cx: &mut Context<crate::App>) {
        let value = serde_json::to_value(&themes).unwrap_or(serde_json::Value::Array(vec![]));
        self.set_setting(CUSTOM_THEMES, value, cx);
    }

    fn update_custom_theme(
        &mut self,
        id: &str,
        cx: &mut Context<crate::App>,
        update: impl FnOnce(&mut CustomTheme),
    ) {
        let mut themes = self.custom_theme_list();
        if let Some(theme) = themes.iter_mut().find(|theme| theme.id == id) {
            update(theme);
            self.settings_page.theme_error = None;
            self.save_custom_themes(themes, cx);
        }
    }

    /// Display name of the active theme, custom names included.
    pub fn theme_display_name(&self) -> String {
        match &self.theme_mode {
            ThemeMode::Custom(id) => self
                .custom_theme_list()
                .iter()
                .find(|theme| &theme.id == id)
                .map(|theme| theme.name.clone())
                .unwrap_or_else(|| "Custom".to_string()),
            mode => ThemeMode::builtin_label(mode.as_str()).to_string(),
        }
    }

    fn next_custom_id(&self) -> String {
        let themes = self.custom_theme_list();
        let mut number = themes.len() + 1;
        loop {
            let id = format!("custom-{number}");
            if themes.iter().all(|theme| theme.id != id) {
                return id;
            }
            number += 1;
        }
    }

    fn unique_custom_name(&self, base_name: &str) -> String {
        let themes = self.custom_theme_list();
        if themes.iter().all(|theme| theme.name != base_name) {
            return base_name.to_string();
        }
        let mut number = 2;
        loop {
            let name = format!("{base_name} {number}");
            if themes.iter().all(|theme| theme.name != name) {
                return name;
            }
            number += 1;
        }
    }

    /// Create a custom theme from a built-in base and apply it, so the
    /// editor below shows the new theme ready to edit. Returns the new
    /// `(id, name)`.
    pub fn create_custom_theme(
        &mut self,
        base: &str,
        name: String,
        cx: &mut Context<crate::App>,
    ) -> (String, String) {
        let base = if BUILTIN_THEME_MODES.contains(&base) {
            base
        } else {
            "relay"
        };
        let trimmed = name.trim();
        let name = if trimmed.is_empty() {
            self.unique_custom_name("Custom theme")
        } else {
            self.unique_custom_name(trimmed)
        };
        let id = self.next_custom_id();
        let mut themes = self.custom_theme_list();
        themes.push(CustomTheme {
            id: id.clone(),
            name: name.clone(),
            base: base.to_string(),
            colors: HashMap::new(),
        });
        self.settings_page.theme_error = None;
        self.save_custom_themes(themes, cx);
        self.set_setting(THEME_MODE, json!(format!("{CUSTOM_THEME_PREFIX}{id}")), cx);
        (id, name)
    }

    /// Duplicate a custom theme, keeping its base and every override.
    pub fn duplicate_custom_theme(&mut self, id: &str, cx: &mut Context<crate::App>) {
        let themes = self.custom_theme_list();
        let Some(source) = themes.iter().find(|theme| theme.id == id).cloned() else {
            return;
        };
        let name = self.unique_custom_name(&format!("{} copy", source.name));
        let new_id = self.next_custom_id();
        let mut themes = themes;
        themes.push(CustomTheme {
            id: new_id.clone(),
            name,
            base: source.base,
            colors: source.colors,
        });
        self.settings_page.theme_error = None;
        self.save_custom_themes(themes, cx);
        self.set_setting(
            THEME_MODE,
            json!(format!("{CUSTOM_THEME_PREFIX}{new_id}")),
            cx,
        );
    }

    /// Delete a custom theme. Deleting the active theme falls back to Relay
    /// rather than leaving a dangling selector behind.
    pub fn delete_custom_theme(&mut self, id: &str, cx: &mut Context<crate::App>) {
        let themes: Vec<CustomTheme> = self
            .custom_theme_list()
            .into_iter()
            .filter(|theme| theme.id != id)
            .collect();
        self.settings_page.theme_error = None;
        let was_active = matches!(&self.theme_mode, ThemeMode::Custom(active) if active == id);
        self.save_custom_themes(themes, cx);
        if was_active {
            self.set_setting(THEME_MODE, json!("relay"), cx);
        }
    }

    /// Apply a custom theme right now (live across the whole app).
    pub fn apply_custom_theme(&mut self, id: &str, cx: &mut Context<crate::App>) {
        let key = format!("{CUSTOM_THEME_PREFIX}{id}");
        self.set_setting(THEME_MODE, json!(key), cx);
    }

    /// Rebase a custom theme onto another built-in; overrides are kept.
    pub fn set_custom_theme_base(&mut self, id: &str, base: &str, cx: &mut Context<crate::App>) {
        if BUILTIN_THEME_MODES.contains(&base) {
            let base = base.to_string();
            self.update_custom_theme(id, cx, |theme| {
                theme.base = base;
            });
        }
    }

    /// Clear one override back to the base color and refresh its field.
    pub fn clear_theme_override(
        &mut self,
        id: &str,
        role: &str,
        field_id: &str,
        cx: &mut Context<crate::App>,
    ) {
        let role = role.to_string();
        self.fields.remove(field_id);
        self.update_custom_theme(id, cx, |theme| {
            theme.colors.remove(&role);
        });
    }

    /// Commit the active custom theme's name; empty names are rejected with
    /// a note. Built-ins show no name field, so anything else is ignored.
    pub fn commit_active_theme_name(&mut self, field_id: &str, cx: &mut Context<crate::App>) {
        if field_id != "theme-active-name" {
            return;
        }
        let ThemeMode::Custom(id) = self.theme_mode.clone() else {
            return;
        };
        let name = self.field_text(field_id).trim().to_string();
        if name.is_empty() {
            self.settings_page.theme_error =
                Some("Give the theme a name — the empty name was ignored.".to_string());
            return;
        }
        self.fields.remove(field_id);
        self.update_custom_theme(&id, cx, |theme| {
            theme.name = name;
        });
    }

    /// Commit one hex override on the active theme. Garbage keeps the old
    /// color and explains itself under the editor instead of corrupting
    /// anything. Editing a built-in forks a personal copy first — customs
    /// edit in place — so the editor never needs an explicit duplicate step.
    pub fn commit_active_theme_hex(&mut self, field_id: &str, cx: &mut Context<crate::App>) {
        let Some(role_key) = field_id.strip_prefix("theme-hex-") else {
            return;
        };
        if THEME_ROLES.iter().all(|role| role.key != role_key) {
            return;
        }
        let text = self.field_text(field_id);
        let Some(hex) = normalize_hex_color(&text) else {
            self.settings_page.theme_error = Some(format!(
                "“{}” is not a hex color — use #RRGGBB.",
                text.trim()
            ));
            return;
        };
        if !matches!(self.theme_mode, ThemeMode::Custom(_)) {
            let base = self.theme_mode.as_str().to_string();
            let name = format!("{} copy", self.theme_display_name());
            let (_, new_name) = self.create_custom_theme(&base, name, cx);
            self.toast(
                ToastKind::Info,
                format!("Created “{new_name}” — now editing your copy."),
            );
        }
        let ThemeMode::Custom(active_id) = self.theme_mode.clone() else {
            return;
        };
        let role_key = role_key.to_string();
        self.fields.remove(field_id);
        self.update_custom_theme(&active_id, cx, |theme| {
            theme.colors.insert(role_key, hex);
        });
    }
}

/// Workbench section label: 11px semibold uppercase with tracking, muted —
/// the OUTPUT/DESTINATIONS/CAPTIONS pattern.
fn section_label(theme: &crate::theme::Theme, label: &str) -> Div {
    div()
        .mt(px(SPACING_XXL))
        .mb(px(SPACING_SM))
        .child(tracked(label))
        .text_size(px(11.0))
        .text_color(theme.muted)
        .font_weight(FontWeight::SEMIBOLD)
}

fn group_title(theme: &crate::theme::Theme, title: &str) -> Div {
    div()
        .child(title.to_string())
        .text_size(px(15.0))
        .text_color(theme.text)
        .font_weight(FontWeight::MEDIUM)
}

/// Capped detail width for the settings page.
const SETTINGS_MAX_W: f32 = 880.0;

/// Workbench-aligned settings card: flat surface, hairline seam, 2px radius.
fn seamed_group(theme: &crate::theme::Theme) -> Div {
    div()
        .w_full()
        .rounded(px(RADIUS_SM))
        .bg(theme.surface)
        .border_1()
        .border_color(theme.border)
        .p(px(20.0))
        .flex()
        .flex_col()
        .gap(px(12.0))
}

fn settings_query(app: &crate::App) -> String {
    app.fields
        .get("settings-search")
        .map(|field| field.text.trim().to_lowercase())
        .unwrap_or_default()
}

/// A section stays visible when the section nav selects it (or selects all)
/// and the search query matches one of its keywords.
fn section_open(app: &crate::App, id: &str, keywords: &[&str]) -> bool {
    let query = settings_query(app);
    if query.is_empty() {
        // Browsing shows the rail-selected section only.
        return app
            .settings_page
            .active_section
            .as_deref()
            .unwrap_or("interface")
            == id;
    }
    // Searching matches every query word across all sections and ignores
    // the rail selection: "file limit" finds X HANDOFF even though no
    // single keyword contains the whole phrase.
    query
        .split_whitespace()
        .all(|word| keywords.iter().any(|key| key.contains(word)))
}

/// Big section title for the detail pane. Search results use the small
/// tracked section_label instead so matches stay scannable.
fn detail_title(theme: &crate::theme::Theme, title: &str) -> Div {
    div()
        .child(title.to_string())
        .text_size(px(22.0))
        .text_color(theme.text)
        .font_weight(FontWeight::SEMIBOLD)
}

/// Section heading that adapts to the mode: the big title while
/// browsing, the small tracked label inside search results.
fn section_head(theme: &crate::theme::Theme, title: &str, searching: bool) -> Div {
    if searching {
        section_label(theme, &title.to_uppercase())
    } else {
        detail_title(theme, title)
    }
}

/// One-line live summary for a rail row: what the user would find if
/// they opened that section right now.
fn rail_status(app: &crate::App, id: &str) -> String {
    match id {
        "interface" => app.theme_display_name(),
        "performance" => {
            if app.settings_value(PERFORMANCE_MODE) == "maximum" {
                "Maximum".to_string()
            } else {
                "Automatic".to_string()
            }
        }
        "files" => {
            let root = app.settings_value(LIBRARY_ROOT);
            if root.is_empty() {
                "No folder".to_string()
            } else {
                std::path::Path::new(&root)
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default()
            }
        }
        "telegram" => {
            let bot = app.bot_connected();
            let personal = app.personal_configured();
            match (bot, personal) {
                (true, true) => "Bot + personal".to_string(),
                (true, false) => "Bot only".to_string(),
                (false, true) => "Personal only".to_string(),
                (false, false) => "Not configured".to_string(),
            }
        }
        "x" => {
            let limit = app
                .settings
                .get(X_LIMIT_MB)
                .and_then(|value| value.as_f64())
                .unwrap_or(512.0);
            if limit.fract() == 0.0 {
                format!("{} MB cap", limit as i64)
            } else {
                format!("{limit} MB cap")
            }
        }
        _ => {
            let ready = !app.diagnostics.ffmpeg.is_empty()
                && !app.diagnostics.ffprobe.is_empty()
                && !app.diagnostics.database.is_empty()
                && !app.diagnostics.secret_backend.is_empty();
            if ready {
                "Tools ready".to_string()
            } else {
                "Checking…".to_string()
            }
        }
    }
}

/// The section rail: six icon-led rows, each naming its section and its
/// live state. The rail is navigation, so it never hides content the way a
/// filter does. Each row pairs a 32px icon tile drawn for its section with
/// the section name and status; selection uses the shared tactile
/// selection face, edge, and shadow so the rail matches theme cards and
/// stays legible across opaque and glass themes.
fn section_rail(
    app: &mut crate::App,
    cx: &mut Context<crate::App>,
    theme: &crate::theme::Theme,
    active: &str,
    stacked: bool,
    gutter: f32,
) -> Div {
    let mut rail = div().flex().gap(px(4.0));
    if stacked {
        rail = rail
            .w_full()
            .px(px(gutter))
            .pt(px(4.0))
            .pb(px(12.0))
            .flex_row()
            .flex_wrap()
            .border_b_1()
            .border_color(theme.border);
    } else {
        rail = rail
            .w(px(248.0))
            .flex_none()
            .flex_col()
            .py(px(8.0))
            .pl(px(12.0))
            .pr(px(16.0))
            .border_r_1()
            .border_color(theme.border);
    }
    for (id, label, glyph) in [
        ("interface", "Interface", "settings-interface"),
        ("performance", "Performance", "settings-performance"),
        ("files", "Files", "settings-files"),
        ("telegram", "Telegram", "settings-telegram"),
        ("x", "X handoff", "settings-x"),
        ("diagnostics", "Diagnostics", "settings-diagnostics"),
    ] {
        let selected = active == id;
        let status = rail_status(app, id);
        let id = id.to_string();
        let click_id = id.clone();
        let activate_id = id.clone();
        let space_id = id;
        // The tile carries the section color: an accent-tinted tile on the
        // selected row, a quiet raised tile otherwise. The row itself keeps
        // the workbench near-square geometry and tactile selection.
        let tile = div()
            .w(px(32.0))
            .h(px(32.0))
            .flex_none()
            .rounded(px(RADIUS_MD))
            .bg(if selected {
                theme.accent_soft
            } else {
                theme.raised
            })
            .border_1()
            .border_color(if selected {
                theme.tactile_edge(TactileState::Rest, true)
            } else {
                theme.border
            })
            .flex()
            .items_center()
            .justify_center()
            .child(icon(
                glyph,
                16.0,
                if selected {
                    theme.accent_text
                } else {
                    theme.muted
                },
            ));
        let mut row = div()
            .id(SharedString::from(format!("settings-rail-{click_id}")))
            .px(px(8.0))
            .py(px(8.0))
            .rounded(px(RADIUS_MD))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(12.0))
            .bg(if selected {
                theme.selection_face(TactileState::Rest, true)
            } else {
                theme.transparent().into()
            })
            .border_1()
            .border_color(if selected {
                theme.tactile_edge(TactileState::Rest, true)
            } else {
                theme.transparent()
            })
            .shadow(if selected {
                theme.tactile_shadow(TactileState::Rest, true)
            } else {
                Vec::new()
            })
            .hover(|style| {
                style
                    .bg(if selected {
                        theme.selection_face(TactileState::Hover, true)
                    } else {
                        theme.control_face(TactileState::Hover)
                    })
                    .border_color(theme.tactile_edge(TactileState::Hover, selected))
            })
            .active(|style| {
                style
                    .bg(theme.selection_face(TactileState::Pressed, selected))
                    .border_color(theme.tactile_edge(TactileState::Pressed, selected))
            })
            .focus(|style| style.border_color(theme.accent))
            .cursor_pointer()
            .tab_index(0)
            .child(tile)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .child(label.to_string())
                            .text_size(px(14.0))
                            .text_color(if selected {
                                theme.accent_text
                            } else {
                                theme.text_soft
                            })
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_ellipsis(),
                    )
                    .child(
                        div()
                            .child(status)
                            .text_size(px(12.0))
                            .text_color(theme.muted)
                            .text_ellipsis(),
                    ),
            )
            .on_click(cx.listener(move |app, _event, _window, cx| {
                app.settings_page.active_section = Some(click_id.clone());
                cx.notify();
            }))
            .on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
                app.settings_page.active_section = Some(activate_id.clone());
                cx.notify();
            }))
            .on_action(
                cx.listener(move |app, _: &crate::ActivateSpace, _window, cx| {
                    app.settings_page.active_section = Some(space_id.clone());
                    cx.notify();
                }),
            );
        if stacked {
            row = row.flex_1().min_w(px(170.0));
        } else {
            row = row.w_full();
        }
        rail = rail.child(row);
    }
    rail
}

/// 32px accent chip identifying one theme row; the selected theme earns
/// the same check badge the Interface cards use.
fn theme_accent_tile(accent: Hsla, theme: &crate::theme::Theme, selected: bool) -> Div {
    let mut tile = div()
        .w(px(32.0))
        .h(px(32.0))
        .flex_none()
        .rounded(px(RADIUS_MD))
        .bg(accent)
        .border_1()
        .border_color(theme.border)
        .flex()
        .items_center()
        .justify_center();
    if selected {
        tile = tile.child(icon("✓", 14.0, theme.accent_content));
    }
    tile
}

/// Custom theme rows with apply/duplicate/delete actions. The editor below
/// always follows the active theme, so rows never need their own Edit.
fn theme_custom_group(
    app: &mut crate::App,
    cx: &mut Context<crate::App>,
    theme: &crate::theme::Theme,
) -> Option<Div> {
    let mut group = seamed_group(theme);
    group = group.child(
        div()
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(12.0))
            .child(group_title(theme, "Custom"))
            .child(div().flex_1()),
    );
    let customs = app.custom_theme_list();
    if customs.is_empty() {
        return None;
    }
    let active_key = app.theme_mode.selection_key();
    for custom in customs {
        let resolved = custom.resolve();
        let selected = active_key == format!("{CUSTOM_THEME_PREFIX}{}", custom.id);
        let subtitle = match custom.colors.len() {
            0 => format!("Based on {}", ThemeMode::builtin_label(&custom.base)),
            1 => format!(
                "Based on {} · 1 override",
                ThemeMode::builtin_label(&custom.base)
            ),
            count => format!(
                "Based on {} · {count} overrides",
                ThemeMode::builtin_label(&custom.base)
            ),
        };
        let apply_id = custom.id.clone();
        let duplicate_id = custom.id.clone();
        let delete_id = custom.id.clone();
        let mut actions = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap(px(8.0));
        if selected {
            actions = actions.child(
                div()
                    .child("Active")
                    .text_size(px(13.0))
                    .text_color(theme.muted)
                    .font_weight(FontWeight::MEDIUM),
            );
        } else {
            actions = actions.child(button(
                format!("theme-apply-{}", custom.id),
                "Apply",
                ButtonKind::Secondary,
                None,
                true,
                cx,
                move |app, cx| {
                    app.apply_custom_theme(&apply_id, cx);
                },
            ));
        }
        actions = actions
            .child(button(
                format!("theme-duplicate-{}", custom.id),
                "Duplicate",
                ButtonKind::Ghost,
                Some("copy"),
                true,
                cx,
                move |app, cx| {
                    app.duplicate_custom_theme(&duplicate_id, cx);
                },
            ))
            .child(button(
                format!("theme-delete-{}", custom.id),
                "Delete",
                ButtonKind::Ghost,
                Some("✕"),
                true,
                cx,
                move |app, cx| {
                    app.delete_custom_theme(&delete_id, cx);
                },
            ));
        group = group.child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(12.0))
                .child(theme_accent_tile(resolved.accent, theme, selected))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .child(custom.name.clone())
                                .text_size(px(14.0))
                                .text_color(theme.text_soft)
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_ellipsis(),
                        )
                        .child(
                            div()
                                .child(subtitle)
                                .text_size(px(12.0))
                                .text_color(theme.muted)
                                .text_ellipsis(),
                        ),
                )
                .child(actions),
        );
    }
    Some(group)
}

/// The color editor for the active theme: rename and rebase for custom
/// themes, plus one hex row per color role with live swatches. Commits
/// apply instantly; editing a built-in forks a personal copy first.
fn theme_color_editor(
    app: &mut crate::App,
    cx: &mut Context<crate::App>,
    theme: &crate::theme::Theme,
) -> Div {
    // The editor always shows the active theme. Customs resolve to
    // themselves; built-ins resolve to their palette wrapped in an empty
    // transient theme, so every row below reads uniformly — and the first
    // committed edit on a built-in forks a personal copy (see
    // `commit_active_theme_hex`).
    let active_custom: Option<CustomTheme> = match &app.theme_mode {
        ThemeMode::Custom(id) => app
            .custom_theme_list()
            .into_iter()
            .find(|custom| &custom.id == id),
        _ => None,
    };
    let name = app.theme_display_name();
    let view = active_custom.unwrap_or_else(|| CustomTheme {
        id: String::new(),
        name: name.clone(),
        base: app.theme_mode.as_str().to_string(),
        colors: HashMap::new(),
    });
    let resolved = view.resolve();
    let is_custom = !view.id.is_empty();
    let empty_field = FieldState::default();

    let mut group = seamed_group(theme);
    let mut header = div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .child(group_title(theme, &format!("Colors of {name}")))
        .child(div().flex_1());
    if !is_custom {
        header = header.child(
            div()
                .child("Built-in")
                .text_size(px(13.0))
                .text_color(theme.muted)
                .font_weight(FontWeight::MEDIUM),
        );
    }
    group = group.child(header);

    if is_custom {
        group = group.child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(12.0))
                .child(
                    div()
                        .flex_1()
                        .child("Name")
                        .text_size(px(15.0))
                        .text_color(theme.text_soft)
                        .font_weight(FontWeight::MEDIUM),
                )
                .child(
                    field(
                        "theme-active-name",
                        view.name.as_str(),
                        app.fields.get("theme-active-name").unwrap_or(&empty_field),
                        app.focused_field.as_deref() == Some("theme-active-name"),
                        true,
                        false,
                        cx,
                    )
                    .flex_1()
                    .min_w(px(200.0)),
                ),
        );

        let base_index = BUILTIN_THEME_MODES
            .iter()
            .position(|mode| *mode == view.base)
            .unwrap_or(0);
        let base_id = view.id.clone();
        group = group.child(setting_row(
            cx,
            theme,
            "Based on",
            &THEME_BASE_LABELS,
            base_index,
            "theme-base",
            app.open_combos.contains("theme-base"),
            move |app, cx, index| {
                app.set_custom_theme_base(&base_id, BUILTIN_THEME_MODES[index], cx);
            },
        ));
    }

    group = group.child(help_text(
        theme,
        if is_custom {
            "Edits apply instantly. Overrides store RGB only, so glass bases keep their translucency."
        } else {
            "Editing any color creates a personal copy of this theme and applies your edit."
        },
    ));
    if let Some(error) = app.settings_page.theme_error.clone() {
        group = group.child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(icon("!", 13.0, theme.error))
                .child(error)
                .text_size(px(13.0))
                .text_color(theme.error),
        );
    }

    for group_name in [
        "Chrome",
        "Surfaces",
        "Text & hairlines",
        "Accent",
        "Status",
        "Media",
    ] {
        group = group.child(sub_label(theme, &group_name.to_uppercase()));
        for role in THEME_ROLES.iter().filter(|role| role.group == group_name) {
            group = group.child(theme_role_row(
                app,
                cx,
                theme,
                &view,
                &resolved,
                role,
                &empty_field,
            ));
        }
    }
    group
}

/// One editable color: override dot, live swatch, label, hex field, reset.
fn theme_role_row(
    app: &crate::App,
    cx: &mut Context<crate::App>,
    theme: &crate::theme::Theme,
    custom: &CustomTheme,
    resolved: &Theme,
    role: &ThemeRole,
    empty_field: &FieldState,
) -> Div {
    let overridden = custom.colors.contains_key(role.key);
    // One editor is ever visible, so the role alone identifies the field.
    let field_id = format!("theme-hex-{}", role.key);
    let reset_id = custom.id.clone();
    let reset_role = role.key.to_string();
    let reset_field = field_id.clone();
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .child(
            div()
                .w(px(6.0))
                .h(px(6.0))
                .flex_none()
                .rounded(px(3.0))
                .bg(if overridden {
                    theme.accent
                } else {
                    theme.transparent()
                }),
        )
        .child(
            div()
                .w(px(24.0))
                .h(px(24.0))
                .flex_none()
                .rounded(px(RADIUS_SM))
                .bg((role.get)(resolved))
                .border_1()
                .border_color(theme.border),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .child(role.label)
                .text_size(px(13.0))
                .text_color(theme.text_soft)
                .font_weight(FontWeight::MEDIUM)
                .text_ellipsis(),
        )
        .child(div().w(px(120.0)).flex_none().child(field(
            field_id.clone(),
            &custom.role_hex(role),
            app.fields.get(&field_id).unwrap_or(empty_field),
            app.focused_field.as_deref() == Some(field_id.as_str()),
            true,
            false,
            cx,
        )))
        .child(button(
            format!("theme-reset-{}-{}", custom.id, role.key),
            "Reset",
            ButtonKind::Ghost,
            Some("↻"),
            overridden,
            cx,
            move |app, cx| {
                app.clear_theme_override(&reset_id, &reset_role, &reset_field, cx);
            },
        ))
}

/// Live validation for the X handoff file limit: confirming a non-positive
/// or non-numeric value keeps the saved limit, so say so before confirm.
fn x_limit_feedback(app: &crate::App, theme: &crate::theme::Theme) -> Div {
    let typed = app
        .fields
        .get("x-limit")
        .map(|field| field.text.trim().to_string())
        .unwrap_or_default();
    if typed.is_empty() {
        // The limit is stored as JSON float (see the "x-limit" commit path),
        // so read it back as f64 — as_i64 would miss a saved 300.0.
        let saved = app
            .settings
            .get(X_LIMIT_MB)
            .and_then(|value| value.as_f64())
            .unwrap_or(512.0);
        let saved_label = if saved.fract() == 0.0 {
            format!("{}", saved as i64)
        } else {
            format!("{saved}")
        };
        return help_text(
            theme,
            &format!("Videos above {saved_label} MB are compressed before handoff. Press Enter to apply a new limit."),
        );
    }
    match typed.parse::<f64>() {
        Ok(mb) if mb > 0.0 => help_text(
            theme,
            &format!(
                "Videos above {typed} MB are compressed before handoff. Press Enter to apply."
            ),
        ),
        _ => div()
            .mt(px(4.0))
            .mb(px(8.0))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .child(icon("!", 13.0, theme.error))
            .child(format!(
                "“{typed}” is not a positive number — confirming keeps the saved limit."
            ))
            .text_size(px(13.0))
            .text_color(theme.error),
    }
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
        .child(tracked(label))
        .text_size(px(11.0))
        .text_color(theme.muted)
        .font_weight(FontWeight::SEMIBOLD)
}

fn static_field(
    id: &'static str,
    theme: &crate::theme::Theme,
    placeholder: &str,
    value: String,
) -> Stateful<Div> {
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
        .child(if empty {
            placeholder.to_string()
        } else {
            value
        })
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
        ("frosted_glass", "Frosted glass", "System backdrop"),
        ("graphite_glass", "Graphite glass", "Silver material"),
    ];
    let mut row = div()
        .w_full()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap(px(12.0))
        .mt(px(12.0));
    for (mode, title, _subtitle) in choices {
        let selected = app.theme_mode.as_str() == mode;
        let palette = crate::theme::Theme::for_mode(crate::theme::ThemeMode::parse(mode));
        let mode = mode.to_string();
        let click_mode = mode.clone();
        let activate_mode = mode.clone();
        let space_mode = mode.clone();
        let rest_face = if selected {
            theme.selection_face(TactileState::Rest, true)
        } else {
            theme.surface.into()
        };
        let hover_face = if selected {
            theme.selection_face(TactileState::Hover, true)
        } else {
            theme.control_face(TactileState::Hover)
        };
        let mut card = div()
            .id(SharedString::from(format!("theme-{mode}")))
            .w(px(196.0))
            .h(px(92.0))
            .rounded(px(RADIUS_MD))
            .relative()
            .top(px(0.0))
            .border_1()
            .border_color(if selected { theme.accent } else { theme.border })
            .bg(rest_face)
            .shadow(if selected {
                theme.tactile_shadow(TactileState::Rest, false)
            } else {
                Vec::new()
            })
            .cursor_pointer()
            .tab_index(0)
            .hover(|style| {
                style
                    .bg(hover_face)
                    .border_color(theme.tactile_edge(TactileState::Hover, selected))
                    .shadow(theme.tactile_shadow(TactileState::Hover, false))
            })
            .active(|style| {
                style
                    .top(px(1.0))
                    .bg(theme.selection_face(TactileState::Pressed, selected))
                    .border_color(theme.tactile_edge(TactileState::Pressed, selected))
                    .shadow(theme.tactile_shadow(TactileState::Pressed, false))
            })
            .focus(|style| style.border_2().border_color(theme.accent))
            .p(px(10.0))
            .flex()
            .flex_row()
            .gap(px(10.0))
            .on_click(cx.listener(move |app, _event, _window, cx| {
                app.set_setting(THEME_MODE, json!(click_mode), cx);
            }))
            .on_action(cx.listener(move |app, _: &crate::Activate, _window, cx| {
                app.set_setting(THEME_MODE, json!(activate_mode), cx);
                cx.stop_propagation();
            }))
            .on_action(
                cx.listener(move |app, _: &crate::ActivateSpace, _window, cx| {
                    app.set_setting(THEME_MODE, json!(space_mode), cx);
                    cx.stop_propagation();
                }),
            );
        // Mini preview.
        card = card.child(
            div()
                .w(px(62.0))
                .h(px(58.0))
                .rounded(px(RADIUS_MD))
                .bg(palette.application_background())
                .border_1()
                .border_color(palette.border)
                .relative()
                .overflow_hidden()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .p(px(6.0))
                .child(
                    div()
                        .w_full()
                        .h(px(8.0))
                        .rounded(px(2.0))
                        .bg(palette.raised),
                )
                .child(
                    div()
                        .w(px(30.0))
                        .h(px(6.0))
                        .rounded(px(2.0))
                        .bg(palette.accent),
                )
                .child(
                    div()
                        .w_full()
                        .h(px(4.0))
                        .rounded(px(2.0))
                        .bg(palette.border),
                ),
        );
        card = card.child(
            div().flex_1().flex().flex_col().justify_center().child(
                div()
                    .child(title)
                    .text_size(px(15.0))
                    .text_color(theme.text)
                    .font_weight(FontWeight::SEMIBOLD),
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

#[allow(clippy::too_many_arguments)]
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
        .min_h(px(52.0))
        .py(px(6.0))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .child(
            div()
                .flex_1()
                .child(label)
                .text_size(px(15.0))
                .text_color(theme.text_soft)
                .font_weight(FontWeight::MEDIUM),
        )
        .child(combo(cx, id, options, selected, 240.0, open, on_change))
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
        .relative()
        .top(px(0.0))
        .bg(theme.control_face(TactileState::Rest))
        .shadow(theme.tactile_shadow(TactileState::Rest, false))
        .border_1()
        .border_color(if open { theme.accent } else { theme.border })
        .hover(|style| {
            style
                .bg(theme.control_face(TactileState::Hover))
                .border_color(theme.tactile_edge(TactileState::Hover, false))
                .shadow(theme.tactile_shadow(TactileState::Hover, false))
        })
        .active(|style| {
            style
                .top(px(1.0))
                .bg(theme.control_face(TactileState::Pressed))
                .border_color(theme.tactile_edge(TactileState::Pressed, false))
                .shadow(theme.tactile_shadow(TactileState::Pressed, false))
        })
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
    // The trigger always owns the row's layout slot; the open menu floats
    // above the page so it never pushes surrounding content around.
    let popup = open.then(|| {
        let mut menu = div()
            .id(id)
            .w(px(width))
            .rounded(px(MENU_RADIUS))
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
                    .bg(if index == selected {
                        theme.active
                    } else {
                        theme.transparent()
                    })
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
                            .text_color(if index == selected {
                                theme.text
                            } else {
                                theme.text_soft
                            })
                            .font_weight(if index == selected {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::MEDIUM
                            }),
                    )
                    .on_click(cx.listener(move |app, _event, _window, cx| {
                        app.close_combo(id, cx);
                        on_change(app, cx, index);
                    })),
            );
        }
        menu
    });
    anchored_overlay(
        trigger,
        popup,
        OverlayPlacement::BelowStart,
        size(px(width), px(CONTROL_HEIGHT)),
    )
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
                .font_weight(FontWeight::SEMIBOLD),
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
        .min_w(px(160.0))
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .child(label.to_string())
                .text_size(px(12.0))
                .text_color(theme.muted),
        )
        .child(
            div()
                .child(value.to_string())
                .text_size(px(15.0))
                .text_color(theme.text)
                .font_weight(FontWeight::MEDIUM)
                .text_ellipsis(),
        )
}
