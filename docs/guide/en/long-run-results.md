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

### 2 decks, 4 hours

| Measure | Value |
|---|---|
| Started | 2026-11-10 06:35 UTC |
| Minutes measured | 240 |
| Buffer, rate, latency | 256 frames · 48000 Hz · 5.33 ms |
| Engine callbacks | 2700026 |
| Deadline misses (dropouts) | **7** in 5 of the minutes |
| Render time longer than the buffer | 6 |
| Render time, median of the minute medians | 2.56 ms (48 %) |
| Render time p99, typical minute / worst minute | 2.96 ms (55 %) / 3.57 ms (67 %) |
| Longest single callback | 5.82 ms (109 %) |
| Memory (RSS) first / last / highest minute | 1025 / 936 / 1025 MB |
| Machine load average (1 min), median / highest | 11.4 / 115.3 |
| Recorded to WAV | 240 min |
| Deck reloads during the run | 70 |
| Log | `docs/soak/soak-2-decks-4h.jsonl` |

Hour by hour:

| Hour | Misses | Render p99, worst minute | Longest callback | RSS at the end | Load average (median) |
|---|---|---|---|---|---|
| 1 | 2 | 3.44 ms | 5.82 ms | 936 MB | 12.4 |
| 2 | 0 | 3.46 ms | 5.22 ms | 620 MB | 11.4 |
| 3 | 4 | 3.57 ms | 5.56 ms | 322 MB | 10.3 |
| 4 | 1 | 3.42 ms | 5.47 ms | 936 MB | 11.8 |

### 4 decks, 4 hours

| Measure | Value |
|---|---|
| Started | 2026-11-10 10:35 UTC |
| Minutes measured | 240 |
| Buffer, rate, latency | 256 frames · 48000 Hz · 5.33 ms |
| Engine callbacks | 2700003 |
| Deadline misses (dropouts) | **1007** in 105 of the minutes |
| Render time longer than the buffer | 947 |
| Render time, median of the minute medians | 1.97 ms (37 %) |
| Render time p99, typical minute / worst minute | 3.27 ms (61 %) / 5.38 ms (101 %) |
| Longest single callback | 7.37 ms (138 %) |
| Memory (RSS) first / last / highest minute | 1219 / 970 / 1339 MB |
| Machine load average (1 min), median / highest | 8.2 / 111.2 |
| Recorded to WAV | 240 min |
| Deck reloads during the run | 70 |
| Log | `docs/soak/soak-4-decks-4h.jsonl` |

Hour by hour:

| Hour | Misses | Render p99, worst minute | Longest callback | RSS at the end | Load average (median) |
|---|---|---|---|---|---|
| 1 | 83 | 4.60 ms | 6.95 ms | 592 MB | 13.3 |
| 2 | 158 | 4.76 ms | 5.99 ms | 686 MB | 11.7 |
| 3 | 379 | 5.29 ms | 6.49 ms | 916 MB | 4.4 |
| 4 | 387 | 5.38 ms | 7.37 ms | 970 MB | 3.9 |
<!-- soak-results:end -->

### What the numbers show

- **Two decks:** 7 late buffers in four hours, across 2.7 million buffers. No hour had more than 4. Memory went up and down with the deck reloads and did not grow over the run.
- **Four decks:** 1007 late buffers, and they did not come evenly. In 135 of the 240 minutes there were none at all. The rest came in bursts, for example minutes 165 to 181 and 231 to 238. During those minutes the same computer was compiling OpenDJDecks, running its test suite and taking screenshots of the app. The engine's median time per buffer stayed between 1.7 and 2.6 ms for the whole four hours. The engine did not get slower over time; it lost the processor to other work.
- **What this means for you:** with four decks, stems and effects at a 5.3 ms buffer, close everything else on the computer, or pick a larger buffer. Two decks were stable even next to heavy work.
- **Still to do:** these runs were made on a computer in daily use. A run on a quiet computer and runs of eight hours have not been made yet. They will be added here, measured the same way.

## The whole app with its window

The runs above measure the engine alone. This one measures the whole app, window drawing included, on the same computer: MacBookPro18,2 (Apple M1 Max, 10 cores), 32 GB, macOS 27.0.1, its built-in 120 Hz display, window 1440 × 900, Perform with the demo library (tracks of 2 min 35 s). No audio device was opened (the app's virtual output, as in the tests). After 20 s to load, the processor time of the app was read over 30 s; 100 % is one core busy. Memory is the resident size at the end.

| | Idle in Perform | One deck playing | Two decks playing | Memory, two decks |
|---|---|---|---|---|
| 0.9.0 as released (optimised) | 4 % | 59–61 % | 98–99 % | 0.47 GB |
| Development build, Low-CPU mode off | 5–7 % | 100–101 % | 99–100 % | 0.50 GB |
| Development build, Low-CPU mode on | 6 % | 70–71 % | 89–90 % | 0.50 GB |

Two runs each; ranges are the two results. The development build is unoptimised, so it costs more than a released app, and with the mode off it is held back by the drawing, which already fills a core. With Low-CPU mode on, one deck playing took about 30 % less of a core in the same build, two decks about 10 % less. How much the mode saves in an optimised build has not been measured yet; nor were slower computers, Windows, Linux or a Raspberry Pi.

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
