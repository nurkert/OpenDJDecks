//! State the application pushes to a controller (LEDs, meters, pad colours).

use crate::action::{CrossfaderSide, PadMode, ParamTarget, StemPart};
use od_core::CueColor;
use serde::{Deserialize, Serialize};

/// A light or indicator on a deck.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeckElement {
    Play,
    Cue,
    Sync,
    Master,
    KeyLock,
    KeySync,
    Slip,
    Quantize,
    Reverse,
    Censor,
    Vinyl,
    TempoReset,
    TempoRange,
    LoopIn,
    LoopOut,
    Reloop,
    LoopActive,
    Loaded,
    Shift,
    PadMode(PadMode),
    /// Pad `n` (0-based) of the given pad mode page.
    Pad {
        mode: PadMode,
        index: u8,
    },
    /// Jog ring / platter indicator (level, 0..1 = one revolution).
    JogIndicator,
    /// The deck is the active one on its side (4-deck controllers).
    Selected,
    /// A part of the track is heard: on while the part plays, off while it
    /// is muted (lit = active, like the part buttons on the hardware).
    StemActive(StemPart),
    /// The jog display shows the remaining time (on) or the elapsed time.
    TimeRemaining,
    // Values (`FeedbackValue::Number`) for jog displays, scaled by the
    // application; the mapping only says where they go.
    /// Effective BPM × 10 (128.0 BPM = 1280), 0 without a track.
    Bpm,
    /// Tempo change from the fader or sync on a linear scale from -100 %
    /// (0) to +100 % (9999), 0 % = 5000; 5000 without a track.
    Speed,
    /// Minutes of the shown time (elapsed or remaining), 0..=99.
    TimeMinutes,
    /// Seconds of the shown time, 0..=59.
    TimeSeconds,
    /// Playhead on a turning platter, 0..=359 degrees (one turn per 1.8 s).
    PositionAngle,
    /// The main cue point on the same platter, 0..=359 degrees; off (the
    /// output's `off` value) while there is none.
    CueAngle,
    /// Track key, each major key followed by its relative minor in
    /// chromatic order: 1 = C, 2 = Am, 3 = Db, 4 = Bbm … 23 = B, 24 = Abm;
    /// 0 = no key.
    Key,
    /// Key shift in semitones, -12..=12.
    KeyShift,
}

impl DeckElement {
    /// Elements whose feedback is a number, not a light.
    pub fn is_value(self) -> bool {
        matches!(
            self,
            DeckElement::Bpm
                | DeckElement::Speed
                | DeckElement::TimeMinutes
                | DeckElement::TimeSeconds
                | DeckElement::PositionAngle
                | DeckElement::CueAngle
                | DeckElement::Key
                | DeckElement::KeyShift
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MixerElement {
    /// Headphone cue (PFL) button.
    Cue,
    /// Channel level meter (level).
    Vu,
    CrossfaderAssign(CrossfaderSide),
}

/// Addressable feedback element.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackTarget {
    Deck {
        deck: u8,
        element: DeckElement,
    },
    Mixer {
        channel: u8,
        element: MixerElement,
    },
    MasterVuLeft,
    MasterVuRight,
    MasterCue,
    TempoFxOn {
        unit: u8,
    },
    TempoFxRelease {
        unit: u8,
    },
    /// Channel FX selection button, by effect name.
    ChannelFx(String),
    /// FX part button: on while the Channel FX acts on that part (every
    /// part on while it acts on the whole channel).
    ChannelFxPart(StemPart),
    SamplerCue,
    Recording,
    /// Keys are shown in Camelot notation (on) or classic (off).
    KeyCamelot,
}

/// LED rendering state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedState {
    Off,
    On,
    Dim,
    Blink,
    BlinkFast,
}

impl LedState {
    pub fn from_bool(on: bool) -> Self {
        if on { LedState::On } else { LedState::Off }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackValue {
    Led(LedState),
    /// Meter or indicator level, 0.0..=1.0.
    Level(f32),
    /// A coloured element. `None` colour renders as off.
    Color {
        color: Option<CueColor>,
        state: LedState,
    },
    /// A number for `value` / `value14` outputs (see the value elements of
    /// [`DeckElement`]).
    Number(i32),
}

/// A message from the application to the controller layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlFeedback {
    Set {
        target: FeedbackTarget,
        value: FeedbackValue,
    },
    /// Current engine value of a continuous parameter (action domain), used
    /// for soft takeover after deck layer switches or software changes.
    Parameter { target: ParamTarget, value: f32 },
    /// Forget what was sent and resend every element (e.g. after reconnect).
    ResendAll,
    /// Turn every known element off.
    AllOff,
}

impl ControlFeedback {
    pub fn led(target: FeedbackTarget, on: bool) -> Self {
        ControlFeedback::Set {
            target,
            value: FeedbackValue::Led(LedState::from_bool(on)),
        }
    }

    pub fn led_state(target: FeedbackTarget, state: LedState) -> Self {
        ControlFeedback::Set {
            target,
            value: FeedbackValue::Led(state),
        }
    }

    pub fn level(target: FeedbackTarget, level: f32) -> Self {
        ControlFeedback::Set {
            target,
            value: FeedbackValue::Level(level),
        }
    }

    pub fn number(target: FeedbackTarget, n: i32) -> Self {
        ControlFeedback::Set {
            target,
            value: FeedbackValue::Number(n),
        }
    }

    pub fn deck_led(deck: u8, element: DeckElement, on: bool) -> Self {
        Self::led(FeedbackTarget::Deck { deck, element }, on)
    }

    pub fn channel_vu(channel: u8, level: f32) -> Self {
        Self::level(
            FeedbackTarget::Mixer {
                channel,
                element: MixerElement::Vu,
            },
            level,
        )
    }

    pub fn pad_color(deck: u8, mode: PadMode, index: u8, color: Option<CueColor>) -> Self {
        ControlFeedback::Set {
            target: FeedbackTarget::Deck {
                deck,
                element: DeckElement::Pad { mode, index },
            },
            value: FeedbackValue::Color {
                color,
                state: LedState::On,
            },
        }
    }
}
