//! CREATE's render (roadmap [75]): the notes as a NEW WAV file at a chosen
//! tempo, with a locked beatgrid and one hot cue per note, added to the
//! library like any track. The source track is only read.
//!
//! Two layouts, both from the arrange strip ([`crate::arrange`]):
//!
//! - SINGLE NOTES ([`Layout::Single`]): each distinct note once, low to
//!   high, each on the downbeat of its own bar and cut after
//!   [`SINGLE_BEATS`] beats, so a beat of silence comes before the next cue.
//!   Every cue lands on a downbeat; the file is one bar per note.
//! - MELODY ([`Layout::Melody`]): the strip in time, step 1 on the first
//!   downbeat, each note on its step and cut where it ends (a hold `_`
//!   lengthens it, a rest stays silent), over the strip's whole bars. Each
//!   cue lands on its step of the grid: on a beat line, or on the half or
//!   quarter of a beat on 1/8 and 1/16.
//!
//! What the stick formats take ([`check`]) is what this app's own code
//! carries: [`CUE_LIMIT`] hot cues.
//!
//! The model here is pure but for [`write`]; create.rs wires it to the
//! RESULT bar, the note render cache and the library.

use crate::arrange::{self, BEATS_PER_BAR, Melody};
use od_core::{Beatgrid, Cue, DataSource, ScaleKey, TrackId};
use od_engine::SampleData;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Hot cues a rendered file may carry: the slots the library's cue tables
/// and the engine hold (pads A–H in [`od_engine::HOT_CUE_BANKS`] banks),
/// which is what both stick formats we write store. The app's own stick
/// format keeps the library's cues as they are; the club stick export
/// writes each hot cue with its slot number (od-format-clubusb
/// `anlz::build`). Whether a given player shows them all is not known here.
pub(crate) const CUE_LIMIT: usize = od_engine::HOT_CUES * od_engine::HOT_CUE_BANKS;
/// Hot cues on the first pad bank (A–H).
pub(crate) const BANK: usize = od_engine::HOT_CUES;
/// SINGLE NOTES: beats a note may sound before it is cut.
pub(crate) const SINGLE_BEATS: f64 = 3.0;
/// The fade where a note is cut, seconds.
const CUT_FADE: f64 = 0.005;

/// How the notes are laid out in the file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Layout {
    /// One cue per distinct note, to play them by hand.
    Single,
    /// One cue per note in order, ready as a transition.
    #[default]
    Melody,
}

impl Layout {
    pub fn id(self) -> &'static str {
        match self {
            Layout::Single => "single",
            Layout::Melody => "melody",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        [Layout::Single, Layout::Melody].into_iter().find(|l| l.id() == id)
    }
}

/// Where the file's tempo comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TempoFrom {
    /// The master tempo.
    #[default]
    Master,
    /// The tempo of the track the note was picked from.
    Track,
}

impl TempoFrom {
    pub fn id(self) -> &'static str {
        match self {
            TempoFrom::Master => "master",
            TempoFrom::Track => "track",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        [TempoFrom::Master, TempoFrom::Track].into_iter().find(|t| t.id() == id)
    }
}

/// The choices of RESULT, as kept (`create.render`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Choice {
    pub layout: Layout,
    pub tempo: TempoFrom,
}

/// One note in the file: its pitch (MIDI), the beat it starts on and how
/// many beats it may sound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placed {
    pub note: i32,
    pub beat: f64,
    pub beats: f64,
}

/// The file's notes and its length in beats (whole bars).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Plan {
    pub notes: Vec<Placed>,
    pub beats: u32,
}

impl Plan {
    /// Whole bars.
    pub fn bars(&self) -> u32 {
        self.beats / BEATS_PER_BAR
    }

    /// One hot cue per note.
    pub fn cues(&self) -> usize {
        self.notes.len()
    }
}

/// Where the notes of `m` go in the file in `layout` (empty without notes).
pub(crate) fn plan(m: &Melody, layout: Layout) -> Plan {
    match layout {
        Layout::Single => {
            let mut notes: Vec<i32> = m.steps.iter().flatten().map(|s| s.note).collect();
            notes.sort_unstable();
            notes.dedup();
            Plan {
                notes: notes
                    .iter()
                    .enumerate()
                    .map(|(i, &note)| Placed { note, beat: f64::from(BEATS_PER_BAR) * i as f64, beats: SINGLE_BEATS })
                    .collect(),
                beats: BEATS_PER_BAR * notes.len() as u32,
            }
        }
        Layout::Melody => {
            let per_beat = f64::from(m.grid.per_beat());
            Plan {
                notes: m
                    .steps
                    .iter()
                    .enumerate()
                    .filter_map(|(i, s)| {
                        s.map(|s| Placed { note: s.note, beat: i as f64 / per_beat, beats: f64::from(s.len) / per_beat })
                    })
                    .collect(),
                beats: m.bars() as u32 * BEATS_PER_BAR,
            }
        }
    }
}

/// What the stick formats take of a file with `cues` hot cues: whether it
/// fits, and how many cues lie beyond the first pad bank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Check {
    pub cues: usize,
    pub fits: bool,
    pub later: usize,
}

pub(crate) fn check(cues: usize) -> Check {
    Check { cues, fits: cues <= CUE_LIMIT, later: cues.min(CUE_LIMIT).saturating_sub(BANK) }
}

/// The tempo rounded to a hundredth of a BPM, as the file is rendered.
pub(crate) fn tempo(bpm: f64) -> f64 {
    (bpm * 100.0).round() / 100.0
}

/// Seconds of `beat` at `bpm`.
pub(crate) fn seconds(beat: f64, bpm: f64) -> f64 {
    beat * 60.0 / bpm
}

fn frame_at(beat: f64, bpm: f64, rate: u32) -> usize {
    (seconds(beat, bpm) * f64::from(rate)).round() as usize
}

/// The file's audio: interleaved stereo at `rate`, exactly the plan's bars
/// long at `bpm`. Each note (from `notes`, rendered at `rate`) starts on
/// its beat and is cut with a short fade where its beats end; a note not
/// in `notes` stays silent.
pub(crate) fn mix(plan: &Plan, bpm: f64, rate: u32, notes: &HashMap<i32, Arc<SampleData>>) -> Vec<f32> {
    let frames = frame_at(f64::from(plan.beats), bpm, rate);
    let mut out = vec![0.0f32; frames * 2];
    let fade = ((CUT_FADE * f64::from(rate)) as usize).max(1);
    for p in &plan.notes {
        let Some(s) = notes.get(&p.note) else { continue };
        let start = frame_at(p.beat, bpm, rate).min(frames);
        let room = (frame_at(p.beat + p.beats, bpm, rate).min(frames)).saturating_sub(start);
        let len = (s.samples.len() / 2).min(room);
        let cut = s.samples.len() / 2 > room;
        for i in 0..len {
            let g = if cut && len - i <= fade { (len - i - 1) as f32 / fade as f32 } else { 1.0 };
            for c in 0..2 {
                out[(start + i) * 2 + c] += s.samples[i * 2 + c] * g;
            }
        }
    }
    out
}

/// The file's beatgrid: constant at `bpm` from the first downbeat at 0,
/// locked so analysis keeps it.
pub(crate) fn grid(plan: &Plan, bpm: f64) -> Beatgrid {
    let mut g = Beatgrid::constant(bpm, 0.0, 1, seconds(f64::from(plan.beats), bpm));
    g.locked = true;
    g
}

/// One hot cue per note, A, B, C … in the plan's order, named by the note
/// (spelled as `key`).
pub(crate) fn cues(plan: &Plan, bpm: f64, key: Option<ScaleKey>) -> Vec<Cue> {
    plan.notes
        .iter()
        .take(CUE_LIMIT)
        .enumerate()
        .map(|(i, p)| Cue { label: arrange::name(key, p.note), source: DataSource::Manual, ..Cue::hot(i as u8, seconds(p.beat, bpm)) })
        .collect()
}

/// The tempo as a title shows it: "124", "124.5".
fn bpm_text(bpm: f64) -> String {
    format!("{}", tempo(bpm))
}

/// "<title> (Melody 124 BPM)" or "<title> (Notes 124 BPM)".
pub(crate) fn title(source: &str, layout: Layout, bpm: f64) -> String {
    let what = match layout {
        Layout::Single => "Notes",
        Layout::Melody => "Melody",
    };
    let base = if source.trim().is_empty() { "CREATE" } else { source.trim() };
    format!("{base} ({what} {} BPM)", bpm_text(bpm))
}

/// Where rendered files go: `created` beside the library, as the sample
/// tracks go to `samples`.
pub(crate) fn dir(library: &Path) -> PathBuf {
    library.parent().unwrap_or(Path::new(".")).join("created")
}

/// "<title>.wav" in `dir` (characters a file name cannot hold replaced),
/// or "<title> 2.wav" … when taken: a render never replaces a file.
pub(crate) fn file_path(dir: &Path, title: &str) -> PathBuf {
    let stem: String = title.chars().map(|c| if "/\\:*?\"<>|".contains(c) || c.is_control() { '-' } else { c }).collect();
    (1..)
        .map(|n| dir.join(if n == 1 { format!("{stem}.wav") } else { format!("{stem} {n}.wav") }))
        .find(|p| !p.exists())
        .unwrap_or_else(|| dir.join(format!("{stem}.wav")))
}

/// Everything a render needs, handed to its worker thread.
pub(crate) struct Job {
    pub library: PathBuf,
    /// The title and artist of the track the note was picked from.
    pub title: String,
    pub artist: String,
    pub layout: Layout,
    pub plan: Plan,
    pub bpm: f64,
    pub rate: u32,
    pub notes: HashMap<i32, Arc<SampleData>>,
    pub key: Option<ScaleKey>,
    /// The strip as typed ("A3 C4 - E4"), for the comment.
    pub text: String,
}

/// What a render made.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Made {
    pub track: TrackId,
    pub title: String,
    pub path: PathBuf,
}

/// Mixes the job, writes it as a new WAV file into [`dir`] and adds it to
/// the library with its grid, cues, tags and key as one edit (one undo
/// step). Runs on a worker thread; the source file is only read.
pub(crate) fn write(job: &Job) -> anyhow::Result<Made> {
    if job.plan.notes.is_empty() {
        anyhow::bail!("there are no notes to render");
    }
    if !check(job.plan.cues()).fits {
        anyhow::bail!("{} hot cues are more than the {} the stick formats hold", job.plan.cues(), CUE_LIMIT);
    }
    let mut lib = od_library::Library::open(&job.library)?;
    let audio = mix(&job.plan, job.bpm, job.rate, &job.notes);
    let title = title(&job.title, job.layout, job.bpm);
    let folder = dir(&job.library);
    std::fs::create_dir_all(&folder)?;
    let path = file_path(&folder, &title);
    crate::track_edit::write_wav(&path, &audio, 2, job.rate)?;
    let id = crate::workers::import_file(&mut lib, &path)?;
    let tags = od_core::TrackTags {
        title: title.clone(),
        artist: job.artist.clone(),
        comment: format!("CREATE: {}", job.text),
        ..Default::default()
    };
    let mut states = vec![
        od_library::EntityState::Tags { track: id, tags },
        od_library::EntityState::Grid { track: id, grid: Some((grid(&job.plan, job.bpm), DataSource::Manual)) },
        od_library::EntityState::Cues { track: id, cues: cues(&job.plan, job.bpm, job.key) },
    ];
    if let Some(k) = job.key.and_then(ScaleKey::musical_key) {
        states.push(od_library::EntityState::Key { track: id, key: Some((k, DataSource::Manual)) });
    }
    lib.edit("Render notes", states)?;
    Ok(Made { track: id, title, path })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrange::{Grid, Step};

    fn melody(grid: Grid, steps: &[(usize, i32, u8)]) -> Melody {
        let mut m = Melody { grid, ..Melody::default() };
        for &(i, note, len) in steps {
            m.steps[i] = Some(Step { note, len });
        }
        m.normalize();
        m
    }

    /// A note of `frames` frames of 1.0 at `rate`.
    fn tone(frames: usize, rate: u32) -> Arc<SampleData> {
        Arc::new(SampleData { samples: vec![1.0; frames * 2].into(), sample_rate: rate })
    }

    #[test]
    fn single_notes_put_each_distinct_note_on_its_own_bar_low_to_high() {
        // E4 twice, C4, A3: three distinct notes, low to high.
        let m = melody(Grid::Eighth, &[(0, 64, 1), (1, 60, 1), (2, 64, 2), (6, 57, 1)]);
        let p = plan(&m, Layout::Single);
        assert_eq!(p.notes.iter().map(|n| n.note).collect::<Vec<_>>(), [57, 60, 64]);
        assert_eq!(p.notes.iter().map(|n| n.beat).collect::<Vec<_>>(), [0.0, 4.0, 8.0], "each on a downbeat");
        assert!(p.notes.iter().all(|n| n.beats == SINGLE_BEATS));
        assert_eq!((p.beats, p.bars(), p.cues()), (12, 3, 3));
        assert_eq!(plan(&Melody::default(), Layout::Single), Plan::default(), "nothing to render");
    }

    #[test]
    fn melody_keeps_the_strip_in_time_with_rests_and_holds() {
        // 1/8: A3, C4 held two steps, a rest, E4 on step 5 (beat 2.5).
        let m = melody(Grid::Eighth, &[(0, 57, 1), (1, 60, 2), (5, 64, 1)]);
        let p = plan(&m, Layout::Melody);
        let got: Vec<(i32, f64, f64)> = p.notes.iter().map(|n| (n.note, n.beat, n.beats)).collect();
        assert_eq!(got, [(57, 0.0, 0.5), (60, 0.5, 1.0), (64, 2.5, 0.5)]);
        assert_eq!((p.beats, p.bars(), p.cues()), (4, 1, 3), "the strip's whole bars");
        // 1/4: a step is a beat; a note on step 5 starts bar 2.
        let q = plan(&melody(Grid::Quarter, &[(0, 57, 1), (4, 59, 1)]), Layout::Melody);
        assert_eq!(q.notes[1].beat, 4.0);
        assert_eq!(q.beats, 8);
    }

    #[test]
    fn the_file_is_its_bars_long_at_the_tempo_with_notes_on_their_frames() {
        let rate = 48_000;
        let m = melody(Grid::Eighth, &[(0, 57, 1), (1, 60, 2), (5, 64, 1)]);
        let p = plan(&m, Layout::Melody);
        // 1 bar at 120 BPM: 2 s.
        let notes: HashMap<i32, Arc<SampleData>> =
            [(57, tone(100, rate)), (60, tone(rate as usize, rate)), (64, tone(100, rate))].into_iter().collect();
        let out = mix(&p, 120.0, rate, &notes);
        assert_eq!(out.len() / 2, 96_000, "exactly one bar at 120 BPM");
        // At 124 BPM a bar is 4 × 60 / 124 s.
        assert_eq!(mix(&p, 124.0, rate, &notes).len() / 2, (4.0 * 60.0 / 124.0 * f64::from(rate)).round() as usize);
        // A3 sounds from frame 0 for its 100 frames, then silence.
        assert_eq!((out[0], out[99 * 2], out[100 * 2]), (1.0, 1.0, 0.0));
        // C4 (a second long) starts at beat 0.5 (12000) and is cut with a
        // fade where its beat ends (beat 1.5, frame 36000).
        let c4 = 12_000;
        assert_eq!(out[(c4 - 1) * 2], 0.0);
        assert_eq!(out[c4 * 2], 1.0);
        assert!(out[(36_000 - 1) * 2] < 0.01, "faded out at the cut");
        assert_eq!(out[36_000 * 2], 0.0, "silent after its beats");
        // E4 on beat 2.5.
        assert_eq!((out[(60_000 - 1) * 2], out[60_000 * 2]), (0.0, 1.0));
        // SINGLE NOTES: one bar per note.
        let s = plan(&m, Layout::Single);
        assert_eq!(mix(&s, 120.0, rate, &notes).len() / 2, 3 * 96_000);
    }

    #[test]
    fn the_grid_is_locked_and_the_cues_lie_on_it() {
        let m = melody(Grid::Eighth, &[(0, 57, 1), (2, 60, 1), (8, 64, 1)]);
        let bpm = 124.0;
        // MELODY: cues in order, named by their notes, on their steps.
        let p = plan(&m, Layout::Melody);
        let g = grid(&p, bpm);
        assert!(g.locked);
        assert_eq!(g.first_bpm(), Some(bpm));
        assert_eq!((g.beats[0].time, g.beats[0].bar_position), (0.0, 1), "the first downbeat at 0");
        assert_eq!(g.beats.len(), 9, "two bars of beats, the end included");
        let c = cues(&p, bpm, None);
        assert_eq!(c.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(), ["A3", "C4", "E4"]);
        assert_eq!(c.iter().map(|c| c.kind).collect::<Vec<_>>(), [od_core::CueKind::Hot(0), od_core::CueKind::Hot(1), od_core::CueKind::Hot(2)]);
        for (cue, beat) in c.iter().zip([0usize, 1, 4]) {
            assert!((cue.start - g.beats[beat].time).abs() < 1e-9, "{} on beat {beat}", cue.label);
        }
        assert_eq!(g.beats[4].bar_position, 1, "E4 starts bar 2");
        // SINGLE NOTES: every cue on a downbeat; a key spells the notes.
        let s = plan(&m, Layout::Single);
        let key = Some(ScaleKey::new(7, od_core::Scale::Major));
        let gs = grid(&s, bpm);
        for (i, cue) in cues(&s, bpm, key).iter().enumerate() {
            let b = gs.beat_index_at(cue.start + 1e-9).unwrap();
            assert!((gs.beats[b].time - cue.start).abs() < 1e-9 && gs.beats[b].bar_position == 1, "cue {i} on a downbeat");
        }
        assert_eq!(cues(&plan(&melody(Grid::Eighth, &[(0, 66, 1)]), Layout::Single), bpm, key)[0].label, "F#4");
    }

    #[test]
    fn the_check_holds_the_cues_to_what_the_stick_formats_store() {
        assert_eq!(CUE_LIMIT, 32, "4 banks of A–H");
        assert_eq!(check(5), Check { cues: 5, fits: true, later: 0 });
        assert_eq!(check(8), Check { cues: 8, fits: true, later: 0 });
        assert_eq!(check(12), Check { cues: 12, fits: true, later: 4 }, "4 on the next bank");
        assert_eq!(check(32), Check { cues: 32, fits: true, later: 24 });
        assert!(!check(33).fits, "one over");
        // A full strip never asks for more than the formats hold.
        let full = melody(Grid::Sixteenth, &(0..32).map(|i| (i, 40 + i as i32, 1)).collect::<Vec<_>>());
        assert!(check(plan(&full, Layout::Melody).cues()).fits);
        assert!(check(plan(&full, Layout::Single).cues()).fits);
    }

    #[test]
    fn titles_and_file_names_never_replace_a_file() {
        assert_eq!(title("Night Drive", Layout::Melody, 124.0), "Night Drive (Melody 124 BPM)");
        assert_eq!(title("Night Drive", Layout::Single, 123.456), "Night Drive (Notes 123.46 BPM)");
        assert_eq!(title(" ", Layout::Single, 120.0), "CREATE (Notes 120 BPM)");
        let d = tempfile::tempdir().unwrap();
        assert_eq!(dir(&d.path().join("library.sqlite")), d.path().join("created"));
        assert_eq!(file_path(d.path(), "A/B: C"), d.path().join("A-B- C.wav"));
        std::fs::write(d.path().join("A-B- C.wav"), b"x").unwrap();
        assert_eq!(file_path(d.path(), "A/B: C"), d.path().join("A-B- C 2.wav"));
    }
}
