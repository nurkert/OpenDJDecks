# Roadmap

What comes next for OpenDJDecks, in the order it will be built. Numbers in brackets stay the same as items move, so they can be referred to in issues and commits.

The priorities come from 30 simulated expert interviews (UI and UX designers, DJs, lighting operators) held in November 2026, each point checked against the code. Simulated interviews give hypotheses, so feedback from real users can change this order.

## How each step is built

- One step per pull request, so a release can stop after any step.
- A step is done when its "Done when" is true, tests cover it, English and German text and the guides are updated, and screenshots or clips show the new state.
- Sources and naming follow [CONTRIBUTING.md](../CONTRIBUTING.md).
- Anything that needs hardware stays open until it has been confirmed on that hardware.

## 0.7.0: readable and safe at a gig

### Step 1: stem model setup

- [ ] **[S1] Guided model download.** Asks before the one-time download, shows progress, explains errors with retry and help, and can start again from nothing in Preferences. Done when an interrupted or damaged download can always be repaired from Preferences, and the guide explains how.

### Step 2: a shared language for the interface

These come first because every later screen builds on them.

- [ ] **[4] Plain words.** One glossary applied everywhere: full words such as Keylock where space allows, Stems on the deck, LIGHT and LIGHTING told apart, side rails spelled out. Done when each concept has one name in the app and the guides.
- [ ] **[2] One state language.** Off, selected, on, running, disabled and focused as documented tokens, used in Perform, Lighting, FX and Preferences. Each accent colour has one job. Done when dashed outlines no longer mean "selected" and idle toggles never look lit.
- [ ] **[1] Legibility.** A minimum of 11 px for labels read live and 12 px on the Raspberry Pi, with contrast of at least 4.5:1 measured on each surface. Done when no label is below the floor and a test enforces it.
- [ ] **[3] Gig preset.** High contrast, colour-blind palette and interface scale can be found near the top bar and in the first run, and combine into one preset. Scale applies without a restart. Done when all of it is reachable in one step.

### Step 3: orientation

- [ ] **[5] Top bar in zones.** Modes, panels and status are visibly separate. Recording is red with a timer and separate from the panel toggle. Status text is no longer cut off. Done when each element in the bar has a clear kind and state.
- [ ] **[7] Guided first start.** A checklist that ticks itself: add music, wait for analysis, load two tracks, play and sync, with sample tracks and an audio test tone. Done when a new user reaches a first mix without the guides.

### Step 4: nothing goes wrong during a set

- [ ] **[10] Audio health and gig check.** A permanent status with device, latency and dropouts, a clear alert when the device is lost, device change without a restart, and a check before the gig for outputs, controller, disk and missing files. Done when unplugging the audio interface is shown at once and can be recovered from.
- [ ] **[9] Restore after a crash.** Deck state is saved continuously and offered back after an abnormal exit, and a running recording survives. Done when killing the app mid-set restores tracks and positions and keeps the recording.
- [ ] **[11] Performance lock.** Blocks loading onto a playing deck, deleting and grid editing. Keeps the computer awake and pauses background work while decks play. Done when none of these can happen by accident while locked.
- [ ] **[12] Phase and tempo between decks.** A phase meter and tempo difference in Perform with a clear in-sync state, and an option to practise without sync. Done when manual beatmatching can be judged from the screen.

### Step 5: a lighting rig you can trust

- [ ] **[14] Patch table.** Universe, address, mode and channel count are editable, with bulk add, collision display and identify. Done when a real rig can be patched without editing files.
- [ ] **[16] Output status and failsafe.** Status per universe, also visible in Perform, a live channel view, USB reconnect, and a chosen behaviour for a lost link or crash. Done when pulling the interface is shown and recovers by itself.
- [ ] **[19] Safety limits.** A strobe rate cap with a safe default, lockout for fog and lasers, level caps per group, and a PIN that keeps guests out of setup. Done when automatic looks cannot exceed the limits.

### Before releasing 0.7.0

- [ ] **[H1] Controller mappings** confirmed on a real DDJ-FLX10, with unconfirmed entries resolved.
- [ ] **[H2] Club player USB** confirmed on real players, including the WAV conversion for older ones.
- [ ] **[H3] Several sticks at once** and eject confirmed on physical sticks.

## 0.8.0: adapts to the person using it

- [ ] **[6] Simple, Standard and Full layouts,** with saved layouts. Whole groups can be hidden, and a Simple deck has about 25 controls.
- [ ] **[8] Three tiers in Perform.** Play, cue, sync and faders dominate, loops and pads form a middle tier, setup keys recede, and empty hot cues are drawn hollow. Needs a decision on the "outlined only" rule in `DESIGN.md` first.
- [ ] **[13] Confirmed controller list.** A published matrix with status, more mappings for entry-level and battle-style controllers, and a guided setup for unlisted ones.
- [ ] **[15] Own looks and chases** in a pad grid with live colour preview, flash or latch, pages and group masters, mapped to pads and MIDI.
- [ ] **[17] Full lighting controls in Perform and Club,** legible in the dark and sized for touch.
- [ ] **[18] AUTO you can steer.** Palettes, an editable phrase-to-look table, hold and a clear order of AUTO, overrides and release.
- [ ] **[20] Connecting interfaces.** Several universes, port picker, Art-Net node discovery, test pattern and a short guided setup.

## 0.9.0 and later

### Interface

- [ ] **[21]** Lighting split into Setup and Show, with a large blackout and an output test.
- [ ] **[22]** Clearer browser rails, double-click behaviour and Prepare areas.
- [ ] **[23]** Help overlay that names every control, also on touch.
- [ ] **[24]** Touch layout beyond the kiosk, covering the whole deck.
- [ ] **[25]** Full keyboard focus, readable screen reader names, and no meaning carried by hue alone.

### DJ

- [ ] **[26]** Visible browser filters, more room for the list, fuzzy search and type-then-Enter to load.
- [ ] **[27]** Recording for release: editable tracklist, markers, separate tracks, loudness and clip display.
- [ ] **[28]** Stick format as a labelled badge, stem state per track, and a one-press emergency stick.
- [ ] **[29]** Queue in Perform, requests, do-not-play tags, and auto-play with dead-air warning.
- [ ] **[30]** EQ kills, crossfader curves on the mixer, cut-in and reverse, split cue.
- [ ] **[31]** Hot cue edits from Perform, drop markers, phrase-quantized jumps, half and double-time sync.

### Lighting

- [ ] **[32]** Stage as an editor, with fixtures placed on a floor plan and deeper fixture programming.
- [ ] **[33]** Editable cue list and a cue lane on the waveform.
- [ ] **[34]** A separate lighting operator: default MIDI layout and second controller, later a networked instance and OSC.
- [ ] **[35]** Named rig and show files with venue profiles.
- [ ] **[36]** DJ events as triggers, and music sources beyond the decks.
- [ ] **[37]** Unattended mode for a fixed venue.
- [ ] **[38]** Pixel mapping, only if real users ask for it.

### Connections and hardware

- [ ] **[39]** Controller settings and jog displays, built only from USB captures on the owner's own device.
- [ ] **[40]** OneLibrary sticks, once it is clear whether the format needs a key.
- [ ] **[41]** Network connection to club players, read-only first and limited to the user's own network.
- [ ] **[42]** Signed updates, so that a release can be verified independently of GitHub.
