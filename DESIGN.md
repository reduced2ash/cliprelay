---
name: ClipRelay
description: A calm local media workspace for preparing and relaying video posts.
colors:
  ink: "#151416"
  surface: "#1D1B1E"
  surface-raised: "#262328"
  surface-active: "#312C32"
  bone: "#F1ECE8"
  muted: "#ABA3A4"
  border: "#3A343B"
  accent: "#F07858"
  accent-pressed: "#D96247"
  success: "#72B985"
  warning: "#D8A758"
  error: "#DD6B70"
typography:
  family: "system-ui"
  body: "15px/1.5"
  label: "13px/1.35"
  title: "20px/1.25"
rounded:
  sm: "6px"
  md: "10px"
  lg: "14px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
  xxl: "32px"
---

# Design System: ClipRelay

## 1. Overview

**Creative North Star: "The Quiet Relay Desk"**

ClipRelay feels like a compact media desk at night: the room is quiet, the video is bright, and every tool is within reach without competing for attention. The interface uses flat tonal layers and fine dividers rather than floating cards or decorative shadows. Its primary silhouette is a narrow navigation rail, a spacious media library, and an adaptive preparation panel.

The visual system rejects generic corporate dashboards, neon editing suites, oversized SaaS cards, glassmorphism, and miniature nonlinear-editor complexity. Treat the hierarchy and density guidance in this document as the design contract.

**Key Characteristics:**

- Media-first hierarchy with uncropped frames.
- Warm dark surfaces with a subtle red-violet cast.
- One rare persimmon action color.
- Flat-by-default depth and precise status language.
- Familiar desktop behavior with complete keyboard alternatives.

## 2. Colors

The palette uses dark red-violet-tinted neutrals so photography carries the color, with a restrained persimmon accent reserved for selection, focus, and primary actions.

### Primary

- **Relay Persimmon** (`#F07858`, approximately `oklch(70% 0.15 36)`): Primary action, current media selection, progress focus, and keyboard focus rings.
- **Pressed Persimmon** (`#D96247`, approximately `oklch(62% 0.15 34)`): Active and pressed states only.

### Neutral

- **Night Ink** (`#151416`, approximately `oklch(18% 0.008 320)`): Window background.
- **Desk Surface** (`#1D1B1E`, approximately `oklch(22% 0.010 320)`): Navigation and main surface.
- **Raised Surface** (`#262328`, approximately `oklch(27% 0.014 320)`): Inputs, toolbars, timeline, and selected regions.
- **Active Surface** (`#312C32`, approximately `oklch(32% 0.018 320)`): Hover and active neutral state.
- **Soft Bone** (`#F1ECE8`, approximately `oklch(94% 0.010 55)`): Primary text and icons.
- **Muted Mauve Gray** (`#ABA3A4`, approximately `oklch(70% 0.012 350)`): Secondary text.
- **Hairline Plum** (`#3A343B`, approximately `oklch(36% 0.018 320)`): Dividers, inactive outlines, and media frames.

### Semantic

- **Delivered Green** (`#72B985`): Success, paired with an icon and text.
- **Attention Ochre** (`#D8A758`): Warning, paired with an icon and text.
- **Failure Rose** (`#DD6B70`): Errors, paired with an icon and recovery action.

**The One Signal Rule.** Relay Persimmon occupies less than ten percent of a screen. Its rarity makes the next action obvious.

## 3. Typography

**Display Font:** Native system sans (`SF Pro` on macOS, `Segoe UI` on Windows)
**Body Font:** Native system sans
**Precision Font:** Native system monospaced font for timecodes only

**Character:** Native, compact, and unforced. Interface labels, media metadata, and footer status use the native system sans; tabular numerals keep changing values stable without making routine information look like code.

### Hierarchy

- **Page Title** (650, 20px, 1.25): Library, Prepare, History, and Settings headings.
- **Section Title** (600, 16px, 1.35): Named workflow regions.
- **Body** (450, 15px, 1.5): Explanations, captions, and form values, capped at 70 characters where prose appears.
- **Label** (550, 13px, 1.35): Field labels and navigation.
- **Metadata** (450, 12px, 1.35): Duration, codec, path, timestamps, and status details.

**The Stable Numbers Rule.** Timecode, file size, bitrate, and progress always use tabular figures.

## 4. Elevation

Depth is tonal. Higher surfaces become slightly lighter and more chromatic; shadows are avoided inside the main workspace. Only detached menus and toasts may use a subtle ambient shadow.

**The Flat-by-Default Rule.** Surfaces are separated by tone, spacing, and one-pixel dividers. Shadows never substitute for hierarchy.

## 5. Components

### Buttons

- **Shape:** Compact rounded rectangle, 8px radius, minimum 44px target.
- **Primary:** Relay Persimmon with Night Ink text, 12px by 18px internal spacing.
- **Hover / Focus:** Lighter tone on hover; 2px offset persimmon focus ring.
- **Secondary:** Raised Surface, Hairline Plum border, Soft Bone text.
- **Ghost:** Transparent at rest, Active Surface on hover.
- **Loading:** Preserve width, replace leading icon with progress, retain exact action label when space allows.

### Chips

- **Style:** Flat filter control with 6px radius and a one-pixel outline.
- **State:** Selected chips use Active Surface and Soft Bone; accent is reserved for the current high-level selection.

### Media Tiles

- **Corner Style:** 4px technical media frame; text sits below without an enclosing card.
- **Background:** Night Ink letterbox around `PreserveAspectFit` content.
- **State:** One-pixel Hairline Plum outline at rest, Active Surface on hover, 2px Relay Persimmon outline when selected.
- **Preview:** Hover begins muted playback after 350ms; Space or click provides the same action.
- **Thumbnail state:** Visible tiles enter a bounded background queue without placing loading copy over the media. A quiet progress line indicates active work, failures retain a warning glyph, and aggregate state remains available in the library footer.
- **Density:** Default uses readable metadata and 12px gaps; Compact uses a smaller target width and 8px gaps without changing global workspace scale.

### Inputs / Fields

- **Style:** Raised Surface, Hairline Plum stroke, 8px radius, persistent visible label.
- **Focus:** Relay Persimmon stroke and 2px external focus ring.
- **Error / Disabled:** Failure Rose plus actionable text; disabled controls retain readable contrast.

### Navigation

- **Style:** 184px expanded rail with icon and label; a persistent user control collapses it to a 64px icon rail, and constrained windows collapse it automatically.
- **State:** Active item uses Active Surface and a small leading icon color change, never a thick side stripe.

### Prepare Workspace

- **Width:** A persistent Widen control expands the panel while preserving a usable library column; constrained windows keep the safe narrow width.
- **Full-screen editor:** A separate action moves the same live Prepare instance into an app-wide studio, preserving playback, trim, captions, crop, masks, and delivery state. The video and cut controls lead on the left; independently scrollable Edit and Publish tools occupy the right.
- **Header actions:** Open in the default player, Widen or Narrow, Full-screen editor, and Close use compact familiar controls with accessible names and tooltips.
- **Playback:** The uncropped frame remains unobstructed; play, seek, and time controls sit in a compact inline row beneath it without an enclosing card.
- **Source location:** The selected file path stays visible, with a Reveal in library action that opens its containing folder and focuses its tile.
- **Validation:** A newly discovered random selection may appear immediately, but cut, compression, and delivery controls stay disabled until its single-file check completes.

### Workspace Scale

- **Presets:** Standard 100%, Balanced 90%, and Compact 80% scale the complete logical workspace consistently.
- **Behavior:** Scale changes apply immediately and persist; they expose more content without changing the window size or inventing a second compact component vocabulary.

### Timeline

- **Style:** Low-profile tonal track with two explicit trim handles and exact start/end fields.
- **Behavior:** Handles are keyboard-adjustable; exact time fields remain visible; seeking is immediate and motion-free.

## 6. Do's and Don'ts

### Do:

- **Do** give the video more area than controls or metadata.
- **Do** preserve entire thumbnail frames with letterboxing.
- **Do** use the 4px spacing family and 44px interactive targets.
- **Do** expose loading, partial success, retry, cancellation, and recovery states inline.
- **Do** use precise verbs such as `Pick random`, `Send to Telegram`, `Prepare for X`, and `Move export to Trash`.

### Don't:

- **Don't** make ClipRelay feel like a generic corporate dashboard, neon gaming interface, overdecorated AI SaaS product, miniature nonlinear video editor, or disguised system file picker.
- **Don't** use glassmorphism, gradient text, pure black, pure white, decorative motion, or nested cards.
- **Don't** crop media thumbnails or rely on hover as the only way to preview.
- **Don't** use a colored side stripe thicker than one pixel as an accent.
- **Don't** hide destructive cleanup behind vague labels or allow it to target source videos.
