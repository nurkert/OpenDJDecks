# CREATE: concept

CREATE is the fourth tab next to PREPARE, PERFORM and LIGHTING. It covers what DJs today do in a separate production program before a set:

- Play the next track's melody with one note from the current track.
- Prepare that as a file that also works on club players.
- Play a small drum machine.
- See which notes fit.

It is deliberately small. This page is the plan for roadmap 0.10.0 ([70] to [82]).

## Where it comes from

Eight simulated DJ interviews, held in November 2026. Simulated interviews give hypotheses, not evidence; real DJs can change this. The eight were:

- a club techno DJ on club players
- an open-format DJ
- a house DJ who produces
- a beginner
- a drum and bass DJ
- a melodic techno DJ who already does these melody transitions
- a DJ teacher
- a live hybrid performer

| Need | How many raised it |
|---|---|
| A rendered file with one hot cue per note, right tempo and grid, with a club-player check | 6 of 8 |
| Melody from one note, with a pitch that can be corrected | 5–6 |
| Clear words and a first screen that is never empty | 5 |
| Which notes fit, as a quiet overlay rather than a lesson | 5 |
| A small drum machine with built-in sounds, own samples and sync | 5 |
| One sample and sequencer store shared with Perform, not a second one | 4 |
| Pads and a keyboard strip over the same notes | 4 |
| Reachable from Perform while playing | 3 |
| Live coding | 1 |

Two points stayed open in the interviews and are decided here:

- **Own tab, or inside Prepare and Perform?** An own tab for making things, and a pad mode in Perform for playing them.
- **Live coding?** Left out until real users ask for it.

**Name.** CREATE, a verb like PREPARE and PERFORM. PRODUCE promised a full production program and scared beginners. STUDIO suggested a full studio, and SKETCH read like drawing. Tab names stay English in German too.

## What it is not

- No timeline, no piano roll, no plug-ins, no automation and no mixing console.
- No synthesiser design: a few kits with three knobs per pad.
- No melody writing by itself. Suggestions only, always shown as suggestions and always correctable.
- No second audio engine or sample store. CREATE uses the engine, the sampler's slots and sequencer, and the render path of the Prepare player's EDIT tab. The decks keep playing while CREATE is open.

Every control has to pass one test: does it help a melody from one note, a live drum pattern, or the question of which notes fit?

## Layout at 1440×900

```
┌ PREPARE  PERFORM  LIGHTING  CREATE ───────────────────── status ──────┐
│                  [ MELODY ]   DRUMS   THEORY                           │
├ SOURCE (320) ──────┬ NOTES ─────────────────────────────┬ FITS (280) ──┤
│ track: deck 1 ▾    │ KEY A  SCALE MINOR  [NEXT TRACK]   │ notes A B C D │
│ ▁▃▇█▅▃▁ waveform   │ keyboard strip  (root, in key, out)│ E F G         │
│   [IN]━━━━[OUT]    │                                    │ chords Am C G │
│ A3  +12 ct  − +    │ PADS  oct − +   SNAP TO SCALE ●    │ F Dm          │
│ DETECTED   ▶ HEAR  │ [A3][B3][C4][D4][E4][F4][G4][A4]   │               │
│                    │ ARRANGE  ● REC  1/8  CLEAR         │ why: …        │
│                    │ |A3 · C4 · E4 · D4 ·|A3 · · · C4 · │               │
├────────────────────┴────────────────────────────────────┴───────────────┤
│ RESULT  name · 8 notes · 4 bars · 124 BPM · 8 cues · club players OK    │
│ ( SINGLE NOTES | MELODY )   SEND TO PERFORM   RENDER   TO PLAYLIST/STICK │
├ deck strip: 1 ▶ 124.0 ━━│━━   2 ▶ 124.0 ━━│━━   phase ───────────────────┤
```

- The flow reads left to right, then down: source, then notes, then result.
- FITS folds away and is also its own view, THEORY.
- The deck strip keeps phase, tempo and level in view. Below about 1060 px wide it shrinks to the two decks' play states.
- DESIGN.md applies throughout:
  - surfaces rather than outlines, filled states and one state language;
  - off is plate, selected a bone fill, on or playing a sodium fill, armed red with a word, rendered green with READY;
  - groups centred, 44 px targets on touch;
  - no meaning carried by hue alone.

## Melody from one note

1. **Pick the note.** Choose a source: the track on a deck, or one from the library. Drag IN and OUT around one note on a zoomed waveform, with a fade handle at OUT. HEAR plays it.
2. **Pitch.** The note shows as a name and cents, for example "A3 +12 ct", with DETECTED or UNSURE. A stepper (semitones and cents) or typing a note name corrects it, and the badge then says SET BY YOU.
3. **Key and scale.**
   - The default is the other deck's key. NEXT TRACK takes the key of the next track in the set or queue.
   - SCALE offers major and minor (with HAPPY and SAD as hints that can be switched off), and a few modes behind MORE.
   - The scale's notes light up on the keyboard strip.
4. **Notes on pads.**
   - The notes of the scale fill the pads, rooted at the lowest note, with an octave switch.
   - SNAP TO SCALE is on by default; off allows every semitone.
   - A pad shifted more than 7 semitones from the source carries a warning, with a hint to pick a note nearer the target.
   - Short shifts use resampling, which keeps the attack. Long notes keep their length with the engine's pitch shifter.
5. **Arrange.** Either way, the result is the same strip: up to 32 steps over up to 8 bars on the master grid (1/4, 1/8, 1/16).
   - REC and play the pads or the keyboard.
   - Or type the notes, such as "A3 C4 E4 D4".
   - A tap moves or deletes a step.
6. **Render.**
   - SINGLE NOTES makes one hot cue per distinct note, for playing them by hand.
   - MELODY makes one cue per note, in order, ready as a transition.
   - The file is a new WAV at the chosen tempo (master or the source track) with a beat grid. The original track is never touched.
   - Before rendering, a line says what the stick formats take: the cue count and whether the file fits. A limit shows before the render, not after.
   - As built ([75]): SINGLE NOTES puts each distinct note of the strip, low to high, on the downbeat of its own bar, cut after three beats; MELODY plays the strip in time from the first downbeat over its whole bars, rests silent and holds kept. Each cue is named by its note and lands on its step of the grid. The grid is constant and locked.
   - The limit is the 32 hot cues (pads A–H in four banks) the library, the engine and both stick formats we write carry; a full strip of 32 notes fits. Cues after H sit on later pad banks, and the line says how many. How players show them is untested on real players.
   - Files go to `created` beside the library and into the library as tracks with their grid and cues. RENDER then reads READY in green until the notes, layout or tempo change.
7. **Into the set.** TO PLAYLIST and TO STICK use the normal library and sync. The file also lands in a CREATED list. SEND TO PERFORM puts the notes on the MELODY pad mode and into a sampler bank of the same name.
   - As built ([76]): every rendered track is marked in the library (not by its tags or comment, which stay yours to edit) and listed under CREATED in the browser tree, newest first; it sorts, searches and loads onto decks like any list.
   - TO PLAYLIST is enabled once RENDER reads READY (before that its tooltip says to render first). It opens the browser's own add-to-playlist menu: the list added to last, every playlist and set, or a new playlist named after the file. Each is one undo step.
   - There is no separate TO STICK: a stick gets the file like any track, through a playlist it syncs. After adding, the status line names the registered sticks that take it with their next sync, or says that none does.
   - As built ([77]): SEND TO PERFORM sends the strip's different notes in the order they first play, the first 8 of them, low to high (the status line says how many found no pad); with an empty strip it sends the 8 pad notes. They fill pads 1–8 of the sampler's fifth bank, MELODY, and the octave above fills pads 9–16. The notes come from the same render cache as the pads and RENDER, and are kept as files in `created/melody` so the bank is back after a restart like any loaded sample. SEND TO PERFORM then reads SENT in green until the notes change.

## Drums

```
├ KIT ───────────────┬ PADS (bank 1) ─────────────────────────────────────┤
│ TIGHT ▾  MY SAMPLES│ [KICK][SNARE][HAT][OPEN] [CLAP][TOM1][TOM2][RIDE]  │
│ PITCH ◯  DECAY ◯   │ [ 9 ][ 10 ][ 11 ][ 12 ] [ 13 ][ 14 ][ 15 ][ 16 ]   │
│ TONE ◯             ├ STEPS (KICK)   SWING 0 %   HALF TIME ○            │
│                    │ |■···|■···|■···|■·■·|   ▶ PLAY  ● REC  CLEAR       │
├────────────────────┴────────────────────────────────────────────────────┤
│ RESULT  pattern 1   SEND TO PERFORM   RENDER LOOP 4 BARS   SAVE PATTERN  │
```

- Built-in kits are synthesised, so they are the same on every platform and carry no third-party samples.
- LOAD SAMPLE takes a sound from the library.
- Kits, samples and patterns share the sampler's slot store and its 16-step sequencer, so a pattern saved here plays from the PATTERN pad mode in Perform, and the other way round.
- The step row follows the master tempo. HALF TIME is there for drum and bass, where material is often read at half the tempo.
- RENDER LOOP writes a bar-exact WAV with a hot cue per sound.
- As built ([80]): the 16 pads are the sampler's sixth bank, DRUMS. Pads 1–8 hold the kit, rendered and kept as files in `created/drums` (rendered again once the knobs rest after a change, each pad keeping its steps); pads 9–16 take samples: LOAD SAMPLE puts the track open in Prepare's player (or the file selected in a folder) on the selected sample pad, else the first empty one. A pad press selects the pad: the knobs set the kit pad selected last, and STEPS shows the selected pad's 16 steps (a short line marks steps other pads play). A tap or Return sets a step; the row is one Tab stop and the arrow keys move along it. The steps are the sampler's sequencer pattern of that slot, so the sampler panel in Perform shows and edits the same. PLAY runs the sequencer at the master tempo, step 1 on the next bar while a deck plays (NEXT BAR until then), else at once; it plays through the sampler. REC ON is the sequencer's recording: a pad pressed while the steps play lands on the nearest step. SWING (0–75 %, in fives) moves every off-beat step that share of a step late; 33 % is a triplet shuffle. HALF TIME makes a step an eighth note, so the 16 steps take two bars. Both belong to the sequencer and apply to every slot. CLEAR empties every pad's steps, with one UNDO. SAVE PATTERN stores the steps with the swing and half time on a PATTERN pad in Perform (the same pad each time; the first free one at first) and reads SAVED while it matches; SEND TO PERFORM saves it and puts the sampler on the DRUMS bank (SENT). The PATTERN pads and the sequences on them are now kept with the sampler, so they survive a restart.
- As built ([81]): RENDER LOOP writes the pattern as a new WAV file in `created` beside the library, and adds it to the library and the CREATED list like a rendered melody.
  - **What plays:** every pad with steps: the kit's sounds as their knobs set them, and the samples on pads 9–16 as their sampler slots play them (mode, gain, and SYNC to the render's tempo). Swing and half time apply as in the sequencer.
  - **How long:** a stepper beside RENDER LOOP chooses 1, 2, 4 or 8 bars (4 at first, at least 2 at half time, kept with the kit). The 16 steps repeat until the bars are full. The file is exactly that many bars long: the bars at the tempo, rounded to a whole sample at the engine's rate.
  - **Tempo:** the master tempo, the one PLAY runs the steps at. While a deck plays, that is the master deck's tempo; while none plays, it is the master clock's last tempo (120 BPM until a deck has played). The status line says which, and it names the file without its path.
  - **Seamless:** the engine plays the pattern offline with its own step times and voices, so the file sounds like the steps once they have run for a while (a test compares it with the engine's second bar). Each pad is one voice, as in the sampler: a new hit fades the previous one out over 128 samples. A sound that rings past the end wraps to the start, unless the pad's own first hit cuts it there. So the last open hat or clap is heard over the first beat, as when the pattern loops live.
  - **Grid, cues and key:** a locked, constant grid at the tempo with its first downbeat on sample 0. A memory loop, LOOP, spans the whole file. Each sound that plays gets one hot cue, A, B, C … in pad order, at its first hit and named after its pad (KICK, SNARE …, or the sample's name). At most 16 sounds, so the 32 hot cues of the stick formats always hold them, and the RESULT bar shows the same stick-formats line as MELODY.
  - **No key:** the key stays empty. Analysis never gives a track rendered from CREATE a key: it keeps the key CREATE set, or none.
  - **Same input, same file:** the same pattern renders the same bytes. A new render never replaces a file. RENDER LOOP reads READY in green until the steps, a sound, the swing, half time, the bars or the tempo change.
  - **Loud patterns:** a pattern that would peak above full scale is turned down as a whole, never clipped.

## Theory

- The keyboard strip looks the same everywhere: the root in bone, the scale's notes filled, chord notes marked, and notes out of key dimmed. A click plays a note.
- **FITS:** the notes and chords of the chosen key, and which of them fit the track playing now.
- **THEORY view:**
  - Both decks' keys, and the neighbouring keys that move smoothly.
  - A check of a typed melody ("all notes fit", or which do not).
  - Three short lines in plain words on why the notes fit.
- Keys are shown in one language, as note names. The wheel notation is a small label beside them, as in the rest of the app.

## Playing it live

- **MELODY pad mode** on each deck: 8 pads play the sent notes, 8 more play the octave or the chord notes. They follow QUANT and SYNC like sampler pads, and are mappable for controllers.
- The notes sit in a sampler bank named MELODY, so the sampler's controls, sync and quantise apply.
- As built ([77]): MELODY sits next to SAMPLER in each deck's pad modes. Pads A–H play the notes sent; the ‹ › beside the mode switches them to the octave above (an octave rather than chord notes: it needs no chord choice and the notes are already rendered alike). Every deck plays the same notes. A pad press is the sampler's own trigger, so QUANT starts the note on the next beat of the master tempo and the slot's mode and gain apply. A note carries no tempo of its own, so the sampler's SYNC has nothing to stretch: the note keeps its pitch at any master tempo. Before anything was sent the pads stay outlined and the bar says "Send notes from CREATE". On a controller, `deck.pad_mode` with `mode = "melody"` selects it and `deck.pad` plays its pads; that is tested against the mapping, not on hardware.
- A rendered file loads on a deck like any track, with its hot cues.

## Never empty, and a MIDI keyboard

- As built ([82]):
  - **Built-in tone.** Until a note is picked (or when the picked note's track is gone), SOURCE holds **TONE**: an A3 the app synthesises (four harmonics, about a second, the higher ones dying away sooner), marked **BUILT-IN**. It needs no track, so the pads, the keyboard strip, HEAR, RENDER and SEND TO PERFORM work on a first start with an empty library. **TONE** picks it again at any time. It is kept like a picked note. A file rendered from it is named "Tone (…)".
  - **Starter melodies.** An empty strip offers **STARTERS** in ARRANGE and in THEORY's CHECK: **RISE**, **CALL**, **STEPS** and **PULSE**. They are two bars of eighths, written as scale degrees for the app (broken chords, scale steps, a repeated root), not taken from songs. A starter plays in the key in use, from the root at or below the source note, so it lies over the pads. With no key chosen it brings its own scale on the source note's root and sets it as the key, shown as **STARTER KEY** (it stays until OTHER DECK, NEXT TRACK or a key set by hand replaces it). The strip's count and the RESULT bar read "STARTER RISE" while the notes are as the starter put them; the grid and LOOP do not count as a change.
  - **Drum starters.** Empty drum steps (no pad of the DRUMS bank has steps) offer **STRAIGHT**, **BROKEN** and **SPACIOUS** on the kit pads (kick, snare, hats; SPACIOUS adds the open hat), through the sampler's own sequencer. The RESULT bar reads "STARTER STRAIGHT" while the steps are exactly the starter's.
  - **Never over your work.** A starter goes only onto an empty strip or empty steps; otherwise the status line says to CLEAR first. CLEAR brings the starters back.
  - **MIDI keyboard.** A note-on from any MIDI input without a controller mapping plays CREATE while CREATE is the mode. The controller layer opens such inputs only then (and for MIDI learn, the monitor and the guided setup, as before) and sends their note-ons as note events; a port with a mapping, a DJ controller or the one given to the lights, never sends them. In MELODY and THEORY a note (C1 to C7) plays as a key of the strip does, from the source note, and goes into the strip while REC is armed (step by step, or on the master grid while a deck plays). In DRUMS notes 36 to 51 play pads 1 to 16 (with REC, onto the nearest step). Velocity, note-offs, sustain and pitch bend are not used. The first note from a device names it in the status line. Tested with simulated note events only, not with a hardware keyboard.
  - **Guide.** The CREATE guide (`docs/guide/en/create.md`, `docs/guide/de/create.md`) covers MELODY, DRUMS, THEORY, rendering, CREATED, sending to Perform, the MIDI keyboard and the limits, with pictures of the first open, MELODY, DRUMS and THEORY.

## Risks

- **Pitch detection:** a long pad tail, a detuned or noisy note can be misread. The detected pitch is always shown with UNSURE when unsure, and can always be set by hand.
- **Sound of large shifts:** a warning above 7 semitones, and resampling for short notes.
- **Club players:** how many hot cues a player shows, and how it treats a short file with a grid, differs by model. The check uses what the stick formats store. Whether it plays as intended stays open until it has been tried on real players.
- **Scope:** the "what it is not" list is binding.
- **Two of anything:** one engine, one sample store and one sequencer. CREATE is a different view on them.
- **Rights:** a note cut from someone else's track stays the user's matter. The app makes no claims either way.

## First checks with real DJs

Before [72] and [75] are finished, two questions should be asked:

1. How large a shift still sounds good?
2. How do rendered files with many hot cues behave on at least two club player models?

Those two facts decide how simple the tab can stay.
