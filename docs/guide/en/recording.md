---
title: Recording and tracklists
description: One button records your mix as WAV, FLAC or MP3, writes the tracklist as you play, and can split it into one file per track.
keywords: [record, recording, mix, tracklist, track list, cue sheet, wav, flac, mp3, split, recording level, soundcloud, mixcloud, audio file, save mix]
features: [rec-one-button, rec-wav, rec-flac-mp3, rec-tracklist, rec-split, rec-library-section, rec-silence-auto, rec-settings, rec-panel]
---

Press the round record button next to the master level (or `⌘Return`, or the record button on your controller) to record the master output. Press again to stop.

![The recording panel with level, meter and the growing tracklist](../img/recording.png)

## While you record

Press **REC** in the top bar to show the recording section: the level, a meter, the elapsed time and file size, and the tracklist as it grows. Tracks are added as they become audible. Record level goes from −12 to +12 dB.

## After recording

Open **Recordings** from the **RECORDINGS** rail: every mix is listed with its file. Right there:

- **SHOW FILE** opens it in your file manager.
- **SPLIT INTO TRACKS** cuts it into one file per played track.

## Format, folder and automation

In **Preferences › Recording**:

- **Format**: WAV is recorded directly; FLAC (bit-exact) or MP3 (320 kbit) are encoded when recording stops.
- **Folder** and **File name**: use `{date}`, `{time}` and `{n}` (number of the day) in the name.
- **Recording level**, also on the recording panel.
- **Split** into one file per track when recording stops.
- **Start**: start recording when sound starts. The master must carry sound for two seconds. **Stop** ends it after 30 seconds of silence. Stopping by hand disarms the automatic start until the next time the app starts.

## The tracklist

Each recording also gets the tracklist as a `.cue` file and as plain text, ready to paste into Mixcloud or SoundCloud. Timing and titles come from what was actually audible.
