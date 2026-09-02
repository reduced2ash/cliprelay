//! History page: relayed posts with delivery status, retries, and cleanup.

use crate::state::*;
use crate::theme::*;
use crate::widgets::*;
use cliprelay_core::db::PostRow;
use gpui::prelude::FluentBuilder;
use gpui::*;
use std::path::PathBuf;

const HISTORY_ROW_HEIGHT: f32 = 176.0;
/// Earned radius reserved for status pills, dots, and switches.
const HISTORY_PILL_RADIUS: f32 = 13.0;

/// View-side status filter for the loaded history rows. Search stays
/// controller-side (it re-queries the database); this filter only narrows
/// what is already loaded so it never disturbs pagination or cleanup.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HistoryStatusFilter {
    #[default]
    All,
    NeedsAttention,
    Delivered,
    InProgress,
}

impl HistoryStatusFilter {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::NeedsAttention => "Needs attention",
            Self::Delivered => "Delivered",
            Self::InProgress => "In progress",
        }
    }

    fn matches(self, post: &PostRow) -> bool {
        match self {
            Self::All => true,
            Self::NeedsAttention => post_needs_attention(post),
            Self::Delivered => post_delivered(post),
            Self::InProgress => !post_needs_attention(post) && !post_delivered(post),
        }
    }
}

fn post_needs_attention(post: &PostRow) -> bool {
    post.telegram_status == "failed" || post.x_status == "failed" || !post.error.is_empty()
}

fn post_delivered(post: &PostRow) -> bool {
    if post_needs_attention(post) {
        return false;
    }
    post.telegram_status == "sent" || post.x_status == "posted"
}

impl crate::App {
    pub fn render_history(&mut self, cx: &mut Context<Self>) -> impl Element {
        let theme = self.theme.clone();
        let narrow = self.window_size.0 < 820.0;
        let empty_field = FieldState::default();
        let active_filter = self.history_status_filter;
        let search = self.history_search.clone();

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
            .pt(px(if narrow { 16.0 } else { 24.0 }))
            .pb(px(12.0))
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
                            .text_size(px(15.0))
                            .text_color(theme.text)
                            .font_weight(FontWeight::MEDIUM),
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
                .w(px(if narrow {
                    (self.window_size.0 - 96.0).max(240.0)
                } else {
                    280.0
                })),
            );
        page = page.child(header);

        // Status filter chips + relay count. View-side only: search still
        // re-queries, this just narrows the loaded rows.
        let row_count = self.history.rows.len();
        let shown_count = self
            .history
            .rows
            .iter()
            .filter(|post| active_filter.matches(post))
            .count();
        let mut filter_row = div()
            .w_full()
            .px(px(if narrow { 16.0 } else { 26.0 }))
            .pb(px(12.0))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0));
        for option in [
            HistoryStatusFilter::All,
            HistoryStatusFilter::NeedsAttention,
            HistoryStatusFilter::Delivered,
            HistoryStatusFilter::InProgress,
        ] {
            filter_row = filter_row.child(filter_chip(&theme, option, option == active_filter, cx));
        }
        filter_row = filter_row.child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .justify_end()
                .child(tabular(
                    div()
                        .child(if active_filter == HistoryStatusFilter::All {
                            format!(
                                "{} {}",
                                row_count,
                                if row_count == 1 { "relay" } else { "relays" }
                            )
                        } else {
                            format!("{shown_count} of {row_count}")
                        })
                        .text_size(px(12.0))
                        .text_color(theme.muted),
                )),
        );
        page = page.child(filter_row);

        // List.
        let mut list = div()
            .id("history-list")
            .flex_1()
            .mx(px(20.0))
            .relative()
            .overflow_scroll()
            .scrollbar_width(px(10.0))
            .track_scroll(&self.history_scroll);
        if shown_count == 0 {
            let filter_empty = row_count > 0 && active_filter != HistoryStatusFilter::All;
            let (title, body) = if filter_empty {
                (
                    "No relays match this filter".to_string(),
                    "Nothing loaded has this delivery state yet.".to_string(),
                )
            } else if !search.is_empty() {
                (
                    "No matching relays".to_string(),
                    "Try another filename or caption.".to_string(),
                )
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
                            .rounded(px(RADIUS_SM))
                            .bg(theme.raised)
                            .border_1()
                            .border_color(theme.border)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon("▷", 27.0, theme.muted)),
                    )
                    .child(
                        div()
                            .child(title)
                            .text_size(px(15.0))
                            .text_color(theme.text)
                            .font_weight(FontWeight::MEDIUM),
                    )
                    .child(
                        div()
                            .child(body)
                            .text_size(px(13.0))
                            .text_color(theme.muted)
                            .max_w(px(420.0))
                            .text_align(TextAlign::Center),
                    )
                    .child(if filter_empty {
                        button(
                            "history-clear-filter",
                            "Show all relays",
                            ButtonKind::Secondary,
                            None,
                            true,
                            cx,
                            |app, cx| {
                                app.history_status_filter = HistoryStatusFilter::All;
                                cx.notify();
                            },
                        )
                        .into_any()
                    } else if search.is_empty() {
                        button(
                            "history-open-library",
                            "Open library",
                            ButtonKind::Primary,
                            Some("▸"),
                            true,
                            cx,
                            |app, cx| {
                                app.navigate_to(Page::Library, cx);
                            },
                        )
                        .into_any()
                    } else {
                        div().into_any()
                    }),
            );
        } else {
            // History can grow without bound. Keep a fixed-height virtual
            // window so playback ticks and scroll updates render only nearby
            // posts instead of rebuilding every loaded history row.
            let filtered: Vec<PostRow> = self
                .history
                .rows
                .iter()
                .filter(|post| active_filter.matches(post))
                .cloned()
                .collect();
            let visible_count = filtered.len();
            let scroll_y = (-f32::from(self.history_scroll.offset().y)).max(0.0);
            let viewport_height =
                (self.window_size.1 - if narrow { 250.0 } else { 220.0 }).max(HISTORY_ROW_HEIGHT);
            let first = ((scroll_y / HISTORY_ROW_HEIGHT).floor() as isize - 2).max(0) as usize;
            let visible = (viewport_height / HISTORY_ROW_HEIGHT).ceil() as usize + 4;
            let end = (first + visible).min(visible_count);
            let visible_rows = filtered[first.min(visible_count)..end].to_vec();
            let mut window = div()
                .absolute()
                .top(px(first as f32 * HISTORY_ROW_HEIGHT))
                .left(px(0.0))
                .right(px(0.0))
                .flex()
                .flex_col();
            for post in visible_rows.iter() {
                window = window.child(self.render_history_row(cx, &theme, post));
            }
            let inner = div()
                .w_full()
                .h(px(visible_count as f32 * HISTORY_ROW_HEIGHT + 16.0))
                .relative()
                .child(window);
            list = list
                .child(inner)
                .on_scroll_wheel(cx.listener(|_app, _event, _window, cx| {
                    cx.notify();
                }));
            if self.history.has_more {
                let offset = (-f32::from(self.history_scroll.offset().y)).max(0.0);
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
        &mut self,
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
        let edited = post.edit_spec.as_ref().is_some_and(|spec| {
            let value: serde_json::Value = serde_json::from_str(spec).unwrap_or_default();
            !cliprelay_core::media::normalize_edit_spec(&value).is_empty()
        });
        let telegram_status = post.telegram_status.clone();
        let x_status = post.x_status.clone();
        let error_text = post.error.clone();
        let can_trash =
            post.is_generated.unwrap_or(false) && post.cleanup_state.as_deref() != Some("trashed");
        let has_export = post.export_path.is_some();
        let thumbnail = post.thumbnail_path.clone().unwrap_or_default();

        let mut row = div()
            .id(SharedString::from(format!("history-{post_id}")))
            .w_full()
            .h(px(HISTORY_ROW_HEIGHT))
            .px(px(14.0))
            .py(px(14.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .border_b_1()
            .border_color(theme.border)
            .hover(|style| style.bg(theme.surface_soft));
        let mut top = div().w_full().flex().flex_row().gap(px(16.0));

        // Thumbnail.
        top = top.child(
            div()
                .w(px(150.0))
                .h(px(92.0))
                .flex_none()
                .rounded(px(RADIUS_SM))
                .bg(theme.media_overlay)
                .border_1()
                .border_color(theme.border)
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
                        .items_baseline()
                        .gap(px(12.0))
                        .child(
                            div()
                                .flex_1()
                                .child(media_name)
                                .text_size(px(15.0))
                                .text_color(theme.text)
                                .font_weight(FontWeight::MEDIUM)
                                .text_ellipsis(),
                        )
                        .child(tabular(
                            div()
                                .child(created_label)
                                .text_size(px(12.0))
                                .text_color(theme.muted),
                        )),
                )
                .child(
                    div()
                        .w_full()
                        .child(caption_label)
                        .text_size(px(13.0))
                        .text_color(if caption_muted {
                            theme.muted
                        } else {
                            theme.text_soft
                        })
                        .text_ellipsis(),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(if edited {
                            status_pill(theme.accent, &theme.accent_soft, "Edited copy")
                        } else {
                            div()
                        })
                        .child(if telegram_status != "not_requested" {
                            let (color, soft) = match telegram_status.as_str() {
                                "sent" => (theme.success, theme.success_soft),
                                "failed" => (theme.error, theme.error_soft),
                                _ => (theme.warning, theme.warning_soft),
                            };
                            status_pill(
                                color,
                                &soft,
                                &format!("Telegram {}", telegram_status.replace('_', " ")),
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
                            status_pill(color, &soft, &format!("X {}", x_status.replace('_', " ")))
                        } else {
                            div()
                        })
                        .child(if post.cleanup_state.as_deref() == Some("trashed") {
                            status_pill(theme.muted, &theme.active, "Cleaned up")
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
        let more_button = button(
            format!("history-more-{post_id}"),
            "More actions",
            ButtonKind::Ghost,
            Some("⋯"),
            true,
            cx,
            move |app, cx| {
                if app.history_more_menu_post == Some(post_id) {
                    app.history_more_menu_post = None;
                    app.history_more_menu_closed_at = std::time::Instant::now();
                } else if app.history_more_menu_closed_at.elapsed().as_millis() >= 180 {
                    app.dismiss_root_popovers();
                    app.history_more_menu_post = Some(post_id);
                }
                cx.notify();
            },
        )
        .w(px(132.0))
        .when(self.history_more_menu_post == Some(post_id), |button| {
            button.track_focus(&self.history_source_focus)
        });
        let more_menu = (self.history_more_menu_post == Some(post_id))
            .then(|| self.render_history_menu(cx).into_any());
        actions = actions.child(
            anchored_overlay(
                more_button,
                more_menu,
                OverlayPlacement::AboveEnd,
                size(px(132.0), px(CONTROL_HEIGHT)),
            )
            .flex_none()
            .w(px(132.0))
            .h(px(CONTROL_HEIGHT)),
        );
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

fn filter_chip(
    theme: &crate::theme::Theme,
    option: HistoryStatusFilter,
    active: bool,
    cx: &mut Context<crate::App>,
) -> Stateful<Div> {
    let label = option.label();
    div()
        .id(SharedString::from(format!(
            "history-filter-{}",
            label.to_lowercase().replace(' ', "-")
        )))
        .h(px(28.0))
        .px(px(12.0))
        .rounded(px(HISTORY_PILL_RADIUS))
        .flex()
        .items_center()
        .justify_center()
        .border_1()
        .border_color(if active { theme.accent } else { theme.border })
        .bg(if active {
            theme.accent_soft
        } else {
            theme.transparent()
        })
        .hover(|style| {
            style.bg(if active {
                theme.accent_soft
            } else {
                theme.hover
            })
        })
        .cursor_pointer()
        .child(
            div()
                .child(label.to_string())
                .text_size(px(12.0))
                .text_color(if active {
                    theme.accent_text
                } else {
                    theme.muted
                })
                .font_weight(FontWeight::MEDIUM),
        )
        .on_click(cx.listener(move |app, _event, _window, cx| {
            app.history_status_filter = option;
            cx.notify();
        }))
}

/// Status pill with a dot plus text: semantic color is never carried by
/// color alone, and the 13px earned radius keeps pills distinct from the
/// near-square controls.
fn status_pill(dot: Hsla, soft: &Hsla, label: &str) -> Div {
    div()
        .h(px(24.0))
        .pl(px(8.0))
        .pr(px(10.0))
        .rounded(px(HISTORY_PILL_RADIUS))
        .bg(*soft)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .child(div().w(px(6.0)).h(px(6.0)).rounded(px(3.0)).bg(dot))
        .child(
            div()
                .child(label.to_string())
                .text_size(px(12.0))
                .text_color(dot)
                .font_weight(FontWeight::MEDIUM),
        )
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
    use super::{post_delivered, post_needs_attention, pretty_date, HistoryStatusFilter};
    use cliprelay_core::db::PostRow;

    fn post_with(telegram: &str, x: &str, error: &str) -> PostRow {
        PostRow {
            id: 1,
            media_id: 1,
            export_id: None,
            created_at: String::new(),
            updated_at: String::new(),
            telegram_enabled: true,
            x_enabled: false,
            telegram_caption: String::new(),
            x_caption: String::new(),
            telegram_mode: String::new(),
            telegram_destination: String::new(),
            telegram_status: telegram.to_string(),
            telegram_message_id: String::new(),
            telegram_message_link: String::new(),
            x_status: x.to_string(),
            x_url: String::new(),
            cleanup_policy: String::new(),
            error: error.to_string(),
            source_path: None,
            media_name: None,
            thumbnail_path: None,
            source_duration: None,
            export_path: None,
            is_generated: None,
            edit_spec: None,
            cleanup_state: None,
            export_size: None,
        }
    }

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

    #[test]
    fn status_filter_partitions_rows() {
        let sent = post_with("sent", "not_requested", "");
        let failed = post_with("failed", "not_requested", "");
        let prepared = post_with("not_requested", "prepared", "");
        let errored = post_with("sent", "not_requested", "boom");

        assert!(HistoryStatusFilter::All.matches(&sent));
        assert!(HistoryStatusFilter::Delivered.matches(&sent));
        assert!(!HistoryStatusFilter::NeedsAttention.matches(&sent));
        assert!(!HistoryStatusFilter::InProgress.matches(&sent));

        assert!(HistoryStatusFilter::NeedsAttention.matches(&failed));
        assert!(!HistoryStatusFilter::Delivered.matches(&failed));

        assert!(HistoryStatusFilter::InProgress.matches(&prepared));
        assert!(!HistoryStatusFilter::Delivered.matches(&prepared));

        // An error line always needs attention, even when a send succeeded.
        assert!(post_needs_attention(&errored));
        assert!(!post_delivered(&errored));
        assert!(HistoryStatusFilter::NeedsAttention.matches(&errored));
    }
}
