//! Domain types shared by all OpenDJDecks crates.
//!
//! Time positions inside a track are expressed in seconds (`f64`) from the
//! start of the decoded audio. Conversions to sample frames or to the
//! millisecond units of exchange formats happen at the edges.

pub mod beatgrid;
pub mod color;
pub mod cue;
pub mod env;
pub mod ids;
pub mod key;
pub mod note;
pub mod phrase;
pub mod scale;
pub mod theory;
pub mod track;
pub mod waveform;

pub use beatgrid::{Beat, Beatgrid};
pub use color::{CueColor, TRACK_COLORS};
pub use cue::{Cue, CueKind, CueTrigger};
pub use ids::*;
pub use key::{KeyNotation, KeyRelation, Mode, MusicalKey};
pub use note::NotePitch;
pub use phrase::{Phrase, PhraseKind, PhraseMood, mark_drop, normalize_phrases};
pub use scale::{Scale, ScaleKey, Triad};
pub use theory::{Check, Neighbour, Why};
pub use track::{AnalysisConfidence, AudioFormat, DataSource, TrackAnalysis, TrackTags};
pub use waveform::{Waveform, WaveformColumn};
