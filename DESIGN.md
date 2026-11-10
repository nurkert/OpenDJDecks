---
name: OpenDJDecks
description: A DJ instrument of quiet charcoal surfaces where every signal is a lit emission line.
colors:
  ground: "#151514"
  ground-band: "#1A1A19"
  plate: "#262624"
  plate-raised: "#32322F"
  rule: "#33322F"
  rule-strong: "#4B4A46"
  bone: "#E6E0D2"
  bone-2: "#BDB7AA"
  bone-3: "#A29C90"
  w405-violet: "#9B7BFF"
  w436-indigo: "#7C83FF"
  w486-cyan: "#4FB8F0"
  w546-green: "#82D16B"
  w589-sodium: "#F5CE4A"
  w615-orange: "#F39047"
  w656-deep-red: "#EF5A5A"
  w405-violet-cb: "#CC79A7"
  w486-cyan-cb: "#56B4E9"
  w546-green-cb: "#009E73"
  w589-sodium-cb: "#F0E442"
  w615-orange-cb: "#E69F00"
  w656-deep-red-cb: "#D55E00"
  ground-hc: "#0B0B0B"
  ground-band-hc: "#121212"
  plate-hc: "#1C1C1B"
  rule-hc: "#4B4A46"
  bone-hc: "#FFFFFF"
  bone-2-hc: "#E6E0D2"
  bone-3-hc: "#B7B1A4"
typography:
  display:
    fontFamily: "Barlow Semi Condensed"
    fontSize: "32px"
    fontWeight: 600
    lineHeight: 1
    letterSpacing: "normal"
  headline:
    fontFamily: "Barlow Semi Condensed"
    fontSize: "22px"
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: "normal"
  title:
    fontFamily: "Barlow Semi Condensed"
    fontSize: "22px"
    fontWeight: 600
    lineHeight: 1.25
    letterSpacing: "normal"
  body:
    fontFamily: "Barlow Semi Condensed"
    fontSize: "15px"
    fontWeight: 500
    lineHeight: 1.35
    letterSpacing: "normal"
  nav:
    fontFamily: "Barlow Semi Condensed"
    fontSize: "13px"
    fontWeight: 600
    letterSpacing: "2.2px"
  button-label:
    fontFamily: "Barlow Semi Condensed"
    fontSize: "13px"
    fontWeight: 600
    letterSpacing: "1.8px"
  label:
    fontFamily: "Barlow Semi Condensed"
    fontSize: "11px"
    fontWeight: 500
    letterSpacing: "1.6px"
rounded:
  none: "0px"
  control: "4px"
  panel: "8px"
  overlay: "10px"
  rail-node: "3.5px"
spacing:
  s1: "4px"
  s2: "8px"
  s3: "12px"
  s4: "16px"
  s5: "24px"
  s6: "32px"
  rail-x: "28px"
  row: "28px"
  row-kiosk: "44px"
components:
  button-plate:
    backgroundColor: "{colors.plate}"
    textColor: "{colors.bone-2}"
    typography: "{typography.button-label}"
    rounded: "{rounded.control}"
    padding: "0 14px"
    height: "32px"
  button-plate-compact:
    backgroundColor: "{colors.plate}"
    textColor: "{colors.bone-2}"
    rounded: "{rounded.control}"
    height: "26px"
  button-plate-tall:
    backgroundColor: "{colors.plate}"
    textColor: "{colors.bone-2}"
    rounded: "{rounded.control}"
    height: "44px"
  button-plate-hover:
    backgroundColor: "{colors.plate-raised}"
    textColor: "{colors.bone}"
  button-plate-selected:
    backgroundColor: "{colors.bone}"
    textColor: "{colors.ground}"
  button-plate-on:
    backgroundColor: "{colors.w589-sodium}"
    textColor: "{colors.ground}"
  button-plate-disabled:
    backgroundColor: "{colors.ground-band}"
    textColor: "{colors.bone-3}"
  field:
    backgroundColor: "{colors.plate}"
    textColor: "{colors.bone}"
    typography: "{typography.body}"
    rounded: "{rounded.control}"
    padding: "0 10px"
    height: "32px"
  field-kiosk:
    backgroundColor: "{colors.plate}"
    height: "44px"
  track-row:
    backgroundColor: "transparent"
    textColor: "{colors.bone-2}"
    typography: "{typography.body}"
    padding: "0 12px 0 16px"
    height: "{spacing.row}"
  track-row-hover:
    backgroundColor: "{colors.ground-band}"
  track-row-selected:
    textColor: "{colors.bone}"
  track-row-kiosk:
    height: "{spacing.row-kiosk}"
  rail:
    backgroundColor: "transparent"
    textColor: "{colors.bone-2}"
    typography: "{typography.nav}"
    width: "232px"
  tree-item:
    textColor: "{colors.bone-2}"
    typography: "{typography.body}"
    height: "26px"
  tree-item-hover:
    backgroundColor: "{colors.plate}"
  device-line:
    textColor: "{colors.bone}"
    height: "64px"
  device-line-roomy:
    height: "78px"
  waveform-well:
    backgroundColor: "{colors.ground-band}"
    height: "92px"
---

# Design System: OpenDJDecks

## Overview

**Creative North Star: "The Lit Instrument"**

OpenDJDecks is a calibrated instrument in a dark room. Quiet charcoal surfaces give it structure, and light is reserved for what is happening: the emission lines, the waveforms, the platter ring and whatever a DJ has switched on. When a screen is at rest it is calm; when something is live, it is the brightest thing there.

The product is judged in seconds by its hierarchy and finish, and it is used in the dark at arm's length. So surfaces separate groups without outlining everything, one thing per area is clearly the most important, and a control that is on can never be mistaken for one that is off. Emission lines keep carrying signal and status, where their form still says what state a signal is in. Controls no longer speak in line forms: they are surfaces that fill.

The world refuses grey dashboards, neon glow and boxed widgets on boxed widgets. It also refuses the engineer's panel: dozens of identical outlined rectangles of equal weight.

**Key Characteristics:**
- Three surface levels and one spacing scale carry the structure.
- Light means live: colour appears as emission lines, deck identity, cue identity and things that are on.
- One state language for every control: off, hover, selected, on, armed, disabled, focused.
- Soft small radii on controls and panels, and the circle of the platter as the signature form.
- One family (Barlow Semi Condensed) in five sizes and two weights, with live values as the largest thing on a deck.
- Flat at rest. Only dialogs and menus lift, and only the platter ring and playhead may glow.

## Colors

A charcoal continuum with bone ink and seven narrow spectral lines; each wavelength is a fixed role, not a decoration.

### Primary
- **Sodium 589** (w589-sodium): the attention line. Playheads and preview cursors (doubled), selection ticks, pending sync (dashed), loaded-on-deck marker (doubled), set kind marker, LOCKED legend, armed keys, muted stems and loops that are running, analysis progress.

### Secondary
- **Green 546** (w546-green): the live line. In-sync sticks (solid), syncing fill, PLAY, ADD MUSIC / SYNC / REGISTER plate leads, harmonic-fit marks in the KEY column, on-device dot in the tree, deck 3, hot cue A.
- **Deep Red 656** (w656-deep-red): the fault line. Struck crossbars, error detail text, missing files, DELETE / FORGET / RECORD plates, recordings, level-meter clip, hot cue G.

### Tertiary (deck and cue wavelengths)
- **Cyan 486** (w486-cyan): deck 1; LIBRARY section; selection highlight in text inputs (35% alpha); hot cue B.
- **Orange 615** (w615-orange): deck 2; hot cue C.
- **Violet 405** (w405-violet): deck 4; hot cue D.
- **Indigo 436** (w436-indigo): smart lists (dashed marker), key functions (KEY SYNC, key shift, keylock), hot cue F. It has no colour-blind variant in the build.
- Hot cue E is sodium; hot cue H falls back to bone-2.

### Neutral
- **Charcoal Ground** (ground): the window. Overlaid at start-up by a 640x4 texture that random-walks ±3 levels around the ground value, giving faint vertical banding.
- **Band Charcoal** (ground-band): the panel surface. Decks, the mixer, the browser, waveform wells.
- **Ash Plate** (plate): the control surface. Every button, key and field at rest.
- **Raised Ash** (plate-raised): the control surface under hover and press, and the tree's hover add-target.
- **Rule** (rule) and **Strong Rule** (rule-strong): hairline dividers; strong rule for plate outlines, the rail itself, rulers, off-state lines.
- **Bone, Bone 2, Bone 3** (bone, bone-2, bone-3): three inks, each at 4.5:1 or better on every surface it may sit on, including plate and raised ash. Bone for titles and selected/primary values, bone-2 for secondary text and unselected rows, bone-3 for caps labels, metadata and disabled text.

### Accessibility variants
- **Colour-blind safe** (the `-cb` tokens): the Okabe-Ito set replaces 405, 486, 546, 589, 615 and 656; 436 is unchanged.
- **High contrast** (the `-hc` tokens): darker ground, brighter bone ramp, rule lifted to strong rule, and the hairline weight doubles from 1px to 2px. Plate-raised and rule-strong have no high-contrast variant.

### Named Rules
**The Second Channel Rule.** Colour never carries state alone. Every coloured line also has a form, every deck colour sits beside its numeral, every cue colour beside its letter.

**The Fixed Wavelength Rule.** Each wavelength keeps its role across the product: deck 1 = 486, deck 2 = 615, deck 3 = 546, deck 4 = 405; live = 546, attention = 589, fault = 656. Never introduce an eighth hue or a tint between lines.

**The Doubled Sodium Rule.** A doubled sodium line means "this is where now is" or "this needs you": playheads and preview cursors, changes from players awaiting review. A master deck is its own deck line doubled. Do not double anything else.

## Typography

**Font:** Barlow Semi Condensed (bundled Regular 400, Medium 500, SemiBold 600), with figures in fixed-width columns.

**Character:** One condensed grotesque doing all the work. Tracked capitals are for short control names and legends. Sentences, hints, names and anything longer than a few words use sentence case.

### Scale

Only these five sizes and two weights (500 and 600) are used. A font size outside this list is a defect.

| Rank | Size | Used for |
|---|---|---|
| Hero | 32px, 600 | BPM and remaining time on a deck, the platter centre |
| Heading | 22px, 600 | Deck numerals, screen and section titles, track title on the player |
| Body | 15px, 500 | Track names, table cells, tree items, prose |
| Control | 13px, 600 | Button labels and tabs in tracked caps |
| Label | 11px, 500 | Readout labels and legends |

**Floor:** nothing is smaller than 11px, nothing read live or touched is smaller than 12px, and the Raspberry Pi kiosk uses 12px as its smallest size. The text size preference scales the deck and mixer as well as the list.

### Named Rules
**The Ink Rank Rule.** Hierarchy is size, weight and ink (bone, bone-2, bone-3), not colour. Coloured text is limited to deck numerals, fault messages and short semantic legends such as LOCKED.

**The Tabular Column Rule.** BPM, KEY, TIME and RATING sit right-aligned in fixed columns (58, 46, 52, 74px) so values read down the page like a register.

## Layout

The arrangement follows the layout conventions DJs know from DJ hardware and software (owner decision 2026-11-06); the visual language is our own. Desktop (preferred 1440x900, min 800x480) stacks four bands:

1. **Top bar** (44px): brand, PREPARE / PERFORM tabs (2px sodium underline on the current one), engine state, undo, RECORD on the right.
2. **Player band.** Prepare: the 236px track player (title, BPM / KEY / TIME, round CUE and PLAY, position; detail waveform over the overview; HOT CUE A–D | E–H, MEMORY CUE, grid tools on the right). Perform: one enlarged waveform lane per deck (84px, 58px with four decks, 46px tight) with the deck numeral and centre playhead, then odd decks left, mixer centre, even decks right, then the PHONES / crossfader / MASTER footer (44px).
3. **Browser:** the 260px sidebar tree (COLLECTION, PLAYLISTS, DEVICES with sync and eject, RECORDINGS, SET / SMART / FOLDER footer) beside the list: title, count and duration, list actions, search (260px), set flow and transition note for sets, and the track table with a PREVIEW waveform column.
4. **Status band** (30px).

Rhythm is a 4px grid (s1 to s6: 4, 8, 12, 16, 24, 32). Desktop rows are 28px tall; tree rows 26px; table header 30px.

**Kiosk** (Raspberry Pi, full screen, min 480x320): opens in Perform; the sidebar is hidden and the browser is a separate screen. Spare height goes to the waveform lanes; deck panels keep their natural height. Touch sizes: hot cue pads 40px (32px tight), transports 52px (44px tight), rows and fields 44px. Below 620px of height (set from Rust) the mixer is hidden and lanes shrink to 46px.

## Elevation & Depth

Depth is surface first. The window is ground, panels are band charcoal, controls are ash plate, and hover and press move to raised ash. Groups are separated by surface and by space, and hairline rules appear only where a surface change is not available, such as table rows.

Dialogs and menus are the only things that lift, with one soft shadow (0 16px 48px at 50% black) over a dimmed window.

Light is the other axis. The platter ring and the playhead may glow softly because they are the live centre of the screen. Nothing else emits light, and gradients stay out of controls and panels.

### Named Rules
**The Quiet Surface Rule.** A surface never competes with a signal. If removing a border loses nothing, the border goes.

**The Live Light Rule.** If it is not live, it is not lit.

## Shapes

Controls have a 4px radius, panels 8px, dialogs and menus 10px. The platter and the round transport keys are full circles and stay the signature shape. Status lines and rail nodes keep their own geometry. Waveform wells and table rows stay square, because they are measuring surfaces.

Borders are the exception: focus, the on-state of a key that sits on a coloured area, and hollow empty slots such as an unset hot cue.

### Named Rules
**The Concave Neighbour Rule.** A round element (the jog, the platter) gets concave neighbours. A key beside it has an inner edge that is an arc concentric with the ring, one key gap (4 px) from it all the way round; above and below the arc it fills its corner up to half a key gap short of the centre line, so no dead background is left between keys and ring. Outer edges stay straight with the control radius, the legend sits toward the outer edge and never touches the curve, the fill follows the state language, and the key takes the pointer only where it is drawn (never over the ring), at least over the rectangle it replaced. Built from one component (`ConcaveKey`).

**The Centred Group Rule.** Groups centre instead of stretching. When a deck has spare height, its parts grow a little up to a limit (the platter to 300 px, its key block with it, the transport rows to 1.6 times their base) and the room left over goes evenly above and below the group, never into the gaps between its keys. The platter, its key block and SYNC / CUE / PLAY share one horizontal axis, and the two decks mirror each other.

## Components

### Emission Lines (the primitive)
- **Forms:** solid (full height); dashed (4px dash, 7px period); half (lower half only); struck (line at 45% alpha crossed by a 1px deep-red tick extending 4px each side); double (two lines 3px apart).
- **Weights:** hair 1px (2px high contrast); line 2px; 3px only for the selected set-flow position.
- **Progress:** a dashed line at 50% alpha filled solid from the start by progress, over the panel timing.

### State Language

Every interactive control uses exactly these states, in Perform, Prepare, Lighting, effects and Preferences alike.

| State | Surface | Text | Second cue |
|---|---|---|---|
| Off | Ash plate | Bone 2 | |
| Hover | Raised ash | Bone | |
| Pressed | Raised ash, 1px down | Bone | |
| Selected (one of several) | Bone fill | Ground | Filled segment |
| On (a function is active) | Filled in its tone | Ground | Fill itself, and the label stays |
| Armed (will act on the next event) | Ash plate with a 2px outline in its tone, slow pulse | Bone | Pulse, off when motion is off |
| Running (recording, syncing) | Filled in its tone with a timer or percent | Ground | Moving value |
| Disabled | Band charcoal with a hairline rule edge | Bone 3 | No hover; keeps its width |
| Focused | Any of the above | | 2px bone ring, 2px offset, used for nothing else |

**One job per colour.** Sodium: now and attention, meaning playheads, the selected item in a list, and things that need you. Green: playing, and in sync. Red: recording, clash, danger. Cyan, orange, green and violet also name decks 1 to 4, and appear on a deck only as that deck's identity. Master is shown by the deck's own line doubled and a MASTER key filled in the deck colour.

### Buttons (plate buttons)
- **Shape:** ash plate, 4px radius, no outline at rest. Label in Control rank. A hairline glyph is optional.
- **Sizes:** 32px default, 26px compact, 44px tall on touch. Keys inside a group touch with 2px gaps, and groups are separated by 12px or more.
- **Grouped choices** (pad mode, tempo range, tabs) are one segmented control with a filled current segment.
- **Motion:** background over the press timing.

### Inputs / Fields
- **Style:** ash plate, 4px radius, 32px tall (44px on touch), text in Body rank, placeholder in bone 3.
- **Focus:** the focus ring.
- **Error:** a deep-red message in Control rank below the field that says what to do next.

### Toggles and status lines
- Switches in settings are filled tracks with a thumb. Status marks in lists keep the emission-line grammar: a 2px vertical line plus a label, solid in its tone when on, half-height strong-rule when off. Perform state marks (PLAY, SYNC, MASTER, KEYLOCK, LOOP, SLIP) use the same grammar in Label at weight 600.

### Navigation (sidebar and top bar)
- **Sidebar sections:** caps headings (COLLECTION, PLAYLISTS, DEVICES, RECORDINGS) with a hairline plus glyph for creation where it applies.
- **Tree:** 26px rows indented 14px per level; kind marker lines left of the name (sticks solid green 2px, sets solid sodium 1px, smart lists dashed indigo 1px); a 1px strong-rule length bar under each list proportional to its playing time; counts right-aligned in bone-3. Selection is an 8px sodium tick at the left edge, never a fill.
- **Top bar:** three zones with more space between them than inside them: modes as a segmented control with a filled current mode; panel toggles as one group of slimmer keys; status at the right. Live values have fixed widths and are never cut off. RECORD is an action, not a panel: an ash plate with a bone-3 dot at rest, filled deep red with a running timer while recording, with the section's panel switch joined to it. Editing actions (undo) appear only in Prepare. Under 1360px the brand drops to its rail mark so live values keep their room.

### Track Table
- No boxes, no zebra fills. Caps header with a bone sort label and hairline arrow, 1px rule beneath.
- **Titles lead:** the title in bone at weight 500 (600 when selected); the artist and the figures BPM and TIME in bone 2; every other column recedes to bone 3, never fainter (4.5:1 on every surface, the selection fill included).
- **Figures:** tabular digits (the bundled fonts' default), right-aligned in fixed-width columns; text columns share the rest.
- **Selection:** an ash plate fill (raised ash under the pointer) and a 12x2px sodium tick at the row's left edge; ink lifts one step.
- **Loaded on a deck:** a 3px line in the deck colour at the row's left edge and the deck numeral beside it in Control rank, in the same colour (the numeral is the second channel). No sodium: the mark is the deck's own.
- **Harmonic fit** (vs. the selected track): a 1px green line in the KEY column, solid for same or adjacent key, dashed for two steps.
- **Analysis state:** nothing when analysed; half-height sodium when pending; struck deep red when the file is missing (ink drops to bone-3).

### Track Plate and Hot Cue Ruler
- Title (Heading), artist, then BPM / KEY / TIME readings (Label caps over Heading values).
- **Overview well:** 92px of band charcoal; three-band columns in violet, teal and sodium by default, which the user can change in Preferences; dimmed sodium phrase ticks every 16 bars in the top and bottom margins; cue lines (2px solid in cue colour with their letter, 1px dashed bone for memory cues); the cursor as doubled sodium.
- **Hot cue ruler:** letters A to H on a strong-rule baseline 18px from the bottom. A set cue is a 2px solid line in its colour with a bone letter and its time; an empty slot is a hollow outline in bone 3, and a set one is filled in its cue colour.
- **Beatgrid:** compact plates, struck while the grid is locked; LOCK goes dashed-active in sodium with a LOCKED caps legend.

### First-Start Guide
A band under the top bar on an empty library: four steps with hollow rings, a sodium ring on the current one, a green ring with a tick when done, a sentence that says what to do now, and the armed outline in sodium on the real control. Sample tracks and a test tone are offered in the band. Hidden in the kiosk. Never covers a control and never shows while a library exists unless asked for.

Once the decks come in, the band also offers the deck layout (SIMPLE / STANDARD / FULL as a segmented control; one key that steps through them below 1480px), so the hint keeps room for two lines.

### Controller Setup
A band in the first-start band's place (64px): CONTROLLER SETUP in caps with "Control n of m" over a dashed sodium progress line, the request in Body at weight 600 ("Move the tempo fader of deck 1 up and down") with what arrived beneath it in Control rank (sodium when the control is already used), and BACK / AGAIN / SKIP / CLOSE as compact plates. The matching control on screen gets the armed sodium outline of the guide. Never covers a control.

### Lighting Pads and Panel
- **Pads** (PADS in the LIGHTING view, the lighting panel): eight to a page in two rows of four, 4px apart. A pad is an ash plate with its number in Label caps top left (page.pad, such as 1.3), CHASE or FLASH in Label caps top right, the name in Control rank, and a 4px line of the look's light colours along the bottom (the light's own colour is content, like the stage preview, not a UI hue). An empty pad is a hollow strong-rule outline with EMPTY in bone 3. The pad in control is filled in the live tone; the pad open in the editor carries a 3px sodium selection tick at its left edge.
- **Order of who wins**: Label caps chained with ›, bone 3, the layer in control lit (sodium; fault red for blackout; bone for AUTO).
- **Lighting panel** (STRIP in Perform and Club): in the browser's place, never over or between the decks. Stage preview (from 1360px), page tabs and pads, faders (44px wide) for master and groups, then the keys: mode, DMX OUT, HOLD / RELEASE, the IN CONTROL line (or the lost link) and BLACKOUT / FLASH / STROBE / FOG. Every key is the 44px tall plate; pads at least 44px.

### Set Safety
Nothing here is a dialog: nothing may interrupt a set.
- **Bands under the top bar** (ground-band, panel radius, the first-start band's place): device lost (struck deep-red line, Heading in fault, SWITCH OUTPUT as the green lead), restore after a crash (doubled sodium line, it needs you; RESTORE DECKS green, START EMPTY plain), gig check (one result per check: solid green ready, dashed sodium look at it, struck fix it). The device-lost band is drawn over the waveform zone instead of pushing the layout down, so the decks do not move mid-set (no sound plays, so nothing under it is needed); the others take their place in the layout in the order restore, gig check, first start.
- **Audio line** at the right end of the status band: a 2px line in the status grammar (solid green running, dashed sodium for 20 s after a dropout, struck when lost, half strong-rule before audio starts), AUDIO in Label caps, then device, latency and dropouts in Control rank.
- **Lock key** in the status zone of the top bar: LOCK at rest, LOCKED filled sodium (the legend sodium already has for a locked grid). Under 1500 px it is a padlock that fills the same way. Controls the lock blocks take the disabled state.
- **Phase strip** under the waveform lanes (26px): per deck a strong-rule ruler of half a beat each way, a bone-3 hairline centre, the deck's 2px line where its beat falls (unsmoothed), tempo difference and ms in Control rank. In sync the centre becomes a 2px green line and the readout says IN SYNC in green. PRACTICE is a toggle in sodium.
- **Booth window**: the deck's ranks (hero, heading, body, label) multiplied by one zoom that fits a 640 x 360 canvas to the window, so the scale stays five sizes relative to each other; nothing else in the app scales this way. The last tenth of a playing track turns its time sodium.

### Device Line (signature)
- One stick, one line: 2px, inset 8px top and bottom, 64px rows (78px in the Devices section). In sync is solid green; pending is dashed sodium; error is struck with the detail text in deep red; unknown or unregistered is half-height.
- **Syncing:** the dashed sodium line (60% alpha) fills solid green from the bottom by progress, animated over the mode timing. Motion off sets it to 0ms.
- Name in Body at weight 600, detail in Label, capacity and formats in Label caps; compact SYNC / EJECT plates on the right.

### Set Flow
- 150px. Each position is a vertical line whose height is the track's energy (min 6px, max 92px), bone-2 at 2px, or sodium at 3px when selected; BPM and key are written beside it.
- Transitions between positions are 1px lines whose form is the fit: solid bone-3 good, dashed sodium caution, half unknown, struck deep red clash. A Label caps legend spells the code out.

### Deck Panel and Mixer (Perform)
- The deck's 2px line runs the full panel height on the left (doubled when master) beside its numeral in the deck colour.
- **Header:** numeral, title and artist in Body, BPM and remaining time as Hero readouts, KEY as a Heading, SYNC and MASTER toggles.
- **Overview** (34px, 26px compact) with played-portion veil, cue lines, doubled sodium position line and needle search; LOAD SELECTED when empty.
- **Stems row:** VOCAL / INST / DRUMS / BASS mute toggles (disabled until the track has stems, SEPARATING… while separating) and KEY LOCK.
- **Performance row:** hot cue pads A–D | E–H in two columns, LOOP (½, size, 2×), BEAT JUMP −4 / +4, Q and SLIP, round CUE and PLAY, the tempo fader with its range.
- **Tiers:** a deck has three visual ranks. First: PLAY, CUE (round, mark and legend grow with the key), SYNC (one rank larger label, same column as CUE and PLAY) and the faders (larger handle). Second: pads and loop keys on the full rows. Third, setup keys (SLIP, QUANTIZE, KEYLOCK, tempo range, pad mode, GRID, KEY SYNC, key shift): 24px and not growing with the rows. Windows narrower than 1300 px keep the same rows with tighter columns, tighter header gaps and a one-rank smaller BPM in the header (the platter keeps the Hero BPM), and still spell out KEY SYNC and MASTER; the compact deck is for narrower windows and four decks. Empty hot cues are hollow; set ones filled in the cue colour. On the kiosk the transport keys stay at 44 px.
- **Mixer:** per channel a knob column (TRIM, HI, MID, LOW, CH FX) beside the channel numeral, CUE toggle, a 4px level line (deep red at clip) and the channel fader. Knobs follow the control anatomy below; a changed knob's line takes its band wavelength; double-click resets. Each meter holds its peak.
- The crossfader is a single 2px bone line on a strong-rule ruler between A and B.
- **Layouts:** Simple, Standard and Full. Simple is a calm deck of about 25 controls on larger rows (34px base): pads A–D | E–H, a loop column (size, ½ / 2×, LOOP, jump − / +), SYNC / CUE / PLAY as on the full deck, the tempo fader; no platter and no setup tier, the header keeps the hero readouts. Groups (pads, loops, stems, effects, mixer, phase strip, setup keys) are hidden whole and then not built, never left in place invisible. The view menu holds the layout as a segmented control, the groups as on-keys and the saved layouts as rows with a hairline ×.

## Motion

Motion says that something changed and where; it never decorates. Four named timings and one curve, as tokens in `tokens.slint` (a test forbids any other timing in the `.slint` files):

| Token | Time | Used for |
|---|---|---|
| `t-press` | 90 ms | hover and press fills of keys, plates and tabs |
| `t-toggle` | 140 ms | a control changes state: switch thumb, disclosure arrow |
| `t-panel` | 180 ms | a part opens, a rail tick grows, a progress line fills, the grid meter moves |
| `t-mode` | 240 ms | the current work mode moves; a stick finishes syncing |

The curve is `ease`, cubic-bezier(0.2, 0, 0, 1): a quick start that settles without overshoot. Things that wait repeat on `t-pulse` (900 ms: the armed outline, the loading sweep); the tooltip delay is `t-tip` (500 ms).

**Reduce motion** (Preferences › View) sets every timing to 0 ms and stills the pulse, the loading sweep and the strobe preview. Slint does not report the system's reduced-motion setting, so the switch starts off.

**The Live Signal Rule.** Audio-linked visuals are never smoothed: meters, waveforms, playheads, the phase meter and the booth show the value as it is.

## Control anatomy

- **Knob:** a ring (hairline, strong rule; bone 3 under the pointer) with a 2px indicator line, bone at its default and in its tone once moved. A **value arc** on the ring in the knob's tone runs from the start of the travel, or, on a bipolar knob (trim, EQ, filter, FX parameter: the range spans 0), from a **centre mark** at the top, a 2px notch through the ring.
- **Fader:** a strong-rule track and a **cap** in raised ash with a hairline edge, two grip ridges (caps 16px and taller) and a bone **centre line**, the point the value is read at. A bipolar fader (tempo) has a **centre detent**: two bone 3 ticks beside the track at 0.
- **Drag readout:** while a knob or fader is held, its value sits on a small bone plate (Label rank, ground ink): above a knob, beside a fader cap on the deck's side. It floats over neighbours and takes no room. The waveform lanes show the scrub offset or nudge the same way at the playhead.
- **Meter:** the level as a filled line (live, attention near the top, fault at clip) and a 2px **peak-hold** tick at the highest recent peak (bone, attention or fault by level), unsmoothed.
- Sizes do not change: the anatomy fits inside the controls' existing boxes.

## Waveform options

- **Calm waveform** (Preferences › View, off by default): a crisp 1px bone playhead between two ground rims, beat lines faint (plain beats only as short ticks at the edges; bars and phrases thinner and fainter through the wave) and band colours drawn halfway towards their own grey, so neighbouring columns flicker less in hue while the bands keep their order of lightness.
- **Anchor:** the Perform lanes' playhead stands on the window's centre, the mixer's axis; past and future are drawn around it. The vertical lanes, Club and the booth keep their own centre.
- **Dragging a lane:** a paused deck scrubs 1:1, a playing deck is nudged by the drag's speed (a temporary bend); a grab cursor on hover.

## Do's and Don'ts

### Do:
- **Do** let surfaces and space make the groups, and keep one clearly dominant thing in each area.
- **Do** use the state table for every control, and keep a filled state plus its label for anything that is on.
- **Do** make BPM and remaining time the largest values on a deck, and keep play, cue, sync and faders the strongest controls.
- **Do** pair every deck colour with its numeral and every cue colour with its letter.
- **Do** keep status lines in lists, devices and Set Flow in the line grammar: solid live, dashed pending, half stale, struck error, double outranks.
- **Do** keep text at the type scale, and at 4.5:1 on the surface it sits on.
- **Do** raise touch targets to 44px on touch screens.
- **Do** animate only with the named timings and the one curve, and leave playheads, meters, waveforms and the phase meter unsmoothed.

### Don't:
- **Don't** draw an outline around every control, or stack panels inside panels.
- **Don't** use a dashed outline to mean selected or active.
- **Don't** use a colour for a second job, or show an idle control as lit.
- **Don't** use a font size outside the scale, or text below 11px.
- **Don't** use gradients, blur or glow, except the soft glow of the platter ring and playhead.
- **Don't** copy the look, layout details, names or colours of other DJ products.
