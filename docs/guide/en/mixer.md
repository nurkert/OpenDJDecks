---
title: The mixer
description: Trim, EQ, channel FX, faders, crossfader, headphones and master, and the options for each.
keywords: [mixer, eq, trim, gain, fader, crossfader, headphones, phones, cue, master, level, limiter, meter, vu, booth, mic, microphone, talkover, auto fader, stem button, auto gain]
features: [mixer-trim, mixer-eq, mixer-eq-mode, mixer-channel-fx-knob, mixer-channel-fader, mixer-fader-curve, mixer-crossfader, mixer-crossfader-curve, mixer-crossfader-assign, mixer-headphones, mixer-master-meters-limiter, mixer-master-cue, mixer-booth, mixer-auto-channel-fx-fader, mixer-mic-aux, mixer-auto-gain-toggle, mixer-stem-button]
---

The mixer stands between the decks. Press **MIXER** in the top bar (`⌥⌘M`) to hide it when your controller does the job; the decks then meet in the middle.

## A channel

From top to bottom: **TRIM**, **HI**, **MID**, **LOW**, **CH FX** (the Channel FX knob), **STEM**, **CUE** and the channel fader with its meter. Double-click a knob to reset it.

- **TRIM** sets the level of the channel. **Auto gain** in **Preferences › Controller › Mixer** levels tracks by their analysed loudness first. Each track can be corrected by ±0.5 dB steps in the [GRID panel](beatgrid.md#fix-a-grid-while-the-track-plays).
- The **EQ** knobs default to an isolator. **Preferences › Controller › Mixer › EQ** switches between **ISOLATOR** and **CLASSIC EQ**.
- **CH FX** drives the selected Channel FX; see [Effects](effects.md#channel-fx).
- **STEM** turns the channel's HI, MID and LOW knobs into controls for the vocals, the instruments and the drums of that track. The middle position is full level; the normal EQ stays as you left it. See [Stems](stems.md).

## Crossfader

Below the mixer, the crossfader blends between its sides. The small **1** / **2** / **3** / **4** keys left and right send a deck to side A or B; press again to put the deck straight through.

**SMART** beside the crossfader turns on Auto Fader: the crossfader also swaps the basses of the two sides and keeps the decks on it in tempo with the master.

Curves live in **Preferences › Controller › Mixer**: the **Channel fader curve** (linear, smooth, sharp) and the **Crossfader curve** (constant power, linear, cut).

## Headphones and master

**PHONES** has the **MIX** between cue and master and the **LEVEL**. Press **CUE** on a channel to listen to it. On a controller, the master cue button listens to the master.

**MASTER** sets the output level with **−** and **+**, or the slider, and has a meter and a safety limiter that is always on. The record button and time sit next to it; see [Recording](recording.md). The booth output is set with the controller's booth knob.

## Microphone and aux

**MIC** in the bar below the mixer switches on an input, with the level, a meter and an input device selector. **TALK** ducks the music while you speak. The mic signal goes through the mixer, so Tempo FX and the recording include it.
