# Roadmap

What comes next for OpenDJDecks, in the order it will be built. Numbers in brackets stay the same as items move, so they can be referred to in issues and commits.

The priorities come from 80 simulated expert interviews (UI and UX designers, DJs mostly from electronic music, lighting operators) held in November 2026, each point checked against the code. Simulated interviews give hypotheses, so feedback from real users can change this order.

Two findings set the order. People judge the interface by its hierarchy and finish first, so the design work comes early. And nobody moves their gigs to new software on features alone: they want proof that it holds up, so that proof is part of the plan.

## How each step is built

- One step per pull request, so a release can stop after any step.
- A step is done when its "Done when" is true, tests cover it, English and German text and the guides are updated, and screenshots or clips show the new state.
- Sources and naming follow [CONTRIBUTING.md](../CONTRIBUTING.md).
- Anything that needs hardware stays open until it has been confirmed on that hardware.

## 0.7.0: beautiful, readable and safe at a gig

### Step 1: stem model setup

- [x] **[S1] Guided model download.** Asks before the one-time download, shows progress, explains errors with retry and help, and can start again from nothing in Preferences. Done when an interrupted or damaged download can always be repaired from Preferences, and the guide explains how.

### Step 2: a design language worth looking at

These come first because every later screen builds on them.

- [x] **[44] Surfaces and spacing.** Two or three visible surface levels and one spacing scale, so groups separate without outlines around everything. Keeps the dark base, the deck colours and the platter ring. Needs `DESIGN.md` updated first, because it currently requires outlined plates, square corners and dashed active states. Done when the rules are rewritten and applied to Perform.
- [x] **[4] Plain words.** One glossary applied everywhere: full words such as Keylock where space allows, Stems on the deck, LIGHT and LIGHTING told apart, side rails spelled out. Done when each concept has one name in the app and the guides.
- [x] **[2] One state language.** Off, selected, on, armed, running, disabled and focused as documented tokens, used in Perform, Lighting, FX and Preferences, with a visible focus ring. Each accent colour has one job and colour is never the only cue. Done when dashed outlines no longer mean "selected" and idle toggles never look lit.
- [x] **[43] Type scale.** About five sizes and two weights, fixed-width figures for changing numbers, and BPM and remaining time as the largest values on a deck. Done when hard-coded font sizes are gone.
- [x] **[1] Legibility.** A minimum of 11 px for labels and 12 px for anything read live or touched, with contrast of at least 4.5:1 measured on each surface. The text size setting also scales the deck. Done when no label is below the floor and a test enforces it.
- [x] **[3] Gig preset.** High contrast, colour-blind palette and interface scale can be found near the top bar and in the first run, and combine into one preset. Scale applies without a restart. Done when all of it is reachable in one step.

### Step 3: hierarchy and orientation

- [x] **[5] Top bar in zones.** Modes, panels and status are visibly separate. Recording is red with a timer. Live values are never cut off, and editing actions appear only where editing happens. Done when each element in the bar has a clear kind and state.
- [x] **[8] Three tiers in Perform.** Play, cue, sync and faders dominate, loops and pads form a middle tier, setup keys recede, and empty hot cues are drawn hollow. Done when a deck's state can be read at a glance from arm's length.
- [x] **[7] Guided first start.** A checklist that ticks itself and highlights the real control: add music, wait for analysis, load two tracks, play and sync, with sample tracks and an audio test tone. Done when a new user reaches a first mix without the guides.

### Step 4: nothing goes wrong during a set

- [x] **[10] Audio health and gig check.** A permanent status with device, latency and dropouts, a clear alert when the device is lost, device change without a restart, and a check before the gig for outputs, controller, disk and missing files. Done when unplugging the audio interface is shown at once and can be recovered from.
- [x] **[9] Restore after a crash.** Deck state is saved continuously and offered back after an abnormal exit, and a running recording survives. Done when killing the app mid-set restores tracks and positions and keeps the recording.
- [x] **[11] Performance lock.** Blocks loading onto a playing deck, deleting and grid editing. Keeps the computer awake and pauses background work while decks play. Done when none of these can happen by accident while locked.
- [x] **[12] Phase and tempo between decks.** A phase meter and tempo difference in Perform with a clear in-sync state, and an option to practise without sync. Done when manual beatmatching can be judged from the screen.
- [x] **[58] Booth view.** A large view with two big waveforms and large BPM, key, remaining time and phrase countdown. Done when it is readable from a few metres away.

### Step 5: proof that it holds up

- [x] **[51] Published long-run results.** Sets of 4 to 8 hours with two and four decks, stems, effects and recording on an ordinary laptop, with dropout counts and latency, published where DJs read them. Done when the results are repeatable and linked from the website. Four-hour runs with two and four decks are published in the long-run results guide, with the logs and the command to repeat them. They ran on a computer in daily use; a run on a quiet computer and eight-hour runs are still to come.
- [x] **[52] Hardware status per model and control.** Verified, partial or untested for every mapped control, with a self-test for controls and LEDs before a gig. Done when the list is public.
- [x] **[53] Stick compatibility and check.** A list per player model covering cues, grids, memory points and playlists, and a check of a stick before a gig with a result per track. Done when a stick can be declared ready.

### Step 6: a lighting rig you can trust

- [x] **[14] Patch table.** Universe, address, mode and channel count are editable, with bulk add, collision display and identify. Done when a real rig can be patched without editing files.
- [x] **[16] Output status and failsafe.** Status per universe, also visible in Perform, a live channel view, USB reconnect, and a chosen behaviour for a lost link or crash. Done when pulling the interface is shown and recovers by itself. Loss and recovery are tested with a simulated link; pulling a real USB-DMX interface has not been tried yet.
- [x] **[19] Safety limits.** A strobe rate cap with a safe default, lockout for fog and lasers, level caps per group, and a PIN that keeps guests out of setup. Done when automatic looks cannot exceed the limits.

### Hardware checks

These need the real devices. They do not hold up any release: until one is confirmed, the release notes and guides say it is unconfirmed.

- [ ] **[H1] Controller mappings** confirmed on a real DDJ-FLX10, with unconfirmed entries resolved.
- [ ] **[H2] Club player USB** confirmed on real players, including the WAV conversion for older ones.
- [ ] **[H3] Several sticks at once** and eject confirmed on physical sticks.

## 0.8.0: what electronic DJs need, and it adapts to you

### For the mix

- [x] **[54] A library import people can trust.** A preview of what will carry over before anything is written, then a before and after count, a list of what did not transfer, and undo. Covers documented export formats. No automatic reading of other software's private files. Done for the library XML and Music / iTunes `Library.xml` exports; undo restores a copy taken right before the import and is offered while the result window is open. The preferences import now asks for the folder instead of looking for it.
- [x] **[55] A mixer for bass swaps.** A kill per EQ band, a one-tap and optionally quantised swap between decks, larger EQ and filter with values and double-tap reset, and clip display. Done in the app (the knobs are larger at desktop size only; the mid tier keeps its 16 px knobs to leave the decks their width) and as the mapping actions `mixer.eq_kill` and `eq_swap`; the swap lands within one audio block of the beat or bar. Not tried on a hardware controller or a real touch screen yet (double-tap is the Slint double-click on touch), and no shipped mapping uses the new actions.
- [x] **[56] Phrase-aligned blends.** Bars to the next phrase, breakdown or drop on each deck, phrase-snapped sync, start on the next phrase, and the incoming track drawn over the playing waveform. Done in the app: DROP / BREAK bars on each platter, a right click on PLAY (or `deck.phrase_start`) starts a cued deck on the master's next phrase within one audio block, the PHRASE sync mode jumps a syncing deck whole beats into the same place in its phrase, and the master's lane draws the incoming track's outline. Tested with generated tracks only; not tried at a gig, by ear on a real set, or from a hardware controller, and no shipped mapping uses `deck.phrase_start` yet.
- [x] **[57] High tempos.** Detection up to and beyond 200 BPM without halving, a visible active range, and one-press half and double on the deck. Done: a 150–300 BPM range; generated click, kick, kick-and-snare and house loops at 170, 200, 220, 250 and 280 BPM read as played (within 0.05 BPM) in every range that holds them, and the default 70–180 folds 200 and 220 to 100 and 110. The active range shows in the Prepare player's INFO tab and beside the analysis progress; ÷2 / ×2 sit on the full deck's BPM readout (the GRID panel on narrower decks) and as `deck.bpm_half` / `deck.bpm_double`. Not yet checked on a collection of real records above 200 BPM.
- [x] **[59] Loop roll and loop capture,** quantised from 1/32 beat to 4 beats and shown on the key and waveform. Done as the ROLL pad mode (slip-style: the track goes on underneath and resumes on release, a line on the waveform shows where) with CAPTURE in its mode bar, which stores the loop just heard as a hot loop on the first free pad (a memory loop when the bank is full); also `deck.loop_capture` and the pad mode `loop_roll` in mappings. Not tried on hardware pads yet.
- [x] **[60] Stems ready before the gig.** Separate a whole set ahead of time, show stem state per track and on each key, warn on load, and offer solo and keep-only-this. Done in the app: **Separate stems for the whole list** (row menu, File menu, `⌥⇧⌘P`) queues every track of a playlist or set without stems; an optional STEMS column shows ready, queued, separating and failed (also for tracks a deck separates itself); the stem keys show QUEUED and the progress, and NO STEMS in sodium when a track without stems comes in where stems were in use, with the fix on the status line; right click solos a stem (back as it was on a second right click), ⇧-click keeps only it; also `deck.stem_solo` (held) and `deck.stem_keep`. A deck holding a track the batch finishes takes its stems without a gap. Tested with generated tracks and a mocked model; a whole real set has not been separated end to end for this step, and solo / keep-only have not been tried from a hardware controller.
- [x] **[61] Effects for drops.** Larger controls with visible intensity, a panic off, an XY control, build-ups locked to 8, 16 or 32 bars, and tails after release. Done in the FX panel, which keeps its height: the level as a large percent over a bar you drag; ALL FX OFF (also `⇧Esc` and `fx_panic`) fades every Tempo FX, tail, exit and Hold FX and the channel FX knobs to dry within 5 ms; XY turns a unit into one surface (across the parameter, up the level, played while held); BUILD 8 / 16 / 32 rises to the next bar line of that length on the grid and lets go for the drop (also `tempo_fx.build_up`); TAIL off / echo / reverb rings out after release for effects without a tail of their own. Engine tests render the bar lock, the tails and the panic; not judged by ear at a gig, not tried from a hardware controller, and no shipped mapping uses the new actions yet. With the panel open at 1440×900 the mixer's CH FX caption is clipped as before this step (the panel's height is unchanged).
- [x] **[62] Key relation on the deck,** coloured against the other playing decks, and a confidence value for key and grid analysis with a one-button fix. Done: the deck's KEY is underlined against the other playing decks with the key you hear (key shift, and the tempo without keylock): solid green fits, dashed sodium an energy change, struck deep red a clash, in words on hover; DESIGN.md's own line grammar and wavelengths, no product key colours. The analysis stores how sure it is of key (lead over the runner-up) and grid (beats on a hit, the half / double call) in schema 4; Prepare's INFO tab shows both with USE for the likely key or tempo and ANALYSE AGAIN, and an unsure key shows its percent under the deck's KEY. Tracks analysed before this version show no value until analysed again. The confidence was checked on generated signals only, not yet against a collection of real records.

### For the person


- [ ] **[6] Simple, Standard and Full layouts,** with saved layouts. Whole groups can be hidden, and a Simple deck has about 25 controls.
- [ ] **[13] Confirmed controller list.** A published matrix with status, more mappings for entry-level and battle-style controllers, and a guided setup for unlisted ones.
- [ ] **[15] Own looks and chases** in a pad grid with live colour preview, flash or latch, pages and group masters, mapped to pads and MIDI.
- [ ] **[17] Full lighting controls in Perform and Club,** legible in the dark and sized for touch.
- [ ] **[18] AUTO you can steer.** Palettes, an editable phrase-to-look table, hold and a clear order of AUTO, overrides and release.
- [ ] **[20] Connecting interfaces.** Several universes, port picker, Art-Net node discovery, test pattern and a short guided setup.

## 0.9.0: the rest of the craft

### Interface

- [ ] **[45]** A motion language: named timings and one curve for press, toggle, panel and mode change, with audio-linked visuals left unsmoothed.
- [ ] **[46]** Knobs, faders and meters that feel physical: value arcs, centre marks, readouts while dragging.
- [ ] **[47]** Set Flow redrawn as a clear chart, with transition warnings between tracks.
- [ ] **[48]** Preferences and the stick screen in the new design, with live previews.
- [ ] **[49]** A calmer library table: titles lead, aligned figures, clear loaded and selected states.
- [ ] **[50]** A calmer waveform option with a crisp playhead and faint beat lines.
- [ ] **[21]** Lighting split into Setup and Show, with a large blackout and an output test.
- [ ] **[22]** Clearer browser rails, double-click behaviour and Prepare areas.
- [ ] **[23]** Help overlay that names every control, also on touch.
- [ ] **[24]** Touch layout beyond the kiosk, covering the whole deck.
- [ ] **[25]** Full keyboard focus, readable screen reader names, and no meaning carried by hue alone.

### DJ

- [ ] **[26]** Visible browser filters, more room for the list, fuzzy search and type-then-Enter to load.
- [ ] **[27]** Recording for release: editable tracklist, chapter markers, separate voice track, loudness and clip display.
- [ ] **[28]** Stick format as a labelled badge, stem state per track, and a one-press emergency stick.
- [ ] **[29]** Queue in Perform, requests, do-not-play tags, and auto-play with dead-air warning.
- [ ] **[30]** EQ kills, crossfader curves on the mixer, cut-in and reverse, split cue.
- [ ] **[31]** Hot cue edits from Perform, drop markers, phrase-quantized jumps.
- [ ] **[63]** Search at request speed, a played-tonight mark and bulk tag editing.
- [ ] **[64]** Larger cue pads with names, per-cue trigger behaviour, and a sampler that stays in view.
- [ ] **[65]** The planned set in Perform: next track, transition note, and where you stand against the plan.
- [ ] **[66]** Stems you can audition, and your own stem files as deck parts.
- [ ] **[67]** Output routing, an external-mixer mode, split cue on one interface, and clock sync.

### Lighting

- [ ] **[32]** Stage as an editor, with fixtures placed on a floor plan and deeper fixture programming.
- [ ] **[33]** Editable cue list and a cue lane on the waveform.
- [ ] **[34]** A separate lighting operator: default MIDI layout and second controller, later a networked instance and OSC.
- [ ] **[35]** Named rig and show files with venue profiles.
- [ ] **[36]** DJ events as triggers, and music sources beyond the decks.
- [ ] **[37]** Unattended mode for a fixed venue.
- [ ] **[38]** Pixel mapping, only if real users ask for it.

## 0.10.0: connections and hardware

- [ ] **[68]** Signed desktop builds, stated minimum requirements, and a low-CPU mode.
- [ ] **[39]** Controller settings and jog displays, built only from USB captures on the owner's own device.
- [ ] **[40]** OneLibrary sticks, once it is clear whether the format needs a key.
- [ ] **[41]** Network connection to club players, read-only first and limited to the user's own network.
- [ ] **[42]** Signed updates, so that a release can be verified independently of GitHub.

## Beta

OpenDJDecks stays in beta through this whole roadmap. Finishing it does not end the beta. Items [51], [52], [53] and [H1] to [H3] and real gig reports from other DJs are what that decision will be based on later.
