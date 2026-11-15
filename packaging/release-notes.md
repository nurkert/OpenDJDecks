OpenDJDecks 0.10.1, the open-source DJ software for macOS, Linux, Windows and the Raspberry Pi. Website and guides: [opendjdecks.de](https://opendjdecks.de). Still a beta.

## New in 0.10.1: DDJ-FLX10, signed updates, low-CPU mode

Much of the controller work in this release comes from playing on a real DDJ-FLX10. Good to know lists what has not been tried on the unit yet.

### DDJ-FLX10

- **Plug in any time:** on macOS a controller plugged in after the app started is now found.
- **One press, one deck:** with deck 3 or 4 active, or with SHIFT held, a button no longer acts on two decks at once.
- **Faders as they stand:** on connect the app asks the controller where its faders, knobs and switches are, so a fader at the bottom means a silent channel from the start.
- **Jog wheels at the right speed:** turning the platter now moves the track as far as you turn it. SHIFT + jog searches through the track.
- **Spinback:** letting go of a platter that still spins keeps the track turning until the platter settles near play speed or stops.
- **The 2-deck view follows the controller:** it shows the decks that are active on the unit.
- **Stem buttons** are in the right order and light while their part is heard.
- **Channel FX filter:** reworked to sound like a club mixer filter, with no effect around the centre, a smooth sweep, gentle resonance and no zipper noise.
- **Loops:** LOOP IN marks its point on the waveform. While a loop runs, IN halves it and OUT doubles it, and CUE/LOOP CALL does the same. 4 BEAT/EXIT leaves a running loop with one press.
- **Quantized loops:** with quantize on, LOOP OUT ends on a whole beat from the in point and an auto loop starts on the nearest beat. With quantize off loops stay exact.
- **Jog displays:** they now show the track's BPM and time, and the speed change.
- **Part iso:** SHIFT + headphone CUE switches a channel's EQ knobs to drums, instruments and vocals.
- **FX PART SELECT** puts the Sound Color FX on a single stem part.
- **More of the unit mapped:** the second pad page, the SHIFT layers and the indicator LEDs, taken from the manufacturer's published MIDI list.

### Browsing from the controller

- **The whole lists column:** the encoder steps through everything in it, from the collection and playlists to Explorer folders, sticks, histories, recordings and CREATED.
- **In and out:** scrolling leaves folders closed. A press or a deck's LOAD opens a folder or hands over to a list's tracks, and BACK goes one level up, all the way to the source buttons (ALL, LISTS, SIMILAR and so on).
- **The lists tree** scrolls one row past an edge instead of jumping a page, and the selected row always comes into view.
- **Mouse and controller agree:** a track clicked with the mouse is the one LOAD loads.

### Any controller

- **Guided setup goes further:** after the core controls it asks MORE CONTROLS or SAVE NOW. The further steps cover SHIFT, hot cues E to H, loop in and out, the 4-beat loop, beat jump, key lock, slip, CH FX knobs, headphone mix, browse back and effect on/off and level. Each one can be skipped.
- **Why a deck is silent:** after connecting, a channel stays closed until the controller reports its fader or the fader moves, and PLAY on such a deck tells you so.
- **XF:** a crossfader on/off key in the mixer, above AUTO. Mappings can use it as `crossfader_toggle`.
- **Six new tempo effects:** low cut echo, multi tap, jet, stretch, Shepard saw and Shepard tri, so every position of the DDJ-FLX10's effect selector has its own effect.
- **DDJ-FLX4:** tempo direction, the call buttons and the SHIFT layer lights corrected against the published MIDI list.

### Updates and computers

- **Signed updates:** every release now comes with a signature for its checksum list. From this version on, the app checks that signature before installing an update in place, and a release without a valid one opens its release page instead. The guide explains how to check a download yourself.
- **Low-CPU mode** in Preferences > View > Computer load: 30 frames a second, waveforms that move in steps, no decorative motion, stem separation on half the cores and a larger audio buffer from the next audio start. Engine timing, controllers and their lights, the light show and analysis are unchanged. Always on in the Lite profile on the Raspberry Pi.
- **Minimum requirements** are now listed in Getting started and on the download page.
- **Quit guard:** closing the window while a deck plays asks first (not in kiosk mode).

### Fixed

- **DDJ-FLX10:** tempo direction, the loop buttons, the browse press, deck select and the Sound Color FX order corrected.
- **Vinyl mode:** no longer switches itself off on the DDJ-FLX10.
- **Play history:** a track enters the history once per load, not every time the audible deck changes.
- **Keylock:** after leaving or doubling a loop, the audio no longer lags one loop behind the playhead.
- **Waveforms:** a track loaded on a deck is analysed right away instead of waiting until every deck stops.
- **Waveform drawing** does less work while decks play, for everyone.

## Good to know

**Updating from 0.10.0:** 0.10.1 is the first version that checks the signature before it installs an update in place. Copies of 0.10.0 do not check it yet, so this one update is installed the old way.

On a real DDJ-FLX10 these were tried: hot-plug, the deck 3/4 and SHIFT fix, faders, jog wheels and spinback, the crossfader, the 2-deck view, stem button lights, the filter sound, browsing to the source buttons with folders closed, the LOOP IN marker and halving, doubling and leaving loops, and BPM and time on the jog displays.

Not yet tried on the unit, including:

- the speed value, key, position ring and cue marker on the jog displays,
- part iso with SHIFT + headphone CUE,
- quantized loops and the keylock loop fix,
- one-row scrolling in the lists tree and the question when quitting while a deck plays,
- FX PART SELECT, the tempo direction, the Sound Color FX order and vinyl mode staying on,
- the second pad page, SHIFT layers, indicator LEDs and the new tempo effects on the selector,
- the DDJ-FLX4 changes.

Known on the DDJ-FLX10: the FX PART SELECT lights do not show the chosen part yet, and the key on the jog displays can be wrong. The jog displays show values such as BPM and time, no waveform or artwork. The low-CPU mode has been measured on one computer only. Try your setup before a gig.

OpenDJDecks is an independent project, not affiliated with or endorsed by AlphaTheta Corporation or any other manufacturer. rekordbox and Pioneer DJ are trademarks of AlphaTheta Corporation; names are used only to state compatibility.
