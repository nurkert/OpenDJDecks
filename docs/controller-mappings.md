# Controller mapping files

Controller mappings live in `mappings/controllers/<model>.toml` and are loaded by
`od-controller` (`Mapping::load`, `Mapping::load_dir`). This document is the
reference for the file format (schema version 1).

Mapping files are authored independently for OpenDJDecks (SPEC §2). Every
file lists where its message facts came from (header comment and
`device.sources`), and marks entries not yet confirmed on real hardware with
`verify = true`.

The two bundled mappings took message facts from another project's working
mappings when they were written. New model mappings are not derived from
manufacturer manuals, MIDI implementation charts, vendor software or other
projects' mapping files: they come from owners who capture their own unit
with the guided setup (below) and share the file. Such a file names its
template in `device.sources` and marks every entry `verify = true`.

## Conventions

| Thing | Numbering |
|---|---|
| MIDI channel (`channel`, channel sets) | wire value **0–15** (low nibble of the status byte: `0x92` → 2) |
| Note / CC numbers | 0–127, hex allowed (`0x1F`) |
| Decks, mixer channels, FX units (`deck`, `[layers]`) | **1-based** |
| Hot cue slots, pads, sampler slots | **0-based** (slot 0 = hot cue A) |
| Continuous values in actions | `0.0..=1.0`; tempo and channel FX knobs `-1.0..=1.0` |

## Top level

```toml
schema = 1                 # required, must be 1

[device]
id = "ddj-flx4"           # stable identifier used by the application
name = "Example 200"
vendor = "Example"
model = "Example 200"
decks = 2
module = "jog-controller"  # optional Rust device module (see below)
sources = ["..."]          # where the message facts came from

[device.match]
port_names = ["Example 200"]  # case-insensitive substrings of the MIDI port name
usb_vendor_id = 0x2B73     # optional, informational
usb_product_id = 0x0041    # optional, informational
```

Port name matching is the primary key. USB IDs are reported alongside the
connection when the device is visible on the USB bus.

### Channel sets

```toml
[channels]
deck = [0, 1]          # entry i = wire channel of deck slot i+1
pad = [7, 9]
pad_shift = [8, 10]
```

An input or output whose `channel` is a set name (`channel = "deck"`) is
expanded once per entry and bound to deck slot `i + 1`. A numeric
`channel` is fixed; deck-scoped actions then need an explicit `deck = n`.

### Deck layers (4 decks on a 2-sided surface)

```toml
[layers]
mode = "hardware"          # or "software"
sides = [[1, 3], [2, 4]]   # decks reachable from each side; first is active initially
```

- `hardware`: the controller itself changes the MIDI channel when a deck
  button is pressed. Channel sets name decks directly. The layer
  state is only tracked (from `deck.select` inputs) so that soft takeover
  re-arms for the newly selected deck.
- `software`: the controller always sends on the same channel per side.
  Slot `i` of every channel set is side `i`, and inputs are redirected to the
  side's active deck. Deck-scoped outputs are rendered only for the active
  deck and refreshed on a switch. Use `deck.select` with `args = { deck = n }`
  to switch.

Every layer switch drops the 14-bit MSB cache and forgets the physical
position of the new deck's parameters (soft takeover pickup).

### Options

```toml
[options]
soft_takeover = true          # default for absolute inputs
takeover_threshold = 0.03     # normalised pickup distance
jog_ticks_per_revolution = 720
blink_ms = 500                # software blink period
blink_fast_ms = 200
meter_interval_ms = 33        # minimum interval between messages to a level output
max_messages_per_poll = 256   # output budget per poll; the rest goes out next poll
```

### Raw messages

```toml
[[init]]                       # sent once after connecting
bytes = [0xF0, 0x00, 0xF7]
verify = true

[[keepalive]]                  # sent periodically while connected
bytes = [0xF0, 0x00, 0xF7]
interval_ms = 250
```

### Palettes

```toml
[[palettes.pads]]
value = 0x14          # note-on value the hardware understands
color = "#00ff00"     # colour it shows
```

Colour outputs pick the entry nearest to the requested RGB colour.

## Inputs

```toml
[[input]]
name = "Hot cue pad"     # shown in the MIDI monitor
note = 0x00              # exactly one of note / cc
channel = "pad"          # number or channel set
action = "deck.hot_cue"
args = { }               # action arguments
repeat = 8               # expand to 8 consecutive numbers (step = 1)
index_arg = "slot"       # set this argument to the repetition index
verify = false
```

| Key | Meaning |
|---|---|
| `note` / `cc` | Message type and number. Note-off and note-on with velocity 0 both mean "released". |
| `lsb` | LSB controller of a 14-bit pair; implies `mode = "absolute14"`. Default in that mode is `cc + 0x20`. |
| `channel` | Wire channel or channel set name. |
| `deck` | Explicit deck / mixer channel / slot for fixed-channel inputs. |
| `mode` | `button`, `absolute` (7-bit 0–127), `absolute14`, `relative`. Defaults from the action. |
| `encoding` | For `relative`: `offset64` (64 = 0, 65 = +1, 63 = −1; default), `twos_complement` (1 = +1, 127 = −1), `sign_magnitude` (bit 6 = minus). |
| `invert` | Absolute: `1 - v`. Relative: negate. |
| `takeover` | Override soft takeover for this input (default: `options.soft_takeover` for absolute modes). |
| `shift` | Software shift condition: `true` = only while a `deck.shift` input of the same slot (any slot for global inputs) is held, `false` = only while not held. Leave out for controllers that apply shift in hardware. |
| `repeat`, `step` | Expand into `repeat` bindings on numbers `n, n+step, …`. |
| `index_arg`, `index_base` | Set argument `index_arg` to `index_base + i` for repetition `i`. |
| `args` | Action arguments. An **array** value supplies one element per repetition: `args = { beats = [-1.0, 1.0, -2.0, 2.0] }`. |
| `verify` | Not yet confirmed on hardware and not backed by a public source; flagged in the MIDI monitor. |
| `confirmed` | Confirmed on a real unit with OpenDJDecks (also on `[[output]]`). Drives the hardware status below. |

14-bit pairs: the MSB is stored and the value is emitted when the LSB
arrives (`(msb << 7 | lsb) / 16383`).

### Actions

Deck-scoped actions take the deck from the channel set (or `deck`).
Buttons marked *press* fire on press only; *momentary* report press and
release.

| Action | Value | Arguments | Produces |
|---|---|---|---|
| `deck.play_pause` | press | | `DeckAction::PlayPause` |
| `deck.cue` | momentary | | `CuePress` / `CueRelease` |
| `deck.jump_to_start` | press | | `JumpToStart` |
| `deck.sync`, `deck.sync_off`, `deck.master` | press | | `Sync`, `SyncOff`, `SetMaster` |
| `deck.keylock`, `deck.key_sync`, `deck.key_reset` | press | | |
| `deck.tempo` | absolute | | `Tempo(-1..1)` |
| `deck.tempo_range`, `deck.tempo_reset`, `deck.tap_bpm`, `deck.quantize` | press | | |
| `deck.edit_in_prepare` | press | | `EditInPrepare`: the deck's track in the Prepare player; again (in Prepare) back to Perform |
| `deck.jog_touch` | momentary | | `JogTouch(bool)` |
| `deck.jog_turn` | relative | `mode` = `scratch` / `bend` / `search` | `JogTurn { ticks, mode }` |
| `deck.hot_cue` | momentary | `slot` | `HotCue { slot, pressed }` |
| `deck.hot_cue_delete` | press | `slot` | |
| `deck.memory_cue_set` / `_delete` / `_prev` / `_next` | press | | |
| `deck.loop_in`, `deck.loop_out`, `deck.loop_halve`, `deck.loop_double`, `deck.reloop_exit` | press | | |
| `deck.auto_loop` | press | `beats` | `AutoLoop { beats }` |
| `deck.loop_roll` | momentary | `beats` | `LoopRoll { beats, pressed }` |
| `deck.loop_capture` | press | | `LoopCapture`: the loop just heard (a roll or a loop) becomes a hot loop on the first free pad of the bank, else a memory loop |
| `deck.phrase_start` | press | | `PhraseStart`: a cued, paused deck starts on the master deck's next phrase; a second press cancels |
| `deck.bpm_half`, `deck.bpm_double` | press | | `BpmScale(0.5)` / `BpmScale(2.0)`: the track's grid at half or double the tempo (one undo step) |
| `deck.beat_jump` | press | `beats` (signed) | |
| `deck.search` | momentary | `forward` (bool) | |
| `deck.slip`, `deck.vinyl`, `deck.reverse`, `deck.load` | press | | |
| `deck.censor` | momentary | | `Censor(bool)` |
| `deck.cut_in` | momentary | | `CutIn(bool)`: while held, the deck's channel reaches the master as if the crossfader were open on its side (the channel fader still counts) |
| `deck.phrase_jump` | press | `forward` (bool, default `true`) | `PhraseJump { forward }`: to the next or previous phrase, whole beats so that the phrase start lands on the next downbeat, in time |
| `deck.drop_mark` | press | | `DropMark`: marks the drop on the downbeat nearest the playhead; near the marked drop it clears it (one undo step) |
| `deck.shift` | momentary | | `Shift(bool)`; also drives software shift |
| `deck.pad_mode` | press | `mode` = `hot_cue`, `beat_loop`, `beat_jump`, `sampler`, `hold_fx1`, `hold_fx2`, `keyboard`, `key_shift`, `slicer`, `loop_roll` (pads roll 1/32 … 4 beats while held), `phrase` (phrase jumps and the drop), `cue_edit` (pads pick the hot cue the cue panel edits), `melody` (pads play the notes CREATE sent to the sampler's MELODY bank) | |
| `deck.pad` | momentary | `index` | `Pad { index, pressed, velocity }` (generic, interpreted by pad mode) |
| `deck.needle_search` | absolute | | `NeedleSearch(0..1)` |
| `deck.select` | press | optional `deck` | `DeckSelect` for the bound deck, or for `deck` |
| `deck.stem_level` | absolute | `stem` = `vocals` / `drums` / `bass` / `inst` | `StemLevel { stem, level }`: centre and above full level, fully left muted |
| `deck.stem_mute` | press | `stem` | `StemMuteToggle { stem }` |
| `deck.stem_solo` | momentary | `stem` | `StemSolo { stem, on }`: only this stem while held; the others come back as they were on release |
| `deck.stem_keep` | press | `stem` | `StemKeepOnly { stem }`: the other stems muted, and they stay muted |
| `deck.stem_cue` | press | `stem` | `StemCue { stem }`: adds the stem to the ones heard on their own on the headphones, or takes it out; the master stays as it is (stem audition) |
| `mixer.trim`, `mixer.eq_high`, `mixer.eq_mid`, `mixer.eq_low`, `mixer.fader` | absolute | | `MixerAction::*` |
| `mixer.channel_fx` | absolute | | `ChannelFx(-1..1)` |
| `mixer.cue` | press | | `CueToggle` (headphone cue) |
| `mixer.eq_kill` | press | `band` = `low` / `mid` / `high` | `EqKill(band)`: kill of that band on this channel, on / off per press |
| `mixer.crossfader_assign` | press | `side` = `a` / `thru` / `b` | |
| `crossfader`, `master_level`, `booth_level`, `headphone_mix`, `headphone_level` | absolute | | |
| `master_cue` | press | | |
| `split_cue` | press | | `SplitCue`: split cue on / off (the cue in mono on the left, the master in mono on the right) |
| `crossfader_curve` | press | optional `curve` = `constant_power` / `linear` / `cut` | `CrossfaderCurve(curve)`: sets the curve, or steps to the next one without `curve`; the same setting as Preferences › Controller › Mixer |
| `eq_swap` | press | `band` = `low` / `mid` / `high` | `EqSwap(band)`: swaps that band (knob and kill) between the two decks being mixed, at the swap timing set on the mixer (now, next beat, next bar); a second press while it waits cancels it |
| `tempo_fx.toggle`, `tempo_fx.beat_down`, `tempo_fx.beat_up`, `tempo_fx.auto_beat`, `tempo_fx.tap`, `tempo_fx.select_next`, `tempo_fx.select_prev` | press | `unit` (default 1) | |
| `tempo_fx.release` | momentary | `unit` | |
| `tempo_fx.level` | absolute | `unit` | |
| `tempo_fx.select` | press | `effect` (name), `unit` | |
| `tempo_fx.assign` | press | `channel` (0 = master), `unit` | |
| `tempo_fx.build_up` | press | `bars` = `8` / `16` / `32` (default 16), `unit` | `BuildUp(bars)`: a build-up that peaks on the next line of that many bars and drops; the same press again stops it |
| `fx_panic` | press | | `FxPanic`: every effect off at once (both Tempo FX units with tails, exit and hold FX, the channel FX knobs) |
| `channel_fx.select` | press | `effect` | |
| `channel_fx.next`, `channel_fx.toggle` | press | | |
| `browse.rotate` | relative | `shifted` (bool) | `Rotate { delta, shifted }` |
| `browse.press` | press | `shifted` | |
| `browse.back`, `browse.view_toggle`, `browse.similar` | press | | |
| `sampler.pad` | momentary | `slot` | |
| `sampler.stop` | press | `slot` | |
| `sampler.bank_next`, `sampler.bank_prev`, `sampler.cue` | press | | |
| `sampler.volume` | absolute | | |
| `recording.toggle` | press | | |
| `lighting.look` | press | `look` (0–5: static, pulse, chase, sweep, breathe, build) | switches the light show to Easy with that look |
| `lighting.strobe`, `lighting.flash`, `lighting.fog` | hold | | on while held |
| `lighting.blackout` | press | | toggles blackout |
| `lighting.master` | absolute | | master dimmer |
| `lighting.cue_go`, `lighting.cue_back` | press | | Pro cue stack |
| `lighting.mode` | press | `mode` (`auto`, `easy`, `pro`) | |
| `lighting.pad` | hold | `pad` (1–8, on the current page) | `Pad { pad, pressed }`: plays an own look or chase; a latch pad toggles on press, a flash pad is on while held |
| `lighting.page` | press | `page` (1–4) | `Page`: the page of pads the pad actions and the deck pads play |
| `lighting.page_next`, `lighting.page_prev` | press | | next / previous page, round |
| `lighting.group_master` | absolute | `group` (1 = the first group in the patch) | `GroupMaster { group, level }` |
| `lighting.release` | press | | hands the show back to AUTO: the latched pad and HOLD end |
| `lighting.hold` | press | | AUTO keeps its current look (toggle) |
| `lighting.tap` | press | | taps the tempo for the lights when no deck plays |
| `ignore` | any | | `ControlAction::Ignored` (known control without a function) |

In the application (`crates/od-ui/src/controller.rs`) every action above
reaches the engine: `deck.pad` plays the pad in the deck's current pad mode
(the same logic as the on-screen pads, `pads.rs`), `deck.tap_bpm` sets a
constant grid at the tapped tempo (three taps, keeps the first downbeat),
`deck.memory_cue_set` / `_delete` edit the track's memory cues in the
library, `tempo_fx.release` engages Echo Out while held, `tempo_fx.auto_beat`
returns a tapped unit to the deck tempo, `channel_fx.next` / `toggle` step
through the Channel FX, and `sampler.pad` / `stop` address the bank shown in
the sampler panel (`sampler.bank_*`).

## Outputs

```toml
[[output]]
target = "deck.pad"
note = 0x00
repeat = 8
index_arg = "index"
channel = "pad"
args = { mode = "hot_cue" }
palette = "pads"
```

| Key | Meaning |
|---|---|
| `target` | Feedback element, see below. |
| `note` / `cc`, `channel`, `deck`, `repeat`, `step`, `index_arg`, `args` | As for inputs. |
| `mode` | `led` (default), `level` (default for meters and the jog indicator), `color` (default when `palette` is set). |
| `on`, `off` | Values for lit / dark (default `0x7F` / `0x00`). |
| `dim` | Value for `LedState::Dim` (default: `off`). |
| `blink` | Value that makes the hardware blink by itself; without it blinking is done in software. |
| `min`, `max` | Range for `level` outputs. |
| `palette` | Palette name for colour outputs. |
| `min_interval_ms` | Rate limit for this output (default: `options.meter_interval_ms` for levels, 0 otherwise). |

Several outputs may share a target (e.g. one LED on the button and one
status indicator). Output is diffed per MIDI address: a message is only sent
when its byte value differs from what was last sent.

### Targets

| Target | Arguments |
|---|---|
| `deck.play`, `deck.cue`, `deck.sync`, `deck.master`, `deck.keylock`, `deck.key_sync`, `deck.slip`, `deck.quantize`, `deck.reverse`, `deck.censor`, `deck.vinyl`, `deck.tempo_reset`, `deck.tempo_range`, `deck.loop_in`, `deck.loop_out`, `deck.reloop`, `deck.loop_active`, `deck.loaded`, `deck.shift`, `deck.selected` | |
| `deck.jog_indicator` | level |
| `deck.pad_mode` | `mode` |
| `deck.pad` | `mode`, `index` |
| `mixer.cue`, `mixer.vu` (level) | |
| `mixer.crossfader_assign` | `side` |
| `master.vu_left`, `master.vu_right` (level), `master_cue` | |
| `tempo_fx.on`, `tempo_fx.release` | `unit` (default 1) |
| `channel_fx` | `effect` |
| `sampler.cue`, `recording` | |

## Device modules

`device.module` selects built-in Rust code for behaviour the TOML cannot
express. A module sees every raw message first (and may consume it), then
every action the mapping produced (and may rewrite, drop or add actions).

| Module | Behaviour |
|---|---|
| `jog-controller` | Tempo centre detent (±0.4 % snaps to 0), jog search acceleration (×8), bend scaling with sub-tick remainder. |

New modules implement `od_controller::device_module::DeviceModule` and are
registered in `device_module::module_for`.

## Guided-setup templates

`mappings/templates/*.toml` are not mappings (`Mapping::load_dir` skips the
`templates` folder). Each lists the core controls of one controller class in
the order the guided setup (Preferences › Controller, `od_controller::guided`)
asks for them; the setup fills in the MIDI messages from the owner's unit and
writes a normal mapping file. Templates name no model and hold no MIDI
numbers.

```toml
template = 1                 # required, must be 1
id = "two-deck-battle"
name = "Two decks, battle style"
decks = 2
module = "jog-controller"    # optional, copied into the mapping
crossfader_curve = "cut"     # optional: set when the mapping is saved

[[step]]
action = "deck.play_pause"   # any action of the table above
deck = 1                     # deck / mixer channel, if the action has one
args = { }                   # copied into the mapping
expect = "button"            # button | fader | wheel
name = "Play/Pause (deck 1)" # name in the mapping and the MIDI monitor
screen = "deck1.play"        # optional: on-screen control the setup points at
```

`expect` steers what is learned: a fader or wheel step ignores notes (touch
notes of jogs and knobs), a button step takes a CC that sends 0 / 127 as a
button, a wheel step is always relative. A control that an earlier step
already uses is refused. `two-deck-entry`, `two-deck-battle` and `four-deck`
ship today; `cargo test -p od-controller` checks that each covers play, cue,
sync, jog, tempo and load for every deck, the channel faders, the low EQ and
the crossfader.

While the guided setup, MIDI learn or the monitor runs, the app opens MIDI
inputs that no mapping matches too (`ControllerManager::set_open_unmapped`).

## The lighting layout

`mappings/lighting/lighting-operator.toml` is the default layout for a
second controller that plays only the lights (roadmap [34]). It is built
from OpenDJDecks' own `lighting.*` actions on generic MIDI numbers of
channel 1 (notes 36–62, controllers 20–28; the table is in the light-show
guide) and comes from no other product's chart or manual. Its
`port_names` is empty, so it never matches a port by itself: LIGHTING ›
SETUP › REMOTE gives a connected MIDI input to it
(`ControllerManager::assign`, `Mapping::for_port`), and that port is opened
again with this layout while every other port keeps the mapping that
matches its name. `cargo test -p od-controller` checks that it holds only
lighting actions, binds every address once and matches no port.

The same actions reach the lights over OSC (`od_controller::osc`, off by
default, a UDP port set in REMOTE): `/opendjdecks/light/...` mirrors them
and `/opendjdecks/music` carries the music one instance sends another.

## Authoring workflow

1. Start the monitor: `cargo run -p od-controller --example midi_monitor`.
   Every incoming message is printed with the bindings it hit, suppressed
   actions (soft takeover) and `[unverified]` flags.
2. Unmapped controls show `(unmapped)`. Use MIDI learn to get a starting
   point: `--learn deck.play_pause` prints a ready `[[input]]` entry after
   five seconds of input (`LearnSession` in the API). In the app (Preferences ›
   Controller › MIDI learn), **Pad, page or group** fills in the number of
   `lighting.pad`, `lighting.page` and `lighting.group_master` (and the look
   of `lighting.look`, counted from 1 there).
3. Check outputs with `--leds` (cycles Play/Cue/Sync LEDs and VU meters).
4. Once an entry is confirmed on a real unit, remove `verify = true`, add
   `confirmed = true` and say in the commit which unit and firmware it was.
   Then regenerate the hardware status (below).
5. `cargo test -p od-controller` parses every bundled mapping and checks
   that no MIDI address is bound twice.

## Hardware status

The controllers guide (`docs/guide/{en,de}/controllers.md`) has a
controller list (one row per bundled mapping and per guided-setup class, with
the model's status: verified only when every control is confirmed) and lists
every mapped control with one of three states, both generated from the
mapping files:

| State | Meaning in the file |
|---|---|
| verified | `confirmed = true` |
| partial | neither flag: the message comes from a public source, not yet tried on a unit |
| untested | `verify = true` |

Regenerate the list after changing a mapping:
`cargo run -p od-controller --example hardware_status -- --write`.
`cargo test -p od-controller` fails while the guide is out of date. The
app's self-test (Preferences › Controller › Self-test) ticks controls as
their messages arrive and lights every mapped LED.
