<p align="center"><img src="docs/assets/banner.png" alt="OpenDJDecks" width="100%"></p>

<p align="center">
  <a href="https://github.com/nurkert/OpenDJDecks/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/nurkert/OpenDJDecks/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/nurkert/OpenDJDecks/actions/workflows/release.yml"><img alt="Release build" src="https://github.com/nurkert/OpenDJDecks/actions/workflows/release.yml/badge.svg"></a>
  <a href="https://github.com/nurkert/OpenDJDecks/releases"><img alt="Latest release" src="https://img.shields.io/github/v/release/nurkert/OpenDJDecks?include_prereleases&sort=semver"></a>
  <a href="https://github.com/nurkert/OpenDJDecks/releases"><img alt="Downloads" src="https://img.shields.io/github/downloads/nurkert/OpenDJDecks/total"></a>
  <a href="LICENSE"><img alt="License: GPL-3.0-or-later" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg"></a>
  <img alt="Platform: macOS, Linux, Windows, Raspberry Pi" src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows%20%7C%20Raspberry%20Pi-lightgrey.svg">
  <img alt="Rust" src="https://img.shields.io/badge/rust-stable-orange.svg">
  <a href="https://github.com/nurkert/OpenDJDecks/commits/main"><img alt="Last commit" src="https://img.shields.io/github/last-commit/nurkert/OpenDJDecks"></a>
</p>

<p align="center">
  <a href="https://opendjdecks.de">Website</a> ·
  <a href="https://github.com/nurkert/OpenDJDecks/releases/latest">Download</a> ·
  <a href="docs/SPEC.md">Specification</a> ·
  <a href="docs/feature-matrix.md">Feature list</a> ·
  <a href="docs/keyboard.md">Keyboard shortcuts</a> ·
  <a href="docs/parity.md">Verification</a>
</p>

# OpenDJDecks

Open-source DJ software in Rust for macOS, Linux, Windows and the Raspberry
Pi.
Prepare your library and sets, keep USB sticks in sync automatically, and
perform with compatible DJ controllers — on your laptop or as a standalone unit built from a Raspberry
Pi and your controller. See [docs/SPEC.md](docs/SPEC.md) for the full
specification.

## Screenshots

**Perform** — two decks with scrolling waveforms, phrases, stems, the deck
grid and the mixer:

![Perform: two decks, mixer and library](docs/assets/screenshots/perform.png)

**Prepare** — the track player with cues and phrases above a set with its
tempo / key flow:

![Prepare: player and set flow](docs/assets/screenshots/prepare.png)

**Club mode** — a reduced player layout for practising beatmatching
by ear:

![Club mode](docs/assets/screenshots/club-mode.png)

<sub>Shown with a demo library of generated tracks; names, artists and
cover art are made up.</sub>

## What it does

- **Library** — referenced music files, fast search, playlists, folders,
  smart playlists with a text rule syntax (`bpm 120-126, key 8A~, genre house`),
  undo/redo for every edit, automatic backups, relocation of moved files.
  AUTO SORT groups the collection offline by tempo, energy, mood, character
  and genre; the MATCH column rates every track against the master deck
  (key and tempo); cue points can be set like on your other tracks (the
  places you use, learned from your library).
- **Analysis** — BPM and beatgrid with downbeats, key (Camelot, Open Key or
  musical), three-band waveforms, phrases, loudness; pure Rust, a 6-minute
  track in about 0.3 s on an M1.
- **Sets** — ordered set lists with the flow of tempo, key and energy,
  transition hints (good / caution / clash), transition notes and backups.
- **USB sticks that stay in sync** — register a stick once with the lists
  that belong on it. Every change to a playlist, cue or beatgrid is written
  to the stick automatically and incrementally while it is plugged in, with a
  visible state per stick and one-click eject. Sticks hold the OpenDJDecks
  format and are played in OpenDJDecks, for example on the Raspberry Pi unit.
- **Prepare** — beatgrid editor with tap / typed BPM and tempo markers,
  phrase editor, cue editor (names, colours, saved and active loops), one
  or two players, a sub-browser for moving tracks between lists, an edit
  mode that cuts, trims or repeats a section into a new track, and stem
  separation ahead of time.
- **Perform** — 2 or 4 decks in standard, vertical-waveform or browse
  layouts, sync (beat or tempo only) and quantize, keylock (Rubber Band),
  hot and memory cues, loops, slip, beat jump, pad modes (hot cue, hold FX,
  beat jump / loop, sampler, keyboard, key shift, slicer, stems, memory cue,
  censor, pattern recall, lighting), Tempo FX with three-slot multi mode and
  Stem FX on a single part, channel FX incl. Auto Channel FX, Auto Fader, part
  isolation on the mixer, sampler with sequencer, vocal position from the
  separated parts, mappings for compatible DJ controllers with LED feedback, microphone / aux input with talkover.
- **Club mode** — one switch turns each deck into a
  player-style screen with the players' cue rules, no on-screen sync, and a
  checklist that ticks itself off while you load, cue and beatmatch by ear,
  so a controller owner is ready for the booth.
- **Lighting** — a DMX light show that follows the music: an automatic mode
  that reads phrases, beat and energy and blends the looks of both tracks
  during a mix, an easy mode with looks and colours, and a pro mode with
  per-group programs, beat-synced effects, an XY programmer, a cue stack and
  lighting cues stored with each track. Moving heads, LED bars, PARs,
  strobes, blinders, lasers and fog (built-in or Open Fixture Library
  files), Art-Net, sACN and USB-DMX interfaces, a live stage preview in its
  own tab, inside Perform or in a window on a second screen, and every
  control mappable to MIDI.
- **Recording** — one button; WAV, FLAC or MP3 with an automatic tracklist
  as `.cue` and as text for Mixcloud/SoundCloud, optional split into one
  file per track.
- **Club player USB** — sticks can also be written in the USB format of Pioneer DJ players, chosen per stick, and such sticks can be imported, including cues and histories changed on a player. Not yet confirmed on real players.
- **Exchange** — library XML export and import (rekordbox-compatible), a whole-library archive (`.odlib`) for moving to a new
  computer, and a duplicate finder.
- **Raspberry Pi standalone** — boots straight into Perform, plays sticks
  prepared on your Mac, controlled entirely from the controller.
- English and German interface; colour-blind-safe state display (state is
  carried by line form, not only by colour).

## Build

Requirements: Rust (stable), Rubber Band, pkg-config.

```sh
# macOS
brew install rubberband pkg-config
# Debian / Ubuntu / Raspberry Pi OS (64-bit)
packaging/linux/install-build-deps.sh

cargo build --release -p opendjdecks -p od-cli
./target/release/opendjdecks            # the app
./target/release/opendjdecks --lang de  # German interface
```

Windows (x86_64, MSVC toolchain; Rubber Band comes from
[vcpkg](https://vcpkg.io), pkg-config is not needed):

```powershell
vcpkg install rubberband:x64-windows
$env:VCPKG_ROOT = "C:\path\to\vcpkg"
$env:VCPKGRS_TRIPLET = "x64-windows"
$env:VCPKGRS_DYNAMIC = "1"
$env:PATH += ";$env:VCPKG_ROOT\installed\x64-windows\bin"   # Rubber Band DLLs
cargo build --release -p opendjdecks -p od-cli
.\target\release\opendjdecks.exe
```

Instead of vcpkg, `RUBBERBAND_LIB_DIR` can point at a folder holding
`rubberband.lib`. Audio runs through WASAPI (shared mode); ASIO is not built
in. The Windows build is compiled and tested in CI but not yet verified on
real hardware (audio latency, controllers, USB sticks).

Options: `--library <path>`, `--profile lite|full`, `--kiosk`,
`--scale <factor>`, `--lang en|de`, `--keys camelot|openkey|musical`,
`--colorblind`, `--high-contrast`, `--no-motion`, `--window WxH`.

Packages: `packaging/macos/build-dmg.sh` (.app/.dmg),
`packaging/windows/build-zip.ps1` (portable Windows zip),
`packaging/linux/build-appimage.sh` (AppImage),
`packaging/linux/de.opendjdecks.OpenDJDecks.yml` (Flatpak),
`packaging/raspberry-pi/` (kiosk bundle, installer and SD-image stage — see
its README).

## First steps

1. **Add music** (or import a library XML file).
2. Analysis runs in the background; the status band shows progress.
3. Create playlists and sets; select tracks and press **+** next to a list in
   the sidebar to add them. Click a list's title to rename it.
4. Plug in a USB stick, select the list or folder that should go on it and
   press **Register with …** on the device board. From then on the stick
   follows every change.
5. Switch to **Perform**, double-click a track (or press the browse encoder)
   to load it on a free deck.

Keyboard: ⌘Z / ⌘⇧Z undo/redo, ↑/↓ select, Space preview, 1–8 hot cues,
Delete removes from the list.

## Headless tool

`od-cli` drives the same crates without a window:

```sh
od-cli import ~/Music/House
od-cli analyze
od-cli playlist "Warmup" --search deep
od-cli register /Volumes/STICK --playlist Warmup
od-cli sync /Volumes/STICK
od-cli verify /Volumes/STICK
od-cli lighting add "Spot L" moving-head --group Movers
od-cli lighting output 0 --artnet 2.0.0.10:6454
od-cli lighting test --seconds 60     # simulated track: intro, build, drop …
```

## Architecture

| Crate | Purpose |
|---|---|
| `od-core` | Domain types: beatgrids, cues, keys, waveforms, phrases |
| `od-decode` | Decoding, tags, resampling (symphonia) |
| `od-analysis` | BPM/grid, key, waveform, phrases, loudness |
| `od-library` | SQLite library, undo journal, devices, backups |
| `od-format-native` | OpenDJDecks stick format |
| `od-devices` | Volume detection, incremental sync, background manager |
| `od-import` | One-time import (library XML, iTunes, settings) |
| `od-engine` | Real-time engine: decks, mixer, FX, sampler, record tap |
| `od-audio-io` | CoreAudio / ALSA / JACK / PipeWire output |
| `od-controller` | MIDI, mappings (`mappings/`), LED feedback |
| `od-recorder` | Recording and tracklists |
| `od-i18n` | Translations of messages produced in Rust |
| `od-ui` | Slint interface |
| `apps/opendjdecks`, `apps/od-cli` | Executables |

Design: [PRODUCT.md](PRODUCT.md) and [DESIGN.md](DESIGN.md).
Controller mapping format: [docs/controller-mappings.md](docs/controller-mappings.md).

## Status and open items

See [docs/verification.md](docs/verification.md) for what is verified (including a one-hour
four-deck soak without dropouts and sync to FAT32/exFAT sticks) and what needs hardware.


- The controller mappings are not yet fully confirmed on real hardware;
  entries marked `verify` in the mapping files need checking, and individual
  controls or LEDs may not work. `cargo run -p od-controller --example
  midi_monitor` helps with that.
- LAN sync is planned for a later milestone.

## Trademarks and independence

OpenDJDecks is an independent project, not affiliated with or endorsed by
AlphaTheta Corporation or any other manufacturer. rekordbox and Pioneer DJ
are trademarks of AlphaTheta Corporation; other names belong to their
respective owners. Names are used only to state compatibility. Sources and
third-party notices are in [NOTICE.md](NOTICE.md), the rules for contributions
in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

GPL-3.0-or-later. See [LICENSE](LICENSE). The bundled Barlow Semi Condensed
font is licensed under the SIL Open Font License (`crates/od-ui/fonts/OFL.txt`).
