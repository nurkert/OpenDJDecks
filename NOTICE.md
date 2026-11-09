# Notices

OpenDJDecks is licensed under GPL-3.0-or-later. See [LICENSE](LICENSE).

## Independence and trademarks

OpenDJDecks is an independent project. It is not affiliated with, endorsed by or sponsored by
AlphaTheta Corporation or any other manufacturer. rekordbox and Pioneer DJ are trademarks of
AlphaTheta Corporation. All other names belong to their respective owners. They are used only to
state what OpenDJDecks is compatible with.

## Bundled

| What | Licence | Where |
|---|---|---|
| Barlow Semi Condensed (font) | SIL Open Font License 1.1 | `crates/od-ui/fonts/OFL.txt`, `website/public/fonts/OFL.txt` |

Rust and npm dependencies are listed with their licences in `Cargo.lock` and
`website/package-lock.json`.

## Downloaded on first use

| What | Licence | Source |
|---|---|---|
| HT-Demucs v4 stem separation model | MIT | <https://github.com/facebookresearch/demucs>, ONNX export from <https://huggingface.co/StemSplitio/htdemucs-onnx> |

## Public sources for formats and message facts

Formats and controller messages were implemented from these public descriptions. No code from
them is included.

| Topic | Source | Licence |
|---|---|---|
| USB stick database and analysis file layouts | [Deep Symmetry: rekordbox Export Structure Analysis](https://djl-analysis.deepsymmetry.org/rekordbox-export-analysis/) and [crate-digger](https://github.com/Deep-Symmetry/crate-digger) | EPL-2.0 OR MPL-2.0 OR LGPL-3.0-only (format definitions) |
| Table details, settings files | [rekordcrate](https://github.com/holzhaus/rekordcrate) (also a test-only dependency), [pyrekordbox](https://github.com/dylanljones/pyrekordbox) | MPL-2.0, MIT |
| Analysis file folder names | Function published in [baken-export](https://crates.io/crates/baken-export), which credits the fourfour project. Independent implementation | MIT |
| Library XML | Official format documentation for developers | - |
| Encrypted page format | [SQLCipher documentation](https://www.zetetic.net/sqlcipher/sqlcipher-api/) | - |
| Controller MIDI messages | [Mixxx](https://github.com/mixxxdj/mixxx) controller mappings, as facts only. Each mapping file lists its sources and marks unconfirmed entries with `verify = true` | GPL-2.0-or-later |

OpenDJDecks contains no key for encrypted libraries and does not fetch one.
