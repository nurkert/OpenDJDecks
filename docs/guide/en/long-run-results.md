---
title: Long-run results
description: How long OpenDJDecks plays without a dropout, measured for hours with two and four decks, stems, effects and recording, and how to repeat the measurement on your own computer.
keywords: [dropouts, dropout, latency, stability, long run, soak, reliability, performance, cpu, memory, buffer]
features: []
---

These are measurements, not promises. Each run plays real tracks on two or four decks for hours and writes one line per minute: how often a buffer was late, how long the engine took per buffer, the latency, the memory and how busy the computer was. Only numbers that were measured appear here.

## What runs

- Two or four decks playing and looping real tracks with key lock on, sync on every deck after the first, 8-beat loops, loop halving, beat jumps and tempo changes.
- Stems on every deck, from the stem cache (separated earlier by OpenDJDecks), with stem levels changing during the run.
- Echo on the master, reverb on the drums of deck 1, channel FX on every channel.
- A deck is loaded fresh every three minutes (decoding and stems, as in the app).
- The master is recorded to WAV the whole time. The file is closed and a new one started every ten minutes, so hours of recording do not fill the disk.

## How it is measured, and what it does not cover

No audio device is opened and nothing is audible. A thread with the scheduling class the system gives an audio device's thread (a time-constraint thread on macOS) asks the engine for one buffer per buffer period, paced by the clock. A buffer counts as a **deadline miss** (a dropout) when it is ready later than one buffer period after its turn began, which is when an audio device would have needed it. **Render time** is how long the engine took for one buffer, also given as a share of the buffer period. **Latency** is the buffer length: 256 frames at 48 kHz is 5.3 ms.

Not covered: the audio driver's own work, the interface's converters, the window drawing at the same time, and controller input. The real app also runs these. On the other hand, the runs below were made on a computer that was in use at the same time, which is the reason the load average is part of every line.

## Results

<!-- soak-results:start (generated: cargo run -p od-soak --bin soak-report -- docs/soak/*.jsonl) -->

Machine: MacBookPro18,2 (Apple M1 Max), 32 GB, macOS 27.0.1. Build: optimised (`--profile soak`).

**No run of four hours or more has finished yet.** Only what is listed below was measured so far.

### Short smoke run: 4 decks, not a long run (2 min)

| Measure | Value |
|---|---|
| Started | 2026-11-10 06:32 UTC |
| Minutes measured | 2 |
| Buffer, rate, latency | 256 frames · 48000 Hz · 5.33 ms |
| Engine callbacks | 22533 |
| Deadline misses (dropouts) | **12** in 2 of the minutes |
| Render time longer than the buffer | 12 |
| Render time, median of the minute medians | 2.21 ms (42 %) |
| Render time p99, typical minute / worst minute | 4.62 ms (87 %) / 4.62 ms (87 %) |
| Longest single callback | 6.97 ms (131 %) |
| Memory (RSS) first / last / highest minute | 1083 / 1283 / 1283 MB |
| Machine load average (1 min), median / highest | 11.0 / 11.0 |
| Recorded to WAV | 2 min |
| Deck reloads during the run | 1 |
| Log | `docs/soak/smoke-4-decks-2min.jsonl` |
<!-- soak-results:end -->

## Repeat it on your computer

You need the source code and the Rust toolchain. Build the measurement optimised (a debug build would understate what the engine can do):

```sh
cargo run --profile soak -p od-soak --bin soak -- --decks 4 --hours 4 \
  --track "/music/a.mp3" --track "/music/b.flac" \
  --stems-dir "<OpenDJDecks data folder>/stems" \
  --record-dir /tmp/soak-rec --out soak-4-decks.jsonl
```

`--track PATH=HASH` attaches the cached stems of a track (the content hash is the stem file's name). `--buffer` and `--rate` change the buffer size and sample rate. Then turn the logs into these tables:

```sh
cargo run -p od-soak --bin soak-report -- soak-4-decks.jsonl
```

Run one measurement at a time; two at once compete for the same processor. If you send us your log, say what else the computer was doing.
