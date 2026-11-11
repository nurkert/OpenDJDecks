//! Where the show takes its music from (roadmaps [34] and [36]).
//!
//! * **Decks**: what the decks of this app play (the default).
//! * **Input**: an audio input analysed for beat and energy
//!   ([`crate::listen::BeatFollower`]).
//! * **Network**: beat, phrase and energy sent by another OpenDJDecks on the
//!   local network over OSC, so a second computer can run the lights while
//!   the first one plays.
//!
//! The input and network music arrive now and then; [`External`] carries
//! the beat forward between updates and lets go of a source that has gone
//! quiet.

use serde::{Deserialize, Serialize};

use crate::show::DeckMusic;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MusicSource {
    #[default]
    Decks,
    Input,
    Network,
}

/// Seconds without an update after which network music counts as gone.
pub const NETWORK_TIMEOUT: f32 = 2.0;

/// Music from outside the decks, carried between updates.
#[derive(Clone, Debug, Default)]
pub struct External {
    music: Option<DeckMusic>,
    age: f32,
}

impl External {
    /// A new update.
    pub fn set(&mut self, music: DeckMusic) {
        self.music = Some(music);
        self.age = 0.0;
    }

    pub fn clear(&mut self) {
        self.music = None;
    }

    /// Moves `dt` seconds on: the beat runs on at its tempo while playing.
    /// `None` once nothing has arrived for `timeout` seconds.
    pub fn advance(&mut self, dt: f32, timeout: f32) -> Option<DeckMusic> {
        let m = self.music.as_mut()?;
        self.age += dt.max(0.0);
        if self.age > timeout {
            self.music = None;
            return None;
        }
        if m.playing {
            m.beat += dt.max(0.0) as f64 * m.bpm.max(1.0) / 60.0;
            if let Some(r) = m.remaining.as_mut() {
                *r = (*r - dt.max(0.0)).max(0.0);
            }
        }
        Some(*m)
    }
}

/// The music the show follows for `source`: the decks, or the outside
/// music alone (the decks are left out, so a source that goes quiet is
/// silence for the show, not a deck left playing in the headphones).
pub fn compose(source: MusicSource, decks: &[DeckMusic], external: Option<DeckMusic>) -> Vec<DeckMusic> {
    match source {
        MusicSource::Decks => decks.to_vec(),
        MusicSource::Input | MusicSource::Network => external.into_iter().collect(),
    }
}

/// Something audible in this music (the show is not idle).
pub fn audible(decks: &[DeckMusic]) -> bool {
    decks.iter().any(|d| d.playing && d.weight > 0.01)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_music_runs_on_and_times_out() {
        let mut e = External::default();
        assert!(e.advance(0.1, NETWORK_TIMEOUT).is_none());
        e.set(DeckMusic {
            playing: true,
            bpm: 120.0,
            beat: 10.0,
            weight: 1.0,
            ..Default::default()
        });
        let m = e.advance(0.5, NETWORK_TIMEOUT).unwrap();
        assert!((m.beat - 11.0).abs() < 1e-6, "{}", m.beat);
        assert!(e.advance(1.0, NETWORK_TIMEOUT).is_some());
        assert!(e.advance(1.0, NETWORK_TIMEOUT).is_none(), "gone after 2 s");
        let decks = [DeckMusic::default()];
        assert_eq!(compose(MusicSource::Decks, &decks, None).len(), 1);
        assert!(compose(MusicSource::Network, &decks, None).is_empty());
    }
}
