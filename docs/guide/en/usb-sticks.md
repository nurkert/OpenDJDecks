---
title: USB sticks and sync
description: Register a stick once. From then on every change reaches it automatically and incrementally, ready for OpenDJDecks on another computer or the Raspberry Pi.
keywords: [usb, stick, sticks, usb stick, sync, synchronise, export, export stick, register, eject, rebuild, stick status, device, sync manager]
features: [export-stick-check, export-sync-all, export-club-usb-compat, export-auto-sync, export-device-registry, export-play-from-stick, export-native-format, export-club-usb, export-club-usb-mysetting, cues-from-players, import-club-usb]
---

The stick is a mirror of your library. Register it once with the lists that belong on it, and every changed cue, playlist or beatgrid reaches it within seconds, while it is plugged in. There is no export step to remember.

![The USB stick board with a registered stick](../img/usb.png)

## Register a stick

1. Plug in the stick. It appears under **DEVICES** in the tree (or on the **USB** rail).
2. Select the playlist, set or folder that should go on it.
3. Press **REGISTER WITH …**. The first sync writes everything once; afterwards only changes are written.

The stick board shows its state, what is pending, and the progress of the sync.

- **SYNC NOW** (`⌘S`) syncs at once. Syncing starts by itself anyway, a few seconds after any change.
- **REBUILD STICK** writes the whole stick again from your library.
- **EJECT** (`⌘E`) ejects it safely. Wait for that before you pull it out.
- **FORGET** stops keeping that stick in sync.

## Sync all sticks at once (from version 0.6)

Have several sticks to fill, say a set of backups for a tour? Plug them all in and open the stick board:

1. Press **SYNC ALL STICKS**. Every stick that is plugged in starts syncing at the same time, each with its own progress line.
2. Turn on **Eject when done** and each stick ejects itself as soon as it has finished, so you can pull them out one by one. This choice stays on until you turn it off.

If something goes wrong with one stick, for example it is full, the error shows on that stick only and the others keep going. A stick that failed is not ejected, so you can read the message and try again.

The small sync arrow next to **DEVICES** in the tree does the same with the same setting.

## What is on the stick

Under **CONTENT — LISTS ON THIS STICK** you choose the lists. The stick holds your music with its cues, beatgrids, phrases and, if you have them, the stems. Sticks are read by OpenDJDecks, for example on another computer or the [Raspberry Pi unit](raspberry-pi.md). To also play a stick on club players, turn on the Club player USB format below.

## Playing from a stick

Open a stick in the tree to browse and load from it directly, read-only. This is what the Raspberry Pi unit does all day.

## Play on club players (Club player USB)

From version 0.6, a stick can also carry the Club player USB format, which is compatible with Pioneer DJ players. You choose this per stick: select the stick and turn on **Club player USB (export.pdb)** under **FORMATS**. OpenDJDecks' own format stays on by default and you can keep both.

With the format on, every sync also writes the database and the analysis the players read, with your playlists, cues, beatgrids and waveforms, and takes over your default player settings (quantize, auto cue, tempo range, jog mode, language). Only changed tracks are rewritten. Ogg and Opus files are left out because players cannot play them.

### Maximum compatibility for older club players (from version 0.6)

Older club players often cannot read FLAC or M4A files (AAC and ALAC). If you carry a stick for those, turn on **Maximum compatibility (WAV for older club players)** for it. The option appears under **FORMATS** once **Club player USB** is on, and it applies only to that stick.

With the option on:

- Those tracks are written to the stick as 16-bit WAV files, in a file with the `.wav` ending. Very high-resolution tracks above 48 kHz are also brought down so they play everywhere. MP3 and WAV tracks are left as they are.
- Your original files in your library are never changed.
- A track is only converted once. It is converted again only if the original file changes; editing cues or a beatgrid just updates the stick's analysis.
- If you turn the option off, or a track leaves the stick, its WAV file is removed from the stick.
- A WAV file is larger than the original, so plan a little more room on the stick.

If a stick also carries the OpenDJDecks format, it keeps the original next to the WAV, so nothing changes for playing that stick in OpenDJDecks.

### Cues and histories from the players

If a player changes cues on the stick or writes a history, OpenDJDecks does not overwrite that. The next sync lists it under **CHANGES MADE ON PLAYERS — REVIEW** on the stick board. **TAKE STICK CUES** moves the player's cues into your library, **KEEP LIBRARY** keeps yours and writes them to the stick again; a history can be imported as a playlist or dismissed.

### Import a stick from other software

From version 0.6, **File › Import a Club Player USB…** reads a Club player USB written by other software: tracks, tags, cues, beatgrids, phrases, playlists and the history. Choose the stick's root folder. Nothing on the stick is changed; imported lists appear in a **Library import** folder.

## Check a stick before a gig

Select the stick under **Devices** and press **CHECK STICK**. OpenDJDecks reads the stick back and compares it with your library:

- for every track: the audio file is there and opens as audio (**FILE**), the cues are yours (**CUES**), the beatgrid is yours (**GRID**);
- for the whole stick: every playlist holds the library's tracks in the library's order, the formats you chose are there, and no sync was cut off.

Both formats are read back when the stick carries them: the OpenDJDecks library and the Club player USB export. Each mark uses the line forms you know from the stick list: solid means as in the library, dashed means changed since the last sync, struck means missing or unreadable, half means there is nothing to compare (a track without cues). Problems are listed first. The check ends with **Ready** or **Not ready**; a dashed mark or a playlist that differs goes away with **SYNC NOW**.

![The stick check on the Devices page](../img/stick-check.png)

The check tells you whether the stick holds what the library says. It cannot tell you whether a particular player model accepts the stick; see the next section.

## Compatibility per player family

What a synced stick carries for each kind of player, and how far that is confirmed. The Club player USB format is written from public format documentation. **None of it has been confirmed on real club players yet**; that check is still open on the roadmap. Until then, try a stick on the players you will use before a gig.

| Player family | Hot cues | Memory points | Beatgrid | Playlists | Status |
|---|---|---|---|---|---|
| OpenDJDecks on a computer or the Raspberry Pi unit (OpenDJDecks format) | yes, all banks, with colours and loops | yes | yes | yes, with folders | written and read back by OpenDJDecks itself, in automated tests and by the stick check |
| Current club players reading the Club player USB with extended analysis | yes, with colours (`.EXT`) | yes | yes | yes, with folders | written per format documentation, not confirmed on players |
| Older club players reading only the basic analysis | yes, without colours (`.DAT`) | yes | yes | yes, with folders | written per format documentation, not confirmed on players; FLAC and AAC/ALAC files need **Maximum compatibility** |
| Other software importing the Club player USB | as the importing software reads them | | | | OpenDJDecks' own import reads it back in automated tests; other software not checked |
