//! History page: relayed posts with delivery status, retries, and cleanup.

use crate::state::*;
use crate::widgets::*;
use cliprelay_core::db::PostRow;
use gpui::*;
use std::path::PathBuf;

impl crate::App {
    pub fn render_history(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let rows = self.history.rows.clone();
        let has_more = self.history.has_more;
        let search = self.history_search.clone();
        let narrow = self.window_size.0 < 820.0;
        let empty_field = FieldState::default();

        let mut page = div()
            .id("history")
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .bg(theme.ink);

        // Header collapses to two rows before the title and search compete.
        let mut header = div()
            .w_full()
            .px(px(if narrow { 16.0 } else { 26.0 }))
            .py(px(if narrow { 16.0 } else { 24.0 }))
            .flex()
            .gap(px(if narrow { 12.0 } else { 16.0 }));
        if narrow {
            header = header.flex_col();
        } else {
            header = header.flex_row().items_end();
        }
        header = header
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .child("Relay history")
                            .text_size(px(20.0))
                            .text_color(theme.text)
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        div()
                            .child("Every post prepared through this app stays visible here")
                            .text_size(px(12.0))
                            .text_color(theme.muted)
                            .text_ellipsis(),
                    ),
            )
            .child(
                field(
                    "history-search",
                    "Search history",
                    self.fields.get("history-search").unwrap_or(&empty_field),
                    self.focused_field.as_deref() == Some("history-search"),
                    true,
                    false,
                    cx,
                )
                .w(px(if narrow { (self.window_size.0 - 96.0).max(240.0) } else { 280.0 })),
            );
        page = page.child(header);

        // List.
        let mut list = div()
            .id("history-list")
            .flex_1()
            .mx(px(20.0))
            .relative()
            .overflow_scroll()
            .scrollbar_width(px(10.0))
            .track_scroll(&self.history_scroll);
        if rows.is_empty() {
            let (title, body) = if !search.is_empty() {
                ("No matching relays".to_string(), "Try another filename or caption.".to_string())
            } else {
                (
                    "Nothing relayed yet".to_string(),
                    "Telegram sends and X handoffs will appear here with their exact status and generated file.".to_string(),
                )
            };
            list = list.child(
                div()
                    .w_full()
                    .h_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(11.0))
                    .child(
                        div()
                            .w(px(58.0))
                            .h(px(58.0))
                            .rounded(px(16.0))
                            .bg(theme.active)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon("▷", 27.0, theme.accent)),
                    )
                    .child(
                        div()
                            .child(title)
                            .text_size(px(20.0))
                            .text_color(theme.text)
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        div()
                            .child(body)
                            .text_size(px(13.0))
                            .text_color(theme.muted)
                            .max_w(px(420.0))
                            .text_align(TextAlign::Center),
                    )
                    .child(if search.is_empty() {
                        button(
                            "history-open-library",
                            "Open library",
                            ButtonKind::Primary,
                            Some("▸"),
                            true,
                            cx,
                            |app, cx| {
                                app.page = Page::Library;
                                cx.notify();
                            },
                        )
                        .into_any()
                    } else {
                        div().into_any()
                    }),
            );
        } else {
            // Paged list: render every loaded row (rows are light elements)
            // and load the next page when the scroll nears the end. The
            // measured scroll extent keeps pagination exact regardless of
            // variable row heights.
            let mut inner = div().w_full().flex().flex_col().pb(px(16.0));
            for post in rows.iter() {
                inner = inner.child(self.render_history_row(cx, &theme, post));
            }
            list = list
                .child(inner)
                .on_scroll_wheel(cx.listener(|_app, _event, _window, cx| {
                    cx.notify();
                }));
            if has_more {
                let offset = f32::from(self.history_scroll.offset().y);
                let max = f32::from(self.history_scroll.max_offset().height);
                if max - offset < 500.0 {
                    let controller = self.controller.clone();
                    cx.defer(move |_cx| {
                        let _ = controller.send(Command::LoadMoreHistory);
                    });
                }
            }
        }

        page = page.child(list);
        page
    }

    fn render_history_row(
        &self,
        cx: &mut Context<Self>,
        theme: &crate::theme::Theme,
        post: &PostRow,
    ) -> impl Element {
        let post_id = post.id;
        let media_name = post.media_name.clone().unwrap_or_default();
        let created = post.created_at.clone();
        let created_label = pretty_date(&created);
        let caption = if !post.telegram_caption.is_empty() {
            post.telegram_caption.clone()
        } else {
            post.x_caption.clone()
        };
        let caption_label = if caption.is_empty() {
            "No caption".to_string()
        } else {
            caption.clone()
        };
        let caption_muted = caption.is_empty();
        let edited = post
            .edit_spec
            .as_ref()
            .is_some_and(|spec| {
                let value: serde_json::Value = serde_json::from_str(spec).unwrap_or_default();
                !cliprelay_core::media::normalize_edit_spec(&value).is_empty()
            });
        let telegram_status = post.telegram_status.clone();
        let x_status = post.x_status.clone();
        let error_text = post.error.clone();
        let can_trash = post.is_generated.unwrap_or(false) && post.cleanup_state.as_deref() != Some("trashed");
        let has_export = post.export_path.is_some();
        let thumbnail = post.thumbnail_path.clone().unwrap_or_default();

        let mut row = div()
            .id(SharedString::from(format!("history-{post_id}")))
            .w_full()
            .px(px(14.0))
            .py(px(14.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .border_b_1()
            .border_color(theme.border)
            .hover(|style| style.bg(theme.surface_soft));
        let mut top = div()
            .w_full()
            .flex()
            .flex_row()
            .gap(px(16.0));

        // Thumbnail.
        top = top.child(
            div()
                .w(px(150.0))
                .h(px(92.0))
                .flex_none()
                .rounded(px(10.0))
                .bg(theme.surface)
                .overflow_hidden()
                .child(if thumbnail.is_empty() {
                    div()
                        .w_full()
                        .h_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon("▷", 25.0, theme.muted_soft))
                        .into_any()
                } else {
                    div()
                        .w_full()
                        .h_full()
                        .child(
                            img(PathBuf::from(thumbnail))
                                .w_full()
                                .h_full()
                                .object_fit(ObjectFit::Contain),
                        )
                        .into_any()
                }),
        );

        // Body.
        top = top.child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .gap(px(12.0))
                        .child(
                            div()
                                .flex_1()
                                .child(media_name)
                                .text_size(px(15.0))
                                .text_color(theme.text)
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_ellipsis(),
                        )
                        .child(
                            div()
                                .child(created_label)
                                .text_size(px(12.0))
                                .text_color(theme.muted),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .child(caption_label)
                        .text_size(px(13.0))
                        .text_color(if caption_muted { theme.muted } else { theme.text })
                        .text_ellipsis(),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(8.0))
                        .child(if edited {
                            pill(theme, "Edited copy", theme.accent, &theme.accent_soft)
                        } else {
                            div()
                        })
                        .child(if telegram_status != "not_requested" {
                            let (color, soft) = match telegram_status.as_str() {
                                "sent" => (theme.success, theme.success_soft),
                                "failed" => (theme.error, theme.error_soft),
                                _ => (theme.warning, theme.warning_soft),
                            };
                            pill(
                                theme,
                                &format!("Telegram {}", telegram_status.replace('_', " ")),
                                color,
                                &soft,
                            )
                        } else {
                            div()
                        })
                        .child(if x_status != "not_requested" {
                            let (color, soft) = match x_status.as_str() {
                                "posted" => (theme.success, theme.success_soft),
                                "failed" => (theme.error, theme.error_soft),
                                _ => (theme.accent, theme.accent_soft),
                            };
                            pill(
                                theme,
                                &format!("X {}", x_status.replace('_', " ")),
                                color,
                                &soft,
                            )
                        } else {
                            div()
                        }),
                )
                .child(if error_text.is_empty() {
                    div()
                } else {
                    div()
                        .w_full()
                        .child(error_text)
                        .text_size(px(12.0))
                        .text_color(theme.error)
                        .text_ellipsis()
                }),
        );

        // Actions: fixed 166px column on the right of the content
        // (mirrors the original's HistoryPage row layout).
        let mut actions = div()
            .w(px(166.0))
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(6.0));
        actions = actions.child(button(
            format!("history-view-{post_id}"),
            "View",
            ButtonKind::Secondary,
            Some("▶"),
            true,
            cx,
            move |app, cx| {
                app.command(Command::ViewHistoryPost(post_id));
                cx.notify();
            },
        ));
        if telegram_status == "failed" {
            actions = actions.child(button(
                format!("history-retry-{post_id}"),
                "Retry Telegram",
                ButtonKind::Secondary,
                Some("↻"),
                true,
                cx,
                move |app, cx| {
                    app.command(Command::RetryTelegram(post_id));
                    cx.notify();
                },
            ));
        }
        if x_status == "prepared" {
            actions = actions.child(button(
                format!("history-x-{post_id}"),
                "Mark X posted",
                ButtonKind::Primary,
                Some("✓"),
                true,
                cx,
                move |app, cx| {
                    app.command(Command::MarkXPosted(post_id, String::new()));
                    cx.notify();
                },
            ));
        }
        actions = actions.child(button_at(
            format!("history-more-{post_id}"),
            "More actions",
            ButtonKind::Ghost,
            Some("⋯"),
            true,
            cx,
            move |app, position, cx| {
                // Toggle with the original's 180ms reopen guard; the menu
                // anchors to the clicked row (mirrors the original's
                // button-anchored Menu popup).
                if app.history_more_menu_post == Some(post_id) {
                    app.history_more_menu_post = None;
                    app.history_more_menu_closed_at = std::time::Instant::now();
                } else if app.history_more_menu_closed_at.elapsed().as_millis() >= 180 {
                    app.history_more_menu_post = Some(post_id);
                    let y: f32 = position.y.into();
                    app.history_more_menu_y = y;
                }
                cx.notify();
            },
        ));
        let _ = (can_trash, has_export);
        let row_inner = div()
            .w_full()
            .flex()
            .flex_row()
            .items_start()
            .gap(px(12.0))
            .child(top.flex_1())
            .child(actions);
        row = row.child(row_inner);
        row
    }
}

fn pill(
    _theme: &crate::theme::Theme,
    label: &str,
    color: Hsla,
    soft: &Hsla,
) -> Div {
    div()
        .h(px(24.0))
        .px(px(9.0))
        .rounded(px(6.0))
        .bg(*soft)
        .flex()
        .items_center()
        .child(label.to_string())
        .text_size(px(12.0))
        .text_color(color)
        .font_weight(FontWeight::MEDIUM)
}

fn pretty_date(iso: &str) -> String {
    let parsed = chrono::DateTime::parse_from_rfc3339(iso).ok();
    match parsed {
        Some(dt) => dt
            .with_timezone(&chrono::Local)
            .format("%b %-d, %Y · %I:%M %p")
            .to_string(),
        None => iso.to_string(),
    }
}

#[cfg(test)]
mod history_tests {
    use super::pretty_date;

    #[test]
    fn pretty_date_formats_and_falls_back() {
        // The rendered time is LOCAL; assert the instant round-trips and
        // the format is stable regardless of the machine's timezone.
        let rendered = pretty_date("2026-08-05T14:30:00Z");
        // The rendering must equal the input instant converted to LOCAL
        // time (format-stable across machines).
        let expected = chrono::DateTime::parse_from_rfc3339("2026-08-05T14:30:00Z")
            .unwrap()
            .with_timezone(&chrono::Local)
            .format("%b %-d, %Y · %I:%M %p")
            .to_string();
        assert_eq!(rendered, expected);
        assert!(pretty_date("2026-08-10T09:05:00Z").starts_with("Aug 10, 2026 · "));
        // Unparseable input passes through unchanged.
        assert_eq!(pretty_date("not-a-date"), "not-a-date");
    }
}
