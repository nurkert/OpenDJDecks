OpenDJDecks 0.9.0, the open-source DJ software for macOS, Linux, Windows and the Raspberry Pi. Website and guides: [opendjdecks.de](https://opendjdecks.de). Still a beta.

## New in 0.9.0: the rest of the craft

### Interface

- **A calmer, more physical interface:**
  - One motion language and knobs, faders and meters with value arcs and readouts.
  - A calmer library table and an optional calm waveform.
  - Perform lanes centred on the window, and waveforms you can drag.
- **Set Flow as a chart,** with warnings between tracks for tempo jumps, key clashes and energy drops.
- **Preferences and the USB screen in the new design,** with live previews.
- **A help overlay** (`F12` or `?`) that names every control, also on touch.
- **A touch layout** for the whole deck, not only the kiosk.
- **Full keyboard focus and readable screen reader names.**
- **Edit in Prepare:** an EDIT key and a right-click menu on the deck open the track's phrases, grid, cues, key, gain or info in Prepare while the decks keep playing. Quick fixes stay in Perform: downbeat ±1 beat, phrase boundary ±1 bar, use the likely key.
- **Clearer browser:** filter chips, a foldable lists column, fuzzy search, and type-then-Enter to load.
- **Smarter phrase detection,** measured on a generated test set.

### DJ

- **Recording for release:** loudness, true peak and clips while recording, chapter markers, a separate voice track, and a tracklist editor with text and CUE export.
- **Sticks:** labelled format badges, stems per track on the stick, and a one-press emergency stick.
- **A queue in Perform:**
  - Requests matched to your library.
  - A do-not-play tag.
  - Auto-play, and a dead-air warning before the music runs out.
- **Search at request speed,** a played-tonight mark, and tags edited on many tracks at once.
- **Mixer and deck:**
  - EQ kills also in stem mode, crossfader curves on the mixer, cut-in and reverse.
  - Split cue: the cue on one ear, the master on the other.
- **Cues:**
  - Edit hot cues from Perform, set drop markers, and jump by phrase on the beat.
  - Each cue gets its own play mode: cue, latch, gate or momentary.
- **A sampler that stays in view,** and the planned set in Perform: the next track, its transition note, and how you stand against the plan.
- **Stems you can audition** on the headphones, and your own stem files as deck parts.
- **Output routing:** master and headphones on any output pair, an external-mixer mode, split cue on one stereo output, and MIDI clock out.

### Lighting

- **SETUP and SHOW,** with a large blackout and an output test.
- **A floor plan** for placing and turning fixtures, with per-fixture tuning.
- **An editable cue list** and a cue lane in Prepare.
- **Named rig, show and venue files.**
- **A separate lighting operator:** a second controller for the lights, OSC input, and a second computer that follows the music over the network.
- **DJ events as triggers,** with the audio input or the network as music sources.
- **Unattended mode** for a fixed venue, behind the setup PIN.

### Fixed

- **Stick sync:** after a restored backup or an undone import, SYNC now writes everything the stick check reports as changed.

## Good to know

Many of these features have not yet been tried on real hardware, at a gig or by ear, including:

- controller mappings, the lighting controller layout and OSC,
- multi-output audio interfaces, an external mixer and MIDI clock receivers,
- lighting rigs and DMX interfaces,
- touch screens and screen readers,
- physical sticks and club players.

Try your setup before a gig.

OpenDJDecks is an independent project, not affiliated with or endorsed by AlphaTheta Corporation or any other manufacturer. rekordbox and Pioneer DJ are trademarks of AlphaTheta Corporation; names are used only to state compatibility.
