---
title: The basics of Perform
description: Decks, layouts, sync, keylock, tempo, loops and beat jump, and every pad mode on the deck.
keywords: [booth, booth view, second screen, distance, large, phase, phase meter, beatmatching, beatmatch, practice, tempo difference, in sync, perform, deck, decks, play, cue, sync, beat sync, master, keylock, key lock, master tempo, tempo, tempo fader, pitch, bend, nudge, jog, vinyl, scratch, slip, quantize, beat jump, pads, pad mode, hot cue pad, key shift, key sync, key clash, harmonic mixing, compatible key, slicer, four decks, 4 decks, layout, vertical, browse, waveform, zoom, simple, standard, full, deck layout, hide, groups, saved layout, queue, up next, request, requests, auto-play, autoplay, auto mix, do not play, load next]
features: [perform-queue, perform-requests, perform-auto-play, deck-key-relation, deck-count-layers, deck-play-cue, deck-jog, deck-tempo-fader, deck-key-lock, deck-key-shift, deck-key-sync, deck-beat-sync, deck-quantize, deck-slip, deck-reverse, deck-censor, deck-cut-in, deck-beat-jump, deck-beat-jump-size, deck-needle-search, deck-search-buttons, pad-mode-hot-cue, pad-mode-beat-loop, pad-mode-beat-jump, pad-mode-keyboard, pad-mode-key-shift, pad-mode-slicer, pad-mode-memory-cue, pad-mode-timed-censor, pad-mode-phrase, pad-mode-cue-edit, deck-layout-platter, perform-set-plan, pad-led-feedback, deck-waveform-zoom, deck-waveform-overview-detail, deck-header-readouts, deck-platter-readout, deck-waveform-zone, shell-perform-layouts, perform-view-menu, deck-layouts, deck-layout-groups, deck-layout-saved, shell-panel-toggles, shell-splitters, deck-phase-meter, perform-booth-view, deck-centre-group, deck-waveform-calm, deck-waveform-anchor, deck-waveform-drag, shell-help-overlay, shell-touch-layout, shell-compact-decks-fill, deck-edit-in-prepare]
---

Perform is the live mode: the decks, the mixer and the browser in one view.

![Perform with two decks, mixer and browser](../img/perform.png)

## Load and play

Double-click a track in the browser, drag it onto a deck, or use `⇧←` and `⇧→`. **Play** and **CUE** work as on a player: CUE at rest sets the cue point, CUE while playing returns to it, holding CUE plays from it until you let go.

## The queue, requests and auto-play

**QUEUE** in the panel strip beside the list shows what comes next (`View › Queue` opens it too):

![Perform with the queue: tracks, a request, an open request and a do-not-play track, auto-play mixing](../img/perform-queue.png)

- **ADD** (or `⌥⇧⌘Q`, or *Add to the queue* in the row menu) adds the tracks selected in the browser. Each row has keys to move it up or down and **×** to remove it. The queue is kept with the library and survives a restart.
- **LOAD NEXT** (or `⌥⇧⌘N`) loads the next track of the queue to a free deck: one that is empty or has played its track to the end.
- **Requests**: type what someone asked for into the request field (*Artist, Title*) and press Return. If a track of the library matches every word, the request is queued with it; if not, it stays as a note (*REQUEST · NOT IN THE LIBRARY*) until you select the track in the browser and press the tick key on its row.
- **Do not play**: a track tagged so (see [Library](library.md#tags-rating-and-colour)) shows struck red in the queue and in the list, LOAD NEXT and auto-play pass over it, and loading it anyway brings up a red warning over the foot of the list with **UNLOAD** and **KEEP**.

**AUTO-PLAY** plays the queue by itself. It keeps a deck cued with the next track, starts the queue when nothing plays, and when the playing track is that many beats from its end it starts the cued deck and mixes into it: over **CUT** (at the very end), **8**, **16** or **32** beats under **MIX BEATS**. The mix crosses the two channel faders (or moves the crossfader when the two decks sit on its two sides) and turns on SYNC for the incoming deck when both tracks have a beatgrid. While it mixes the panel says so in yellow. Turning auto-play off in the middle of a mix finishes the levels at once. The outgoing channel's fader stays down afterwards; push it up before you play that deck by hand.

## Deck readouts

The deck header shows the number, the artwork, title and artist, **BPM**, the remaining or elapsed time, the **KEY** and the sync state. The platter below shows the tempo, the bar counter and the current phrase with a countdown, and under the tempo the bars to the next drop and break ([Phrases](phrases.md#mix-on-the-phrase)). **½** and **2×** beside **BPM** halve or double the track's grid ([Beatgrids](beatgrid.md#fast-music-and-half-or-double-on-the-deck)).

The narrow waveform in each deck is the whole track with hot cues and phrases. Click it to jump there (needle search); on a controller the touch strip does the same.

Above the decks, the wide zone shows one scrolling waveform per deck. Turn the wheel over a lane to zoom that deck, double-click to reset, and use the keys at the left for all decks: one column with zoom in, zoom out, centre the zoom, then a smaller and a larger waveform zone. When the lanes are too short for all five, the last keys move behind **⋯**. The playhead stands on the window's centre, in line with the mixer below, so the past side of a lane is a little narrower than the future side.

### Dragging a waveform

Drag a lane with the mouse (the pointer turns into a hand over a lane with a track):

- **Paused deck:** the waveform follows the pointer one to one at the current zoom, and the track stays where you put it. Drag left to move later in the track.
- **Playing deck:** nothing jumps. The speed of your drag bends the tempo for a moment, like a hand on a platter, so you can push the beats into line; a still hand lets the deck run, and on release it plays at its tempo again. With **SLIP** on, release returns to where the track would have been.

While you drag, a small readout at the playhead shows the offset in seconds (paused) or the bend in percent (playing). A click without movement does what it did before, and the performance lock blocks neither.

### Calm waveform

**Preferences › View › Calm waveform** draws the large waveforms more quietly: a crisp, thin playhead between two dark edges, faint beat lines (plain beats only as short ticks at the edges) and softer band colours. It is off by default.

## Fix the analysis while you play

Notice in the middle of a set that a deck's phrases, grid, cues or key are wrong? Fix them without losing your place:

- **EDIT** in the deck header (under **MASTER**; beside the title on compact decks) puts the deck's track in the Prepare player at the deck's position, with the **PHRASE** tab open (the grid editor when the deck's **GRID** panel is open). The decks play on: Prepare only shows the same track, nothing is loaded again. `⌥⇧⌘D` does it for deck 1 (`⌥⇧⌘J`, `F`, `H` for decks 2 to 4), `deck.edit_in_prepare` on a controller.
- **Right-click** the deck's waveform, its phrase strip or its **KEY** for the menu: *Edit phrases*, *Edit beatgrid*, *Edit cues*, *Edit key*, *Edit gain*, *Edit track info* and *Edit stems* open Prepare with that editor. Below them are the quick fixes that stay in Perform: the downbeat one beat earlier or later, the phrase boundary nearest the playhead one bar earlier or later, and *Use the likely key* when the analysis was unsure ([Key against the other decks](#key-against-the-other-decks)). Each quick fix is one undo step.
- Every edit you make in Prepare, and every undo, reaches the loaded decks at once: the phrase countdowns, phrase-aligned blends, the light show and the waveforms use the corrected data while the track plays on.
- **BACK TO PERFORM** in the top bar, `Esc`, `⌥⌘2` or the **PERFORM** tab take you back to the decks exactly as you left them.

## The centre of the deck

![Perform in a large window: the platter centred with its keys, SYNC, CUE and PLAY on the same axis](../img/perform-large.png)

The platter sits in the middle of the deck's rows, with **REV**, **SLIP**, **QUANTIZE**, **KEYLOCK**, the tempo range and **CUT IN** as one block on its inner side. Their inner edges follow the ring, one key gap away, and **SYNC**, **CUE** and **PLAY** stand on the same axis. In a larger window the platter grows up to a limit and the keys grow with it; any room left goes above and below, never between the keys. On the player screens of Club mode, the ◀◀ / ◀ CALL and ▶▶ / CALL ▶ keys wrap the jog the same way.

## Knobs, faders and meters

A knob shows its value as an arc on its ring. Knobs that turn both ways from a middle (trim, EQ, filter, the FX parameter) have a small mark at the top and draw the arc from there. While you drag a knob, its value stands above it; while you drag the tempo fader, the tempo stands beside the cap, towards the deck. The tempo fader has a centre detent mark at 0, and every fader cap has a centre line to read it by. See [The mixer](mixer.md#knobs-faders-and-meters) for the meters.

## Layout and views

The view button at the top of Perform opens a menu with a picture of each view: **2 Decks** or **4 Decks**, each **horizontal** or **vertical**, and **Browse**, which keeps the decks small and gives the browser the room. Shortcuts: `⌥⌘V` for vertical waveforms and `⌥⌘B` for Browse. A four-deck controller switches to four decks automatically.

The top bar has three zones. On the left are the work modes, in the middle what the current mode shows, and on the right what is live: the master tempo, recording, the controller and the master out. **FX**, **PREVIEW**, **SAMPLER** and **MIXER** show or hide panels, plus **CLUB** and **LIGHT STRIP**. **RECORD** starts a recording and turns red with the running time; the small key next to it shows or hides the recording section. **UNDO** appears in Prepare, where you edit. Drag the separator bars between the areas to resize them, double-click to reset.

## Help on every control

Press `F12`, or `?` where no shortcut takes it, or the **?** key at the right of the top bar: the help names every control on screen. Each one gets a small number on its corner, and the list at the side gives each number its name and its shortcut. The numbers never cover each other; where a control is too small for its number on top, the number sits beside it with a short line to the control. Click (or tap) a control while the help is open and the panel says what it does, from the same text as its tooltip, instead of pressing it. The arrow keys move through the list, the arrow key at the top of the list moves it to the other side, and `F12`, `?`, `Esc` or **CLOSE** end the help. In Perform on an English keyboard `?` taps FX 2, so use `F12` there.

![The help over Perform: numbered controls and the list with names and shortcuts](../img/help-overlay.png)

## Touch layout

**Preferences › View › Touch** makes every control of the decks, the mixer, the pads, the effects, the browser and the lighting at least 44 px, for a touch screen or a tablet beside the controller. The first touch on a screen offers it once in a band under the top bar (**TOUCH LAYOUT** or **NOT NOW**). The decks become touch decks: the read-outs, part keys and transport on top, the hot cue pads A–D over E–H, a row with beat jump, loop size and **LOOP**, and the deck's own waveform in the rest of the column. The mixer puts each knob beside its legend, which is also the kill key; the effect units stand one under the other; the browser has 44 px rows and leaves out the cover column; the panes keep their sizes (a thin grip is no target). On a window narrower than about 1060 px the touch decks leave the mixer to the controller, as the Raspberry Pi kiosk does. Prepare keeps its desktop keys.

![Perform with the touch layout](../img/touch-layout.png)

The touch layout has not been tried on a real touch screen yet.

## Small windows

In a small window (or at a large interface scale) the decks are compact: the read-outs on top, then key, sync and master, the part keys, the transport, the hot cue pads and the loop row, standing in the middle of the room the mixer leaves, so nothing stays empty under them.

![Perform in a 960 × 640 window](../img/perform-compact.png)

## Deck layouts

![Perform with the Simple layout: large pads, loop, SYNC, CUE, PLAY and the tempo fader](../img/deck-simple.png)

How much each deck shows is up to you. Choose in the view menu at the top of Perform, under **View › Deck Layout**, or in the first-start guide:

- **Simple** is a calm deck of about 25 large controls: hot cue pads A–H, the loop (**½**, **2×**, **LOOP**) and beat jump, **SYNC**, **CUE** and **PLAY**, the tempo fader, the stems and **MASTER**. The setup keys, the platter, the phase strip and the effects are left out, and the pads always play hot cues. BPM and time stay large in the header.
- **Standard** is the deck as described on this page.
- **Full** shows everything, with the Tempo FX panel open.

Under **SHOW** in the same menu, whole groups can be hidden or shown again: **PADS**, **LOOPS**, **STEMS**, **FX** (the Tempo FX panel and its key, the CH FX knobs), **MIXER** (the mixer column, the same as the top bar's **MIXER**), **PHASE** (the phase strip) and **SETUP KEYS** (REV, SLIP, QUANTIZE, KEYLOCK, the tempo range, CUT IN, GRID, KEY SYNC, key shift and ½ / 2× on the BPM) and **PLATTER** (the platter with its key block: hidden, the pads take its room, so they are much larger and show the cue names in full). A hidden group is gone completely, so the keyboard cannot land on it either. The pad mode stays with the pads.

To keep a layout, type a name under **SAVED LAYOUTS** and press **SAVE**. A press on the name brings it back, **×** deletes it. A saved layout remembers the deck layout, the hidden groups, the mixer column and the Tempo FX panel, and stays with your settings. Saving under a name that exists replaces it.

The kiosk on the Raspberry Pi keeps its own deck, and Club mode its player screens, whatever the layout. Simple and Full have not been tried on a touch screen yet.

## The set plan

![Perform with the plan over the list: the track on air, the next one with its note, and how far behind the plan the set is](../img/perform-plan.png)

The last set you opened in the browser is your plan. In Perform a line over the list follows it:

- **PLAN** and the set's name, then **ON AIR 4 / 10**: where the track you hear stands in the set.
- **NEXT** with the next track's title, artist, tempo, key and length.
- **NOTE**: the transition note you wrote for the change from the track on air to the next one (see [Preparing sets](preparing-sets.md#notes-and-backups)).
- At the right, where you stand: **ON TIME** within a minute of the plan, else **BEHIND 1:20** or **AHEAD 0:45**, counted from the set's **PLANNED START** and the track lengths, and **1 TRACK BEHIND** when the plan expects another position by now. Without a planned start it says **NO PLANNED START** and counts positions only.

The line beside it follows the status grammar: solid green on time, dashed sodium ahead or behind, half without a planned start, struck red when the track on air is not in the set (**OFF PLAN**). **×** hides the line until you open another set. The plan does not change what plays; it only shows. It was tried with the demo library and the clock, not at a gig.

## Tempo and sync

- The **tempo fader** changes the speed; up is slower, as on a player. The range key with the setup keys beside the fader cycles through **±6 %**, **±10 %**, **±16 %** and **WIDE**. The default comes from **Preferences › Controller › Decks**, and you can type a BPM into the field on the platter.
- **SYNC**, above **CUE** and **PLAY**, locks a deck to the **MASTER** deck by tempo and beat position. `⇧Q` and `⇧Y` make decks 1 and 2 master. Under **Preferences › Controller › Decks › Sync** you choose full beat sync, tempo only or phrase sync. A right click on **PLAY** starts a cued deck on the master's next phrase ([Phrases](phrases.md#mix-on-the-phrase)).
- **Q** (quantize) makes cues, loops and jumps land on the beat.
- **KEY SYNC** moves the deck to the key of the master deck.
- **‹ ±0 ›** in the deck header shifts the key a semitone at a time; click the value to reset.
- **KEYLOCK** keeps the key the same when the tempo changes (also known as master tempo).
- Keyboard bend: `W` / `E` and `U` / `I` while held.

### Key against the other decks

From version 0.8 the **KEY** in each deck header is underlined while another deck plays, so you see at a glance whether the two keys go together:

- a solid green line: the keys **fit** (the same key, one step on the circle of fifths, or the relative major or minor);
- a dashed yellow line: an **energy change** (a whole tone or a semitone up or down, or one step with major and minor swapped), a move that lifts or drops the mood on purpose;
- a red line struck through with two ticks: the keys **clash**.

![Two decks whose keys clash after a key shift of +1 on deck 2](../img/key-relation.png)

It compares the key you hear: key shift counts, and without **KEYLOCK** so does the tempo (to the nearest semitone). With more decks playing it shows the worst match. Hover over the line to read it in words, such as "Clashes with deck 1 (11B)". If the analysis was unsure of a key, its percentage stands under it in yellow; see [How sure the analysis is](beatgrid.md#how-sure-the-analysis-is). The colours are our own and follow the rest of the app: green fits, yellow needs a thought, red warns.

## Phase and tempo between decks

The strip under the waveforms compares each deck with the **reference** deck: the sync master, otherwise the deck that plays. **PHASE TO 1** names it.

- The ruler spans half a beat to each side. The centre is the reference beat; the deck's own line (in its colour, beside its numeral) sits where its beat falls. Left of the centre is behind, right is ahead.
- On the right: the tempo difference in BPM and the phase in milliseconds.
- When tempo and phase both match (within 0.05 BPM and 2 % of a beat) the centre turns into a solid green line and the readout says **IN SYNC**.

To beatmatch by hand, bring the tempo difference to ±0.00 with the tempo fader, then nudge with the jog wheel or the bend keys until the line sits in the centre.

**PRACTICE** at the right of the strip turns SYNC off on every deck and keeps it off, so you can practise without it. Click it again to use SYNC.

## Booth view

**View › Booth View** (`⌥⇧⌘B`, `Ctrl+Alt+Shift+B` on Linux and Windows) opens the decks in their own window, drawn large enough to read from a few metres: the waveform, BPM, key, the time left and the phrase with the count to the next one. Put it on a second screen in the booth, or double-click it for full screen. Everything scales with the window: on a 1080p screen the BPM and time are about three times their size on the deck. It shows what Perform shows, frame by frame; the last tenth of a playing track turns its time yellow.

![The booth view with two decks](../img/booth.png)

The booth view is a window of the desktop app; the Raspberry Pi kiosk has no second window.

## Jog wheels

On a controller the jog wheel scratches, bends or searches. Vinyl mode is switched per deck, and in **Preferences › Controller › Decks › Jog wheel**. The on-screen platter shows where you are. On the player screens the ◀◀ and ▶▶ keys search while held; they and the CALL keys wrap the jog ([The centre of the deck](#the-centre-of-the-deck)).

## Slip and censor

**SLIP** lets you loop, cue or scratch while the track carries on silently underneath; when you let go, it comes back where it would have been.

**Censor** is available from a controller and as the **CENSOR** pad mode (see below): the deck plays backwards with slip on, then carries on where it would be.

### Reverse and cut-in

**REV** above **SLIP** plays the deck backwards until you press it again; with **SLIP** on, the track comes back where it would have been. **CUT IN** below the tempo range lets the deck's channel through to the master as if the crossfader were open on its side, for as long as you hold it: for cuts and scratch patterns with the crossfader closed. The channel fader still counts. The two keys join the platter's block where six keys fit its height (a 1440 × 900 window and larger); in shorter decks, such as 1280 × 800, the block keeps its four setup keys and REV and CUT IN are on the controller only. On a controller: `deck.reverse` and `deck.cut_in`. Both are tested in software; they have not been tried on a hardware controller.

## Loops and beat jump

**LOOP** starts a loop of the shown length, with **½** and **2×** to change it; **IN** / **OUT** / **EXIT** make a manual one. More in [Cues and loops](cues-and-loops.md).

**−** and **+** (with the jump size on the key) jump by beats; the size is set in the **BEAT JUMP** pad mode, from 1/8 to 64 beats.

## Pads and pad modes

The sixteen pads at the left of each deck change meaning with the mode. Pick a mode with **‹ ›** next to its name, or click the name for the list. `⇧⌘D` and `⇧⌘S` step through them for deck 1, `⇧⌘;` and `⇧⌘K` for deck 2. On a controller the pad mode buttons light up the current mode; pads glow when engaged, dimly when set.

| Mode | The pads do |
|---|---|
| **HOT CUE** | Jump to or set hot cues A to H; bank with ‹ › |
| **BEAT LOOP** | Loop this many beats; press again to exit |
| **BEAT JUMP** | Jump by this many beats; ‹ › sets the size |
| **SAMPLER** | Play sampler slots: odd decks pads 1–8, even decks 9–16 |
| **HOLD FX** | Beat and release effects while held; two pages with ‹ › |
| **KEYBOARD** | Play the chosen hot cue at −3 to +4 semitones |
| **KEY SHIFT** | Set the key shift from −3 to +4 |
| **SLICER** | Play one of eight slices while held; domain 4 to 64 beats |
| **STEMS** | Mute and shape the stems, see [Stems](stems.md) |
| **MEMORY CUE** | Jump to the memory cues of the track |
| **CENSOR** | Censor for ¼ to 32 beats, then carry on where it would be |
| **PATTERN** | Store and recall sampler sequences |
| **LIGHTING** | A–F looks, G strobe, H flash, see [Light show](light-show.md) |
| **ROLL** | Roll 1/32 to 4 beats while held; CAPTURE keeps it, see [Loop roll](cues-and-loops.md#loop-roll-and-capture) |
| **PHRASE** | A back and B on one phrase, C to the marked drop, D marks the drop, E–H to the next build-up, drop, breakdown and outro, see [Phrases](phrases.md#phrase-jumps-and-the-drop) |
| **CUE EDIT** | A pad picks the hot cue to edit beside the list, see [Cues](cues-and-loops.md#edit-hot-cues-in-perform) |

The **GRID** key next to the mode swaps the pads for the grid and gain panel; see [Beatgrids](beatgrid.md#fix-a-grid-while-the-track-plays).
