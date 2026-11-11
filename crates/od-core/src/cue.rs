use crate::{CueColor, DataSource};
use serde::{Deserialize, Serialize};

/// Hot cues are numbered 0..HOT_CUE_SLOTS (shown as A..H).
pub const HOT_CUE_SLOTS: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "slot", rename_all = "snake_case")]
pub enum CueKind {
    Hot(u8),
    Memory,
}

impl CueKind {
    pub fn hot_slot_letter(self) -> Option<char> {
        match self {
            CueKind::Hot(n) if n < 26 => Some((b'A' + n) as char),
            _ => None,
        }
    }
}

/// How a hot cue pad plays its cue.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CueTrigger {
    /// The usual pad: jumps while playing; while paused it plays from the
    /// cue as long as it is held and returns to it on release.
    #[default]
    Cue,
    /// Jumps and plays on, also from a paused deck.
    Latch,
    /// Plays from the cue while held; on release the deck stops and
    /// returns to the cue, also while playing.
    Gate,
    /// Plays from the cue while held; on release the track continues
    /// where it would have been without the press (slip).
    Momentary,
}

impl CueTrigger {
    pub const ALL: [CueTrigger; 4] = [Self::Cue, Self::Latch, Self::Gate, Self::Momentary];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cue => "cue",
            Self::Latch => "latch",
            Self::Gate => "gate",
            Self::Momentary => "momentary",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.as_str() == s)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cue {
    pub kind: CueKind,
    /// Start in seconds.
    pub start: f64,
    /// Loop end in seconds; `Some` makes this cue a stored loop.
    pub loop_end: Option<f64>,
    pub label: String,
    pub color: Option<CueColor>,
    pub source: DataSource,
    /// Active loop: a stored loop that engages by itself when playback
    /// reaches it (loops only).
    #[serde(default)]
    pub active: bool,
    /// How the hot cue pad plays it (hot cues only).
    #[serde(default, skip_serializing_if = "is_default_trigger")]
    pub trigger: CueTrigger,
}

fn is_default_trigger(t: &CueTrigger) -> bool {
    *t == CueTrigger::Cue
}

impl Cue {
    pub fn hot(slot: u8, start: f64) -> Self {
        Self {
            kind: CueKind::Hot(slot),
            start,
            loop_end: None,
            label: String::new(),
            color: None,
            source: DataSource::Manual,
            active: false,
            trigger: CueTrigger::Cue,
        }
    }

    pub fn memory(start: f64) -> Self {
        Self {
            kind: CueKind::Memory,
            ..Self::hot(0, start)
        }
    }

    pub fn is_loop(&self) -> bool {
        self.loop_end.is_some()
    }
}
