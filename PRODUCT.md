# Product

<!-- impeccable:product-schema 1 -->

## Platform

adaptive

## Users

ClipRelay is for creators and operators working through large local video libraries. They need to browse quickly, inspect a selected clip without losing library context, make a safe derivative, and deliver it without turning the workflow into a full nonlinear-editing session.

## Product Purpose

ClipRelay is a local-first desktop workspace for discovering videos, preparing non-destructive derivatives, sending them to Telegram, and handing them to X for deliberate manual posting. Success means moving from a large folder tree to a reviewed, correctly prepared, delivered clip with clear state and minimal interruption.

## Positioning

ClipRelay combines persistent local-library navigation, lightweight media preparation, and platform delivery in one desktop workspace while keeping source media read-only and user data local. The docked Prepare surface supports rapid review in context; focused Prepare provides precise editing and delivery without becoming a general-purpose video editor.

## Operating Context

- Users repeatedly browse folder trees, search, sort, preview, or request a random clip.
- Selecting a video opens Prepare beside the library for review, playback, rough trimming, source details, and a compact delivery action.
- Focused Prepare expands the same selected clip into a dedicated workspace for precise trim, crop, masks, captions, compression, and delivery.
- Delivery targets include Telegram bot and personal-account flows plus a manual X composer handoff.

## Capabilities and Constraints

- Source videos are never modified; edits and delivery operate on generated derivatives.
- Preserve the existing playback, trim, crop, masks, captions, compression, delivery, history, cleanup, persistence, keyboard, and platform behavior.
- The dock must prioritize a materially larger video preview while retaining compact playback, simple trim, source context, Open Studio, and compact delivery.
- Focused Prepare should use a dominant media workspace, a compact timeline, and a persistent right inspector with editing and delivery tabs.
- The native Rust/GPUI application targets macOS, Windows, and Linux and must remain usable with pointer and keyboard at compact and large window sizes.

## Brand Commitments

- Keep the ClipRelay name and its direct, workmanlike product voice.
- The new interface system should feel at home beside VS Code, Raycast, and Zed: compact, precise, restrained, tool-like, and spatially disciplined.
- Prepare is the first surface of a broader application-wide visual update, so its chrome and controls must generalize beyond media editing.

## Evidence on Hand

- The restored native implementation in `crates/app/src/prepare_render.rs`, `crates/app/src/render_impls.rs`, `crates/app/src/theme.rs`, and `crates/app/src/widgets.rs`.
- Isolated GUI captures in `artifacts/ui-test/` showing the current library, docked Prepare, and focused Prepare states.
- Existing user documentation and behavior in `README.md` and `docs/USER_GUIDE.md`.

## Product Principles

- Keep the current task and system state legible at a glance.
- Spend space on the selected media, not decorative containers or repeated controls.
- Keep quick work in context and move precision work into focused Prepare.
- Make expert workflows fast without hiding essential actions from occasional users.
- Prefer quiet, consistent controls whose hierarchy comes from placement and state rather than visual noise.

## Accessibility & Inclusion

All workflows must remain keyboard reachable with visible focus, stable semantic identities, readable text, non-color-only state cues, and layouts that survive text expansion and the supported minimum window size. Respect platform motion, transparency, and contrast preferences where available.
