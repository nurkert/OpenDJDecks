---
title: Controllers and MIDI learn
description: Plug in a compatible controller and play. Set up any other controller with the guided setup, map single controls with MIDI learn and see what your controller sends in the MIDI monitor.
keywords: [controller, controllers, midi, midi learn, mapping, ddj, ddj-flx10, ddj-flx4, flx10, flx4, pioneer, led, soft takeover, browse, encoder, midi monitor, hardware, controller list, guided setup, setup, template, battle, entry level, unlisted controller, new controller]
features: [ctrl-controller-list, ctrl-guided-setup, ctrl-class-templates, ctrl-self-test, ctrl-hardware-status, ctrl-led-feedback, ctrl-soft-takeover, ctrl-midi-learn-monitor, ctrl-mapping-four-deck, ctrl-mapping-two-deck, browser-controller-browse]
---

OpenDJDecks ships with mappings compatible with the Pioneer DJ DDJ-FLX10 and DDJ-FLX4, with LED feedback for play, cue, sync, master, key lock, slip, loop, the pads and the stem buttons (lit while their part is heard), and the VU meters. The mappings are not yet fully confirmed on real devices, so individual controls or LEDs may not work: try them before a gig. Connect a controller by USB; it is picked up within a few seconds, and its audio interface is used automatically (**Preferences › Audio › Output device**).

The first time a controller is connected, Perform shows as many decks as it has; switch to 2 or 4 decks and that count comes back the next time you connect the same controller. With 2 decks shown on a four-deck controller, the two lanes show the decks active on it: press **DECK 1/3** or **DECK 2/4** and that side's lane switches, and the keyboard's deck 1 and 2 keys act on the left and right lane. Controllers that can report their faders and knobs (the DDJ-FLX10 does) set the mixer to them on connecting, so nothing needs to be moved first.

When you move a knob after a preset jumped, soft takeover makes sure the value only changes once the physical control has reached it.

![The controller page in Preferences](../img/controllers.png)

## Controller list

Which controllers OpenDJDecks knows and how far each is confirmed. A model counts as **verified** only when every one of its controls has been confirmed on a real unit with OpenDJDecks. None has been yet: the DDJ-FLX10 row says **partly confirmed** for the controls checked on one unit, the others say **not confirmed**. Vendor and model names only say which hardware a mapping is meant to be compatible with.

<!-- controller-list:start (generated: cargo run -p od-controller --example hardware_status -- --write) -->

| Controller | Decks | Mapping | Recognised by | Status |
|---|---|---|---|---|
| compatible with DDJ-FLX10 | 4 | built in | MIDI port name `DDJ-FLX10` | partly confirmed: 10 verified, 461 partial, 13 untested |
| compatible with DDJ-FLX4 | 2 | built in | MIDI port name `DDJ-FLX4` | not confirmed: 0 verified, 219 partial, 2 untested |
| Any two-deck entry-level controller | 2 | guided setup, template `two-deck-entry` (76 controls) | the port of the unit you set up | not confirmed: your own capture, nobody else's unit |
| Any two-deck battle-style controller | 2 | guided setup, template `two-deck-battle` (74 controls) | the port of the unit you set up | not confirmed: your own capture, nobody else's unit |
| Any four-deck controller | 4 | guided setup, template `four-deck` (134 controls) | the port of the unit you set up | not confirmed: your own capture, nobody else's unit |
<!-- controller-list:end -->

Any other controller can be set up with the guided setup below, from one of three generic templates. More model mappings, for entry-level and battle-style controllers too, come from owners who capture their own unit with the guided setup and share the file. None of those is confirmed for other units yet. OpenDJDecks does not derive mappings from manufacturer manuals, MIDI charts, vendor software or other projects' mapping files.

## Guided setup for other controllers

![The controller setup under the top bar, asking for the tempo fader of deck 1](../img/controller-setup.png)

For a controller without a mapping, press **SET UP A CONTROLLER** in **Preferences › Controller** (or **View › Controller Setup**). When you plug in a MIDI device OpenDJDecks has no mapping for, the status line names it and points here (not in CREATE, where a MIDI keyboard plays as it is). A band opens under the top bar:

1. Connect the controller by USB and choose what it is closest to: **2 DECKS · ENTRY LEVEL**, **2 DECKS · BATTLE** or **4 DECKS**. Press **START**.
2. The band asks for one control at a time: PLAY, CUE and SYNC, the jog wheel (touch, then turn), the tempo fader and LOAD of each deck, then the hot cue pads, the channel faders, TRIM and the EQ knobs, headphone CUE, the crossfader, the browse knob and the master and headphone levels. The matching control on screen gets the guide outline.
3. Move the control. **Heard** says what arrived; once the control rests for a moment the step is taken and the next one comes. A control that an earlier step already uses is refused with the number of that step. **SKIP** leaves out what your controller does not have, **BACK** goes one step back to learn it again, **AGAIN** forgets what arrived.
4. After the core controls the band asks whether to go on: **MORE CONTROLS** learns SHIFT, hot cue pads E to H, LOOP IN and OUT, the 4-beat loop, beat jump back and forward, KEY LOCK and SLIP of each deck, the CH FX knobs, the headphone mix, browse back and an effect on/off key and level, each one skippable. **SAVE NOW** goes straight to saving with the core controls.
5. At the end, give the controller a name and press **SAVE MAPPING**. The file is written next to your library in `mappings/`, becomes your own mapping file and is used from the next start. The battle-style template also sets the crossfader curve to **CUT**.

The file is an ordinary mapping file you can share. Every entry in it is marked as not confirmed (`verify = true`), because only your unit has been heard. While the setup, MIDI learn or the monitor runs, OpenDJDecks also opens MIDI inputs that no mapping knows; on some systems another program cannot use such a port at the same time. The guided setup has been tested with simulated MIDI messages, not yet with a real controller.

## Browse from the controller

Turn the browse encoder to move through the list. **BACK** moves the focus into the lists column, where turning steps through everything in it from top to bottom: the collection, playlists, the Explorer's folders, similar tracks, sticks, histories, recordings, the tag list, created tracks and AUTO SORT (hold shift to jump by eight). The column scrolls along, and folders stay closed while you turn past them. A press, or a deck's **LOAD**, goes one level in: a closed folder opens and its first entry is selected, an open folder closes again, a list hands over to its tracks. **BACK** goes one level out each time: from the tracks to their list, from an entry to the folder holding it (which closes), an open folder closes, and from the top level on to the rail of sources at the column's left edge. There turning steps through the sources (**ALL**, **LISTS**, **SIMILAR**, **SAMPLER**, **FILES**, **USB**, **HISTORY**, **RECORDINGS**, **TAG LIST**) and the column scrolls to each; a press or **LOAD** goes back in at the source's first entry (**SAMPLER** shows the sampler panel). Folded lists unfold when the encoder goes into them. In the track list a press loads into a free deck, **LOAD** onto its deck, and shift-press adds to the tag list. A track you click with the mouse is the one **LOAD** loads. **VIEW** toggles the full browser in Prepare and the Browse layout in Perform.

When a controller with channel faders connects, its channels start closed, because the app cannot know where the faders are until they move: move each fader once. **PLAY** on a deck whose fader has not moved yet says so in the status line.

Controllers with displays in the jog wheels (the DDJ-FLX10) show each loaded deck there: BPM, the remaining time as on the screen's decks (Preferences › View › **Remaining time** picks track time or real time at tempo), the key in Camelot or classic notation after the **Key notation** preference (Open Key shows as Camelot), the key shift, the playhead as a marker turning once every 1.8 seconds like a record, the main cue point on the same ring, and the MASTER, SYNC and Keylock indicators. Track title, waveform and artwork are not shown there.

## MIDI learn

In **Preferences › Controller › MIDI learn**:

1. Choose an **Action** and, if it applies, a **Deck / channel**.
2. Press **LEARN**, then move the control.
3. The control is described and shown as a mapping line. Press **ADD TO MY MAPPING** to write it into your own mapping file, which is used from the next start.

**Own mapping file** adds a mapping on top of the built-in ones and wins for its device. `docs/controller-mappings.md` in the source describes the format; [the developer docs](../../controller-mappings.md) have the details.

## MIDI monitor

Turn on **Monitor** to see every incoming MIDI message and what it does, for example when you are working out what a control sends.

## Self-test before a gig

In **Preferences › Controller › Self-test**, check a controller before you rely on it:

1. Pick the controller under **Mapping**. A connected controller is picked for you.
2. Press **START** and move every control once: press each button and pad, turn each knob, move each fader and the jog wheels. Each control is ticked as soon as its message arrives. The count shows how many have answered.
3. Press **FLASH LEDS**. Every LED the mapping drives lights up for two seconds and then goes back to what the decks show. An LED that stays dark is not mapped correctly for your unit.

The self-test lists the controls of each deck separately, so its count is larger than the list below.

![The controller self-test in Preferences](../img/controller-self-test.png)

A control that stays unticked after you moved it does not reach OpenDJDecks the way the mapping expects. Turn on the **Monitor** to see what it sends instead, and report it with the monitor lines.

## Hardware status

Each mapped control has one of three states:

- **verified**: someone confirmed it on a real unit with OpenDJDecks.
- **partial**: the MIDI message comes from a public source, another project's working mapping, but nobody has tried it with OpenDJDecks on a unit yet.
- **untested**: there is no public source and no confirmation yet (`verify = true` in the mapping file).

A few DDJ-FLX10 controls are verified on one unit: browsing and loading, PLAY, headphone CUE, the channel fader, the platter and the deck switch. The other checks on real units are still open. For the four-deck mapping, the public source covers related models of the same family. The list below is generated from the mapping files, so it changes with them. The self-test above is how you find out for your own unit.

<!-- hardware-status:start (generated: cargo run -p od-controller --example hardware_status -- --write) -->

| Model | Controls | Verified | Partial | Untested |
|---|---|---|---|---|
| DDJ-FLX10 | 484 | 10 | 461 | 13 |
| DDJ-FLX4 | 221 | 0 | 219 | 2 |

### DDJ-FLX10

| Control | Kind | Status |
|---|---|---|
| Browse rotate | input | verified |
| Browse rotate (shift) | input | verified |
| Back | input | verified |
| Back (shift) | input | partial |
| Browse view | input | partial |
| Browse view (long press) | input | partial |
| Browse view (shift) | input | partial |
| Browse press | input | partial |
| Browse press (shift, deck 1 active) | input | partial |
| Browse press (shift, deck 2 active) | input | partial |
| Browse press (shift, deck 3 active) | input | partial |
| Browse press (shift, deck 4 active) | input | partial |
| Deck select | input | verified |
| Deck select button | input | partial |
| Deck select 1/3 (shift) | input | partial |
| Deck select 2/4 (shift) | input | partial |
| Play/Pause | input | verified |
| Play/Pause (shift) | input | partial |
| Cue | input | partial |
| Cue (shift) | input | partial |
| Sync | input | partial |
| Sync (long press) | input | partial |
| Sync (shift) | input | partial |
| Key sync | input | partial |
| Key sync (long press) | input | partial |
| Key sync (shift) | input | partial |
| Tempo reset | input | partial |
| Tempo reset (shift) | input | partial |
| Shift | input | partial |
| Quantize | input | partial |
| Quantize (shift) | input | partial |
| Slip | input | partial |
| Slip (shift) | input | untested |
| Slip reverse | input | partial |
| Slip reverse (shift) | input | partial |
| Beat jump back | input | partial |
| Beat jump forward | input | partial |
| Beat jump back (shift) | input | partial |
| Beat jump forward (shift) | input | partial |
| Search back (long press) | input | partial |
| Search forward (long press) | input | partial |
| Memory cue call back | input | partial |
| Memory cue call forward | input | partial |
| Memory cue call back (shift) | input | partial |
| Memory cue call forward (shift) | input | partial |
| Mix point select back | input | partial |
| Mix point select forward | input | partial |
| Mix point select back (shift) | input | partial |
| Mix point select forward (shift) | input | partial |
| Mix point link | input | partial |
| Mix point link (shift) | input | partial |
| Stem button DRUMS | input | partial |
| Stem button VOCAL | input | partial |
| Stem button INST | input | partial |
| Stem button DRUMS (shift, twice) | input | partial |
| Stem button VOCAL (shift, twice) | input | partial |
| Stem button INST (shift, twice) | input | partial |
| Tempo fader | input | untested |
| Tempo fader (shift) | input | untested |
| Jog touch | input | verified |
| Jog touch (shift) | input | partial |
| Jog platter (vinyl on) | input | partial |
| Jog platter (vinyl off) | input | verified |
| Jog side bend | input | verified |
| Jog platter search (shift) | input | partial |
| Jog side search (shift) | input | untested |
| Jog (4 beat jump held) | input | partial |
| Loop in | input | partial |
| Loop out | input | partial |
| Loop in (shift) | input | untested |
| Loop out (shift) | input | untested |
| 4 beat loop | input | partial |
| 4 beat / exit (shift) | input | partial |
| Trim | input | partial |
| EQ high | input | partial |
| EQ mid | input | partial |
| EQ low | input | partial |
| Part isolator vocals | input | untested |
| Part isolator inst | input | untested |
| Part isolator drums | input | untested |
| Channel fader | input | verified |
| CH FX knob | input | partial |
| Crossfader | input | partial |
| Master level | input | partial |
| Booth level | input | partial |
| Headphone mix | input | partial |
| Headphone level | input | partial |
| Headphone cue | input | verified |
| Headphone cue (shift) | input | partial |
| Master cue | input | partial |
| Master cue (shift) | input | partial |
| Crossfader assign A | input | partial |
| Crossfader assign THRU | input | partial |
| Crossfader assign B | input | partial |
| Input select (PC-A / line-phono / PC-B) 1 | input | partial |
| Input select (PC-A / line-phono / PC-B) 2 | input | partial |
| Input select (PC-A / line-phono / PC-B) 3 | input | partial |
| Fader start (play) | input | partial |
| Fader start (sync) | input | partial |
| Fader start (cue) | input | partial |
| Line/phono (channel 3) | input | partial |
| Line/phono (channel 4) | input | partial |
| Mic EQ high | input | partial |
| Mic EQ low | input | partial |
| Mic switch (off / on / talkover) 1 | input | partial |
| Mic switch (off / on / talkover) 2 | input | partial |
| Mic switch (off / on / talkover) 3 | input | partial |
| CH FX select 1 | input | partial |
| CH FX select 2 | input | partial |
| CH FX select 3 | input | partial |
| CH FX select 4 | input | partial |
| CH FX select 5 | input | partial |
| CH FX select 6 | input | partial |
| CH FX select (shift) 1 | input | partial |
| CH FX select (shift) 2 | input | partial |
| CH FX select (shift) 3 | input | partial |
| CH FX select (shift) 4 | input | partial |
| CH FX select (shift) 5 | input | partial |
| CH FX select (shift) 6 | input | partial |
| FX part select 1 | input | partial |
| FX part select 2 | input | partial |
| FX part select 3 | input | partial |
| FX part select (shift) 1 | input | partial |
| FX part select (shift) 2 | input | partial |
| FX part select (shift) 3 | input | partial |
| Sampler volume | input | partial |
| Sampler cue | input | partial |
| Sampler cue (shift) | input | partial |
| Tempo FX beat down | input | partial |
| Tempo FX beat up | input | partial |
| Tempo FX auto beat (shift) | input | partial |
| Tempo FX tap (shift) | input | partial |
| Tempo FX select 1 | input | partial |
| Tempo FX select 2 | input | partial |
| Tempo FX select 3 | input | partial |
| Tempo FX select 4 | input | partial |
| Tempo FX select 5 | input | partial |
| Tempo FX select 6 | input | partial |
| Tempo FX select 7 | input | partial |
| Tempo FX select 8 | input | partial |
| Tempo FX select 9 | input | partial |
| Tempo FX select 10 | input | partial |
| Tempo FX select 11 | input | partial |
| Tempo FX select 12 | input | partial |
| Tempo FX select 13 | input | partial |
| Tempo FX select 14 | input | partial |
| Tempo FX channel 1 | input | partial |
| Tempo FX channel 2 | input | partial |
| Tempo FX channel 3 | input | partial |
| Tempo FX channel 4 | input | partial |
| Tempo FX channel master | input | partial |
| Tempo FX channel mic/sampler 1 | input | partial |
| Tempo FX channel mic/sampler 2 | input | partial |
| Tempo FX level | input | partial |
| Tempo FX on | input | partial |
| Tempo FX release (shift) | input | partial |
| Pad mode hot cue | input | partial |
| Pad mode hot cue (long press) | input | partial |
| Pad mode keyboard (shift) | input | partial |
| Pad mode hold FX 1 | input | partial |
| Pad mode hold FX 2 (shift) | input | partial |
| Pad mode beat jump | input | partial |
| Pad mode beat loop (shift) | input | partial |
| Pad mode sampler | input | partial |
| Pad mode key shift (shift) | input | partial |
| Pad page left 1 | input | partial |
| Pad page left 2 | input | partial |
| Pad page left 3 | input | partial |
| Pad page left 4 | input | partial |
| Pad page right 1 | input | partial |
| Pad page right 2 | input | partial |
| Pad page right 3 | input | partial |
| Pad page right 4 | input | partial |
| Sampler bank down | input | partial |
| Sampler bank up | input | partial |
| Pad page left (shift) 1 | input | partial |
| Pad page left (shift) 2 | input | partial |
| Pad page left (shift) 3 | input | partial |
| Pad page left (shift) 4 | input | partial |
| Pad page left (shift) 5 | input | partial |
| Pad page left (shift) 6 | input | partial |
| Pad page left (shift) 7 | input | partial |
| Pad page left (shift) 8 | input | partial |
| Pad page right (shift) | input | partial |
| Pad page right (shift) 1 | input | partial |
| Pad page right (shift) 2 | input | partial |
| Pad page right (shift) 3 | input | partial |
| Pad page right (shift) 4 | input | partial |
| Pad page right (shift) 5 | input | partial |
| Pad page right (shift) 6 | input | partial |
| Pad page right (shift, key shift mode) | input | partial |
| Hot cue pad 1 | input | partial |
| Hot cue pad 2 | input | partial |
| Hot cue pad 3 | input | partial |
| Hot cue pad 4 | input | partial |
| Hot cue pad 5 | input | partial |
| Hot cue pad 6 | input | partial |
| Hot cue pad 7 | input | partial |
| Hot cue pad 8 | input | partial |
| Hot cue pad (shift) 1 | input | partial |
| Hot cue pad (shift) 2 | input | partial |
| Hot cue pad (shift) 3 | input | partial |
| Hot cue pad (shift) 4 | input | partial |
| Hot cue pad (shift) 5 | input | partial |
| Hot cue pad (shift) 6 | input | partial |
| Hot cue pad (shift) 7 | input | partial |
| Hot cue pad (shift) 8 | input | partial |
| Hold FX 1 pad 1 | input | partial |
| Hold FX 1 pad 2 | input | partial |
| Hold FX 1 pad 3 | input | partial |
| Hold FX 1 pad 4 | input | partial |
| Hold FX 1 pad 5 | input | partial |
| Hold FX 1 pad 6 | input | partial |
| Hold FX 1 pad 7 | input | partial |
| Hold FX 1 pad 8 | input | partial |
| Beat jump pad 1 | input | partial |
| Beat jump pad 2 | input | partial |
| Beat jump pad 3 | input | partial |
| Beat jump pad 4 | input | partial |
| Beat jump pad 5 | input | partial |
| Beat jump pad 6 | input | partial |
| Beat jump pad 7 | input | partial |
| Beat jump pad 8 | input | partial |
| Sampler pad 1 | input | partial |
| Sampler pad 2 | input | partial |
| Sampler pad 3 | input | partial |
| Sampler pad 4 | input | partial |
| Sampler pad 5 | input | partial |
| Sampler pad 6 | input | partial |
| Sampler pad 7 | input | partial |
| Sampler pad 8 | input | partial |
| Sampler pad 9 | input | partial |
| Sampler pad 10 | input | partial |
| Sampler pad 11 | input | partial |
| Sampler pad 12 | input | partial |
| Sampler pad 13 | input | partial |
| Sampler pad 14 | input | partial |
| Sampler pad 15 | input | partial |
| Sampler pad 16 | input | partial |
| Sampler pad (shift) 1 | input | partial |
| Sampler pad (shift) 2 | input | partial |
| Sampler pad (shift) 3 | input | partial |
| Sampler pad (shift) 4 | input | partial |
| Sampler pad (shift) 5 | input | partial |
| Sampler pad (shift) 6 | input | partial |
| Sampler pad (shift) 7 | input | partial |
| Sampler pad (shift) 8 | input | partial |
| Sampler pad (shift) 9 | input | partial |
| Sampler pad (shift) 10 | input | partial |
| Sampler pad (shift) 11 | input | partial |
| Sampler pad (shift) 12 | input | partial |
| Sampler pad (shift) 13 | input | partial |
| Sampler pad (shift) 14 | input | partial |
| Sampler pad (shift) 15 | input | partial |
| Sampler pad (shift) 16 | input | partial |
| Keyboard pad 1 | input | partial |
| Keyboard pad 2 | input | partial |
| Keyboard pad 3 | input | partial |
| Keyboard pad 4 | input | partial |
| Keyboard pad 5 | input | partial |
| Keyboard pad 6 | input | partial |
| Keyboard pad 7 | input | partial |
| Keyboard pad 8 | input | partial |
| Hold FX 2 pad 1 | input | partial |
| Hold FX 2 pad 2 | input | partial |
| Hold FX 2 pad 3 | input | partial |
| Hold FX 2 pad 4 | input | partial |
| Hold FX 2 pad 5 | input | partial |
| Hold FX 2 pad 6 | input | partial |
| Hold FX 2 pad 7 | input | partial |
| Hold FX 2 pad 8 | input | partial |
| Beat loop pad 1 | input | partial |
| Beat loop pad 2 | input | partial |
| Beat loop pad 3 | input | partial |
| Beat loop pad 4 | input | partial |
| Beat loop pad 5 | input | partial |
| Beat loop pad 6 | input | partial |
| Beat loop pad 7 | input | partial |
| Beat loop pad 8 | input | partial |
| Key shift pad 1 | input | partial |
| Key shift pad 2 | input | partial |
| Key shift pad 3 | input | partial |
| Key shift pad 4 | input | partial |
| Key shift pad 5 | input | partial |
| Key shift pad 6 | input | partial |
| Key shift pad 7 | input | partial |
| Key shift pad 8 | input | partial |
| Hot cue pad (page 2) 1 | input | partial |
| Hot cue pad (page 2) 2 | input | partial |
| Hot cue pad (page 2) 3 | input | partial |
| Hot cue pad (page 2) 4 | input | partial |
| Hot cue pad (page 2) 5 | input | partial |
| Hot cue pad (page 2) 6 | input | partial |
| Hot cue pad (page 2) 7 | input | partial |
| Hot cue pad (page 2) 8 | input | partial |
| Hot cue pad (page 2, shift) 1 | input | partial |
| Hot cue pad (page 2, shift) 2 | input | partial |
| Hot cue pad (page 2, shift) 3 | input | partial |
| Hot cue pad (page 2, shift) 4 | input | partial |
| Hot cue pad (page 2, shift) 5 | input | partial |
| Hot cue pad (page 2, shift) 6 | input | partial |
| Hot cue pad (page 2, shift) 7 | input | partial |
| Hot cue pad (page 2, shift) 8 | input | partial |
| Hold FX 1 pad (page 2) 1 | input | partial |
| Hold FX 1 pad (page 2) 2 | input | partial |
| Hold FX 1 pad (page 2) 3 | input | partial |
| Hold FX 1 pad (page 2) 4 | input | partial |
| Hold FX 1 pad (page 2) 5 | input | partial |
| Hold FX 1 pad (page 2) 6 | input | partial |
| Hold FX 1 pad (page 2) 7 | input | partial |
| Hold FX 1 pad (page 2) 8 | input | partial |
| Beat jump pad (page 2) 1 | input | partial |
| Beat jump pad (page 2) 2 | input | partial |
| Beat jump pad (page 2) 3 | input | partial |
| Beat jump pad (page 2) 4 | input | partial |
| Beat jump pad (page 2) 5 | input | partial |
| Beat jump pad (page 2) 6 | input | partial |
| Beat jump pad (page 2) 7 | input | partial |
| Beat jump pad (page 2) 8 | input | partial |
| Keyboard pad (page 2) 1 | input | partial |
| Keyboard pad (page 2) 2 | input | partial |
| Keyboard pad (page 2) 3 | input | partial |
| Keyboard pad (page 2) 4 | input | partial |
| Keyboard pad (page 2) 5 | input | partial |
| Keyboard pad (page 2) 6 | input | partial |
| Keyboard pad (page 2) 7 | input | partial |
| Keyboard pad (page 2) 8 | input | partial |
| Hold FX 2 pad (page 2) 1 | input | partial |
| Hold FX 2 pad (page 2) 2 | input | partial |
| Hold FX 2 pad (page 2) 3 | input | partial |
| Hold FX 2 pad (page 2) 4 | input | partial |
| Hold FX 2 pad (page 2) 5 | input | partial |
| Hold FX 2 pad (page 2) 6 | input | partial |
| Hold FX 2 pad (page 2) 7 | input | partial |
| Hold FX 2 pad (page 2) 8 | input | partial |
| Beat loop pad (page 2) 1 | input | partial |
| Beat loop pad (page 2) 2 | input | partial |
| Beat loop pad (page 2) 3 | input | partial |
| Beat loop pad (page 2) 4 | input | partial |
| Beat loop pad (page 2) 5 | input | partial |
| Beat loop pad (page 2) 6 | input | partial |
| Beat loop pad (page 2) 7 | input | partial |
| Beat loop pad (page 2) 8 | input | partial |
| Key shift pad (page 2) 1 | input | partial |
| Key shift pad (page 2) 2 | input | partial |
| Key shift pad (page 2) 3 | input | partial |
| Key shift pad (page 2) 4 | input | partial |
| Key shift pad (page 2) 5 | input | partial |
| Key shift pad (page 2) 6 | input | partial |
| Key shift pad (page 2) 7 | input | partial |
| Key shift pad (page 2) 8 | input | partial |
| Hold FX 1 pad (shift) 1 | input | partial |
| Hold FX 1 pad (shift) 2 | input | partial |
| Hold FX 1 pad (shift) 3 | input | partial |
| Hold FX 1 pad (shift) 4 | input | partial |
| Hold FX 1 pad (shift) 5 | input | partial |
| Hold FX 1 pad (shift) 6 | input | partial |
| Hold FX 1 pad (shift) 7 | input | partial |
| Hold FX 1 pad (shift) 8 | input | partial |
| Hold FX 1 pad (shift) 9 | input | partial |
| Hold FX 1 pad (shift) 10 | input | partial |
| Hold FX 1 pad (shift) 11 | input | partial |
| Hold FX 1 pad (shift) 12 | input | partial |
| Hold FX 1 pad (shift) 13 | input | partial |
| Hold FX 1 pad (shift) 14 | input | partial |
| Hold FX 1 pad (shift) 15 | input | partial |
| Hold FX 1 pad (shift) 16 | input | partial |
| Beat jump pad (shift) 1 | input | partial |
| Beat jump pad (shift) 2 | input | partial |
| Beat jump pad (shift) 3 | input | partial |
| Beat jump pad (shift) 4 | input | partial |
| Beat jump pad (shift) 5 | input | partial |
| Beat jump pad (shift) 6 | input | partial |
| Beat jump pad (shift) 7 | input | partial |
| Beat jump pad (shift) 8 | input | partial |
| Beat jump pad (shift) 9 | input | partial |
| Beat jump pad (shift) 10 | input | partial |
| Beat jump pad (shift) 11 | input | partial |
| Beat jump pad (shift) 12 | input | partial |
| Beat jump pad (shift) 13 | input | partial |
| Beat jump pad (shift) 14 | input | partial |
| Beat jump pad (shift) 15 | input | partial |
| Beat jump pad (shift) 16 | input | partial |
| Keyboard pad (shift) 1 | input | partial |
| Keyboard pad (shift) 2 | input | partial |
| Keyboard pad (shift) 3 | input | partial |
| Keyboard pad (shift) 4 | input | partial |
| Keyboard pad (shift) 5 | input | partial |
| Keyboard pad (shift) 6 | input | partial |
| Keyboard pad (shift) 7 | input | partial |
| Keyboard pad (shift) 8 | input | partial |
| Keyboard pad (shift) 9 | input | partial |
| Keyboard pad (shift) 10 | input | partial |
| Keyboard pad (shift) 11 | input | partial |
| Keyboard pad (shift) 12 | input | partial |
| Keyboard pad (shift) 13 | input | partial |
| Keyboard pad (shift) 14 | input | partial |
| Keyboard pad (shift) 15 | input | partial |
| Keyboard pad (shift) 16 | input | partial |
| Hold FX 2 pad (shift) 1 | input | partial |
| Hold FX 2 pad (shift) 2 | input | partial |
| Hold FX 2 pad (shift) 3 | input | partial |
| Hold FX 2 pad (shift) 4 | input | partial |
| Hold FX 2 pad (shift) 5 | input | partial |
| Hold FX 2 pad (shift) 6 | input | partial |
| Hold FX 2 pad (shift) 7 | input | partial |
| Hold FX 2 pad (shift) 8 | input | partial |
| Hold FX 2 pad (shift) 9 | input | partial |
| Hold FX 2 pad (shift) 10 | input | partial |
| Hold FX 2 pad (shift) 11 | input | partial |
| Hold FX 2 pad (shift) 12 | input | partial |
| Hold FX 2 pad (shift) 13 | input | partial |
| Hold FX 2 pad (shift) 14 | input | partial |
| Hold FX 2 pad (shift) 15 | input | partial |
| Hold FX 2 pad (shift) 16 | input | partial |
| Beat loop pad (shift) 1 | input | partial |
| Beat loop pad (shift) 2 | input | partial |
| Beat loop pad (shift) 3 | input | partial |
| Beat loop pad (shift) 4 | input | partial |
| Beat loop pad (shift) 5 | input | partial |
| Beat loop pad (shift) 6 | input | partial |
| Beat loop pad (shift) 7 | input | partial |
| Beat loop pad (shift) 8 | input | partial |
| Beat loop pad (shift) 9 | input | partial |
| Beat loop pad (shift) 10 | input | partial |
| Beat loop pad (shift) 11 | input | partial |
| Beat loop pad (shift) 12 | input | partial |
| Beat loop pad (shift) 13 | input | partial |
| Beat loop pad (shift) 14 | input | partial |
| Beat loop pad (shift) 15 | input | partial |
| Beat loop pad (shift) 16 | input | partial |
| Key shift pad (shift) 1 | input | partial |
| Key shift pad (shift) 2 | input | partial |
| Key shift pad (shift) 3 | input | partial |
| Key shift pad (shift) 4 | input | partial |
| Key shift pad (shift) 5 | input | partial |
| Key shift pad (shift) 6 | input | partial |
| Key shift pad (shift) 7 | input | partial |
| Key shift pad (shift) 8 | input | partial |
| Key shift pad (shift) 9 | input | partial |
| Key shift pad (shift) 10 | input | partial |
| Key shift pad (shift) 11 | input | partial |
| Key shift pad (shift) 12 | input | partial |
| Key shift pad (shift) 13 | input | partial |
| Key shift pad (shift) 14 | input | partial |
| Key shift pad (shift) 15 | input | partial |
| Key shift pad (shift) 16 | input | partial |
| deck play | light | partial |
| deck cue | light | partial |
| deck sync | light | partial |
| deck master | light | partial |
| deck key sync | light | partial |
| deck keylock | light | partial |
| deck tempo reset | light | partial |
| deck tempo range | light | partial |
| deck quantize | light | partial |
| deck slip | light | partial |
| deck vinyl | light | untested |
| deck censor | light | partial |
| deck reverse | light | partial |
| deck stem active | light | partial |
| deck loop in | light | partial |
| deck loop out | light | partial |
| deck loop active | light | partial |
| mixer cue | light | partial |
| mixer vu | light | partial |
| deck loaded | light | partial |
| deck position angle | light | partial |
| deck bpm | light | partial |
| deck cue angle | light | partial |
| deck time minutes | light | partial |
| deck time seconds | light | partial |
| deck time remaining | light | partial |
| deck key | light | untested |
| deck key shift | light | partial |
| key camelot | light | untested |
| deck pad mode | light | partial |
| deck pad | light | partial |
| master cue | light | untested |
| channel fx | light | partial |
| channel fx part | light | partial |
| tempo fx on | light | partial |
| tempo fx release | light | partial |
| sampler cue | light | partial |

### DDJ-FLX4

| Control | Kind | Status |
|---|---|---|
| Browse rotate | input | partial |
| Browse rotate (shift) | input | partial |
| Browse press | input | partial |
| Browse press (shift) | input | partial |
| Load deck 1 | input | partial |
| Load deck 2 | input | partial |
| Load deck 1 (shift) | input | partial |
| Load deck 2 (shift) | input | partial |
| Play/Pause | input | partial |
| Play/Pause (shift) | input | partial |
| Cue | input | partial |
| Cue (shift) | input | partial |
| Sync | input | partial |
| Sync (long press) | input | partial |
| Sync (shift) | input | partial |
| Shift | input | partial |
| Tempo fader | input | untested |
| Jog touch | input | partial |
| Jog touch (shift) | input | partial |
| Jog scratch | input | partial |
| Jog top bend | input | partial |
| Jog side bend | input | partial |
| Jog search (shift) | input | partial |
| Loop in | input | partial |
| Loop out | input | partial |
| 4 beat loop | input | partial |
| 4 beat loop (shift) | input | partial |
| Loop in (shift) | input | partial |
| Loop out (shift) | input | partial |
| Call left | input | partial |
| Call right | input | partial |
| Call left (shift) | input | partial |
| Call right (shift) | input | partial |
| Trim | input | partial |
| EQ high | input | partial |
| EQ mid | input | partial |
| EQ low | input | partial |
| Channel fader | input | partial |
| CH FX knob ch1 | input | partial |
| CH FX knob ch2 | input | partial |
| Crossfader | input | partial |
| Master level | input | partial |
| Headphone mix | input | partial |
| Headphone level | input | partial |
| Headphone cue | input | partial |
| Headphone cue (shift) | input | partial |
| Auto Channel FX | input | partial |
| Auto Channel FX select | input | partial |
| Auto fader | input | partial |
| Master cue | input | partial |
| Master cue (shift) | input | partial |
| Auto fader (shift) | input | partial |
| Mic level | input | partial |
| Android mono/stereo | input | partial |
| Fader start (play) | input | partial |
| Fader start (cue) | input | partial |
| Tempo FX lever CH1 | input | partial |
| Tempo FX lever CH2 | input | partial |
| Tempo FX beat down | input | partial |
| Tempo FX beat up | input | partial |
| Tempo FX auto beat (shift) | input | partial |
| Tempo FX tap (shift) | input | partial |
| Tempo FX select | input | partial |
| Tempo FX select (shift) | input | partial |
| Tempo FX level | input | partial |
| Tempo FX level (lever CH2) | input | untested |
| Tempo FX on | input | partial |
| Tempo FX on (lever CH2) | input | partial |
| Tempo FX release (shift) | input | partial |
| Tempo FX release (shift, lever CH2) | input | partial |
| Pad mode hot cue | input | partial |
| Pad mode keyboard (shift) | input | partial |
| Pad mode hold FX 1 | input | partial |
| Pad mode hold FX 2 (shift) | input | partial |
| Pad mode beat jump | input | partial |
| Pad mode beat loop (shift) | input | partial |
| Pad mode sampler | input | partial |
| Pad mode key shift (shift) | input | partial |
| Hot cue pad 1 | input | partial |
| Hot cue pad 2 | input | partial |
| Hot cue pad 3 | input | partial |
| Hot cue pad 4 | input | partial |
| Hot cue pad 5 | input | partial |
| Hot cue pad 6 | input | partial |
| Hot cue pad 7 | input | partial |
| Hot cue pad 8 | input | partial |
| Hot cue pad (shift) 1 | input | partial |
| Hot cue pad (shift) 2 | input | partial |
| Hot cue pad (shift) 3 | input | partial |
| Hot cue pad (shift) 4 | input | partial |
| Hot cue pad (shift) 5 | input | partial |
| Hot cue pad (shift) 6 | input | partial |
| Hot cue pad (shift) 7 | input | partial |
| Hot cue pad (shift) 8 | input | partial |
| Hold FX 1 pad 1 | input | partial |
| Hold FX 1 pad 2 | input | partial |
| Hold FX 1 pad 3 | input | partial |
| Hold FX 1 pad 4 | input | partial |
| Hold FX 1 pad 5 | input | partial |
| Hold FX 1 pad 6 | input | partial |
| Hold FX 1 pad 7 | input | partial |
| Hold FX 1 pad 8 | input | partial |
| Beat jump pad 1 | input | partial |
| Beat jump pad 2 | input | partial |
| Beat jump pad 3 | input | partial |
| Beat jump pad 4 | input | partial |
| Beat jump pad 5 | input | partial |
| Beat jump pad 6 | input | partial |
| Beat jump pad 7 | input | partial |
| Beat jump pad 8 | input | partial |
| Sampler pad 1 | input | partial |
| Sampler pad 2 | input | partial |
| Sampler pad 3 | input | partial |
| Sampler pad 4 | input | partial |
| Sampler pad 5 | input | partial |
| Sampler pad 6 | input | partial |
| Sampler pad 7 | input | partial |
| Sampler pad 8 | input | partial |
| Sampler pad (shift) 1 | input | partial |
| Sampler pad (shift) 2 | input | partial |
| Sampler pad (shift) 3 | input | partial |
| Sampler pad (shift) 4 | input | partial |
| Sampler pad (shift) 5 | input | partial |
| Sampler pad (shift) 6 | input | partial |
| Sampler pad (shift) 7 | input | partial |
| Sampler pad (shift) 8 | input | partial |
| Keyboard pad 1 | input | partial |
| Keyboard pad 2 | input | partial |
| Keyboard pad 3 | input | partial |
| Keyboard pad 4 | input | partial |
| Keyboard pad 5 | input | partial |
| Keyboard pad 6 | input | partial |
| Keyboard pad 7 | input | partial |
| Keyboard pad 8 | input | partial |
| Hold FX 2 pad 1 | input | partial |
| Hold FX 2 pad 2 | input | partial |
| Hold FX 2 pad 3 | input | partial |
| Hold FX 2 pad 4 | input | partial |
| Hold FX 2 pad 5 | input | partial |
| Hold FX 2 pad 6 | input | partial |
| Hold FX 2 pad 7 | input | partial |
| Hold FX 2 pad 8 | input | partial |
| Beat loop pad 1 | input | partial |
| Beat loop pad 2 | input | partial |
| Beat loop pad 3 | input | partial |
| Beat loop pad 4 | input | partial |
| Beat loop pad 5 | input | partial |
| Beat loop pad 6 | input | partial |
| Beat loop pad 7 | input | partial |
| Beat loop pad 8 | input | partial |
| Key shift pad 1 | input | partial |
| Key shift pad 2 | input | partial |
| Key shift pad 3 | input | partial |
| Key shift pad 4 | input | partial |
| Key shift pad 5 | input | partial |
| Key shift pad 6 | input | partial |
| Key shift pad 7 | input | partial |
| Key shift pad 8 | input | partial |
| Hold FX 1 pad (shift) 1 | input | partial |
| Hold FX 1 pad (shift) 2 | input | partial |
| Hold FX 1 pad (shift) 3 | input | partial |
| Hold FX 1 pad (shift) 4 | input | partial |
| Hold FX 1 pad (shift) 5 | input | partial |
| Hold FX 1 pad (shift) 6 | input | partial |
| Hold FX 1 pad (shift) 7 | input | partial |
| Hold FX 1 pad (shift) 8 | input | partial |
| Beat jump pad (shift) 1 | input | partial |
| Beat jump pad (shift) 2 | input | partial |
| Beat jump pad (shift) 3 | input | partial |
| Beat jump pad (shift) 4 | input | partial |
| Beat jump pad (shift) 5 | input | partial |
| Beat jump pad (shift) 6 | input | partial |
| Beat jump pad (shift) 7 | input | partial |
| Beat jump pad (shift) 8 | input | partial |
| Keyboard pad (shift) 1 | input | partial |
| Keyboard pad (shift) 2 | input | partial |
| Keyboard pad (shift) 3 | input | partial |
| Keyboard pad (shift) 4 | input | partial |
| Keyboard pad (shift) 5 | input | partial |
| Keyboard pad (shift) 6 | input | partial |
| Keyboard pad (shift) 7 | input | partial |
| Keyboard pad (shift) 8 | input | partial |
| Hold FX 2 pad (shift) 1 | input | partial |
| Hold FX 2 pad (shift) 2 | input | partial |
| Hold FX 2 pad (shift) 3 | input | partial |
| Hold FX 2 pad (shift) 4 | input | partial |
| Hold FX 2 pad (shift) 5 | input | partial |
| Hold FX 2 pad (shift) 6 | input | partial |
| Hold FX 2 pad (shift) 7 | input | partial |
| Hold FX 2 pad (shift) 8 | input | partial |
| Beat loop pad (shift) 1 | input | partial |
| Beat loop pad (shift) 2 | input | partial |
| Beat loop pad (shift) 3 | input | partial |
| Beat loop pad (shift) 4 | input | partial |
| Beat loop pad (shift) 5 | input | partial |
| Beat loop pad (shift) 6 | input | partial |
| Beat loop pad (shift) 7 | input | partial |
| Beat loop pad (shift) 8 | input | partial |
| Key shift pad (shift) 1 | input | partial |
| Key shift pad (shift) 2 | input | partial |
| Key shift pad (shift) 3 | input | partial |
| Key shift pad (shift) 4 | input | partial |
| Key shift pad (shift) 5 | input | partial |
| Key shift pad (shift) 6 | input | partial |
| Key shift pad (shift) 7 | input | partial |
| Key shift pad (shift) 8 | input | partial |
| deck play | light | partial |
| deck cue | light | partial |
| deck loop in | light | partial |
| deck loop out | light | partial |
| mixer cue | light | partial |
| deck sync | light | partial |
| deck tempo range | light | partial |
| deck loop active | light | partial |
| deck vinyl | light | partial |
| mixer vu | light | partial |
| deck loaded | light | partial |
| deck pad mode | light | partial |
| deck pad | light | partial |
| tempo fx on | light | partial |
| channel fx | light | partial |
<!-- hardware-status:end -->
