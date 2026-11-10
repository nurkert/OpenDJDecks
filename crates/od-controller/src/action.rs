//! Vendor-neutral, engine-independent control actions.
//!
//! Numbering conventions used throughout the crate:
//! - decks, mixer channels and FX units are **1-based** (`deck: 1` is Deck 1),
//! - hot cue slots, pads and sampler slots are **0-based** (slot 0 = hot cue A).
//!
//! Continuous values are normalised: unipolar controls use `0.0..=1.0`,
//! bipolar controls (tempo, channel FX) use `-1.0..=1.0` with `0.0` at the
//! centre detent.

use serde::{Deserialize, Serialize};

/// One thing the user did on a controller, independent of how the hardware
/// encoded it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlAction {
    Deck {
        deck: u8,
        action: DeckAction,
    },
    Mixer {
        channel: u8,
        action: MixerAction,
    },
    /// 0.0 = full A side, 1.0 = full B side.
    Crossfader(f32),
    MasterLevel(f32),
    BoothLevel(f32),
    /// 0.0 = cue only, 1.0 = master only.
    HeadphoneMix(f32),
    HeadphoneLevel(f32),
    /// Press of the master cue button.
    MasterCue,
    /// One-tap EQ swap of a band between the two decks being mixed, at the
    /// swap timing set in the app (now, next beat or next bar).
    EqSwap(EqBand),
    TempoFx {
        unit: u8,
        action: TempoFxAction,
    },
    ChannelFx(ChannelFxAction),
    Browse(BrowseAction),
    Sampler(SamplerAction),
    RecordingToggle,
    /// Light show controls (MIDI-mappable, SPEC §10.5).
    Lighting(LightingAction),
    /// Every effect off at once: Tempo FX units, their tails, exit and
    /// hold effects, and the channel FX knobs.
    FxPanic,
    /// A physical control the mapping knows but deliberately ignores.
    Ignored,
}

/// Light show actions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LightingAction {
    /// Easy look `0..6` (static, pulse, chase, sweep, breathe, build).
    Look(u8),
    Strobe(bool),
    Flash(bool),
    Fog(bool),
    BlackoutToggle,
    /// Master dimmer 0..1.
    Master(f32),
    /// Pro cue stack.
    CueGo,
    CueBack,
    /// "auto", "easy" or "pro".
    Mode(String),
    /// An own look or chase: pad `0..8` of the current page, pressed or
    /// released (a flash pad is on while held).
    Pad { pad: u8, pressed: bool },
    /// Page `0..4` of the pads.
    Page(u8),
    PageNext,
    PagePrev,
    /// Group master `group` (0-based, in the rig's group order), 0..1.
    GroupMaster { group: u8, level: f32 },
    /// Hands the show back to the music: latched pad off, HOLD off.
    Release,
    /// AUTO keeps its current look (toggle).
    Hold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JogMode {
    /// Top surface turned while touched (vinyl mode): scratch.
    Scratch,
    /// Top surface without touch, or the side ring: pitch bend / nudge.
    Bend,
    /// Turned with shift held: fast search.
    Search,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PadMode {
    HotCue,
    BeatLoop,
    BeatJump,
    Sampler,
    HoldFx1,
    HoldFx2,
    Keyboard,
    KeyShift,
    /// Slicer: the pads play eight slices of a beat-aligned domain.
    Slicer,
    /// Parts: A–D mute / unmute vocals, inst, drums, bass; E–H solo them.
    Stems,
    /// The first eight memory cues, in track order.
    MemoryCue,
    /// The light show: A–F looks, G strobe and H flash while held.
    Lighting,
    /// Timed censor: each pad censors for a beat length (¼ … 32 beats).
    TimedCensor,
    /// Pattern recall: pads store and recall sampler sequences.
    PatternRecall,
    /// Loop roll: each pad rolls a loop of 1/32 … 4 beats while held; the
    /// track carries on underneath and resumes on release.
    LoopRoll,
}

impl PadMode {
    pub const ALL: [PadMode; 15] = [
        PadMode::HotCue,
        PadMode::BeatLoop,
        PadMode::BeatJump,
        PadMode::Sampler,
        PadMode::HoldFx1,
        PadMode::HoldFx2,
        PadMode::Keyboard,
        PadMode::KeyShift,
        PadMode::Slicer,
        PadMode::Stems,
        PadMode::MemoryCue,
        PadMode::Lighting,
        PadMode::TimedCensor,
        PadMode::PatternRecall,
        PadMode::LoopRoll,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PadMode::HotCue => "hot_cue",
            PadMode::BeatLoop => "beat_loop",
            PadMode::BeatJump => "beat_jump",
            PadMode::Sampler => "sampler",
            PadMode::HoldFx1 => "hold_fx1",
            PadMode::HoldFx2 => "hold_fx2",
            PadMode::Keyboard => "keyboard",
            PadMode::KeyShift => "key_shift",
            PadMode::Slicer => "slicer",
            PadMode::Stems => "stems",
            PadMode::MemoryCue => "memory_cue",
            PadMode::Lighting => "lighting",
            PadMode::TimedCensor => "timed_censor",
            PadMode::PatternRecall => "pattern_recall",
            PadMode::LoopRoll => "loop_roll",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.name() == s)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeckAction {
    PlayPause,
    CuePress,
    CueRelease,
    JumpToStart,
    Sync,
    SyncOff,
    /// Make this deck the tempo master.
    SetMaster,
    KeyLock,
    KeySync,
    KeyReset,
    /// Tempo fader position, -1.0..=1.0 (0.0 = centre). Positive = faster.
    Tempo(f32),
    TempoRange,
    TempoReset,
    TapBpm,
    Quantize,
    JogTouch(bool),
    /// Relative jog movement in hardware ticks (sign = direction).
    JogTurn {
        ticks: i32,
        mode: JogMode,
    },
    HotCue {
        slot: u8,
        pressed: bool,
    },
    HotCueDelete {
        slot: u8,
    },
    MemoryCueSet,
    MemoryCueDelete,
    MemoryCuePrev,
    MemoryCueNext,
    LoopIn,
    LoopOut,
    AutoLoop {
        beats: f32,
    },
    LoopRoll {
        beats: f32,
        pressed: bool,
    },
    LoopHalve,
    LoopDouble,
    ReloopExit,
    /// Saves the current (or last) loop as a hot loop on the first free pad.
    LoopCapture,
    /// Arms (or cancels) a start on the master deck's next phrase.
    PhraseStart,
    /// Halves (`0.5`) or doubles (`2.0`) the track's beatgrid tempo.
    BpmScale(f32),
    BeatJump {
        beats: f32,
    },
    Search {
        forward: bool,
        pressed: bool,
    },
    Slip,
    VinylMode,
    Shift(bool),
    PadMode(PadMode),
    /// Generic pad hit; the engine interprets it according to the deck's pad mode.
    Pad {
        index: u8,
        pressed: bool,
        velocity: u8,
    },
    Load,
    /// Momentary slip-reverse ("censor").
    Censor(bool),
    Reverse,
    /// Absolute position in the track, 0.0..=1.0.
    NeedleSearch(f32),
    /// This deck became the active deck on its side of a 4-deck controller.
    DeckSelect,
    /// Stem (part isolator) level 0.0..=1.0 for one part of the track.
    StemLevel {
        stem: StemPart,
        level: f32,
    },
    /// Stem mute button: toggles the part between muted and full level.
    StemMuteToggle {
        stem: StemPart,
    },
    /// Solo a part while held (`on`); the other parts come back on release.
    StemSolo {
        stem: StemPart,
        on: bool,
    },
    /// Keep only this part: the others are muted and stay muted.
    StemKeepOnly {
        stem: StemPart,
    },
}

/// A separated part of a track (see `od_engine::Stem`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StemPart {
    Vocals,
    Drums,
    Bass,
    /// Instrumental remainder (everything but vocals, drums and bass).
    Inst,
}

impl StemPart {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "vocals" | "vocal" => Self::Vocals,
            "drums" | "drum" => Self::Drums,
            "bass" => Self::Bass,
            "inst" | "instrumental" | "other" => Self::Inst,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrossfaderSide {
    A,
    Thru,
    B,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MixerAction {
    Trim(f32),
    EqHigh(f32),
    EqMid(f32),
    EqLow(f32),
    /// Channel FX knob, -1.0..=1.0.
    ChannelFx(f32),
    Fader(f32),
    CueToggle,
    CrossfaderAssign(CrossfaderSide),
    /// Kill of an EQ band on this channel, on / off per press.
    EqKill(EqBand),
}

/// An EQ band of a mixer channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EqBand {
    Low,
    Mid,
    High,
}

impl EqBand {
    /// The engine's band index (0 = low, 1 = mid, 2 = high).
    pub fn index(self) -> usize {
        match self {
            EqBand::Low => 0,
            EqBand::Mid => 1,
            EqBand::High => 2,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "low" => Some(EqBand::Low),
            "mid" => Some(EqBand::Mid),
            "high" | "hi" => Some(EqBand::High),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TempoFxAction {
    Toggle,
    /// Momentary "release" style effect.
    Release(bool),
    Level(f32),
    BeatDown,
    BeatUp,
    AutoBeat,
    Tap,
    SelectNext,
    SelectPrev,
    /// Select by effect name (`echo`, `reverb`, ...).
    Select(String),
    /// Route the unit to a mixer channel (1-based) or `0` for master.
    Assign(u8),
    /// A build-up that reaches its top on the next 8, 16 or 32 bar line
    /// (press again: off).
    BuildUp(u8),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelFxAction {
    /// Select an effect type by name (`filter`, `echo`, `noise`, ...).
    Select(String),
    Next,
    Toggle,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowseAction {
    Rotate { delta: i32, shifted: bool },
    Press { shifted: bool },
    Back,
    ViewToggle,
    Similar,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SamplerAction {
    Pad {
        slot: u8,
        pressed: bool,
        velocity: u8,
    },
    Stop {
        slot: u8,
    },
    BankNext,
    BankPrev,
    Volume(f32),
    CueToggle,
}

/// Identifies a continuous parameter independent of its value. Used for
/// soft takeover and parameter feedback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamTarget {
    Tempo { deck: u8 },
    NeedleSearch { deck: u8 },
    Trim { channel: u8 },
    EqHigh { channel: u8 },
    EqMid { channel: u8 },
    EqLow { channel: u8 },
    ChannelFx { channel: u8 },
    Fader { channel: u8 },
    Crossfader,
    MasterLevel,
    BoothLevel,
    HeadphoneMix,
    HeadphoneLevel,
    TempoFxLevel { unit: u8 },
    SamplerVolume,
}

impl ParamTarget {
    /// Bipolar parameters are expressed as -1..1 in actions; 0..1 otherwise.
    pub fn is_bipolar(self) -> bool {
        matches!(
            self,
            ParamTarget::Tempo { .. } | ParamTarget::ChannelFx { .. }
        )
    }

    /// Deck or mixer channel this parameter belongs to.
    pub fn deck(self) -> Option<u8> {
        match self {
            ParamTarget::Tempo { deck } | ParamTarget::NeedleSearch { deck } => Some(deck),
            ParamTarget::Trim { channel }
            | ParamTarget::EqHigh { channel }
            | ParamTarget::EqMid { channel }
            | ParamTarget::EqLow { channel }
            | ParamTarget::ChannelFx { channel }
            | ParamTarget::Fader { channel } => Some(channel),
            _ => None,
        }
    }

    /// Converts an action-domain value to the normalised 0..1 domain.
    pub fn to_unit(self, v: f32) -> f32 {
        if self.is_bipolar() {
            (v + 1.0) * 0.5
        } else {
            v
        }
    }

    /// Converts a normalised 0..1 value to the action domain.
    pub fn from_unit(self, u: f32) -> f32 {
        if self.is_bipolar() { u * 2.0 - 1.0 } else { u }
    }

    /// Builds the action that sets this parameter to `value` (action domain).
    pub fn action(self, value: f32) -> ControlAction {
        use ControlAction as A;
        match self {
            ParamTarget::Tempo { deck } => A::Deck {
                deck,
                action: DeckAction::Tempo(value),
            },
            ParamTarget::NeedleSearch { deck } => A::Deck {
                deck,
                action: DeckAction::NeedleSearch(value),
            },
            ParamTarget::Trim { channel } => A::Mixer {
                channel,
                action: MixerAction::Trim(value),
            },
            ParamTarget::EqHigh { channel } => A::Mixer {
                channel,
                action: MixerAction::EqHigh(value),
            },
            ParamTarget::EqMid { channel } => A::Mixer {
                channel,
                action: MixerAction::EqMid(value),
            },
            ParamTarget::EqLow { channel } => A::Mixer {
                channel,
                action: MixerAction::EqLow(value),
            },
            ParamTarget::ChannelFx { channel } => A::Mixer {
                channel,
                action: MixerAction::ChannelFx(value),
            },
            ParamTarget::Fader { channel } => A::Mixer {
                channel,
                action: MixerAction::Fader(value),
            },
            ParamTarget::Crossfader => A::Crossfader(value),
            ParamTarget::MasterLevel => A::MasterLevel(value),
            ParamTarget::BoothLevel => A::BoothLevel(value),
            ParamTarget::HeadphoneMix => A::HeadphoneMix(value),
            ParamTarget::HeadphoneLevel => A::HeadphoneLevel(value),
            ParamTarget::TempoFxLevel { unit } => A::TempoFx {
                unit,
                action: TempoFxAction::Level(value),
            },
            ParamTarget::SamplerVolume => A::Sampler(SamplerAction::Volume(value)),
        }
    }
}

impl ControlAction {
    /// The continuous parameter this action sets, with its value (action domain).
    pub fn param(&self) -> Option<(ParamTarget, f32)> {
        use ControlAction as A;
        Some(match self {
            A::Deck {
                deck,
                action: DeckAction::Tempo(v),
            } => (ParamTarget::Tempo { deck: *deck }, *v),
            A::Deck {
                deck,
                action: DeckAction::NeedleSearch(v),
            } => (ParamTarget::NeedleSearch { deck: *deck }, *v),
            A::Mixer { channel, action } => {
                let c = *channel;
                match action {
                    MixerAction::Trim(v) => (ParamTarget::Trim { channel: c }, *v),
                    MixerAction::EqHigh(v) => (ParamTarget::EqHigh { channel: c }, *v),
                    MixerAction::EqMid(v) => (ParamTarget::EqMid { channel: c }, *v),
                    MixerAction::EqLow(v) => (ParamTarget::EqLow { channel: c }, *v),
                    MixerAction::ChannelFx(v) => (ParamTarget::ChannelFx { channel: c }, *v),
                    MixerAction::Fader(v) => (ParamTarget::Fader { channel: c }, *v),
                    _ => return None,
                }
            }
            A::Crossfader(v) => (ParamTarget::Crossfader, *v),
            A::MasterLevel(v) => (ParamTarget::MasterLevel, *v),
            A::BoothLevel(v) => (ParamTarget::BoothLevel, *v),
            A::HeadphoneMix(v) => (ParamTarget::HeadphoneMix, *v),
            A::HeadphoneLevel(v) => (ParamTarget::HeadphoneLevel, *v),
            A::TempoFx {
                unit,
                action: TempoFxAction::Level(v),
            } => (ParamTarget::TempoFxLevel { unit: *unit }, *v),
            A::Sampler(SamplerAction::Volume(v)) => (ParamTarget::SamplerVolume, *v),
            _ => return None,
        })
    }

    /// Deck this action addresses, if any.
    pub fn deck(&self) -> Option<u8> {
        match self {
            ControlAction::Deck { deck, .. } => Some(*deck),
            _ => None,
        }
    }
}
