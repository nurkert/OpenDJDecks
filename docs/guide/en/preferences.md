---
title: Preferences and keyboard shortcuts
description: Audio devices, language and appearance, waveforms, deck defaults, every keyboard shortcut and how to rebind it.
keywords: [preferences, settings, options, audio, output device, buffer, sample rate, language, german, english, theme, appearance, dark, light, colour blind, contrast, scale, zoom, waveform colour, shortcut, shortcuts, keyboard, key binding, keybindings, keys, tasten, belegen, rebind]
features: [analysis-bpm-ranges, prefs-window, prefs-audio-device, prefs-display, prefs-waveform-options, prefs-deck-defaults, prefs-browser-font, analysis-waveform-styles, shell-ui-scaling, shell-colour-blind-contrast, shell-keyboard-layouts, browser-keyboard-shortcuts, audio-coreaudio, audio-linux-backends]
---

Open **Preferences** with the gear in the top bar, or `⌘,` on the Mac and `Ctrl+,` on Windows and Linux. `Esc` closes the window. **FIND A SETTING** at the top searches all pages.

![Preferences](../img/preferences.png)

## Audio

**Output device** defaults to the controller's audio interface when one is connected. **Buffer size** runs from 64 to 1024 samples: lower reacts faster, higher is safer on a slow computer. **Headphones (cue)** can be automatic, outputs 3/4, or off. Audio settings apply the next time audio starts, marked **APPLIES AFTER RESTART**.

On Windows audio goes through WASAPI, on macOS through CoreAudio, and on Linux through ALSA, JACK or PipeWire, whichever you pick here.

## View

**GIG** in the top bar is a preset for playing out: one click sets high contrast, the colour-blind palette and a larger interface at once, and a second click puts back what you had. The arrow beside it adjusts each part. The empty library offers the same **GIG PRESET** button on the first run.

Language, key notation (**Camelot**, **Open Key** or **musical**), the appearance (dark, light or following the system), **UI scale** from 75 to 200 % (applies at once), and the **Waveform colour**: 3-band, one colour or RGB. With **OWN COLOURS** you can pick your own colours for the lows, mids and highs; **STANDARD COLOURS** brings back the default ones.

- **Colour-blind-safe palette** and **High contrast (bright light)** apply at once.
- **Reduce motion** calms the interface.
- **Waveform zoom** sets how many seconds a waveform shows when a track opens; the wheel still zooms.
- Under **Browser** set the **Text size** and **Line spacing** of the list.

## Analysis, Recording

See [Beatgrids](beatgrid.md#if-the-tempo-keeps-coming-out-wrong) for the **BPM range** and [Recording](recording.md) for the recording page.

**BPM range** (from version 0.6) tells the tempo analysis which range your music lives in. Tracks that come out at half or double the tempo are moved into the range you pick:

- **70–180** is the default and suits most music.
- **58–115** is for slow music like reggae or downtempo.
- **98–195** is for faster genres, where a track should not read as half tempo.
- **118–236** is for very fast music like drum and bass or hardcore.
- **150–300** is for music above 200 BPM, such as gabber or speedcore; see [Beatgrids](beatgrid.md#fast-music-and-half-or-double-on-the-deck).

The range only applies to tracks analysed from now on. To change an analysed track, choose **Analyse again** from its menu in the player or list.

## Controller

**Decks** holds the defaults sent to every deck: tempo range, quantize, sync mode (**BEAT**, **TEMPO ONLY** or **PHRASE**, see [Phrases](phrases.md#mix-on-the-phrase)), jog mode and auto cue. **Mixer** holds EQ and fader curves. The rest is in [Controllers](controllers.md).

## Keyboard

The **Keyboard** page lists every action with its binding. Search by action, id or key.

1. Click a binding (or pick one with `↑` `↓` and press Return) and press the new keys. `Esc` cancels, Backspace removes the binding.
2. If a key is taken, the page says by whom. **REPLACE** moves it, **CANCEL** keeps both as they were.
3. **DEFAULT** resets one action and **RESET ALL** resets everything. **IMPORT…** and **EXPORT…** read and write the bindings as a file.

The defaults follow a layout DJs commonly use: decks 1 and 2 use the left and right halves of the keyboard, for example play `Z` and `N`, cue `A` and `H`, sync `Q` and `Y`, hot cues on the number rows, loop in, out and exit on `D F G` and `K L ;`. Decks 3 and 4 sit on `F1` to `F11`. View and layout switches use `⌥⌘` on the Mac.

The full table, per deck and per mode, is on the [Keyboard shortcuts](../../keyboard.md) page.

## Advanced

Backups, the library location, consolidate and relocate, and the update check. Updates only read GitHub's public release list; nothing is sent about you, checks do not run while a deck plays in Perform, and an update never interrupts a performance.
