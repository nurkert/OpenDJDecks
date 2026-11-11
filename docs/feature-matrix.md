<!-- Generated from tests/parity/features.toml by `cargo run -p od-parity --bin parity -- matrix`. Do not edit by hand. -->

# Feature matrix (OpenDJDecks)

Snapshot: 2026-11-06, from a read of the source tree.

**Summary:** 288 features; 282 in scope: 282 DONE (100 %), 0 PARTIAL, 0 ENGINE-ONLY, 0 MISSING; 1 deferred to a later milestone; 5 out of scope (SPEC §1).

The list covers the professional feature set a DJ expects when preparing music and performing
with a compatible controller (SPEC §1, §5.5, §10, §11, §12). Behaviour,
layout and naming in OpenDJDecks follow [SPEC.md](SPEC.md) and [DESIGN.md](../DESIGN.md).
Statuses come from the 2026-11-06 read of the source tree; this file is generated from
`tests/parity/features.toml`, see [parity.md](parity.md).

Evidence written before the `od-ui` rebuild names `perform.rs` (`crates/od-ui/src/perform.rs`); that
code now lives in `decks.rs`, `controller.rs`, `mixer.rs` and `recorder.rs`. Other short paths:
`app.rs` = `crates/od-ui/src/app.rs`, `command.rs` = `crates/od-engine/src/command.rs`,
`action.rs` = `crates/od-controller/src/action.rs`.

## Legend

| Status | Meaning |
|---|---|
| DONE | Works end to end, including UI and/or controller |
| ENGINE-ONLY | Backend (engine, library, format) exists; no UI and no controller path |
| PARTIAL | Usable, but an expected part is missing (noted) |
| MISSING | Nothing in the code |
| OUT-OF-SCOPE | Excluded by SPEC §1 (streaming, cloud, video, lighting, DVS) |

A milestone after the status ("deferred to M6") marks an in-scope feature that SPEC §15 schedules later; it is tracked but not counted as a gap. The Tests column counts the automated tests linked in `features.toml` (SPEC §16.1).

Priority (gaps only): **P1** = an experienced DJ hits it in the first session and the app feels broken or foreign without it; **P2** = expected, regular use; **P3** = niche, power-user or later.

---

## 1. Browser / collection

| Feature | Status | Evidence / note | Tests | Prio |
|---|---|---|---|---|
| Collection table with sortable columns | DONE | `ui/table.slint` header cells; `app.rs::sort_key` (title, artist, genre, BPM, key, time, rating, date added) | 2 |  |
| Column choice (album, label, comment, colour, date added, play count, bitrate, Tags, …) | DONE | `browser/columns.rs`, `ui/table.slint`; Header right-click or the columns button opens the column menu (preview, title, artist, album, genre, label, comment, Tags, colour, BPM, key, time, year, date added, play count, bitrate, rating); the choice is stored in the settings (`browser.columns`); "Default columns" resets | 5 | P2 |
| Artwork in browser / deck | DONE | `crates/od-ui/src/browser/mod.rs::artwork_image`, `crates/od-ui/src/browser/tracks.rs::list_images`, `ui/info.slint`; Stored (`schema.sql` `artwork`); shown as a thumbnail in the info panel and next to the title of the Prepare player (deck artwork is part of the deck header), and in the ARTWORK column of the track list and Explorer (default on; decoded off the UI thread). Taken from embedded pictures and folder images. | 3 | P3 |
| Tree: folders, playlists, smart lists, sets | DONE | `nodes.rs::create_node/rename_node/delete_node`; `app.rs` `on_create_node`, `on_rename_node`, `on_delete_node` | 2 |  |
| Reorder tree nodes (drag) | DONE | `nodes.rs::move_node`, `browser/mod.rs::node_dropped`; Drag a list or folder in the tree onto a folder (moves into it) or onto another list (moves before it); undoable | 2 | P2 |
| Add tracks to playlist | DONE | `app.rs` `on_add_to_node` | 1 |  |
| Reorder playlist / set entries | DONE | `nodes.rs::move_entries`, `browser/mod.rs::reorder_rows`; Drag rows (with the selection) between rows of a playlist or set sorted by #; one undo step | 1 | **P1** |
| Drag and drop (table → tree, Finder → library) | DONE | `ui/app.slint`, `crates/od-ui/src/import.rs::files_dropped`, `ui/drag.slint`; Rows drag onto decks, the Prepare players, playlists / sets / the tag list in the tree and between rows of a playlist; tree nodes drag onto folders. Files and folders from Finder / the file manager: dropped on the window they join the collection and the open playlist or set; in Perform a file dropped on a deck loads there. Slint's `DropArea` takes OS drops where the backend delivers them (UI test via the testing backend); the winit backend of Slint 1.18 does not yet, so `import::wire_native_drop` also takes winit `DroppedFile` events (no drop position there: collection and open list) | 4 | **P1** |
| Search | DONE | `tracks.rs::search`; `app.rs` `on_search` | 2 |  |
| Filter (BPM range, key, rating, Tags, relative to master deck) | DONE | `browser/filter.rs`, `ui/filter.slint`; Filter bar under the header (filter button, ⌘J): BPM range, key compatible with the master deck, BPM within ±% of the master deck (double / half time count; without a loaded deck the selected track is the reference), minimum rating, one tag value; CLEAR. Smart lists keep the full rule language | 4 | P2 |
| Rating (stars) | DONE | `tracks.rs::set_tags`; Five cells per row, click to rate (click again clears); also in the row context menu. Verified by clicking in the running app | 1 | **P1** |
| Track colour | DONE | `od-core/src/track.rs`; Colour tag bar per row; set via the row context menu (eight colours, none) | 1 | **P1** |
| Comments | DONE | Edited in the track info panel (INFO), saved with Return as one undo step. Verified in the running app | 1 | **P1** |
| Edit tags (title, artist, album, genre, label, year …) | DONE | `tracks.rs::set_tags`; Track info panel edits title, artist, album, genre, label, remixer, composer, year, comment | 2 | **P1** |
| Tags | DONE | `browser/sources.rs`, `ui/user_tags.slint`; TAGS panel: type "Category: Value" to tag the selected tracks, click a value to tag / untag them (undoable); TAGS column; Tags filter in the filter bar. | 3 | P2 |
| Tag list / palette (holding area) | DONE | `browser/sources.rs`, `ui/sidebar.slint`; TAG LIST in the tree: collect tracks with + (⌥⌘T), the row menu or by dragging rows onto it; Delete removes, SAVE AS PLAYLIST makes a playlist, CLEAR empties; kept in the library | 2 | P3 |
| Similar tracks | DONE | `sets.rs::suggest_next`, `browser/sources.rs`; SIMILAR in the tree: tracks within ±6 % tempo and a compatible key after the master deck or deck 1–4 (selector in the title row), or after the selected track (row menu "Show similar tracks"); `browse.similar` is still not mapped | 2 | P2 |
| Automatic classification of the collection (AUTO SORT) | DONE | `autosort.rs::classify`, `browser/auto.rs`, `ui/sidebar.slint`; AUTO SORT in the tree, computed offline off the UI thread whenever the change log moves (imports, analysis, tag edits; unchanged analyses are not read again): TEMPO bands (< 100, 100–115, 115–124, 124–130, 130–140, 140–160, 160+ BPM), ENERGY (thirds of the collection by mean energy), MOOD (key mode × phrase mood: euphoric, warm, driving, dark), CHARACTER (thirds by waveform band balance: bass-heavy, balanced, bright) and GENRE (genre tags grouped into families, frequent other tags as their own list). A list opens in the track table: sortable, loadable, draggable to playlists | 5 | P2 |
| Track compatibility at a glance (MATCH column) | DONE | `browser/filter.rs::match_with`, `browser/sources.rs::match_reference`, `ui/table.slint`; MATCH column (default on): every row rated against the master deck, else a playing deck, else the selected track — Camelot distance (≤ 1 good, 2 workable) and tempo (±3 % good, ±6 % workable, half / double time counted) as 0–3 lit lines, the reason on hover; sortable (best fit first, then the closer tempo); follows the master deck while you play | 3 | P2 |
| Track suggestion / harmonic matching | DONE | `ui/user_tags.slint::SuggestionPanel`, `browser/sources.rs`; SIMILAR panel: what fits after the selected track (tempo ±6 %, compatible key), double-click loads into a free deck, SHOW AS LIST opens it as the Similar list; key-distance marks in the KEY column | 3 | P2 |
| Intelligent (smart) playlists | DONE | `od-library/src/smart.rs`, text rule editor `od-ui/src/smart_text.rs`, `on_set_smart_rule` | 4 |  |
| Histories: own play history | DONE | `recorder.rs::log_play`, `browser/sources.rs`; Every track that becomes the audible deck is logged in the run's session ("Session 2026-11-07 21:30"), recording or not; HISTORIES in the tree lists the sessions, each opens its plays in order | 2 | P2 |
| Track preview (Prepare) | DONE | `perform.rs::toggle_preview` on a preview deck; Space key | 1 |  |
| Loaded-on-deck indicator | DONE | `TrackRow.deck` | 1 |  |
| Duplicate track check | DONE | `crates/od-ui/src/browser/sources.rs::duplicates_action`; File ▸ Find Duplicate Tracks (⌥⌘Y) lists the groups in the browser; REMOVE SELECTED and KEEP ONE OF EACH remove copies from the collection (files stay on disk) | 3 |  |
| Import folder / files | DONE | `app.rs` `on_import_folder`, `od-cli import` | 1 |  |
| Preferences and keyboard shortcuts import (rekordbox-compatible) | DONE | `crates/od-import/src/settings.rs`, `crates/od-ui/src/import.rs::import_settings`; File ▸ Import rekordbox Preferences and Shortcuts and Preferences > Advanced: view, analysis, recording, deck and mixer values are applied; shortcuts are taken where they do not collide; `od-cli rekordbox-settings` reports the mapping; (SPEC §7) | 5 |  |
| Library XML (rekordbox-compatible) and iTunes library import / export | DONE | `crates/od-ui/src/import.rs::export_xml`, `crates/od-ui/src/import.rs::import_xml`; File ▸ Import / Export Library XML (rekordbox-compatible), also `od-cli xml-export` / `xml-import`; File ▸ Import Music / iTunes Library XML reads a `Library.xml` export (tracks with tags and rating, playlist folders; built-in lists skipped) | 6 | P3 |
| Library import you can check: preview, before and after, what did not transfer, undo | DONE | `crates/od-import/src/review.rs::plan`, `crates/od-ui/src/import.rs::open_review`, `crates/od-library/src/backup.rs::restore_in_place`; Roadmap [54]. For the documented export formats (library XML, Music / iTunes Library.xml) the user picked: what carries over and what will not, with the reason, before anything is written; then the library's counts before and after, the list of what did not transfer, and UNDO IMPORT (an undo copy taken right before) while the window is open. Nothing reads other software's private files on its own; the preferences import asks for the folder | 6 | P2 |
| Missing files / relocate | DONE | `relocate.rs::check_missing/find_relocations`; `on_find_moved_files` | 1 |  |
| Consolidate | DONE | `relocate.rs::consolidate`, `crates/od-ui/src/import.rs::consolidate`; File ▸ Consolidate Library copies every track into <folder>/<Artist>/ in the background and points the library at the copies; originals stay | 2 |  |
| Undo/redo with history | DONE (beyond the usual) | `journal.rs`; Cmd-Z / Shift-Cmd-Z in `app.rs::key` | 3 |  |
| Sets with transition notes and flow view | DONE (own) | `sets.rs::set_flow`, `on_set_transition_note`, `ui/flow.slint` | 3 |  |
| Set backup tracks / planned start | DONE | `nodes.rs::set_entry_backups/set_set_meta`, `crates/od-ui/src/browser/mod.rs::set_backups`; Set view: planned start field; backups of the selected position are taken from the tag list (BACKUPS FROM TAG LIST) or cleared | 1 |  |
| Keyboard shortcuts for Prepare | DONE | `keymap.rs::registry`, `commands.rs::run`, `docs/keyboard.md`; One command registry (177 actions, conventional DJ-software defaults for decks 1/2); Preferences > Keyboard rebinds with conflict detection, reset, TOML import/export | 4 | P2 |
| Browse from controller | DONE | `crates/od-ui/src/browser/navigate.rs`; `browser::navigate`: BACK moves the encoder into the tree, turning steps through the lists (⇧ by eight), a press opens a folder / steps into it or moves into the track list; there turning steps rows, a press loads into a free deck, ⇧-press adds to the tag list; VIEW toggles the full browser (Prepare) or the Perform browse layout; SIMILAR opens the similar tracks. The tree marks the encoder focus | 3 | P2 |
| Explorer: browse folders on disk and play directly | DONE | `crates/od-ui/src/explorer.rs::ExplorerState::rows`, `crates/od-ui/src/explorer.rs::open_folder`, `crates/od-ui/ui/sidebar.slint::ExplorerItem`; User-added roots (persisted in library meta) as a nested folder tree of any depth: sub-folders are listed lazily on expand/select, each probed one level so only expandable folders get a mark; hidden and player data folders are skipped. Selecting a folder expands it and lists its audio files; (SPEC §5.5; UX §2.2) | 5 |  |
| Explorer: "Include subfolders" lists the files of a folder and all its sub-folders | DONE (beyond the usual) | `crates/od-ui/src/explorer.rs::list_files`, `crates/od-ui/src/explorer.rs::set_recursive`, `crates/od-ui/ui/app.slint`; Toggle in the folder list header (persisted as `explorer.recursive`). Depth-first, each folder by name, symlink loops followed once, capped at 32 levels / 20 000 files; (SPEC §5.5) | 3 |  |
| Explorer: loading a file adds it to the collection and analyses it in the background | DONE | `crates/od-ui/src/explorer.rs::start_import`, `crates/od-ui/src/explorer.rs::poll`; Loading a file from the Explorer adds it to the collection in the background (`workers::import_file`) and loads it; the flow test asserts the deck and the collection; (SPEC §5.5) | 1 |  |
| Source rail switching the tree (Collection, Playlists, Similar, Explorer, Devices, Histories, Recordings) | DONE | `crates/od-ui/ui/sidebar.slint`; 38 px rail left of the tree in the familiar source order (Collection, Playlists, Similar, Sampler, Explorer, Devices, Histories, Recordings, Tag list): each opens its source or scrolls the tree to it and marks the open one; Sampler opens the sampler panel in Perform; (UX §1.6, §4 P4.9) | 1 | P2 |
| Re-orderable columns and multi-sort | DONE | `browser/columns.rs`, `browser/tracks.rs::sort_rows`; Drag a header cell onto another column to move it there; ⇧-click adds (or reverses) up to three secondary sort columns, shown with their rank; order and secondary sort persist in the settings; (SPEC §5.5) | 3 | P3 |
| Load a track: drag to deck, double-click (Prepare), context menu Load ▸ Deck n | DONE | `crates/od-ui/src/decks.rs::free_deck`, `crates/od-ui/src/decks.rs::prepare_in_background`; Double-click / activate loads into a free deck, drag a row onto a deck, context menu Load to deck ▸ Deck n, controller load; all driven through the window by UI tests. The old track leaves at once and the new one shows with a LOADING sweep; the mix is decoded first (playable at once), cached parts swap in without a gap; (UX §2.1) | 4 |  |
| Drag tracks from the browser (and Explorer, Finder) onto a deck or the Prepare player | DONE | `crates/od-ui/src/import.rs::files_dropped`, `crates/od-ui/ui/app.slint`; List, Explorer and sub-browser rows and files from Finder / the file manager onto any deck (1–4) or either Prepare player, with drag ghost and the target highlighting while dragging; files dropped on a list in the tree go into that list, elsewhere into the collection and the open list (at the row under the pointer when it is in its own order). UI-tested on the Slint drop path. Native winit drops carry no position: on macOS the pointer is read from AppKit while files hover; on Linux they go to the collection and the open list; (SPEC §5.5) | 5 | **P1** |
| Track row context menu (Load ▸, Add to playlist ▸, Analyse, Show in Finder, …) | DONE | Right-click a row: Load to deck ▸ 1-4, Import to collection (Explorer files), Analyse, Lock / unlock analysis, Separate parts, the recently used lists inline (Add to “…”), Add to playlist ▸, Add to tag list, Reload tags from file, Export file…, Reset play count, Memory cues to hot cues, Sort hot cues by time, Show similar tracks, Colour ▸, Rating ▸, Remove from this list, Show info, Show in file manager. UI tests drive the real menu (rating, colour, remove, load, inline recent list); the order is checked by the view test `ctx.track-row`; (UX §1.9) | 4 |  |
| Info panel (track details beside the list) | DONE | INFO in the right panel strip opens the track info panel beside the list: SUMMARY (every tag editable in place, Return stores, undoable), INFO (time, file type, size, added, sample rate, bit rate, plays, path) and ARTWORK tabs; checked by the view test `browser.info-panel`; (SPEC §5.5; UX §1.6) | 1 |  |
| Sub-browser (second tree + list for dragging between playlists) | DONE | `crates/od-ui/src/browser/sub.rs`, `crates/od-ui/ui/subbrowser.slint`; SUB in the list header, View > Sub Browser and a shortcut open a second tree (all library lists) and list under the main list; main-list rows dropped on its lists are added there, its rows go to main-tree lists, the open list, decks and players; (UX §1.6, §4 P4.12) | 2 | P3 |
| Automatic database backups (rotating) | DONE (beyond the usual) | `crates/od-library/src/backup.rs`; (SPEC §5.3) | 1 |  |
| Export / import the whole library as one archive (database + analysis, optional audio) | DONE | `crates/od-ui/src/import.rs::export_archive`, `crates/od-ui/src/import.rs::restore_archive`; File ▸ Export Library Archive (asks whether to include the audio; runs in the background) and Restore Library Archive (into a chosen folder, opened from the next start); (SPEC §5.3) | 2 |  |
| Import a Club player USB written by other software | DONE | `od-import/src/club_stick.rs`; `od-import` (`club_stick`, shared engine in `lib.rs`), File menu in `od-ui/src/import.rs`, `od-cli club-usb-import` | 3 |  |
| Import an encrypted library database (opt-in, the user supplies the key) | DONE | `od-import/src/encrypted_library.rs`, `od-import/src/sqlcipher.rs`, `od-ui/src/import.rs::start_encrypted_import`; The user picks the file and types the key under Advanced (or `od-cli encrypted-library-import`, `OD_ENCRYPTED_LIBRARY_KEY`). No key is contained, fetched, stored or logged; the source is read-only and the temporary decrypted copy is removed, also on error | 9 |  |
| MCP server: full library access for preparing sets with an LLM | DONE | `apps/od-mcp/src/tools.rs`, `apps/od-mcp/src/server.rs`, `apps/od-mcp/src/lighting.rs`, `crates/od-lighting/src/rig.rs`; Owner request 2026-11-06 (SPEC §5.6). `opendjdecks-mcp` (MCP 2025-06-18, stdio or loopback HTTP): 8 read and 10 write tools, track/node resources, build_set and fix_transitions prompts; each write is one undo step "MCP: …" plus change-log rows (stick sync); setup in docs/mcp.md. Owner request 2026-11-07: the light show too — 8 lighting tools (rig overview, patch, settings, group looks, outputs, cue stack, track cues, live actions); the app reloads the edited rig and runs live actions from the rig's inbox; (SPEC §5.6) | 10 | **P1** |
| Played tick in the list (on air this session) | DONE | `recorder.rs::mark_played`, `ui/table.slint`; A track that was audible on the master for 30 s in all gets a tick in the list and the Explorer; a short listen in the headphones does not count | 2 | P3 |
| Preview a track at another tempo or key (TEMPO / KEY tab in the Prepare player) | DONE | `player.rs::apply_shift`, `player.rs::shift_readout`, `ui/player.slint::PreparePlayer`; Steps of whole percents (with key lock, so the key stays) and semitones on the hidden preview deck. Nothing is stored with the track or its analysis; the shift ends when another track is previewed and is not remembered as a deck switch | 2 | P2 |
| KEY column sorts by circle of fifths or alphabetically (stored choice) | DONE | `browser/tracks.rs::KeyOrder`, `browser/tracks.rs::compare_keys`, `ui/table.slint::TrackTable`; Two checkable items in the column menu; circle of fifths (Camelot order) is the default. Alphabetical sorts by the key text in the chosen notation. The key sort is always done in the app, not by SQL | 2 | P2 |
| Calmer track table: titles lead, figures right-aligned in tabular digits, a selection fill, the loaded mark in the deck colour | DONE | `crates/od-ui/ui/table.slint::TrackLine`, `crates/od-ui/ui/sidebar.slint::Sidebar`; The lists column stops at its content width and clips, so it never draws over the table.; (Roadmap [49]) | 4 | P3 |
| Set Flow as a chart: energy bars, a tempo line and key per track, transition markers with named warnings (tempo jump, key clash, energy drop or jump); a click selects the pair | DONE | `crates/od-ui/src/browser/flow.rs`, `crates/od-ui/ui/flow.slint::SetFlow`; Warns above 6 % tempo change (half and double time count as the same tempo), on a key clash by the key relation of [62], and on an energy step of 0.4 of the set's range that is also a quarter of the louder track. Above 3 % tempo or a key that changes the energy is a caution (dashed). The energy thresholds were set on generated tracks and the demo library, not yet on real sets.; (Roadmap [47]) | 5 | P2 |
| Browser rails with whole words or signs with tooltips (never cut words), one double-click and Return rule, separate surfaces for lists, list and panels, a foldable lists column | DONE | `crates/od-ui/ui/sidebar.slint::RailItem`, `crates/od-ui/ui/controls.slint::Glyph`, `crates/od-ui/src/player.rs::play_in_player`; Double-click and Return: the free deck (or the fixed LOAD TO deck) in Perform, the player in Prepare, where the track plays and never pauses. The touch layout shows the signs as well (no tooltips on touch). The folded lists column is not remembered across starts.; (Roadmap [22]) | 3 | P3 |
| Fuzzy search: typos forgiven, words across title, artist and album, best matches first; typing in the list searches, arrows move, Return loads the top hit | DONE | `crates/od-ui/src/browser/fuzzy.rs`, `crates/od-ui/src/browser/mod.rs::list_typed`, `crates/od-ui/src/browser/mod.rs::search_key`; The collection search no longer uses the database's full-text index: every track is scored in memory (fast enough for tens of thousands of tracks; not measured on a 100 000-track library). After a click in the list, letters and digits go to the search, so deck shortcuts on letters wait until Esc or a load; the list shows a focus line while it has the keys.; (Roadmap [26]) | 6 | P2 |
| Active filters as removable chips over the list, a compact list header and a foldable lists column | DONE | `crates/od-ui/ui/filter.slint::FilterChips`, `crates/od-ui/src/browser/filter.rs::chips`; (Roadmap [26]) | 3 | P3 |

## 2. Analysis

| Feature | Status | Evidence / note | Tests | Prio |
|---|---|---|---|---|
| BPM + beatgrid incl. downbeats | DONE | `od-analysis/src/tempo.rs::detect_beatgrid`; background worker `od-ui/src/workers.rs` | 6 |  |
| Variable-tempo grid | DONE | `od-core/src/beatgrid.rs::set_bpm_about`, `od-core/src/beatgrid.rs::through_pins`, `crates/od-ui/src/grid_edit.rs::measure_tempo`; EDIT GRID: TEMPO CHANGE HERE starts a new tempo at the beat at the cursor, measured from the kicks after it; pins in between give piecewise-constant tempos; tempo keys change only the section at the cursor (markers survive, the bar count carries on) | 5 | P3 |
| Grid editor: move, tempo about the cursor, pins, auto-fix, evidence | DONE (beyond the usual) | `crates/od-ui/src/grid_edit.rs::apply_op`, `od-analysis/src/transients.rs::transients`, `od-analysis/src/transients.rs::fit_line`, `crates/od-ui/src/render.rs::drift_strip`, `ui/player.slint::GridSessionKeys`, `od-engine/src/deck.rs::render_metronome`; EDIT GRID session in the Prepare player: header with FIT, the waveform shows transients (kicks long) and beat ticks coloured by their error (≤ 5 / ≤ 15 / > 15 ms), DRIFT strip over the whole track (click jumps, worst spot), readout (BPM to type, error at the cursor, pins, source). Moving and tempo are separate: MOVE ±1/±10 ms, ±1 beat, SET 1 HERE (snapped to the kick); TEMPO ±0.01, ÷2/×2, TAP pivot on the cursor and keep markers; PIN (one pin moves, two solve the tempo), AUTO-FIX (least-squares slip and tempo, finds tempo jumps), TEMPO CHANGE HERE; CLICK metronome, SNAP (Alt while dragging: off). Drag the grid (live, snaps ±15 ms), double-click a beat = the 1, wheel scrolls, ⌘-wheel zooms; own keys (←/→, ↑/↓, 1, P, F, C, S, T, [ ], Return, Esc, ⌘Z). The session has its own undo; DONE stores one undo step, CANCEL nothing | 16 |  |
| Grid editor: tap tempo, type BPM | DONE | `crates/od-ui/src/player.rs::tap`, `crates/od-ui/src/player.rs::type_bpm`, `crates/od-ui/src/grid_edit.rs::tap`; EDIT GRID: the BPM field (Return) and TAP (from the fourth tap; T in the editor, B in Prepare) set the tempo about the cursor; while the preview plays the taps, snapped to the kicks, also set the phase; a locked grid is not changed | 4 |  |
| BPM analysis range setting | DONE | Preferences ▸ Analysis BPM range (`preferences.rs` `bpm_range`) feeds the tempo analysis fold range | 1 | P3 |
| Key detection, notation Camelot/Open Key/musical | DONE | `od-analysis/src/key.rs`; `--keys` flag, `KeyNotation` | 7 |  |
| Phrase analysis | DONE | `od-analysis/src/phrase_eval.rs`, `anlz/build.rs`; Analysed (`od-analysis/src/phrase.rs`) and stored: bar features from a spectrum, a self-similarity novelty curve, boundaries by dynamic programming with an 8/16/32-bar prior, labels by musical role (a track without breakdown or build gets no drop). Measured on 31 generated arrangements and a held-out variant (other seeds, saturation, noise, detected grid) against the earlier detector: boundaries within one bar 79 % to 99 %, labels 73 % to 99 %, fake drops 21 to 0; generated tracks, not records. Display: see `deck-phrase-display` | 5 |  |
| Waveform: own 3-band colour | DONE | `od-analysis/src/waveform.rs`, `render.rs` (+ colour-blind palette) | 4 |  |
| Waveform style options (RGB / blue / 3-band) | DONE | `PWAV/PWV2–PWV7`; Preferences ▸ View ▸ Waveform colour: 3-band (low / mid / high stacked), blue, or RGB (one envelope, hue from the band balance) | 2 | P3 |
| Vocal position | DONE | Derived from the separated vocal part when a track has parts: spans where the vocals sing are drawn as a band along the top of the deck overview | 2 | P3 |
| Loudness + auto gain | DONE | `od-analysis/src/loudness.rs`; gain in `perform.rs::load_track`, engine default on (`mixer.rs` `auto_gain: true`); no toggle in UI | 3 |  |
| Re-analyse selected, never overwrite manual/imported | DONE | `on_analyse_selected`; `DataSource` priority in `tracks.rs::store_analysis` | 3 |  |
| Phrase editing (GRID EDIT \| PHRASE EDIT, cut / clear) | DONE | `crates/od-ui/src/phrase_edit.rs`, `crates/od-library/src/tracks.rs::set_phrases`; PHRASE tab next to GRID in the Prepare player: kind of the phrase at the cursor (a gap gets a new phrase), CUT at the nearest bar (at least 4 bars each side), BOUNDARY TO CURSOR, CLEAR; keyboard commands. Stored with Library::set_phrases (normalized with od_core::normalize_phrases, one undo step, change log → sticks resync). A new analysis of the track replaces edited phrases; (UX §2.5) | 6 | P3 |
| BPM range presets for tempo detection (70–180, 58–115, 98–195, 118–236, 150–300) | DONE | `settings.rs::BPM_RANGES`, `preferences.rs::bpm_choices`, `workers.rs::AnalysisControl`, `tempo.rs::sanitize_range`; Preferences > Analysis offers five BPM ranges (70–180 is the default), used for new analyses and for analysing a track again. A range stored by an older version stays selected and in use until another is chosen | 2 | P2 |
| Tempos up to 300 BPM without halving, with the active range shown | DONE | `tempo.rs::refine_tempo`, `settings.rs::BPM_RANGES`, `shell.rs::bpm_range_text`; Roadmap [57]. BPM range 150–300 in Preferences > Analysis; generated click, kick, kick-and-snare and house loops at 170, 200, 220, 250 and 280 BPM read as played in every range that holds them. The active range shows in the Prepare player's INFO tab and beside the analysis progress. Not yet checked on a set of real hardcore or speedcore records | 2 | P2 |
| Confidence for key and grid analysis with a one-button fix | DONE | `od_core::AnalysisConfidence`, `key.rs::ranked_key`, `tempo.rs::grid_confidence`; Roadmap [62]. The key detection reports its lead over the runner-up, the grid its share of beats on a hit (lower when half / double was close). Prepare's INFO tab shows both in percent beside BPM and KEY, an unsure one with a half-height sodium line and USE for the likely key or tempo (your key, or the grid at half / double; one undo step) and ANALYSE AGAIN; an unsure key shows its percent under the deck's KEY. Analyses from before this version show none until analysed again | 5 | P2 |
| Confidence per phrase, unsure phrases marked in Prepare | DONE | `od_core::Phrase::confidence`, `phrase.rs::label`, `ui/player.slint::PhraseStrip`; Roadmap [62] for phrases. Each analysed phrase says how sure the analysis is: the weaker of how clear its boundaries are and how clearly its type tests pass. Below 50 % the Prepare phrase strip marks it with a half-height sodium line (as an unsure key) and its percent on hover, and the PHRASE tab says UNSURE with the percent while the cursor is in it; picking its type, cutting, merging or moving an edge confirms it (one undo step). Phrases set by hand, imported or from older analyses carry no value and count as sure | 6 | P3 |

## 3. Cues and loops

| Feature | Status | Evidence / note | Tests | Prio |
|---|---|---|---|---|
| 8 hot cues A–H (Prepare) | DONE | `app.rs` `on_hot_cue/on_remove_hot_cue`; keys 1–8 | 2 |  |
| 8 hot cues A–H (Perform, pads/UI) | DONE | `DeckCommand::HotCue`, `od-ui/src/decks.rs::persist_cue_events`; Set/delete on pads or the controller are saved to the library (one undo step each) and reach sticks through the normal sync | 2 | **P1** |
| Hot cue banks / hot cue bank lists | DONE | `pads.rs::bank_label`; Banks A–H, I–P, Q–X, Y–AF with ‹ › in Hot Cue mode (keyboard too); pads show the bank's letters and labels | 2 | P3 |
| Memory cues: add (Prepare) | DONE | `on_add_memory_cue` | 1 |  |
| Memory cues: delete, move (Prepare) | DONE | Cue list (ALL / MEMORY / HOT CUE): select a cue to edit it: name, colour, move to the cursor (beat-snapped, loops keep their length), make a memory cue a hot cue, delete | 2 | **P1** |
| Memory cues: call prev/next (Perform) | DONE | `MemoryCueJump` via controller | 1 |  |
| Memory cue set/delete from controller | DONE | `decks.rs::memory_cue`; `deck.memory_cue_set` / `_delete` edit the library cue list (one undo step) and update the deck without a gap | 1 | **P1** |
| Saved loops (memory/hot-cue loops) | DONE | Loop hot cues and memory loops (MEMORY while a loop plays) are stored with their loop end; the Prepare cue editor moves loops keeping their length; import / export / playback carry them | 3 | **P1** |
| Automatic personalised cue points | DONE | `autocue.rs::learn`, `autocue.rs::apply`, `workers.rs::AnalysisWorker`, `browser/mod.rs::set_cues_like`; Learns where you put hot cues (per pad) and memory cues relative to the first downbeat, the phrases (1st–3rd Intro, Chorus, Up, …) and the last downbeat in whole bars, with their names, colours and loop lengths (≥ 3 analysed tracks with cues, most of them agreeing). Row menu "Set cues like my others" fills free pads / missing memory cues on the selected tracks; Preferences > Analysis > Cue points (off by default) does it for newly analysed tracks without cues. One undo step per track; the change log syncs sticks | 4 | P2 |
| Active loop (auto-engaged saved loop) | DONE | ACTIVE in the cue editor marks a saved loop; it is stored (library schema 3) and engages by itself when the playhead reaches it (`DeckCommand::SetActiveLoop` on load) | 3 | P3 |
| Cue colours | DONE | Eight cue colours in the cue editor; shown on overviews, the cue list, Prepare pads and the Perform hot cue pads | 3 | **P1** |
| Cue comments / labels | DONE | Double-click a cue in the list, type the name, Return; names show on pads and in the list | 2 | **P1** |
| Auto cue on load (first memory cue / first beat) | DONE | `DeckCommand::Load` behaviour (`command.rs` doc, `deck.rs`) | 1 |  |
| Cues and histories changed on players come back as proposals | DONE (beyond the usual) | `od-devices/src/club_usb.rs` proposals, reviewed in the stick board (`on_accept_proposal`) | 2 |  |

## 4. Deck / player

| Feature | Status | Evidence / note | Tests | Prio |
|---|---|---|---|---|
| 2 / 4 decks, deck layers | DONE | `on_set_deck_count`, four-deck controllers switch to 4 decks (`perform.rs::frame`), `DeckSelect` in `od-controller/src/session.rs` | 3 |  |
| Play / Cue (player-style cue logic) | DONE | `DeckCommand::Cue`; UI + controller | 2 |  |
| Jog: vinyl scratch, bend, search; vinyl mode toggle | DONE | `DeckCommand::JogTurn/JogTouch/SetJogMode`, `handle_action` | 2 |  |
| Tempo fader, ranges ±6/10/16/WIDE, reset | DONE | `TempoRange`; the range key above CUE (and on compact decks) and the controller cycle ±6 / ±10 / ±16 % / WIDE; the platter shows the range | 3 |  |
| Master tempo / key lock (Rubber Band) | DONE | `od-engine/src/keylock`, UI toggle + controller | 4 |  |
| Key shift (±semitones) | DONE | `decks.rs::key_shift`; ‹ ±0 › in the deck header (click resets), Key Shift pad mode, keyboard | 2 | P2 |
| Key sync | DONE | `decks.rs::toggle_key_sync`; KEY SYNC in the deck header, keyboard and controller (`deck.key_sync` with LED); the track key reaches the engine on load | 1 | P2 |
| Beat sync + master deck | DONE | `SetSync`, `SetMasterDeck`; UI + controller | 1 |  |
| Quantize | DONE | `Command::SetQuantize`; UI + controller | 1 |  |
| Slip | DONE | `SetSlip`; UI + controller | 1 |  |
| Reverse | DONE | `SetReverse` via controller; no UI button | 2 |  |
| Censor (slip reverse) | DONE | `DeckAction::Censor` in `handle_action` | 1 |  |
| Beat jump | DONE | `DeckCommand::BeatJump`; UI ±4, controller, pad mode | 1 |  |
| Beat jump size setting | DONE | `decks.rs::step_jump_size`; ‹ › in Beat Jump pad mode (1/8 … 64); the deck's −/+ buttons and the keyboard jump by it | 1 | P3 |
| Manual loop in/out, reloop/exit, ½ / 2× | DONE | `LoopIn/LoopOut/Reloop/LoopExit/LoopHalve/LoopDouble`; IN / OUT / EXIT key beside LOOP on every deck and in the Prepare player | 1 |  |
| Auto beat loop (1/32 … 32) | DONE | `DeckCommand::AutoLoop`; UI size stepper | 1 |  |
| Loop roll | DONE | `DeckCommand::LoopRoll`, `crates/od-ui/src/pads.rs::ROLL_BEATS`, `ui/wave.slint::WaveLane`; Roadmap [59]. ROLL pad mode on the deck (and `deck.loop_roll` / pad mode `loop_roll` in mappings): pads A–H roll 1/32 … 4 beats while held, quantised to the grid; the track carries on underneath (slip line on the waveform) and resumes there on release. The held pad is filled and the waveform's loop band carries ROLL and the size | 3 |  |
| Needle search | DONE | `NeedleSearch`; overview click + the controller touch strip | 1 |  |
| Search (FF/REW buttons) | DONE | ◀◀ / ▶▶ on the player screens search while held (`DeckCommand::Search`) | 2 | **P1** |
| Tap BPM | DONE | `decks.rs::tap_bpm`; Controller TAP: three taps give a constant grid at the tapped tempo, keeping the first downbeat (saved, undoable; locked grids stay) | 2 | **P1** |
| Pad mode: Hot Cue | DONE | `PadMode::HotCue` in `handle_action` | 1 |  |
| Pad mode: Beat Loop | DONE | `PadMode::BeatLoop` → `AutoLoop` | 2 |  |
| Pad mode: Beat Jump | DONE | `PadMode::BeatJump` | 2 |  |
| Pad mode: Sampler | DONE | `pads.rs::sampler_slot`; Odd decks play pads 1–8, even decks 9–16 of the shown bank; gate samples follow the pad | 1 | P2 |
| Pad mode: Hold FX 1/2 | DONE | `pads.rs::press_commands`; Hold FX page 1 / 2 (‹ ›), momentary | 2 | P2 |
| Pad mode: Keyboard | DONE | `pads.rs::KEY_STEPS`; −3 … +4 semitones on the chosen hot cue (‹ CUE A ›) | 2 | P3 |
| Pad mode: Key Shift | DONE | `pads.rs::press_commands`; Pads set the key shift −3 … +4 | 1 | P2 |
| Pad mode: Slicer | DONE | `pads.rs::mode_change_commands`; `PadMode::Slicer`; domain 4 … 64 beats (‹ ›), starts and stops with the mode | 2 | P3 |
| Pad LED feedback per mode | DONE | `controller.rs::feedback`; Pad mode LEDs and the pads of the current mode (on = engaged, dim = loaded / set) | 2 | P2 |
| Waveform zoom in Perform | DONE | `perform.rs`; Mouse wheel over a lane zooms that deck, double-click resets (`decks::zoom_by`); zoom buttons of the zone column are tracked by the view test (`wave.zoom-*`) | 4 |  |
| Waveform overview + scrolling detail, cue markers, beat ticks | DONE | `render.rs::overview/detail`, `ui/plate.slint` | 3 |  |
| Phrases on the overview and enlarged waveform, with display options | DONE | `crates/od-ui/src/player.rs::phrase_marks`, `crates/od-ui/src/render.rs`, `crates/od-ui/src/settings.rs::PHRASE_TYPES`; Phrases on the enlarged waveforms and the phrase bar under the overviews switch separately (Preferences > View), phrase types as names or colours only; Prepare player and Perform decks; (SPEC §10.1; UX §2.5) | 3 | P2 |
| Deck header: title, artist, artwork, BPM, remaining / elapsed time | DONE | `crates/od-ui/src/format.rs`; Number, cover art (from the library, as a thumbnail), title, artist, BPM, remaining / elapsed time, key, SYNC / MASTER; (SPEC §10.1; UX §1.3) | 6 | P2 |
| Jog / platter readout in the UI: position, remaining, phrase and bar counter | DONE | Jog display: bar counter (bars.beats, count-down modes), remaining time, phrase with its countdown; (SPEC §10.1; UX §1.3) | 1 | P2 |
| Full-width enlarged waveform zone, one stacked lane per deck (2 and 4 decks), centre playhead | DONE | `crates/od-ui/src/decks.rs`; One full-width lane per deck (2 and 4 decks), stacked, playhead at the centre. The zone column (collapse / zoom buttons / expand) is tracked by the view test; (SPEC §10.1; UX §1.3, §4 P1.1) | 1 |  |
| Pad mode: Stems (part mutes and stem FX on pads) | DONE | (UX §2.4) | 2 | P2 |
| Pad mode: Memory Cue | DONE | (UX §1.3) | 2 | P3 |
| Pad mode: Pattern Recall | DONE | Pad mode PATTERN: an empty pad stores the sampler sequence patterns, a stored pad recalls them and starts the sequencer, the shifted pad clears it; (UX §1.3) | 1 | P3 |
| Pad mode: Timed Censor | DONE | Pad mode CENSOR: each pad censors (reverse with slip) for ¼ … 32 beats of the deck tempo, then playback continues where it would be; (UX §1.3) | 2 | P3 |
| Phase meter and tempo difference between decks with a clear in-sync state; practice without SYNC | DONE | A strip under the waveforms in Perform (desktop layouts; not in Club mode or the kiosk touch layout): one ruler per deck against the sync master (else the playing deck), half a beat each way, with the tempo difference in BPM and the phase in ms. IN SYNC (solid green centre) within 0.05 BPM and 0.02 beat. PRACTICE turns SYNC off on all decks and blocks it.; (Roadmap [12]) | 5 | **P1** |
| Booth view: the decks large in their own window (waveform, BPM, key, remaining time, phrase countdown) | DONE | A separate window like the detached LIGHTING view (View > Booth View, Alt+Shift+Cmd+B), sharing the deck models of Perform. The deck's type ranks are multiplied by one zoom that fills the window (640 x 360 canvas): about 96 px hero figures at 1080p. Double-click for full screen. Not on the kiosk (single screen).; (Roadmap [58]) | 1 | P2 |
| Club mode: player-style screens and behaviour driven by the controller | DONE | CLUB toggle (Alt+J): player screens per deck (waveform, overview needle search, jog display with remaining time / bar counter / phrase, four-segment beat position under the waveform (beat 1 accented) for visual beat matching, hot cues with hold, memory cue call, loop in / out / reloop, beat jump, search, tempo range, master tempo), BROWSE screen (source → playlists → tracks, from the library or a stick), player cue rules, sync off, self-ticking checklist; (SPEC §10.3c) | 5 | P2 |
| Playback reservation (queue for Mix Preview) | DONE | `mix-preview.rs::tick`, `ui/mix_preview.slint`; RESERVE in the MIX PREVIEW panel queues the tracks selected in the list; when a panel deck is empty or has played out while the other plays, the next reserved track loads there and waits at its mix-in point for the next link; NEXT shows what comes, CLEAR empties the queue | 2 | P3 |
| Beatgrid and gain panel on the Perform deck | DONE (beyond the usual) | `deck_grid.rs::act`, `deck_grid.rs::tick`, `ui/perform.slint::GridPanel`, `ui/perform.slint::GridMeter`; The grid key beside the pad mode swaps the pads for the panel (the deck keeps its width), working while the track plays: a meter of how far the grid sits off the kicks (median of the last 8 beats, −20…+20 ms), FIT, the tempo and CLICK (metronome in the headphones); SET 1 (the kick just heard, ±60 ms, becomes the 1), ±1 / ±10 ms, ±1 beat; −.01 / +.01 and ÷2 / ×2 about the playhead, PIN (two pins solve the tempo), AUTO-FIX; EDIT … opens the Prepare editor, LOCK, and the track's auto-gain correction ±0.5 dB. Grid changes go to the library at once (decks follow without a gap); nudges within 1.5 s are one undo step | 5 | P2 |
| Loop capture: the loop or roll just heard becomes a hot loop (or memory loop) | DONE | `crates/od-ui/src/decks.rs::capture_loop`, `DeckCommand::SetHotCueAt`; Roadmap [59]. CAPTURE in the ROLL pad mode bar (and `deck.loop_capture`): the loop heard last on the deck, a roll or a loop, is stored as a hot loop on the first free pad of the bank; with all eight taken it becomes a memory loop. One undo step, saved with the track | 2 | P2 |
| Bars to the next drop and breakdown on each deck | DONE | `crates/od-ui/src/phrase_mix.rs::ahead_text`, `ui/controls.slint::Jog`; Roadmap [56]. Under the platter's tempo line: DROP n · BREAK n in whole bars, nearest first (a drop is a chorus after something else, a breakdown a down or bridge phrase). The bars to the next phrase stay on the platter's top line and the waveform counter | 2 | P2 |
| Start on the master deck's next phrase | DONE | `Command::StartOnBeat`, `crates/od-ui/src/decks.rs::phrase_start`; Roadmap [56]. Right click on PLAY (and `deck.phrase_start`): a cued, paused deck waits for the playing master's next phrase boundary (the next 16-bar mark without phrases) and starts within one audio block of it. The key carries the armed outline, the platter counts START -bars.beats; a second press, PLAY or CUE cancel | 5 | P2 |
| Phrase-snapped sync | DONE | `crates/od-ui/src/phrase_mix.rs::sync_jump`, `crates/od-ui/src/decks.rs::toggle_sync`; Roadmap [56]. Preferences > Controller > Decks > Sync: PHRASE. SYNC on a playing deck also jumps it whole beats (the smallest jump) so it stands as far before one of its phrase boundaries as the master before its next one; beat sync keeps the phase | 2 | P2 |
| The incoming track drawn over the playing waveform | DONE | `crates/od-ui/src/decks.rs::project_overlay`, `crates/od-ui/src/render.rs::outline_tile`, `ui/wave.slint::WaveLane`; Roadmap [56]. On the playing master's lane, the other deck's waveform as an outline in its deck colour, on the same time scale: where it plays now, or, while cued, from its cue placed on the master's next phrase (a numbered mark). Preferences > View > Incoming track switches it off | 3 | P2 |
| One-press half and double BPM on the deck | DONE | `crates/od-ui/src/decks.rs::bpm_scale`, `ui/perform.slint::MiniKey`; Roadmap [57]. ÷2 and ×2 beside BPM on the full deck's readout (and `deck.bpm_half` / `deck.bpm_double`): the track's grid at half or double the tempo, one undo step, stored with the track; locked grids refuse. Narrower decks keep it in the GRID panel | 2 | P2 |
| Key relation on the deck against the other playing decks | DONE | `crates/od-ui/src/key_mix.rs::relation`, `MusicalKey::relation`, `ui/perform.slint::Readout`; Roadmap [62]. The deck's KEY is underlined against the other playing decks with the key you hear (key shift, and the tempo without keylock): solid green fits (same, a fifth, relative), dashed sodium an energy change (a whole tone, a semitone, the diagonal), struck deep red a clash; the worst against all playing decks, in words on hover and for screen readers. Our own colour language from DESIGN.md, no product key colours | 4 | P2 |
| Simple, Standard and Full deck layouts (a Simple deck has about 25 large controls) | DONE | `crates/od-ui/src/deck_layout.rs`, `ui/perform.slint::SimpleControls`, `ui/topbar.slint::ViewMenu`, `ui/first_start.slint`; Chosen in the top bar's view menu, View › Deck Layout and the first-start guide. Simple: pads A–H as hot cues, LOOP with ½ / 2× and beat jump, SYNC / CUE / PLAY, the tempo fader, stems and MASTER (23 controls on deck 1); setup keys, the platter, the phase strip and the effects are not built. Standard is the deck as before (compared at 1440 × 900). Full shows every group with the Tempo FX panel open. Not tried on a touch screen.; (Roadmap [6]) | 3 | P2 |
| Whole groups hidden: pads, loops, stems, effects, mixer column, phase strip, setup keys | DONE | `crates/od-ui/src/deck_layout.rs::toggle_group`, `ui/perform.slint::DeckLayout`; A hidden group is not built at all, so it can neither be seen nor reached with the keyboard. The mixer group is the top bar's MIXER switch; hiding the effects also removes the FX toggle, the CH FX knobs and their selector.; (Roadmap [6]) | 3 | P2 |
| Saved deck layouts: named, switched and deleted, kept in the settings | DONE | `crates/od-ui/src/settings.rs::SavedLayout`, `crates/od-ui/src/deck_layout.rs::save`; A saved layout keeps the layout, the hidden groups, the mixer column and the Tempo FX panel; saving under a known name replaces it.; (Roadmap [6]) | 3 | P2 |
| Deck centre: platter centred with a concave block of setup keys, transport on the same axis; Club keys wrap the jog | DONE | `crates/od-ui/ui/controls.slint::ConcaveKey`, `crates/od-ui/ui/perform.slint::PlatterGroup`; Keys take the pointer only where they are drawn, never over the ring. Not tried on a touch screen.; (Roadmap [46]) | 2 | P3 |
| Calm waveform option: crisp playhead, faint beat lines, softer colours | DONE | `crates/od-ui/src/render.rs::beat_mark`, `crates/od-ui/ui/wave.slint::WaveLane`; Preferences > View > Calm waveform, off by default.; (Roadmap [50]) | 2 | P3 |
| Waveform zone: view keys in one column, the playhead on the window's centre (the mixer's axis) | DONE | `crates/od-ui/ui/perform.slint::WaveZoneColumn`, `crates/od-ui/ui/wave.slint::WaveLane`; Short zones move the height keys behind a ⋯ key. Vertical lanes, Club and the booth keep their own centre.; (Roadmap [50]) | 2 | P3 |
| Drag a waveform lane: scrub a paused deck, nudge a playing one | DONE | `crates/od-ui/src/lane_drag.rs`; Scrubbing is silent. The nudge uses the engine's pitch bend; SLIP return is computed from the tempo at the start of the drag. Tried with a mouse only.; (Roadmap [50]) | 5 | P2 |
| Compact decks of a small window carry pads and loops and share the height with the mixer | DONE | `crates/od-ui/ui/perform.slint::KioskDeck`; (Roadmap [24]) | 1 | P3 |

## 5. Mixer

| Feature | Status | Evidence / note | Tests | Prio |
|---|---|---|---|---|
| Trim | DONE | `ChannelCommand::SetTrim`; UI knob + controller | 2 |  |
| 3-band EQ (isolator) | DONE | `dsp/eq.rs` default `Isolator`; UI + controller | 3 |  |
| EQ mode classic/isolator | DONE | `ChannelCommand::SetEqMode`, `crates/od-ui/src/preferences.rs::mixer_settings`; Preferences > Controller > Mixer: isolator / classic EQ, stored and applied live and at engine start | 2 |  |
| Kill per EQ band | DONE | `ChannelCommand::SetEqKill`, `crates/od-ui/src/mixer.rs::toggle_kill`; Roadmap [55]. K (kill) beside HI, MID and LOW on each channel (and `mixer.eq_kill` in mappings): the band goes silent (isolator) or to −26 dB (classic EQ) while lit; the knob keeps its place and the release returns to it | 5 | P2 |
| One-tap EQ swap between two decks, now or on the next beat or bar | DONE | `MixerCommand::SwapEq`, `crates/od-ui/src/mixer.rs::swap`; Roadmap [55]. LOW / MID / HI in the swap row under the channels (and `eq_swap` in mappings) exchanges the knob and kill of that band between the two loudest playing decks (one playing: it and the next loaded deck); NOW / BEAT / BAR sets the timing on the master clock, a waiting swap shows armed and a second tap cancels it. Landing is block-accurate in the engine | 10 | P2 |
| Larger EQ and filter knobs with their values, double-click reset | DONE | `ui/perform.slint::BandRow`, `crates/od-ui/src/mixer.rs::eq_text`; Roadmap [55]. EQ and CH FX knobs 24 px at desktop size (before 22; the compact tier keeps 16), strips still 50 px; the legend under each knob turns into its value once turned: dB for TRIM and EQ, OFF at the isolator's full cut, KILL while killed, LP / HP and percent for the filter; double-click (double-tap) resets | 3 | P2 |
| Clip lamp per channel | DONE | `MeterReading::clipped`, `MixerCommand::ResetClipIndicators`; Roadmap [55]. CLIP beside the channel number lights red once the channel went over full scale before the fader and stays lit until clicked; the click clears all clip indicators, the master's too | 2 | P2 |
| Channel FX knob (filter) | DONE | `SetChannelFxAmount`; UI + controller (default kind = filter) | 1 |  |
| Channel fader | DONE | UI + controller | 3 |  |
| Channel fader curve | DONE | `crates/od-ui/src/preferences.rs::mixer_settings`; Preferences > Controller > Mixer: linear / smooth / sharp | 3 |  |
| Crossfader | DONE | UI + controller | 2 |  |
| Crossfader curve | DONE | `crates/od-ui/src/preferences.rs::mixer_settings`; Preferences > Controller > Mixer: constant power / linear / cut | 3 |  |
| Crossfader assign A/THRU/B | DONE | Controller (`MixerAction::CrossfaderAssign`) and the A / B assign grids beside the crossfader in the global bar (a key sends the deck to its side, again to THRU) | 2 |  |
| Headphone cue, mix, level | DONE | `SetCue`, `SetHeadphoneMix/Level`; UI + controller | 2 |  |
| Master level, VU meters, limiter | DONE | `SetMasterLevel`, meters in `ChannelView`, limiter on by default | 3 |  |
| Master cue | DONE | `controller.rs::handle_action`; Controller `master_cue` toggles it, with LED | 1 | P2 |
| Booth level / booth output | DONE | `controller.rs::handle_action`; Controller `booth_level` sets the booth output | 1 | P2 |
| Auto Channel FX / Auto Fader | DONE | Auto Channel FX: a channel FX on one knob (low-pass + echo left, high-pass + reverb right); Auto Fader (SMART beside the crossfader): the crossfader swaps the basses of the A and B channels and the decks on the crossfader follow the master tempo | 2 | P3 |
| Mic / aux input | DONE | MIC in the global bar: on, level, input meter, talkover with ducking indicator, input device; engine input channel mixed before Tempo FX / booth / recording | 5 | P3 |
| Auto gain toggle | DONE | `ChannelCommand::SetAutoGain`; Preferences > Controller > Mixer: auto gain on / off for every channel | 2 |  |
| STEM button per mixer channel | DONE | STEM per mixer channel switches its HI / MID / LOW knobs to part isolation (vocals / inst / drums), centre = full level; the channel EQ stays as it was; (UX §1.3, §2.4) | 1 | P2 |
| Knobs, faders and meters that feel physical: value arcs, centre marks, drag readouts, fader caps and detent, peak hold | DONE | `crates/od-ui/ui/controls.slint::Knob`, `crates/od-ui/ui/controls.slint::Fader`, `crates/od-ui/src/mixer.rs::channel_view`; Sizes unchanged at 1440 x 900 and 1280 x 800. Tried with the mouse only, not on a touch screen.; (Roadmap [46]) | 5 | P2 |

## 6. Effects

| Feature | Status | Evidence / note | Tests | Prio |
|---|---|---|---|---|
| Tempo FX (2 units: echo, delay, reverb, flanger, phaser, roll, filter sweep, trans) | DONE | `crates/od-ui/src/fx.rs`, `crates/od-ui/ui/fx.slint`; FX panel (top bar FX) with two units; controller `tempo_fx.*` inputs drive the same functions; ON / release LEDs | 4 | **P1** |
| Tempo FX: beat select, auto beat, tap, channel assign | DONE | `fx.rs::step_beats`, `fx.rs::TapTempo`; Beat ‹ › (1/8 … 16), TAP (three taps) and AUTO (back to the deck tempo), channel assign 1–4 / master, level and parameter knobs; on screen, keyboard and controller | 4 | **P1** |
| Larger Tempo FX list (ping pong, spiral, helix, pitch, vinyl brake, triplet, shimmer …) | DONE | `command.rs::TempoFxKind::ALL`; 17 effects selectable in the panel (list or ‹ ›); controller effect names map to the closest one | 2 | P3 |
| Sound Channel FX selection (filter, noise, space, crush, echo) | DONE | `fx.rs::select_channel_fx`; One global selection (‹ NAME › under the mixer, keyboard, controller CH FX buttons with LEDs), persisted, applied to every channel | 3 | **P1** |
| Exit FX | DONE | `fx.slint::FxUnit`; BRAKE / BACKSPIN / ECHO OUT per unit while held (panel, keyboard); controller release button = echo out | 2 | P2 |
| Hold FX | DONE | `fx.rs::hold_fx_preset`; Two pages of eight presets (beat and exit FX) on the deck pads | 3 | P2 |
| Stem FX (FX on one part) | DONE | PART keys under a unit's channel assign (ALL / VOC / DRUM / BASS / INST): the deck splits that part off (a second stretcher keeps it in step under keylock), the unit processes only the part, and it rejoins the channel before EQ and fader; Lite (Pi) supports it without keylock | 3 | P3 |
| FX panel (FX1 \| FX2 with three slots, beat value, depth, assign, AUTO, TAP; CH FX row) | DONE | `crates/od-ui/ui/fx.slint`; FX1 \| FX2: effect, beat, assign (channels, sampler, master), level, parameter, ON, AUTO, TAP, Exit FX, and ×3 multi mode with three chained slots (effect, beat, depth, ON each); (SPEC §10.2; UX §1.5) | 4 | **P1** |
| Channel FX: pitch, dub echo, sweep | DONE | `settings.rs::CHANNEL_FX`; Pitch, dub echo and sweep selectable like the others; (SPEC §10.2) | 2 | P3 |
| Remix-type effects (build-up riser, breakdown, reverse roll) | DONE | `tempofx.rs::kind_frame`, `command.rs::TempoFxKind::BuildUp`; In the Tempo FX list: BUILD UP cuts the lows ever higher over its length while a noise sweep climbs (parameter sets the noise), BREAKDOWN closes the highs and dips the level, REVERSE ROLL repeats the captured beats backwards; all beat-synced, on pads and controllers by name | 3 | P3 |
| Large effect level with visible intensity | DONE | `ui/fx.slint::Intensity`; Roadmap [61]. Each unit shows its level as a Heading percent over a bar that fills in the unit's tone while the effect is on; drag along it or turn the wheel. Replaces the small depth knob of slot 1 | 2 | P2 |
| FX panic: every effect off at once | DONE | `Command::FxPanic`, `ui/fx.slint::PanicKey`; Roadmap [61]. ALL FX OFF in the FX panel, ⇧Esc and the mapping `fx_panic`: both Tempo FX units with their slots, tails, exit and Hold FX, and the channel FX knobs fade to dry within 5 ms and their buffers are cleared | 3 | P2 |
| XY control for an effect unit | DONE | `ui/fx.slint::XyPad`, `crates/od-ui/src/fx.rs::xy`; Roadmap [61]. XY turns the unit into one surface: across sets the effect's parameter, up its level. A press plays an idle effect while held (its tail rings out on release); an effect that was on stays on | 2 | P2 |
| Build-ups locked to 8, 16 or 32 bars that ramp to the drop | DONE | `TempoFxCommand::BuildUp`, `ui/fx.slint::FxUnit`; Roadmap [61]. BUILD 8 / 16 / 32 starts the build-up riser so it peaks on the next line of that many bars on the grid (the one after when less than a bar is left) and switches off there for the drop; the unit counts the bars left and the key fills with the progress. Mapping `tempo_fx.build_up` with `bars` | 3 | P2 |
| Tails after release (echo or reverb ringing out) | DONE | `TempoFxCommand::SetTail`, `FxTail`; Roadmap [61]. TAIL OFF / ECHO / REVERB per unit: an effect without a tail of its own (roll, flanger, filter, build-up …) rings out after you switch it off as an echo or reverb of its sound at release; echo, delay and reverb kinds ring out anyway | 2 | P2 |

## 7. Stems / track separation

| Feature | Status | Evidence / note | Tests | Prio |
|---|---|---|---|---|
| Offline separation (vocals, drums, bass, inst), cache, background | DONE | `od-stems` (`worker.rs`, `cache.rs`), `perform.rs::request_stems/poll_stems` | 5 |  |
| Part mute toggles in UI | DONE | `on_deck_stem` → `SetStemLevel` | 2 |  |
| Stem levels / part isolator knobs | DONE | Part keys on each deck: click mutes, the wheel sets the level (level line under the key); The part isolator knobs of a four-deck controller drive the same levels (mapping marked verify until checked on hardware) | 2 | **P1** |
| Stem buttons on the controller | ENGINE-ONLY, deferred to M6 | `mappings/controllers/ddj-flx10.toml`; `DeckAction::StemMuteToggle` and the mapping action exist; which notes are the part buttons (vs. mix point buttons) still has to be confirmed on a real controller (M6) | — | **P1** |
| Separate ahead of time (Prepare, batch) | DONE | `crates/od-ui/src/stems_batch.rs`; Row menu Separate parts and ⌥⌘⇧S queue the selected tracks on a background separation worker writing into the decks' stem cache; the status line reports progress and the result. Also `od-cli stems` | 2 | P2 |
| Separation model downloaded once on demand, nothing leaves the machine | DONE | `crates/od-stems/src/model.rs`; (SPEC §10.3) | 1 |  |
| Stem model workflow: asked before downloading, progress, resume, plain errors with HELP, Preferences repair | DONE | `crates/od-stems/src/model.rs`, `crates/od-ui/src/stems_model.rs`, `crates/od-ui/ui/stems_model.slint`; No implicit download: a part key, the STEM switch or batch separation asks (dialog when quiet, notice while music plays or records, dialog after it stops); NOT NOW holds for the session. Resumable download with percent and size, cancellable, SHA-256 verified, free space checked first. Plain failures with RETRY and HELP. Preferences › Analysis › Stems: status, size, DOWNLOAD, DELETE AND DOWNLOAD AGAIN, REMOVE MODEL. Kiosk and builds without separation never ask; (SPEC §10.3) | 18 | P2 |
| Deck shows separation progress; stem controls visible (dimmed) while separating | DONE | Separation is queued on load; the stem strip shows SPLIT and the progress in percent; the part keys stay in place, disabled until parts exist (flow test); (SPEC §10.3; UX §2.4) | 2 | P2 |
| Raspberry Pi plays parts separated on the desktop | DONE | The native sync copies cached parts to /OPENDJDECKS/stems on the stick (and prunes stale ones); tracks loaded from a stick read their parts from there, so the Pi plays parts separated on the desktop; (SPEC §10.3) | 2 | P2 |
| Stems ready before the gig: separate a whole list (playlist or set) in the background | DONE | `crates/od-ui/src/stems_batch.rs::separate_list`, `ui/table.slint::TrackLine`; Roadmap [60]. Row menu Separate stems for the whole list, File > Separate Stems for This List and ⌥⇧⌘P queue every track of the list on the background worker; tracks whose stems are already in the cache are skipped. The queue waits while decks play (performance lock). A deck holding a track the batch finishes takes its stems without a gap | 2 | P2 |
| STEMS column: stem state per track in the library | DONE | `crates/od-ui/src/stems_batch.rs::track_state`, `browser/columns.rs::COLUMNS`, `ui/table.slint::TrackLine`; Roadmap [60]. An optional column (column menu): READY with a solid green line when the stems are in the cache, QUEUED and the percent with a dashed sodium line while the batch works, FAILED struck; empty when not separated. One file check per track, remembered | 2 | P2 |
| Stem state on each stem key and a warning when stems are missing where expected | DONE | `crates/od-ui/src/stem_mix.rs::key_state`, `crates/od-ui/src/decks.rs::stems_warning`, `ui/perform.slint::StemKey`; Roadmap [60]. The stem keys show the batch's QUEUED (dashed sodium line along each key), the separation as a progress fill, and NO STEMS with a sodium line when a track without stems comes in where stems were in use (the track it replaced had them, or another deck plays with them); the status line says so and how to fix it | 3 | P2 |
| Solo a stem and keep only this stem | DONE | `crates/od-ui/src/stem_mix.rs::set_solo`, `crates/od-ui/src/stem_mix.rs::keep_only`, `DeckAction::StemSolo`; Roadmap [60]. Right click on a stem key solos it (the other parts quiet, SOLO on the strip) and a second right click brings the levels back as they were; ⇧-click keeps only this part for good. Mappings: `deck.stem_solo` (held) and `deck.stem_keep` | 3 | P2 |

## 8. Sampler

| Feature | Status | Evidence / note | Tests | Prio |
|---|---|---|---|---|
| 16 slots, one-shot/loop, gain, stop | DONE | `crates/od-ui/src/sampler.rs`; 16 pads per bank in the sampler panel: play, stop all, clear, gain | 3 | P2 |
| Load samples / sample library | DONE | `sampler.rs::load_selected`, `sampler.slint::SampleDrop`; LOAD SELECTED or drop a track row on a pad; decoded off the UI thread; slot contents kept in the library meta and restored when audio starts | 2 | P2 |
| Banks (bank next/prev) | DONE | `sampler.rs::step_bank`; Four banks (panel, keyboard, controller `sampler.bank_*`) | 1 | P2 |
| Sampler volume, cue | DONE | `sampler.rs::set_volume`; VOL knob and CUE in the panel; controller `sampler.volume` / `sampler.cue` with LED | 2 | P2 |
| Sequencer | DONE | `sampler.slint::SamplerPanel`; Footer row of the sampler panel: REC / PLAY / SAVE (onto the next free Pattern Recall pad), the 16 steps of the selected slot, the playing step and CLR; patterns saved with the slots | 4 | P3 |
| Sampler panel (2 x 8 slots, gain, cue, BPM, bank row) | DONE | `crates/od-ui/ui/sampler.slint`; Between the global bar and the browser (top bar SAMPLER): bank, 2 × 8 pads, selected slot (mode, sync, BPM, gain), quantise, cue, volume; (SPEC §10.3a; UX §1.5) | 2 | P2 |
| Gate mode, quantised triggering, sample sync to the master tempo | DONE | `sampler.slint::SamplerPanel`; 1-SHOT / LOOP / GATE (pads report press and release), QUANT, SYNC per slot; (SPEC §10.3a) | 4 | P2 |

## 9. Recording

| Feature | Status | Evidence / note | Tests | Prio |
|---|---|---|---|---|
| One-button mix recording (UI + controller) | DONE | `perform.rs::toggle_recording`, `ui/topbar.slint`, `RecordingToggle` + LED | 2 |  |
| WAV output | DONE | `od-recorder/src/lib.rs` (hound) | 1 |  |
| FLAC / MP3 output | DONE | WAV recorded, then encoded to FLAC (bit-exact) or MP3 320k when recording stops; format chosen in Preferences ▸ Recording | 1 | P2 |
| Automatic tracklist (.cue + text) | DONE | `Recorder::mark_track`, `cue_sheet`, `text_tracklist` | 2 |  |
| Split into per-track files | DONE | `od_recorder::split`, `on_split_recording` | 1 |  |
| Recordings section in the library | DONE | `recording` table, `refresh_recordings`, rail "RECORDINGS" | 1 |  |
| Silence auto start/stop | DONE | `crates/od-ui/src/recorder.rs::auto_step`; `od_recorder::SilenceDetector` drives the recorder: start after 2 s of sound (Preferences > Recording > Start), optional stop after 30 s of silence (> Stop); a stop by hand disarms | 3 |  |
| Output folder / file name setting, recording level | DONE | `crates/od-ui/src/recorder.rs::recording_name`, `crates/od-ui/src/recorder.rs::apply_gain`; Preferences > Recording: folder, file name pattern ({date}, {time}, {n}), recording level −12…+12 dB (also on the recording panel) | 3 |  |
| Recording panel: source, level, meter, elapsed time, file size, live tracklist | DONE | `crates/od-ui/src/recorder.rs::frame`, `crates/od-ui/ui/rec.slint`; Recording panel (Recordings section): REC, source, level, meter, elapsed time, file size, file name and the tracklist as it grows; `RecStrip` (source, level, meter) for the global bar; (SPEC §11; UX §2.3) | 3 |  |

## 10. Export / USB devices

| Feature | Status | Evidence / note | Tests | Prio |
|---|---|---|---|---|
| Automatic incremental sync (Sync Manager equivalent) | DONE (beyond the usual) | `od-devices/src/sync.rs`, `manager.rs`; device sidebar | 4 |  |
| Device registry, formats, assigned content, rebuild, eject | DONE | `app.rs` `on_register_volume/on_set_device_*/on_rebuild_device/on_eject_device` | 4 |  |
| Browse / play from a stick | DONE | `on_open_device`, read-only library | 1 |  |
| OpenDJDecks native stick format | DONE | `od-format-native` | 3 |  |
| Club player USB export (`export.pdb` + ANLZ DAT/EXT/2EX) | DONE | `od-format-clubusb`, `od-devices/src/club_usb.rs`; per-stick choice in the stick board (own format stays the default) | 9 |  |
| Player settings on Club player USB (`MYSETTING`) | DONE | `od-format-clubusb/src/mysetting.rs`, `od-devices/src/club_usb.rs::player_settings`; `/PIONEER/MYSETTING.DAT` written on every Club player USB sync from the deck defaults (quantize, auto cue, tempo range, jog mode, language); other settings get the documented defaults | 5 | P3 |
| Sync all sticks at once, with eject when done | DONE | `manager.rs::sync_all`, `manager.rs::sync_stick`, `devices.rs::wire`; SYNC ALL STICKS (and the sync key in the DEVICES heading) runs every plugged-in stick on its own thread with its own progress line. A stored Eject when done ejects each stick once its sync has finished; a stick that fails keeps its error and stays mounted, the others carry on | 2 | P2 |
| Stick check before a gig: file, cues and grid per track, playlists, a ready verdict | DONE | `crates/od-devices/src/check.rs`, `crates/od-ui/src/stick_check.rs`; CHECK STICK on the Devices page reads both stick formats back (OpenDJDecks library, Club player USB export and analysis files). Ready only when every file opens as audio, cues and grids match the library, playlists hold the library's tracks in order, the chosen formats are there and no sync was cut off. Whether a given player model accepts the stick is not covered (roadmap [H2]).; (Roadmap [53]) | 8 | P2 |
| Maximum compatibility: FLAC and AAC/ALAC as 16-bit WAV on Club player USB sticks | DONE | `convert.rs::write_wav`, `sync.rs::sync_device`, `club_usb.rs::ClubUsbWriter`, `placement.rs::WavCopy`; Off by default, per stick. FLAC, AAC and ALAC tracks are written as stereo 16-bit WAV for Club player USB (sample rates above 48 kHz are reduced); the export and analysis files name the .wav, a conversion is only redone when the source changes and removed when it is stale. Sticks that also carry the OpenDJDecks format keep the original file for it. Originals in the library are never touched. | 5 | P2 |
| Stick screen: each stick with format badges, its sync state in words and in the line, and SYNC / CHECK / EJECT on its row | DONE | `crates/od-ui/ui/devices.slint::DeviceLine`; Tried with simulated sticks (a folder with the stick marker); not yet with several physical sticks ([H3]).; (Roadmap [48]) | 1 | P3 |

## 11. Preferences, audio and controllers

| Feature | Status | Evidence / note | Tests | Prio |
|---|---|---|---|---|
| Preferences window | DONE | `ui/preferences.slint`, `preferences.rs::open`, `settings.rs::Settings`; Separate window (⌘, / Ctrl+, from the native menu and the keymap, gear in the top bar, Esc / ⌘W closes): Audio, View, Analysis, Controller, Keyboard, Recording, Advanced; stored in the library meta, CLI flags override for one run | 4 | **P1** |
| Audio device, buffer size, output routing | DONE | `engine.rs::audio_config`, `preferences.rs::refresh`; Preferences > Audio: output device (automatic = controller interface), buffer 64–1024, sample rate, headphones auto / 3-4 / off; applies when audio next starts | 2 | **P1** |
| Language, key notation, colour-blind, high contrast, motion | DONE | `preferences.rs::apply_live`, `shell.rs::apply_theme`; Preferences > View, applied live in both windows; UI scale (75–200 %) after restart | 3 | **P1** |
| Waveform options (zoom default, phrase display) | DONE | `settings.rs::ViewSettings`, `crates/od-ui/src/preferences.rs::default_deck_zoom`; Phrase display on/off (live, Prepare and Perform); waveform zoom default (4/8/16/32 s) for the Prepare player and the decks; waveform colour 3-band / blue / RGB read by the renderer | 3 | P2 |
| Deck defaults (tempo range, quantize, sync mode, jog mode, auto cue) | DONE | `crates/od-ui/src/preferences.rs::deck_defaults`, `crates/od-ui/src/preferences.rs::apply_engine_settings`; Preferences > Controller > Decks: tempo range, quantize, sync mode (beat or tempo only), jog mode and auto cue, stored and sent to every deck when audio starts and on change | 5 | P2 |
| Controller LED feedback | DONE | `perform.rs::feedback` (play, cue, sync, master, key lock, slip, loop, pads, VU) | 3 |  |
| Soft takeover | DONE | `od-controller/src/takeover.rs` | 4 |  |
| MIDI learn / MIDI monitor | DONE | `crates/od-ui/src/midi_tools.rs`, `od-controller/src/learn.rs`; Preferences ▸ Controller: MIDI monitor (raw messages with the mapping hits) and MIDI learn (action, deck, the learned control described and as a mapping entry; ADD TO MY MAPPING writes it to the own mapping file, checked to load) | 7 | P3 |
| Controller self-test before a gig: controls ticked as they answer, LEDs flashed | DONE | `crates/od-controller/src/hardware.rs`, `crates/od-ui/src/midi_tools.rs`, `od-controller/src/io.rs::light_all`; Preferences ▸ Controller ▸ Self-test: every control of the chosen mapping with its hardware status; START ticks each control when its MIDI message arrives, FLASH LEDS lights every mapped LED for two seconds and then restores the feedback. Tested with simulated MIDI; not yet run on a real unit.; (Roadmap [52]) | 4 | P2 |
| Hardware status per model and control (verified, partial, untested), generated from the mappings | DONE | `crates/od-controller/src/hardware.rs`, `crates/od-controller/examples/hardware_status.rs`, `docs/guide/en/controllers.md`; The controllers guide lists every mapped control; `confirmed = true` marks verified, `verify = true` untested, the rest partial. Nothing is confirmed on hardware yet.; (Roadmap [52]) | 3 | P2 |
| Audio output on macOS (CoreAudio) | DONE | `crates/od-audio-io/src/output.rs`; (SPEC §10.4) | 1 |  |
| Linux audio: PipeWire / JACK, ALSA direct on the Pi | DONE | `crates/od-audio-io/src/devices.rs`; ALSA (always; direct on the Pi), JACK and PipeWire (features `jack`, `pipewire`) selectable in Preferences ▸ Audio; CI builds and tests all three hosts on Linux x86_64 and aarch64; (SPEC §10.4) | 4 | P2 |
| Four-deck controller mapping (LEDs) | DONE | `mappings/controllers/ddj-flx10.toml`; (SPEC §9.1) | 3 |  |
| Two-deck controller mapping (LEDs) | DONE | (SPEC §9.1) | 2 |  |
| Browser font size and line spacing | DONE | `crates/od-ui/src/shell.rs::apply_browser_style`; Preferences > View > Browser: text size 12–18 px and line spacing compact / normal / roomy (22 / 28 / 34 px rows), live; (UX §1.8, §4 P5.17) | 2 |  |
| Published controller list: status per model and per controller class | DONE | `crates/od-controller/src/hardware.rs::controller_list_markdown`, `docs/guide/en/controllers.md`; Generated from the mapping files and the guided-setup templates. A model is verified only when every control of it is confirmed on a unit; today none is, so every row says not confirmed.; (Roadmap [13]) | 2 | P2 |
| Guided setup for controllers without a mapping: move each core control when asked, save a shareable mapping | DONE | `crates/od-controller/src/guided.rs`, `crates/od-ui/src/controller_setup.rs`, `ui/controller_setup.slint`; Preferences › Controller › SET UP A CONTROLLER or View › Controller Setup. Built on MIDI learn; the matching on-screen control gets the guide outline. Ports without a mapping are opened only while the setup, MIDI learn or the monitor runs. Every captured entry is written with verify = true. Tested with simulated MIDI only; not run with a real unit.; (Roadmap [13]) | 4 | P2 |
| Generic controller templates by class: two decks entry level, two decks battle style (CUT curve), four decks | DONE | `mappings/templates/two-deck-entry.toml`, `mappings/templates/two-deck-battle.toml`, `mappings/templates/four-deck.toml`; Templates name no model and hold no MIDI numbers; the guided setup fills them from the owner's unit. No new model mapping was derived from manuals, vendor software or other projects' files.; (Roadmap [13]) | 2 | P2 |
| Preferences in the new design with live previews: waveform colours and calm waveform, deck colours and status lines (colour-blind, contrast), the interface scale and the deck layout | DONE | `crates/od-ui/ui/preferences.slint::LivePreview`, `crates/od-ui/src/preferences.rs::preview_waveform`; The waveform preview draws a made-up bar with the renderer of the app; the scale preview shows a reading at the chosen scale (the window itself follows the operating system's scale). Deck layout is chosen here as well as in View › Deck Layout.; (Roadmap [48]) | 1 | P3 |

## 12. Shell, layouts and panels

| Feature | Status | Evidence / note | Tests | Prio |
|---|---|---|---|---|
| Edit mode (track editing / remix) | DONE | `crates/od-ui/src/track_edit.rs`; Minimal edit mode (EDIT tab of the Prepare player): a region (IN / OUT at the cursor or shift-drag on the overview) is cut out, trimmed to or repeated with 5 ms crossfades at the joins; the result is a new 24-bit WAV next to the original, added as "<title> (Edit)" with the original tags and artwork and analysed; the original stays untouched. No multi-track arrangement | 5 | P3 |
| Prepare / Perform mode switch that keeps loaded tracks | DONE | `crates/od-ui/src/shell.rs`; (SPEC §3.1) | 1 |  |
| Perform layouts: 2 / 4 decks horizontal and vertical, Browse | DONE | 2 and 4 decks in three layouts (persisted): standard (lanes across the top), vertical (Alt+V: upright lanes with zoom keys above the mixer) and Browse (Alt+B: header, overview and parts per deck, browser enlarged); (SPEC §10.1; UX §1.4) | 5 | P2 |
| Prepare layouts: 1 player, 2 players, dual, simple, full browser | DONE | `crates/od-ui/src/shell.rs::set_prepare_layout`; Player and browser, 2 players (full player plus a compact second player), dual (two compact players side by side), simple (one compact player) and full browser: top-bar switch, View menu, ⌥⌘4–⌥⌘8, persisted. Player 2 takes the selected track (LOAD) or a dragged row and plays on the preview deck; (UX §1.7) | 3 | P2 |
| First-start guide: self-ticking checklist that points at the real controls, sample tracks, test tone | DONE | With an empty library a checklist under the top bar ticks itself (add music, analysis, two tracks, play and sync) and outlines the next control. Two sample tracks are synthesised by the app; TEST TONE plays a quiet tone on master and headphones.; (Roadmap [7]) | 3 | P2 |
| Audio health: device, latency and dropouts always shown, device-lost alert, recovery and device change without a restart | DONE | The status band names the output, its latency and the dropouts. A lost device (reported by the backend, or no callback for 1.5 s) raises a band at once; the engine is kept and goes on with SWITCH OUTPUT, TRY AGAIN or by itself when the device is plugged back in. Device, buffer and headphone routing in Preferences apply at once. Device loss is injected in tests (virtual output); it has not been checked with a real interface being unplugged.; (Roadmap [10]) | 4 | **P1** |
| Gig check: audio output, controller, space for the recording and missing track files | DONE | From the gig menu in the top bar or View > Gig Check: a band with one result per check in the line grammar (solid ready, dashed look at it, struck fix it).; (Roadmap [10]) | 4 | P2 |
| Restore after a crash: deck state saved every second, offered back after an abnormal exit; a running recording survives | DONE | session.json beside the library, written atomically once a second (tracks, positions, tempo, keylock, key shift, sync, loops, hot cue bank, channel faders, trim, EQ, crossfader assign, crossfader, master level, the running recording). A run that did not end cleanly is offered back; the recording's WAV header is brought up to date every second and repaired on the next start. Tested by killing a child process with SIGKILL.; (Roadmap [9]) | 6 | **P1** |
| Performance lock: no loading onto a playing deck, no deleting, no grid edits; keep awake and pause background work while decks play | DONE | LOCK in the top bar, View menu and Alt+Shift+Cmd+K. Guards sit in load_track, the delete paths (lists, list entries, collection, hot and memory cues, sticks) and every grid entry point. Keep awake: IOPMAssertion (macOS, checked against pmset in a test), SetThreadExecutionState (Windows, compiled but not run here), systemd-inhibit (Linux, best effort without it). Automatic analysis, the stem batch and automatic stick sync wait while decks play.; (Roadmap [11]) | 3 | **P1** |
| Top-bar panel toggles: FX, Mix Preview, Sampler, Mixer, Recording | DONE | FX, PREVIEW, SAMPLER and MIXER in the top bar (stored): MIXER hides the mixer column, PREVIEW shows the Mix Preview panel, the key beside RECORD shows or hides the recording section, plus CLUB and LIGHT; (UX §1.2, §2.8) | 3 | P2 |
| Mix Preview (mix in / mix out points between decks) | DONE | `crates/od-ui/src/mix_preview.rs`, `crates/od-ui/ui/mix_preview.slint`; MIX PREVIEW panel above the waveforms: MIX IN / MIX OUT at the edited deck's playhead (stored per track), ◀ ▶ through the points (memory cues and mix points), EDIT switches the deck, LINK starts the waiting deck from its mix-in point when the playing deck passes its mix-out point; (UX §1.5; SPEC §16.3) | 5 | P3 |
| Draggable splitters with persisted sizes, double-click reset | DONE | `crates/od-ui/src/shell.rs`; Upper/lower and sidebar splitters: drag, persisted across restarts, double-click reset (UI test). Per-layout sizes and waveform-zone collapse / expand are not there; (SPEC §12; UX §2.6) | 2 |  |
| Status band: analysis / sync state, list summary (tracks, time, size) | DONE | `crates/od-ui/src/shell.rs`; Activity (analysis progress) at the left, the last message, and the open list summary (tracks, time, size, selection) at the right; (SPEC §12; UX §1.6) | 3 |  |
| UI scaling 0.75–2.0 without clipping | DONE | `crates/od-ui/src/app.rs::App::new`; Preferences > View > UI scale 75–200 % (applies after a restart); the view test checks the Prepare view at every scale on a 2560×1600 display for clipping and overlap; (SPEC §12, §16.4) | 3 |  |
| Colour-blind palette and high-contrast theme | DONE | Preferences > View switches (and the CLI flags) set the colour-blind-safe palette and the high-contrast theme live; stored in the settings; (SPEC §12) | 1 |  |
| English and German UI | DONE | `crates/od-i18n`; (SPEC §2) | 2 |  |
| Lite / Full performance profiles | DONE | (SPEC §3.2) | 2 |  |
| Raspberry Pi kiosk (linuxkms, Perform, controller browsing) | DONE (beyond the usual) | Built and driven headless on CI (`packaging/ci/headless-e2e.sh`); the kiosk configuration (Perform, browse screen, 44 px touch rows and keys) is driven by a UI test; (SPEC §3.3) | 1 |  |
| Keyboard shortcuts for layouts and a View menu | DONE | `crates/od-ui/ui/app.slint::MenuBar`; View menu (modes, Prepare layout, info / similar / Tags / filter panels, sections) and keys ⌥⌘4 / ⌥⌘5 (layouts), ⌥⌘I / ⌥⌘E / ⌥⌘G (panels); the menu bar is native on macOS, elsewhere its actions are reached by the keys; (UX §1.9, §4 P3.7) | 3 |  |
| Perform view menu (2 / 4 decks, horizontal / vertical, Browse) | DONE | `ui/topbar.slint::ViewMenu`, `decks.rs::set_perform_view`; One menu in the top bar with a picture per view; the deck count is kept across starts | 1 | P2 |
| Motion language: four named timings and one curve, reduced motion, audio-linked visuals unsmoothed | DONE | `crates/od-ui/ui/tokens.slint::Theme`, `DESIGN.md`; Press 90 ms, toggle 140 ms, panel 180 ms, mode change 240 ms on one curve; all 0 ms with Reduce motion, which starts off because Slint does not report the system setting. Meters, waveforms, playheads and the phase meter never animate.; (Roadmap [45]) | 2 | P2 |
| Help overlay: every control on screen named with its shortcut, also on touch | DONE | `crates/od-ui/src/help.rs::lay_out`, `crates/od-ui/ui/help.slint::HelpOverlay`; F12, ? where no shortcut takes it, or the ? key in the top bar. Names and texts come from the controls' spoken names and tooltips.; (Roadmap [23]) | 4 | P2 |
| Touch layout beyond the kiosk: decks, mixer, pads, FX, browser and lighting at 44 px | DONE | `crates/od-ui/src/touch.rs::touched`, `crates/od-ui/ui/perform.slint::KioskDeck`; Preferences > View > Touch; offered once when a touch arrives (winit touch events). Not tried on a real touch screen.; (Roadmap [24]) | 2 | P2 |
| Every control reached by Tab, with the focus ring; arrows turn knobs and move faders | DONE | `crates/od-ui/ui/controls.slint::KeyFocus`; (Roadmap [25]) | 1 | P2 |
| Readable screen reader names and descriptions (tooltip with shortcut); test ids apart | DONE | `crates/od-ui/src/a11y.rs::name_for`; Not tried with a real screen reader.; (Roadmap [25]) | 2 | P2 |
| No state carried by hue alone: line forms, words and initials beside every colour | DONE | `crates/od-ui/src/render.rs::phrase_short`; (Roadmap [25]) | 1 | P3 |

## 13. DMX lighting

| Feature | Status | Evidence / note | Tests | Prio |
|---|---|---|---|---|
| DMX outputs: Enttec USB-DMX, Art-Net, sACN, several universes | DONE | Art-Net, sACN and USB-DMX (Enttec Pro) from the LIGHTING view or od-cli, several universes; (SPEC §10.5) | 4 | P2 |
| Fixture library (OFL), patch, groups, moving heads / bars / PARs / strobes / lasers / fog | DONE | Built-in PAR / mover / bar / strobe / blinder / laser / fog types and Open Fixture Library files (IMPORT FIXTURE FILE… or od-cli), patch with groups and overlap checks; (SPEC §10.5) | 6 | P2 |
| Patch table: universe, address, mode and channel count editable, bulk add, collisions, identify | DONE | Roadmap [14]. PATCH in the LIGHTING view: fields for universe, address and channel count (Enter applies), a stepper for the mode; ADD places n fixtures from a start address, all or nothing; overlaps and addresses past 512 are kept and shown with the reason (struck line, red text); ID blinks one fixture at 1.5 Hz; (SPEC §10.5) | 4 | P2 |
| Output status per universe, live channel view, reconnect and failsafe (hold, safe scene, blackout) | DONE | Roadmap [16]. A failed send marks the link lost; it is reopened every second. While any link is lost the whole rig shows the chosen failsafe; the same choice applies after a show panic and on quit. Status per universe in OUTPUT, compact in the Perform strip and the top bar; CHANNELS shows the 512 values as sent. Loss and recovery are tested with an injected link (and the OPENDJDECKS_DMX_FAULT hook), not with a real interface; (SPEC §10.5) | 5 | P2 |
| Safety limits: strobe and flash cap, fog and laser lockout, level caps per group, setup PIN | DONE | Roadmap [19]. Enforced on the rendered frame right before sending, so every source passes through: strobe channels capped (default 3 per second, assuming the fastest value is 25 per second), dark-to-bright flashes per fixture held to the same rate, level caps per group, fog and laser channels zero until armed (arming is not saved). A PIN locks patch, outputs and limits; the show controls stay free; (SPEC §10.5) | 6 | P2 |
| Automatic light show from beat, phrase, energy and the mix (crossfade-aware) | DONE | AUTO follows phrase, beat and energy; looks of both decks blend by their share of the mix; phrase changes crossfade over one bar; the LIGHTING view and the Perform strip show what it follows; (SPEC §10.5) | 5 | P2 |
| Easy manual mode: scene pads, dimmer, strobe, blackout, tap/sync, controller pad mode | DONE | EASY: look pads, colour swatches, master dimmer, BLACKOUT, FLASH / STROBE / FOG hold, TAP tempo when no deck plays; LIGHTING pad mode on the decks (and the controller pads): A–F looks, G strobe, H flash; (SPEC §10.5) | 4 | P2 |
| Pro mode: programmer, effects engine, cue stacks, per-track lighting cues, MIDI mapping | DONE | PRO: program / colour / attribute effect (sine, saw, square, random on dimmer, pan, tilt; phase spread; beat-synced) and an XY pan / tilt programmer per group; cue stack with GO / BACK / STORE and beat fades; track cue timeline (cues stored at bars of the track you hear, recalled whenever it plays); every lighting control MIDI-mappable (`lighting.*` mapping actions); (SPEC §10.5) | 6 | P2 |
| LIGHTING tab, split view in Perform, detachable window, 2D visualiser | DONE | LIGHTING tab (topbar, View menu, Alt+3) with RIG \| STAGE (live 2D visualiser) \| SHOW, a lighting strip inside Perform (LIGHT, Alt+K) and a detachable LIGHTING window for a second screen (WINDOW); (SPEC §10.5) | 4 | P2 |
| Own looks and chases on pads: pages, latch or flash, live colour preview, group masters, mapped to pads and MIDI | DONE | Roadmap [15]. PADS in the LIGHTING view: 4 pages of 8 pads; a look sets program, colour and level per group (groups it leaves out keep what runs underneath), a chase steps through looks on the beat (1/4 beat to 2 bars); latch or flash per pad; the pad shows the look's colours and follows every edit; group masters. Saved with the rig (lighting.json). Deck pads in LIGHTING mode play the current page; mapping actions lighting.pad, lighting.page(_next/_prev), lighting.group_master; MIDI learn fills the pad, page or group number. Tested with simulated MIDI only, not from a hardware controller; (SPEC §10.5) | 10 | P2 |
| Full lighting controls in Perform and Club, legible in the dark and sized for touch | DONE | Roadmap [17]. STRIP (LIGHT STRIP, Alt+Cmd+K) opens a lighting panel in the browser's place in Perform and Club: stage preview (from 1360 px), pages and pads, master and group faders, AUTO / EASY / PRO, DMX OUT, HOLD, RELEASE, who is in control or a lost link, BLACKOUT / FLASH / STROBE / FOG. The decks, mixer and waveforms keep every element's place and size at 1440 x 900 and 1280 x 800 (UI test); every key, pad and fader is 44 px or more. Not tried on a real touch screen; (SPEC §10.5) | 2 | P2 |
| AUTO you can steer: palettes, editable phrase-to-look table, hold, and a visible order of AUTO, overrides and release | DONE | Roadmap [18]. PALETTE (track key, warm, cool, sunset, ocean, white, own swatches); PHRASE -> LOOK for intro, verse, build-up, drop, breakdown, outro (a program or a pad); HOLD keeps AUTO's look through phrase changes; who wins, top first: blackout, FLASH / STROBE keys, a flash pad while held, the latched pad, HOLD, AUTO. A pad takes only the groups its look sets; RELEASE hands back with a one-beat fade. The order is shown with the layer in control lit, and IN CONTROL names it; (SPEC §10.5) | 5 | P2 |
| Connecting interfaces: several universes, USB-DMX port picker, Art-Net node discovery, test pattern, guided setup | DONE | Roadmap [20]. OUTPUT: each universe its own Art-Net Port-Address or sACN universe by default; PORTS lists USB serial adapters (REFRESH); FIND NODES sends an ArtPoll to the broadcast address and lists the ArtPollReply answers; TEST PATTERN each / colours / full, through the safety limits; GUIDED SETUP in four steps ending with the test. Tested with a fake node on the loopback address (documentation-range IP) and listed test ports; no real Art-Net node, sACN receiver or USB-DMX interface has been tried; (SPEC §10.5) | 6 | P2 |

## 14. Video, DVS, streaming, cloud

| Feature | Status | Reason |
|---|---|---|
| Video / lyric | OUT-OF-SCOPE | SPEC §1: video; Video and lyric display |
| DVS | OUT-OF-SCOPE | SPEC §1: dvs |
| Streaming services | OUT-OF-SCOPE | SPEC §1: streaming; Beatport, Beatsource, SoundCloud, Spotify, TIDAL, Apple Music sources |
| Cloud library sync / Dropbox / mobile app sync | OUT-OF-SCOPE | SPEC §1: cloud; offline-first, no accounts (SPEC §2) |
| Live streaming / mix upload | OUT-OF-SCOPE | SPEC §1: cloud; needs online accounts; SPEC §2 offline-first |

---

## Counts

| Status | Rows |
|---|---|
| DONE | 282 |
| PARTIAL | 0 |
| ENGINE-ONLY | 0 |
| MISSING | 0 |
| deferred (in scope, later milestone) | 1 |
| OUT-OF-SCOPE | 5 |
| **Total** | **288** |

## P1 gaps and implementation notes

1. **Persist cue edits made in Perform.** Hot cues (incl. loop hot cues) and
   memory cues set or deleted on pads/controller only change engine state.
   *`od-ui` (`perform.rs`)*: after `HotCue`/`DeleteHotCue`/memory-cue
   actions, read the deck's cue table from `DeckState` (or mirror the
   command), build `Vec<Cue>` and call `Library::set_cues` with a journal
   label, unless the deck plays from a stick (`loaded_from_stick`). Wire the
   dropped `MemoryCueSet/MemoryCueDelete` actions the same way. Sync then
   follows automatically via the change log.
2. **Tempo FX and Channel FX selection.** *Done 2026-11-06:* FX panel, CH FX selector,
   keyboard and controller inputs with LEDs (`fx.rs`, `controller.rs`). Original note:
   engine is complete; nothing drives it.
   *`od-ui` (`perform.rs::handle_action`)*: translate `TempoFxAction`
   (toggle/on, level, beat up/down, auto beat, select next/prev, assign,
   tap) into `Command::TempoFx`, and `ChannelFxAction::Select/Next` into
   `ChannelCommand::SetChannelFx` for all channels. *`ui/perform.slint`*: a
   compact FX strip (type, beats, depth, target, on) per unit next to the
   mixer; add FX LEDs in `feedback`. Exit FX can come later.
3. **Track metadata editing.** Rating, colour, comment and core tags are
   stored but read-only. *`od-ui`*: inline rating click in `table.slint`,
   colour/comment columns, a track-info panel in Prepare that calls
   `Library::set_tags` (already journalled, already marks sticks out of
   date). Add colour and comment as optional columns (see column choice, P2).
4. **Playlist and set ordering, drag and drop.** *`od-ui`*: Slint drag
   support (or move up/down + cut/paste rows as a first step) calling
   `nodes.rs::move_entries`, `add_to_node`, `move_node`; file drop from the
   OS into the library uses the existing import path. Sets depend on order,
   so this is core to SPEC §5.
5. **Complete the Prepare cue editor.** *`od-ui` (`app.rs`, `ui/player.slint`)*:
   delete/move memory cues, store the current loop as a memory or hot-cue
   loop (`Cue.loop_end`), edit label and colour, and draw cue
   colours (add a colour field to `CueMark`).
6. **Grid editor: tap tempo and typed BPM; Tap BPM on the controller.**
   *`od-ui` (`app.rs::edit_grid`)*: tap button averaging intervals, numeric
   BPM entry building a constant grid from the current downbeat
   (`Beatgrid::constant`). Wire `DeckAction::TapBpm` to the same helper for
   the loaded track.
7. **Controller coverage for mapped but dropped actions.** *Mostly done 2026-11-06*
   (FX, sampler, pad modes, key sync, tap BPM, memory cues, booth / master cue,
   stems); `deck.search` and `browse.similar / view_toggle` remain. `handle_action`
   falls through to `_ => None` for many controller inputs that the mappings
   already decode. *`od-engine`*: add `DeckCommand::Search { forward, on }`
   (fast scan) and a `KeySync` (shift to master key via `SetKeyShift`).
   *`od-ui`*: wire `Search`, `KeySync`, `StemLevel`/`StemMuteToggle`
   (and map the part buttons and part-iso knobs of the mappings
   instead of `ignore`), `Sampler::Volume`, `Browse::Similar/ViewToggle`.
   Each needs a line in `docs/controller-mappings.md` and a test in
   `crates/od-controller/tests/mappings.rs`.
8. **Preferences window with persisted settings.** *`od-ui`* (+ a small
   settings store, e.g. a `meta` row or a TOML file in the data dir):
   audio device, buffer size and headphone routing (`od-audio-io::list_devices`,
   restart the host on change while not playing), language, key notation,
   colour-blind / high contrast / motion, and the engine options that exist
   but have no switch (EQ mode, fader and crossfader curves, default tempo
   range). CLI flags stay as overrides.

### P2 summary

Column choice and filter bar; Tags; similar-tracks / suggestion panel
(`suggest_next`); own play history with a history view; tree node reorder;
set backups/start time UI; full Prepare keyboard shortcuts and controller
tree navigation; phrase display (in progress); waveform zoom; EQ/fader/crossfader
curve settings (if not covered by P1-8); FLAC/MP3 recording and silence
auto start; batch stem separation in Prepare.

### P3 summary

Artwork display, duplicate check, XML/iTunes exchange, consolidate UI,
variable-tempo grid editing, BPM range setting, waveform style options,
vocal position, active loop, stem FX, Smart
CH FX/Auto Fader, mic/aux input, auto-gain toggle, recording folder/level
settings, MIDI learn UI.
