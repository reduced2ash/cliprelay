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
  menu: "2px"
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
- Hairline pane structure with shallow tactile depth reserved for interaction
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

Wide workspaces pair a dominant task surface with a persistent utility inspector. The inspector is resizable from 400 to 520px with a 428px default. Inspector navigation and any action dock stay outside the body scroll; Edit and Deliver each own an independent, draft-persistent scroll position. Below 1000 layout pixels, the workbench switches to one useful pane at a time instead of squeezing both. At short heights around 560px, transport and timeline rows compact while preserving controls and state. The supported compact composition remains usable at 700×520 native logical pixels. Interface scale (80–150%) multiplies display DPI consistently across text, controls, menus, hit targets, and media. All layout breakpoints use the resulting layout viewport. Compact Prepare swaps with Library without changing the edit draft; short previews scroll rather than compressing media or dropping precision controls. Compact Settings uses a horizontal section navigator, stacked setting rows, and a scrolling detail body.

**The Work Surface Wins Rule.** Give the selected artifact the largest uninterrupted region and keep utilities stable at its edge.

**The One-Pane Compact Rule.** When two panes cannot both remain useful, provide an explicit surface/inspector switch instead of crushing them together.

## Elevation & Depth

Passive panes in Relay, Pitch Black, Full White, and focused Studio remain flat by construction. Their hierarchy comes from the graphite surface ladder, one-pixel steel borders, selective dimming, and the contrast of the media well. Interactive controls are the deliberate exception: buttons and explicit selections use a shallow directional face, a restrained upper reflection, a soft offset contact shadow, and one pixel of pressed travel. Transient menus may use the softer panel tone and stronger border, but they do not float through decoration.

The opt-in **Frosted Glass / macOS Material** theme is the deliberate exception. It uses the platform's whole-window compositor blur with achromatic, low-alpha semantic fills and single-pixel seams above it. The app never manufactures a wallpaper hue: its apparent color comes entirely from the heavily blurred desktop behind the window.

The companion **Graphite Glass** theme copies that exact structure and blur behavior, then adds a denser achromatic silver/graphite tint. It should read as a polished macOS application material while suppressing recognizable desktop shapes. Neither glass theme uses ambient or pane-wide gradients; compact interactive faces may use the same shallow tonal shaping as the opaque themes. Both materials cover the activity rail, header, Explorer, Library, History, Settings, Prepare, overlays, and the focused Studio shell. Video apertures remain opaque so the material never competes with the work. Relay Coral retains ownership of commit actions.

When blur is unavailable, a neutral dark fallback and the semantic alpha ladder preserve grouping without claiming per-element backdrop filtering. Reduced-transparency or opaque environments must preserve the same text, borders, focus states, and pane ownership.

**The Earned Depth Rule.** Passive hierarchy is explained by adjacency, tone, or a one-pixel seam. Elevation belongs only to an actionable control or explicit selection, and must compress on press rather than becoming a decorative halo.

## Shapes

The form language is squared and mechanical. Action controls, fields, tabs, media apertures, and timeline tracks use the two-pixel small corner; five- and seven-pixel radii are reserved for small supporting geometry. Transient menus use the same two-pixel corner throughout their shells, buttons, row highlights, and badges. Outside menus, fully rounded shapes are reserved for status pills, dots, playhead markers, and the track-and-knob hardware of explicit switches.

**The Earned Radius Rule.** A rounded silhouette must communicate a distinct control type or state, never decorate an ordinary container.

## Components

### Buttons

- **Shape:** Near-square controls with a two-pixel corner, a 36px standard height, 44px preparation controls, and taller primary actions only when anchored in an action dock.
- **Primary:** Relay Coral fill and dark on-commit content; use for the single most consequential available action.
- **Construction:** Primary and bordered secondary buttons use a restrained face-to-edge tonal shift, a one-pixel edge, and a soft directional contact shadow. Compact toolbar controls use the same system at lower elevation.
- **Hover / Focus:** Hover slightly strengthens the face and contact shadow. Keyboard focus keeps a distinct two-pixel accent edge. All enabled buttons support pointer, Enter, and Space activation.
- **Pressed:** The face darkens or reverses, the shadow compresses, and the control travels down one pixel so the state reads as physically depressed without moving surrounding layout.
- **Secondary:** A neutral raised face and steel edge establish affordance without competing with the commit action.
- **Ghost / Danger:** Ghost actions stay borderless and quiet until hover; destructive actions use Fault Rose text with an error-soft tactile hover surface.

### Inputs / Fields

- **Style:** Raised graphite fill, one-pixel steel border, two-pixel corner, 13px system text, and 13px horizontal inset.
- **Focus:** Replace the resting seam with a visible two-pixel coral focus border.
- **Error / Disabled:** Errors pair Fault Rose with text; disabled controls keep their geometry and use muted text plus reduced emphasis.

### Navigation

Explorer distinguishes browsing from previewing: only the Library's browsed folder gets the full selected-row face, border, and depth. The folder containing the preview video uses Coral Text on its icon and name, without a second selected background. Both indicators can coexist on the same folder; keyboard focus remains a separate visible outline.

Inspector tabs divide the available width evenly in a 42px row. Resting tabs use soft text; hover adds a graphite fill; the active tab adds Coral Text and a two-pixel coral rule. Keyboard focus adds a visible coral border without relying on the active rule alone.

Focused Studio places **Back to Prepare** at the left of its top bar, ahead of the source name, to return to the docked workspace. **Open Studio** appears only in docked Prepare; never repeat it as a no-op inside the already-focused workspace.

Status pills use a semantic soft fill, low-opacity semantic border, icon plus explicit text, and the fully rounded pill shape. Ordinary panes remain square and use one-pixel seams rather than card styling.

### Library Thumbnails

Library tiles fill their 16:9 media wells by default. The **Fit the whole video** option preserves the complete source frame and uses the media-well background for letterboxing when aspect ratios differ. Static thumbnails and live hover previews always share the selected framing so the image does not jump on hover.

### Timeline Range

The timeline is a dark media strip with dimmed out-of-range regions, coral IN/OUT gates and selection border, and a two-pixel Playhead Blue line with a compact marker. Range labels and exact fields use tabular figures. Hover and drag feedback strengthen the active gate without changing the meaning of the playhead color.

Studio's exact-range controls sit in one inset, fully bordered panel with clear space above and below and padded content on every side. In/Out buttons and fields form aligned pairs; duration and actions move to a second row in narrower panes. Keep the panel near-square and use the existing surface tone, with tactile depth reserved for its controls.

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
- **Do** use one-pixel steel borders and the graphite surface ladder for passive hierarchy, reserving shallow depth for controls and explicit selections.
- **Do** reserve coral for commitment and selection, blue for temporal focus, and semantic colors for labeled state.
- **Do** keep pointer and keyboard states equivalent, including a visible two-pixel accent focus treatment.
- **Do** use visual preset rails, live geometry, and direct manipulation together when an edit changes spatial framing.
- **Do** preserve independent Edit and Deliver scroll positions while inspector navigation and action docks stay fixed.
- **Do** preserve mask identity and preset state across draft restoration, duplication, selection, and list changes.
- **Do** switch to a deliberate one-pane view when width cannot sustain useful panes.

### Don't:

- **Don't** apply control shadows or tonal faces to passive content, ordinary panes, media tiles, or nested card stacks; depth must identify interaction or selection.
- **Don't** use zero-offset halos, hard block shadows, or ornamental bevels. Every control shadow has a downward contact offset and soft blur, then compresses on press.
- **Don't** round every surface; most controls and media apertures use the two-pixel corner.
- **Don't** use coral, blue, green, amber, or red as decoration.
- **Don't** represent boolean edit state with a decorative checkbox, checkmark, or color-only indicator.
- **Don't** align crop or mask overlays to letterbox bars or another coordinate space than the exported frame.
- **Don't** turn an Edit inspector into a checkbox/property form or a miniature nonlinear editor.
- **Don't** expose Open Studio inside focused Studio; provide Back to Prepare instead.
- **Don't** shrink both the work surface and inspector below usefulness at compact sizes.
- **Don't** hide task state behind color alone.
- **Don't** treat screenshots, fixture media, filenames, destinations, or captions as reusable design assets.
