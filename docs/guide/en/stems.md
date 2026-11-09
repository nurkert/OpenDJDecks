---
title: Stems
description: Split tracks into vocals, instruments, drums and bass on your own computer, and mute, level or add effects to each part.
keywords: [stems, stem, separation, separate, split, vocals, vocal, acapella, instrumental, drums, bass, parts, stem fx, karaoke, mute, stem separation]
features: [stems-separation, stems-mute-ui, stems-levels, stems-batch-prepare, stems-model-download, stems-model-setup, stems-status-on-deck, stems-pi-playback, pad-mode-stems, analysis-vocal-position]
---

OpenDJDecks can split a track into four stems: vocals, instruments, drums and bass. This happens on your own computer, in the background, and nothing leaves the machine. The first time, a model is downloaded once; after that it works offline.

![The stems of a deck: VOCAL, INST, DRUMS and BASS](../img/stems.png)

## Separate tracks

There are two ways:

- **On load:** once the [model is installed](#the-one-time-model-download), load a track on a deck. It plays as usual; the strip above the pads shows **SPLIT** and a percentage while the stems are being made. The stem keys stay in place, dimmed, until the stems exist, and you can play the whole time.
- **Ahead of time:** select tracks in Prepare, right-click and choose **Separate stems** (`⌥⇧⌘S`). They are done in the background and the band at the bottom shows the progress.

## Use the stems on a deck

Each deck has **VOCAL**, **INST**, **DRUMS** and **BASS** under the waveform.

- Click a key to mute or unmute the part. Lit means audible.
- Turn the wheel over a key to set the part's level; the line under the key shows it.

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
