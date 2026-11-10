---
title: Tempo FX, Channel FX, remix and hold FX
description: Two Tempo FX units with up to three chained effects, exit FX, Channel FX on the channels, remix effects and effects on single stems.
keywords: [effects, fx, tempo fx, channel fx, echo, delay, reverb, flanger, phaser, roll, filter, trans, vinyl brake, backspin, echo out, exit fx, build up, breakdown, reverse roll, hold fx, stem fx, tap, multi, drop, riser, xy, xy pad, panic, all fx off, tail, reverb tail, 16 bars]
features: [fx-tempo-fx, fx-tempo-fx-controls, fx-tempo-fx-catalogue, fx-channel-fx-select, fx-exit-fx, fx-hold-fx, fx-stem-fx, fx-panel, fx-channel-fx-extended, fx-remix, pad-mode-hold-fx, fx-level-large, fx-panic, fx-xy, fx-build-up-bars, fx-tails]
---

Press **FX** in the top bar (`⌥⌘F`) to open the two Tempo FX units above the decks.

![The FX panel with two units](../img/teaser-fx.png)

## Tempo FX

Each unit, **FX1** and **FX2**, picks an effect (the list or **‹ ›**), a beat length and a target:

1. Choose the effect by clicking its name for the list, or with **‹ ›**.
2. Choose the length with the **BEAT** keys, from 1/8 to 16 beats. It follows the deck tempo; **TAP** sets your own tempo (three taps) and **AUTO** returns to the deck.
3. Assign it: the keys **1** to **4** for channels, the sampler or **MASTER**.
4. Set the level and **PARAM** (the effect's second control, such as filter direction or pitch), then press **ON**. The level is the large percentage beside the effect, with a bar under it: drag along it or turn the wheel over it. It lights in the effect's colour while the effect is on.

**×3** chains three effects in one unit: each slot has its own effect, beat, depth and on/off.

### Effects

Echo, Delay, Ping Pong, Reverb, Flanger, Phaser, Roll, Slip Roll, Filter, Trans, Pitch, Vinyl Brake, Helix, Spiral and more, including the remix effects below. On a controller, its effect names map to the closest one.

### Remix effects

In the same list:

- **BUILD UP** cuts the lows higher and higher over its length, while a noise sweep rises. **PARAM** sets the noise.
- **BREAKDOWN** closes the highs and dips the level.
- **REVERSE ROLL** repeats the captured beats backwards.

### Exit FX

**BRAKE**, **BACKSPIN** and **ECHO OUT** act on a unit's target while held, then the music comes back. On a controller the release button is echo out.

## For the drop

From version 0.8 each unit has what you need around a drop.

![The FX panel: a 16-bar build-up with a reverb tail on FX1, FX2 as an XY surface](../img/fx-drop.png)

- **BUILD 8 / 16 / 32** starts a build-up that rises until the next line of 8, 16 or 32 bars on the track's grid and then lets go, so the drop comes in clean. With less than a bar left before that line it takes the one after. The unit counts the bars left (**-6 BARS**) and the lit key fills as the build-up rises. Press the lit key again to stop early. The bars are counted on the grid of the deck the unit follows (the master deck on **M**); a grid that is off puts the peak off too.
- **TAIL** chooses what rings out after you switch an effect off: **TAIL OFF**, **TAIL ECHO** (the sound at release repeats every beat and fades) or **TAIL REVERB** (it blooms into a reverb). It matters for effects without a tail of their own, such as roll, flanger, filter or the build-up; echo, delay and reverb ring out anyway.
- **XY** turns the unit into one large surface. Across is the effect's parameter, up its level. Press anywhere to play the effect while you hold it; letting go switches it off and its tail rings out. An effect that was already on stays on. **XY** in the corner brings back the controls. On windows narrower than 1400 px the **XY** key sits on the second line of the unit.
- **ALL FX OFF** at the right end of the panel stops every effect at once: both units with all their slots and tails, exit and hold FX, and the **CH FX** knobs, which return to the centre. It is also `⇧Esc` on the keyboard and `fx_panic` in a controller mapping.

These were tested with generated tracks and in the app's tests, not yet by ear at a gig or from a hardware controller (`tempo_fx.build_up` and `fx_panic` are in no shipped mapping yet).

## Channel FX

**‹ FILTER ›** under the mixer selects the Channel FX that every channel's **CH FX** knob controls: Filter, Noise, Space, Crush, Echo, Pitch, Dub Echo and Sweep. Your choice is remembered. `⇧J` and `⇧S` step through them.

Auto Channel FX is one of these: a single knob with low-pass and echo to the left, high-pass and reverb to the right.

## Hold FX

The **HOLD FX** pad mode puts eight presets on the pads, beat effects and release effects, active while held. **‹ ›** switches between two pages.

## Stem FX

Under a unit's channel assign, the stem keys **ALL**, **VOC**, **DRUM**, **BASS** and **INST** limit it to one stem of the channel: the unit works on that stem only, and it rejoins the channel before the EQ and fader. This needs the track to have [stems](stems.md).
