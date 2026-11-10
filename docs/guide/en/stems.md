---
title: Stems
description: Split tracks into vocals, instruments, drums and bass on your own computer, and mute, level or add effects to each part.
keywords: [stems, stem, separation, separate, split, vocals, vocal, acapella, instrumental, drums, bass, parts, stem fx, karaoke, mute, stem separation, solo, keep only, whole playlist, set, before the gig, stems column, queued]
features: [stems-separation, stems-mute-ui, stems-levels, stems-batch-prepare, stems-model-download, stems-model-setup, stems-status-on-deck, stems-pi-playback, pad-mode-stems, analysis-vocal-position, stems-list-prepare, stems-library-column, stems-key-state, stems-solo]
---

OpenDJDecks can split a track into four stems: vocals, instruments, drums and bass. This happens on your own computer, in the background, and nothing leaves the machine. The first time, a model is downloaded once; after that it works offline.

![The stems of a deck: VOCAL, INST, DRUMS and BASS](../img/stems.png)

## Separate tracks

There are two ways:

- **On load:** once the [model is installed](#the-one-time-model-download), load a track on a deck. It plays as usual; the strip above the pads shows **SPLIT** and a percentage while the stems are being made. The stem keys stay in place, dimmed, until the stems exist, and you can play the whole time.
- **Ahead of time:** select tracks in Prepare, right-click and choose **Separate stems** (`⌥⇧⌘S`). They are done in the background and the band at the bottom shows the progress.

## Stems ready before the gig

From version 0.8 you can prepare a whole set. Open the playlist or set, right-click any track and choose **Separate stems for the whole list**, or use **File › Separate Stems for This List** (`⌥⇧⌘P`). Every track of the list is queued in the background; tracks that already have their stems are skipped. While a deck plays the queue waits, so separating never competes with your mix, and a track that is on a deck when its stems are done takes them without a gap.

![The STEMS column with ready and queued tracks; deck 1 separating its track, deck 2 waiting its turn](../img/stems-ahead.png)

To see where each track stands, turn on the **STEMS** column (right-click the column header):

- **READY** with a solid green line: the stems are on this computer.
- **QUEUED**, or a percentage while it is being separated, with a dashed yellow line.
- **FAILED**, struck through: the status line says why.
- Empty: not separated.

The stem keys on a deck say the same: **QUEUED** with a dashed yellow line along each key while the track waits for its turn, and the keys fill from left to right while it is separated. A track separated by a deck itself shows in the column too.

If you load a track without stems where stems were in use (the track it replaces had them, or another deck plays with stems), the deck warns you: the strip says **NO STEMS** in yellow with a yellow line under the keys, and the status line tells you what to do. Press a stem key to separate it there and then, or prepare the list before the next gig.

## Use the stems on a deck

Each deck has **VOCAL**, **INST**, **DRUMS** and **BASS** under the waveform.

- Click a key to mute or unmute the stem. Filled means muted.
- Right-click a key to **solo** it: the other stems go quiet and the strip says **SOLO**. Right-click again and they come back exactly as they were. A click on any stem key also ends the solo.
- ⇧-click a key to **keep only this** stem: the others are muted and stay muted, as if you had clicked each of them.
- Turn the wheel over a key to set the part's level; the line under the key shows it.

On a controller, `deck.stem_solo` solos a stem while the button is held and `deck.stem_keep` keeps only that stem ([Controller mappings](../../controller-mappings.md)). The solo and keep-only keys are tested in the app; they have not been tried on a hardware controller yet.

For hands-on mixing, press **STEM** on a mixer channel: **HI** becomes the vocals, **MID** the instruments and **LOW** the drums. The **STEMS** pad mode gives you the part mutes and stem effects on the pads.

On the overview, spans where the vocals sing are drawn as a band along the top, so you can see where to mix.

To run an effect on one stem only, see [Stem FX](effects.md#stem-fx).

## Stems on a stick and on the Raspberry Pi

When a stick uses the OpenDJDecks format, the stems are copied to it with the sync. Tracks loaded from that stick use them, so the Raspberry Pi plays stems that were separated on your laptop without doing the work itself.

## The one-time model download

From version 0.7, OpenDJDecks asks before it downloads anything. The separation model (about 158 MB) is not part of the app. The first time you press a stem key, switch a mixer channel to **STEM** or choose **Separate stems** without the model, a dialog tells you what is needed and offers **DOWNLOAD NOW** and **NOT NOW**. Your music stays on your computer; only the model comes down, once, and from then on stems work without internet.

While a deck plays or a mix records, no dialog covers the screen. A slim notice under the top bar offers the download, and the dialog follows when the music stops. **NOT NOW** holds for the rest of the session: nothing asks again by itself, and pressing a stem key brings back the notice with the download on it.

The download runs in the background with progress in the same notice and you can keep playing; **CANCEL** stops it, and the next **DOWNLOAD** carries on where it stopped. The model's checksum is verified before it is used. Everything is in **Preferences › Analysis › Stems**, including the size on disk and **REMOVE MODEL**. Removing the model does not remove stems you already separated.

The Raspberry Pi and the kiosk do not download the model; their stems are separated on a desktop computer and arrive on the stick.

## The download stopped and now stems do not work

From version 0.7, a download that stopped never leaves stems broken for good, and you do not have to do anything by hand. The notice under the top bar tells you what happened and offers **RETRY** and **HELP**. If it is gone, or stems still do not work:

1. Open **Preferences › Analysis**. In the **Stems** section the status says **Incomplete** or **Damaged**.
2. For **Incomplete**, press **DOWNLOAD**: it picks up where it stopped.
3. If that does not help, or the status says **Damaged**, press **DELETE AND DOWNLOAD AGAIN**. It removes every partial, damaged or old model file first and starts from nothing.
4. Wait until the status says **Installed**, then press a stem key again. Tracks already on a deck are separated automatically.

If the notice said **No internet connection**, check your network first. If it said there was **not enough free disk space**, make room and press **RETRY**. Removing or downloading the model again never touches stems you already separated.
