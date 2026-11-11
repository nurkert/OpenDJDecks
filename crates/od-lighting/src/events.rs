//! DJ events as triggers (roadmap [36]).
//!
//! The show watches what the decks do from frame to frame and names the
//! moments a lighting operator would react to: a track loaded, a deck
//! started, the phrase of the deck you hear changing (a drop, a breakdown),
//! the crossfader reaching a side, a track close to its end. A [`Trigger`]
//! ties one of them to a pad (a look or a chase), a cue of the stack or a
//! release, for a number of beats or until the next trigger.
//!
//! What a trigger plays is a layer like any other: it is rendered into
//! looks and passes the safety limits with every frame.

use serde::{Deserialize, Serialize};

use crate::looks::PhraseSlot;
use crate::show::DeckMusic;

/// Seconds before the end of a track that count as "track end near".
pub const END_NEAR_SECS: f32 = 30.0;
/// Crossfader position (−1..1) past which it counts as at a side.
pub const XF_SIDE: f32 = 0.9;
/// The crossfader has to come back inside this before a side fires again.
pub const XF_REARM: f32 = 0.5;

/// A moment in the mix the lights can react to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DjEvent {
    /// A track was loaded onto a deck.
    Load,
    /// A deck started playing.
    PlayStart,
    /// The deck you hear moved into another phrase.
    PhraseChange,
    /// … into a chorus or drop.
    Drop,
    /// … into a breakdown or bridge.
    Breakdown,
    /// The crossfader reached the left side.
    CrossfaderLeft,
    /// The crossfader reached the right side.
    CrossfaderRight,
    /// A playing track has [`END_NEAR_SECS`] or less left.
    EndNear,
}

impl DjEvent {
    pub const ALL: [DjEvent; 8] = [
        DjEvent::Load,
        DjEvent::PlayStart,
        DjEvent::PhraseChange,
        DjEvent::Drop,
        DjEvent::Breakdown,
        DjEvent::CrossfaderLeft,
        DjEvent::CrossfaderRight,
        DjEvent::EndNear,
    ];

    /// Label for the interface (English; the UI translates it).
    pub fn label(self) -> &'static str {
        match self {
            DjEvent::Load => "TRACK LOADED",
            DjEvent::PlayStart => "PLAY STARTS",
            DjEvent::PhraseChange => "PHRASE CHANGES",
            DjEvent::Drop => "DROP",
            DjEvent::Breakdown => "BREAKDOWN",
            DjEvent::CrossfaderLeft => "CROSSFADER LEFT",
            DjEvent::CrossfaderRight => "CROSSFADER RIGHT",
            DjEvent::EndNear => "TRACK END NEAR",
        }
    }

    /// More specific events win over general ones fired in the same frame
    /// (a drop is also a phrase change).
    fn rank(self) -> u8 {
        match self {
            DjEvent::Drop | DjEvent::Breakdown => 0,
            DjEvent::PhraseChange => 2,
            _ => 1,
        }
    }
}

/// What a trigger plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerTarget {
    /// A pad's look or chase, over the groups its look sets.
    Pad(u16),
    /// A cue of the stack, over the groups it sets.
    Cue(usize),
    /// Ends the trigger that runs: back to what runs underneath.
    Release,
}

/// One DJ event tied to what it plays.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Trigger {
    pub event: DjEvent,
    pub target: TriggerTarget,
    /// How long it plays, beats (0 = until the next trigger or RELEASE).
    #[serde(default)]
    pub beats: f32,
}

/// Lengths a trigger can play for, beats (0 = until the next one).
pub const TRIGGER_BEATS: [f32; 6] = [0.0, 4.0, 8.0, 16.0, 32.0, 64.0];

/// A trigger playing now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActiveTrigger {
    pub event: DjEvent,
    pub target: TriggerTarget,
    /// Beat of the music at which it ends (`None`: until the next one).
    pub until: Option<f64>,
}

/// Turns successive frames of deck music into DJ events.
#[derive(Clone, Debug, Default)]
pub struct Detector {
    prev: Vec<DeckMusic>,
    /// Phrase slot of the deck you hear, last frame (and which deck).
    phrase: Option<(usize, Option<od_core::PhraseKind>)>,
    /// The crossfader is past a side and has not come back yet.
    xf_side: Option<bool>,
    started: bool,
}

impl Detector {
    /// Events between the last frame and this one, most specific first.
    /// The first frame only learns the state (nothing fires on start-up).
    pub fn detect(&mut self, decks: &[DeckMusic], crossfader: Option<f32>) -> Vec<DjEvent> {
        let mut out = Vec::new();
        let first = !self.started;
        self.started = true;
        for (i, d) in decks.iter().enumerate() {
            let Some(p) = self.prev.get(i) else { continue };
            if first {
                break;
            }
            if d.track.is_some() && d.track != p.track {
                out.push(DjEvent::Load);
            }
            if d.playing && !p.playing && d.track.is_some() && d.track == p.track {
                out.push(DjEvent::PlayStart);
            }
            let near = |m: &DeckMusic| m.playing && m.remaining.is_some_and(|r| r <= END_NEAR_SECS);
            if near(d) && !near(p) && d.track == p.track {
                out.push(DjEvent::EndNear);
            }
        }
        // Phrases of the deck you hear most.
        let dominant = decks
            .iter()
            .enumerate()
            .filter(|(_, d)| d.playing && d.weight > 0.01)
            .max_by(|a, b| a.1.weight.total_cmp(&b.1.weight));
        let now = dominant.map(|(i, d)| (i, d.phrase));
        if !first
            && let (Some((i, kind)), Some((j, before))) = (now, self.phrase)
            && i == j
            && kind != before
            && kind.is_some()
        {
            match PhraseSlot::of(kind) {
                Some(PhraseSlot::Drop) => out.push(DjEvent::Drop),
                Some(PhraseSlot::Breakdown) => out.push(DjEvent::Breakdown),
                _ => {}
            }
            out.push(DjEvent::PhraseChange);
        }
        self.phrase = now;
        // The crossfader at a side, once until it comes back.
        if let Some(x) = crossfader.filter(|x| x.is_finite()) {
            match self.xf_side {
                None if x <= -XF_SIDE => {
                    self.xf_side = Some(false);
                    if !first {
                        out.push(DjEvent::CrossfaderLeft);
                    }
                }
                None if x >= XF_SIDE => {
                    self.xf_side = Some(true);
                    if !first {
                        out.push(DjEvent::CrossfaderRight);
                    }
                }
                Some(_) if x.abs() <= XF_REARM => self.xf_side = None,
                _ => {}
            }
        }
        self.prev = decks.to_vec();
        out.sort_by_key(|e| e.rank());
        out
    }
}

/// The trigger the first event with one tied to it starts (events most
/// specific first, as [`Detector::detect`] returns them).
pub fn pick<'a>(triggers: &'a [Trigger], events: &[DjEvent]) -> Option<&'a Trigger> {
    events
        .iter()
        .find_map(|e| triggers.iter().find(|t| t.event == *e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use od_core::PhraseKind;

    fn deck(track: i64, playing: bool, phrase: Option<PhraseKind>) -> DeckMusic {
        DeckMusic {
            track: Some(track),
            playing,
            bpm: 128.0,
            weight: if playing { 1.0 } else { 0.0 },
            phrase,
            ..Default::default()
        }
    }

    #[test]
    fn events_from_successive_frames() {
        let mut d = Detector::default();
        let empty = DeckMusic::default();
        // Start-up learns the state: nothing fires.
        assert!(d.detect(&[empty, empty], Some(0.0)).is_empty());
        // A track loads on deck 2.
        let loaded = deck(7, false, None);
        assert_eq!(d.detect(&[empty, loaded], Some(0.0)), vec![DjEvent::Load]);
        // It starts.
        let intro = deck(7, true, Some(PhraseKind::Intro));
        assert_eq!(d.detect(&[empty, intro], Some(0.0)), vec![DjEvent::PlayStart]);
        assert!(d.detect(&[empty, intro], Some(0.0)).is_empty(), "once");
        // The drop: the specific event first, then the general one.
        let drop = deck(7, true, Some(PhraseKind::Chorus));
        assert_eq!(
            d.detect(&[empty, drop], Some(0.0)),
            vec![DjEvent::Drop, DjEvent::PhraseChange]
        );
        let down = deck(7, true, Some(PhraseKind::Down));
        assert_eq!(
            d.detect(&[empty, down], Some(0.0)),
            vec![DjEvent::Breakdown, DjEvent::PhraseChange]
        );
        let verse = deck(7, true, Some(PhraseKind::Verse));
        assert_eq!(d.detect(&[empty, verse], Some(0.0)), vec![DjEvent::PhraseChange]);
        // The crossfader to the right, once until it comes back.
        assert_eq!(d.detect(&[empty, verse], Some(0.95)), vec![DjEvent::CrossfaderRight]);
        assert!(d.detect(&[empty, verse], Some(1.0)).is_empty());
        assert!(d.detect(&[empty, verse], Some(0.7)).is_empty(), "not back yet");
        assert!(d.detect(&[empty, verse], Some(0.2)).is_empty());
        assert_eq!(d.detect(&[empty, verse], Some(-1.0)), vec![DjEvent::CrossfaderLeft]);
        // The end draws near.
        let late = DeckMusic {
            remaining: Some(29.0),
            ..verse
        };
        assert_eq!(d.detect(&[empty, late], Some(-1.0)), vec![DjEvent::EndNear]);
        assert!(d.detect(&[empty, late], Some(-1.0)).is_empty());
    }

    #[test]
    fn the_most_specific_trigger_wins() {
        let t = |event, pad| Trigger {
            event,
            target: TriggerTarget::Pad(pad),
            beats: 0.0,
        };
        let triggers = [t(DjEvent::PhraseChange, 1), t(DjEvent::Drop, 2)];
        let events = [DjEvent::Drop, DjEvent::PhraseChange];
        assert_eq!(pick(&triggers, &events).unwrap().target, TriggerTarget::Pad(2));
        let events = [DjEvent::Breakdown, DjEvent::PhraseChange];
        assert_eq!(pick(&triggers, &events).unwrap().target, TriggerTarget::Pad(1));
        assert!(pick(&triggers, &[DjEvent::Load]).is_none());
    }
}
