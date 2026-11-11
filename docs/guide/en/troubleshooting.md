---
title: Troubleshooting and FAQ
description: What to do when a track is missing, a stick will not sync, there is no sound, or a controller is not found.
keywords: [problem, problems, help, not working, error, faq, troubleshooting, no sound, audio, crash, missing, missing track, moved, controller not found, stick not found, slow, dropouts, can't open, cannot open]
features: []
---

## A track shows a struck line

The file is missing or has moved. Press **{n} MOVED** at the top of the list, or **File › Find Moved Files…** (`⇧⌘L`), and point at the folder where the files are now. The tracks are found and reconnected. See [Moved files](library.md#moved-files).

## There is no sound

- Check **Preferences › Audio › Output device**. Automatic chooses the controller when one is connected, otherwise the system output.
- Output device, buffer size and headphone output apply at once. Only a new sample rate needs a restart.
- On Linux, try another of the available audio backends in the same place.
- If a deck has a track but the meters stay flat, check that its fader and **TRIM** are up, and that the crossfader is not on the other side.

## Audio stutters

The status band at the bottom right counts dropouts; its line turns dashed for a while after one. Raise the **Buffer size** in **Preferences › Audio**, close other heavy programs, and on Linux or macOS make sure the computer is not on battery saver.

## The audio output was lost

A red band names the device. The decks wait where they were. Plug the interface back in and the sound returns by itself, or press **SWITCH OUTPUT** to go on on the computer's own output. See [Safe during a set](set-safety.md).

## The controller is not found

Connect it directly by USB. It appears within a few seconds; **Preferences › Controller › Detected** lists it. Turn on the **Monitor** there to see whether the app receives its messages at all.

## The stick does not sync

- The stick must be registered (see [USB sticks](usb-sticks.md)) and plugged in.
- Look at the status on the stick board and at the bottom of the window for error messages. Full sticks and read-only sticks cannot be written.
- **REBUILD STICK** writes it again from scratch.

## macOS says the app cannot be opened

That is expected on the first start, because the app is not signed by Apple. The [download page](/en/download/#macos) walks through "Open Anyway".

## Windows warns about the app

Press **More info** and then **Run anyway**. The program is not code-signed yet.

## Stems stay on SPLIT or fail

Separation needs a one-time download of its model. From version 0.7 the app asks before downloading it, and afterwards it runs offline.

## The stem download stopped

From version 0.7, open **Preferences › Analysis › Stems** and follow the steps in [The download stopped and now stems do not work](stems.md#the-download-stopped-and-now-stems-do-not-work).

## My cue or grid edit is not on the stick

Syncs run within seconds, but only while the stick is plugged in and registered. Press **SYNC** on the stick's row (`⌘S`) to make sure.

## How do I back up my library?

The library is backed up automatically when the app starts, and **BACK UP NOW** is in **Preferences › Advanced**. Use a [library archive](library.md#moving-to-a-new-computer) to move it to a new computer.

## Where are my files?

The library lives in your user data folder (macOS `~/Library/Application Support/OpenDJDecks`, Linux `~/.local/share/opendjdecks`, Windows `%LOCALAPPDATA%\OpenDJDecks`). Your music stays where it always was. Recordings go to the folder set in **Preferences › Recording**.

## Is there video, DVS or streaming?

No. Video, DVS (timecode vinyl), streaming services and cloud sync are out of scope: OpenDJDecks is offline-first and has no accounts.
