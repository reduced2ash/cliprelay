---
name: ClipRelay
description: A compact native relay workbench for reviewing, preparing, and delivering media.
colors:
  ground: "#0C1014"
  panel: "#14181D"
  panel-soft: "#171C21"
  field: "#1B2127"
  active: "#222A32"
  hover: "#1D242B"
  text: "#EEF1F4"
  text-soft: "#C7CDD4"
  muted: "#929BA6"
  muted-soft: "#707A85"
  border: "#2C343D"
  border-strong: "#414B56"
  commit: "#F06B4E"
  commit-pressed: "#DA5B40"
  commit-soft: "#2B1C19"
  commit-text: "#FF8062"
  on-commit: "#0B0E11"
  playhead: "#4DA3FF"
  success: "#36D184"
  success-soft: "#10271B"
  warning: "#F1B32B"
  warning-soft: "#2B2412"
  error: "#F27B83"
  error-soft: "#30191F"
  media-well: "#05070B"
  media-overlay: "#081019"
typography:
  title:
    fontFamily: ".SystemUIFont, system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 500
  body:
    fontFamily: ".SystemUIFont, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
  control:
    fontFamily: ".SystemUIFont, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 500
  label:
    fontFamily: ".SystemUIFont, system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 600
    letterSpacing: "1.25px"
  timecode:
    fontFamily: ".SystemUIFont, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    fontFeature: "\"tnum\" 1"
rounded:
  sm: "2px"
  md: "5px"
  lg: "7px"
  menu: "10px"
  pill: "13px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
  xxl: "32px"
components:
  button-primary:
    backgroundColor: "{colors.commit}"
    textColor: "{colors.on-commit}"
    typography: "{typography.control}"
    rounded: "{rounded.sm}"
    padding: "0 12px"
    height: "36px"
  button-secondary:
    backgroundColor: "transparent"
    textColor: "{colors.text}"
    typography: "{typography.control}"
    rounded: "{rounded.sm}"
    padding: "0 12px"
    height: "36px"
  field:
    backgroundColor: "{colors.field}"
    textColor: "{colors.text}"
    typography: "{typography.body}"
    rounded: "{rounded.sm}"
    padding: "0 13px"
    height: "36px"
---

# Design System: ClipRelay

## Overview

**Creative North Star: "The Focused Relay Workbench"**

ClipRelay is a continuous native work surface, not a card dashboard or a miniature nonlinear editor. Blue-black planes, steel hairlines, near-square controls, and cool system type create the compact confidence of a professional tool; a rare coral signal marks the moments where intent becomes a cut or delivery.

The active artifact receives the largest uninterrupted region. Navigation, inspection, status, and delivery stay spatially stable around it, so an operator can move quickly without losing context. The voice is direct and workmanlike, and every visual distinction must help explain task, state, or consequence.

Editing belongs to that same continuous surface. An Edit inspector is a spatial framing console: visual crop, guide, and privacy-mask choices sit beside live geometry and direct manipulation on the media. It must never regress into a legacy checkbox/property form or expand into a timeline-heavy nonlinear editor.

**Key Characteristics:**

- Continuous pane-based composition
- Compact native controls and system typography
- Spatial framing consoles instead of property forms
- Explicit labeled state instead of decorative checkmarks
- Rare, role-specific color signals
- Hairline structure instead of decorative elevation
- Responsive pane swapping at constrained sizes
- Paired direct manipulation and keyboard-equivalent controls

## Colors

The palette is a cool blue-black graphite ladder with high-clarity text and deliberately scarce operational accents.

### Primary

- **Relay Coral** (`commit`, `commit-pressed`, `commit-soft`, `commit-text`, `on-commit`): marks commit actions, active inspector rules, selected trim gates, and edited-state emphasis.

### Secondary

- **Playhead Blue** (`playhead`): identifies the current temporal position and nothing else.

### Tertiary

- **Ready Green** (`success`, `success-soft`): confirms completed or ready state.
- **Setup Amber** (`warning`, `warning-soft`): flags configuration, limits, or checking states that need attention.
- **Fault Rose** (`error`, `error-soft`): marks failures and destructive affordances.

### Neutral

- **Blue-Black Ground** (`ground`): the continuous application and focused-workspace ground.
- **Graphite Planes** (`panel`, `panel-soft`, `field`, `active`, `hover`): distinguish adjacent panes and interaction states without turning them into floating cards.
- **Steel Hairlines** (`border`, `border-strong`): separate rows, panes, fields, and media apertures.
- **Cool Text** (`text`, `text-soft`, `muted`, `muted-soft`): carries the full information hierarchy from primary labels to quiet metadata.
- **Media Neutrals** (`media-well`, `media-overlay`): preserve light-on-dark legibility independent of source imagery.

**The Rare Coral Rule.** Coral marks committed actions, selected trim gates, and active rules; it is never ambient decoration.

**The Blue Clock Rule.** Playhead blue belongs only to temporal focus and must not become a generic action accent.

**The Semantic Signal Rule.** Green, amber, and red always accompany status text or an icon; color never carries readiness alone.

## Typography

**UI Font:** Native system UI face, with platform fallbacks. **Numeric Font:** The same face with tabular figures enabled.

**Character:** Compact, neutral, and instrument-like; hierarchy comes from measured size, weight, casing, and contrast rather than font changes or oversized headings.

### Hierarchy

- **Title** (medium, 15px): destination names, prominent row titles, and primary action labels.
- **Body / Control** (regular or medium, 13px): form values, descriptions, buttons, tabs, and primary readouts.
- **Secondary** (regular, 12px): metadata, readiness summaries, and supporting explanations.
- **Label** (semibold, 11px, approximately 1.25px tracking, uppercase): compact section markers such as OUTPUT, DESTINATIONS, and CAPTIONS.
- **Timecode** (regular, 13px, tabular figures): transport position, IN/CUT/OUT summaries, duration, and exact range fields.

**The Instrument Readout Rule.** Timecodes and numeric range readouts always use tabular figures so values remain spatially stable while changing.

**The Platform Face Rule.** Use the platform-native system UI face for titles, labels, controls, and tabular readouts; do not introduce Open Sans or another web face into the native workbench.

## Layout

The system uses contiguous panes and border-separated rows on the shared 4, 8, 12, 16, 24, and 32px spacing rhythm. Outer workbench insets are compact, dense inspector content uses 20px horizontal padding, and one-pixel seams—not empty card gutters—establish ownership.

Wide workspaces pair a dominant task surface with a persistent utility inspector. The inspector is resizable from 400 to 520px with a 428px default. Inspector navigation and any action dock stay outside the body scroll; Edit and Deliver each own an independent, draft-persistent scroll position. Below 980px, the workbench switches to one useful pane at a time instead of squeezing both. At short heights around 560px, transport and timeline rows compact while preserving controls and state. The supported compact composition remains usable at 700×520.

**The Work Surface Wins Rule.** Give the selected artifact the largest uninterrupted region and keep utilities stable at its edge.

**The One-Pane Compact Rule.** When two panes cannot both remain useful, provide an explicit surface/inspector switch instead of crushing them together.

## Elevation & Depth

ClipRelay is flat by construction. It uses no shadows, gradients, translucency, or glass; depth comes from the graphite surface ladder, one-pixel steel borders, selective dimming, and the contrast of the media well. Transient menus may use the softer panel tone and stronger border, but they do not float through decoration.

**The Hairline Hierarchy Rule.** If hierarchy can be explained by adjacency, tone, or a one-pixel seam, do not add elevation.

## Shapes

The form language is squared and mechanical. Action controls, fields, tabs, media apertures, and timeline tracks use the two-pixel small corner; five- and seven-pixel radii are reserved for small supporting geometry. Ten-pixel rounding belongs to transient menus, while fully rounded shapes are reserved for status pills, dots, playhead markers, and the track-and-knob hardware of explicit switches.

**The Earned Radius Rule.** A rounded silhouette must communicate a distinct control type or state, never decorate an ordinary container.

## Components

### Buttons

- **Shape:** Near-square controls with a two-pixel corner, a 36px standard height, 44px preparation controls, and taller primary actions only when anchored in an action dock.
- **Primary:** Relay Coral fill and dark on-commit content; use for the single most consequential available action.
- **Hover / Focus:** Primary hover and pressed states deepen to `commit-pressed`; keyboard focus remains visibly distinct. All enabled buttons support pointer, Enter, and Space activation.
- **Secondary:** Transparent with a steel border; hover introduces the graphite hover fill and a stronger seam.
- **Ghost / Danger:** Ghost actions stay borderless and quiet until hover; destructive actions use Fault Rose text with an error-soft hover surface.

### Inputs / Fields

- **Style:** Raised graphite fill, one-pixel steel border, two-pixel corner, 13px system text, and 13px horizontal inset.
- **Focus:** Replace the resting seam with a visible two-pixel coral focus border.
- **Error / Disabled:** Errors pair Fault Rose with text; disabled controls keep their geometry and use muted text plus reduced emphasis.

### Navigation

Inspector tabs divide the available width evenly in a 42px row. Resting tabs use soft text; hover adds a graphite fill; the active tab adds Coral Text and a two-pixel coral rule. Keyboard focus adds a visible coral border without relying on the active rule alone.

Focused Studio uses a real **Back to Prepare** action to return to the docked workspace. **Open Studio** appears only in docked Prepare; never repeat it as a no-op inside the already-focused workspace.

Status pills use a semantic soft fill, low-opacity semantic border, icon plus explicit text, and the fully rounded pill shape. Ordinary panes remain square and use one-pixel seams rather than card styling.

### Timeline Range

The timeline is a dark media strip with dimmed out-of-range regions, coral IN/OUT gates and selection border, and a two-pixel Playhead Blue line with a compact marker. Range labels and exact fields use tabular figures. Hover and drag feedback strengthen the active gate without changing the meaning of the playhead color.

### Framing Console

The framing console is Edit’s signature inspector pattern: a concise heading and reset action lead into one explicit crop state, a full-width visual preset rail, a live geometry strip, preview guides, and a privacy-mask workbench. Use compact silhouettes plus text labels for crop ratios, guide modes, and mask shapes so each choice communicates its spatial result before selection.

- **Boolean state:** Crop enablement uses a labeled **ON/OFF** switch with a moving track and knob. The row also names the affected concept and describes the current consequence; never substitute a decorative checkbox or lone checkmark.
- **Preset rails:** Use equal-width square tiles with a miniature visual and explicit label. Selected tiles pair an active graphite plane, coral border, and coral text; hover strengthens the neutral border, while keyboard focus uses a visible two-pixel coral border.
- **Geometry and control:** Pair direct manipulation on the media with live percentage readouts and keyboard-equivalent nudge, center, size, duplicate, and remove controls. Disabled geometry controls retain their footprint so the console does not jump when crop is off.
- **Coordinate space:** Crop and guide overlays align to the fitted, contained video pixels rather than the surrounding media well. Privacy masks align to the post-crop output frame when cropping is active.
- **Stable mask state:** Each mask keeps a durable identity and its originating preset across draft reloads, duplication, selection, and list changes so row focus and labels do not drift to another mask.

**The Explicit State Rule.** Boolean editing controls state the affected concept and show a labeled ON/OFF switch; never use a decorative checkbox or lone checkmark.

**The Shared Geometry Rule.** Visual overlays, readouts, pointer manipulation, and keyboard controls must describe the same fitted media coordinate space, with masks mapped to the post-crop output frame.

## Do's and Don'ts

### Do:

- **Do** give the active artifact or media surface the largest uninterrupted region, with utility panes fixed, scrollable, and structurally separated.
- **Do** use one-pixel steel borders and the graphite surface ladder to communicate hierarchy.
- **Do** reserve coral for commitment and selection, blue for temporal focus, and semantic colors for labeled state.
- **Do** keep pointer and keyboard states equivalent, including a visible two-pixel accent focus treatment.
- **Do** use visual preset rails, live geometry, and direct manipulation together when an edit changes spatial framing.
- **Do** preserve independent Edit and Deliver scroll positions while inspector navigation and action docks stay fixed.
- **Do** preserve mask identity and preset state across draft restoration, duplication, selection, and list changes.
- **Do** switch to a deliberate one-pane view when width cannot sustain useful panes.

### Don't:

- **Don't** introduce shadows, gradients, glass, or floating card stacks.
- **Don't** round every surface; most controls and media apertures use the two-pixel corner.
- **Don't** use coral, blue, green, amber, or red as decoration.
- **Don't** represent boolean edit state with a decorative checkbox, checkmark, or color-only indicator.
- **Don't** align crop or mask overlays to letterbox bars or another coordinate space than the exported frame.
- **Don't** turn an Edit inspector into a checkbox/property form or a miniature nonlinear editor.
- **Don't** expose Open Studio inside focused Studio; provide Back to Prepare instead.
- **Don't** shrink both the work surface and inspector below usefulness at compact sizes.
- **Don't** hide task state behind color alone.
- **Don't** treat screenshots, fixture media, filenames, destinations, or captions as reusable design assets.
