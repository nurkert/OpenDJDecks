//! Recorder: recording the master, the tracklist, the recording panel
//! (level, meter, elapsed time, file size, live tracklist; `ui/rec.slint`),
//! automatic start / stop on silence, and the recordings list.
//!
//! Recording for release ([27]): chapter markers (MARK) beside the tracks,
//! a voice track (the MIC input alone) next to the mix, loudness, true peak
//! and clips while recording and on every finished recording, and the
//! tracklist editor (rename, remove, insert, move by time; export as text
//! or CUE with the chapters written into the files).

use crate::app::{App, handler};
use crate::models::model;
use crate::{RecEditRow, RecTrack, RecordingRow, engine, format};
use od_recorder::TracklistEntry;
use od_engine::{Command, EngineState};
use od_recorder::{AutoAction, Recorder, SilenceDetector};
use slint::{ComponentHandle, Model, SharedString};
use od_core::TrackId;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

/// Sets a string property of a global only when it changed.
macro_rules! set_if {
    ($g:expr, $get:ident, $set:ident, $v:expr) => {{
        let v: String = $v;
        if $g.$get().as_str() != v {
            $g.$set(v.into());
        }
    }};
}

/// Master level (dBFS) that counts as sound for the automatic start / stop.
pub(crate) const SILENCE_DB: f32 = -40.0;
/// Seconds of sound before the automatic start fires.
pub(crate) const START_AFTER: f32 = 2.0;
/// Seconds of silence before the automatic stop fires.
pub(crate) const STOP_AFTER: f32 = 30.0;

pub(crate) struct RecorderState {
    recorder: Option<Recorder>,
    /// Deck that was audible last (tracklist marks).
    last_dominant: Option<usize>,
    /// The deck audible now and since when (seconds into the recording),
    /// until it has stayed long enough to count ([`STAYS_FOR`]).
    candidate: Option<(Option<usize>, f64)>,
    /// Start automatically when sound starts (Preferences > Recording).
    armed: bool,
    /// Deck that was audible last (play history), and this run's history
    /// session ("Session 2026-11-07 21:30"), named at the first play.
    last_played: Option<usize>,
    history_session: Option<String>,
    /// Silence detection while armed.
    detector: SilenceDetector,
    /// Last frame (time step of the detector).
    last_frame: Option<Instant>,
    /// The file being written and the tracklist so far (offset, title,
    /// artist) for the panel.
    path: Option<PathBuf>,
    tracks: Vec<(f64, String, String)>,
    /// Seconds each track has been on air this run, and the tracks past
    /// [`PLAYED_AFTER`]: the browser ticks those as played.
    on_air: HashMap<TrackId, f32>,
    pub(crate) played: HashSet<TrackId>,
    /// When each played track passed the mark ("22:41"), for the list.
    pub(crate) played_at: HashMap<TrackId, String>,
    /// The tracklist editor ([27]): the recording open in it.
    edit: Option<Editing>,
}

/// A finished recording open in the tracklist editor.
pub(crate) struct Editing {
    pub id: i64,
    pub path: PathBuf,
    pub duration: f64,
    pub tracklist: Vec<TracklistEntry>,
}

impl RecorderState {
    /// The recorder while a recording runs.
    pub(crate) fn recording(&self) -> Option<&Recorder> {
        self.recorder.as_ref().filter(|r| r.is_recording())
    }
}

impl Default for RecorderState {
    fn default() -> Self {
        Self {
            recorder: None,
            last_dominant: None,
            candidate: None,
            armed: false,
            last_played: None,
            history_session: None,
            detector: SilenceDetector::new(SILENCE_DB, START_AFTER, STOP_AFTER),
            last_frame: None,
            path: None,
            tracks: Vec::new(),
            on_air: HashMap::new(),
            played: HashSet::new(),
            played_at: HashMap::new(),
            edit: None,
        }
    }
}

impl RecorderState {
    pub fn is_recording(&self) -> bool {
        self.recorder.as_ref().is_some_and(|r| r.is_recording())
    }
}

/// Hands the engine's record tap (and its voice tap) to the recorder (once
/// audio starts).
pub(crate) fn attach(
    app: &App,
    tap: od_engine::RecordTap,
    voice: Option<od_engine::RecordTap>,
    sample_rate: u32,
) {
    let mut r = Recorder::new(tap, sample_rate);
    if let Some(v) = voice {
        r = r.with_voice(v);
    }
    app.recorder.borrow_mut().recorder = Some(r);
    apply_gain(app);
}

/// Recording level from Preferences > Recording (also while recording).
pub(crate) fn apply_gain(app: &App) {
    let db = app.state.borrow().settings.recording.gain_db;
    if let Some(r) = app.recorder.borrow().recorder.as_ref() {
        r.set_gain_db(db);
    }
    let options = crate::settings::REC_GAINS;
    let g = app.ui.global::<crate::Rec>();
    g.set_gain_options(model(
        options
            .iter()
            .map(|g| slint::SharedString::from(gain_text(*g)))
            .collect(),
    ));
    g.set_gain(
        options
            .iter()
            .position(|x| (*x - db).abs() < 0.01)
            .unwrap_or(3) as i32,
    );
}

/// "0 dB", "+3 dB", "−6 dB".
pub(crate) fn gain_text(db: f32) -> String {
    if db == 0.0 {
        "0 dB".into()
    } else if db > 0.0 {
        format!("+{db} dB")
    } else {
        format!("−{} dB", -db)
    }
}

/// The file name of a recording from the pattern (Preferences >
/// Recording): `{date}`, `{time}` and `{n}` (number of the recording that
/// day) from a library timestamp; empty = "Mix {date} {time}".
pub(crate) fn recording_name(pattern: &str, now: &str, n: usize) -> String {
    let t = now.trim_end_matches('Z');
    let (date, time) = t.split_once('T').unwrap_or((t, ""));
    let pattern = if pattern.trim().is_empty() {
        "Mix {date} {time}"
    } else {
        pattern.trim()
    };
    let name = pattern
        .replace("{date}", date)
        .replace("{time}", &time.replace(':', "-"))
        .replace("{n}", &n.to_string());
    let clean: String = name
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':') || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect();
    if clean.trim().is_empty() {
        mix_name(now)
    } else {
        clean.trim().to_string()
    }
}

/// Meter position (0..=1) of a linear peak: −48 dBFS … 0 dBFS.
pub(crate) fn meter(peak: f32) -> f32 {
    if peak <= 0.0 {
        return 0.0;
    }
    ((20.0 * peak.log10() + 48.0) / 48.0).clamp(0.0, 1.0)
}

/// What the silence detection asks for this frame (pure): `armed` =
/// automatic start on, `stop` = automatic stop on.
pub(crate) fn auto_step(
    detector: &mut SilenceDetector,
    peak: f32,
    dt: f32,
    recording: bool,
    armed: bool,
    stop: bool,
) -> Option<AutoAction> {
    if !armed {
        return None;
    }
    let db = if peak > 0.0 {
        20.0 * peak.log10()
    } else {
        -120.0
    };
    match detector.update(db, dt, recording) {
        Some(AutoAction::Stop) if !stop => None,
        a => a,
    }
}

/// Recordings started today in `dir` (for `{n}`).
fn recordings_today(lib: &od_library::Library, now: &str) -> usize {
    let day = now.get(..10).unwrap_or_default().to_string();
    lib.conn()
        .query_row(
            "SELECT count(*) FROM recording WHERE substr(started, 1, 10) = ?1",
            [day],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0) as usize
}

/// History session name of a performance from a library timestamp:
/// "Session 2026-11-07 21:30".
pub(crate) fn session_name(now: &str) -> String {
    let t = now.trim_end_matches('Z').replace('T', " ");
    format!("Session {}", t.get(..16).unwrap_or(&t))
}

/// Play history: every track that becomes the audible one is logged in this
/// run's session (SPEC §5.5: every performance session is a history list).
fn log_play(app: &App, state: &EngineState) {
    let dom = state.audible.dominant;
    if dom == app.recorder.borrow().last_played {
        return;
    }
    app.recorder.borrow_mut().last_played = dom;
    let Some(d) = dom else { return };
    let id = {
        let decks = app.decks.borrow();
        if decks.source_is_stick() {
            return;
        }
        decks.loaded(d).map(|m| m.id)
    };
    let Some(id) = id else { return };
    if app.state.borrow().read_only() {
        return;
    }
    let session = {
        let mut r = app.recorder.borrow_mut();
        r.history_session
            .get_or_insert_with(|| session_name(&app.state.borrow().lib.now().unwrap_or_default()))
            .clone()
    };
    if app
        .state
        .borrow_mut()
        .lib
        .record_play(id, &session, "opendjdecks")
        .is_ok()
    {
        crate::browser::refresh_tree(app);
    }
}

/// How long a deck has to stay the audible one before it enters the
/// tracklist (two decks at the same level swap the lead back and forth
/// within a mix; the entry keeps the time it took the lead).
pub(crate) const STAYS_FOR: f64 = 4.0;

/// Tracklist hysteresis (pure): the audible deck `dom` at `now`; returns
/// the deck to enter and since when, once it has stayed [`STAYS_FOR`]
/// (the first one of a recording at once).
pub(crate) fn settle(
    candidate: &mut Option<(Option<usize>, f64)>,
    dom: Option<usize>,
    now: f64,
    last: Option<usize>,
    first: bool,
) -> Option<(Option<usize>, f64)> {
    if candidate.is_none_or(|c| c.0 != dom) {
        *candidate = Some((dom, now));
    }
    let (d, since) = (*candidate)?;
    if d == last || d.is_none() {
        return None;
    }
    (first || now - since >= STAYS_FOR).then_some((d, since))
}

/// Time on air after which a track counts as played (a short listen or a
/// check in the headphones does not).
pub(crate) const PLAYED_AFTER: f32 = 30.0;

/// Adds `dt` to every track on air; true when one just passed
/// [`PLAYED_AFTER`].
pub(crate) fn add_air_time(
    on_air: &mut HashMap<TrackId, f32>,
    played: &mut HashSet<TrackId>,
    audible: &[TrackId],
    dt: f32,
) -> bool {
    let mut new = false;
    for id in audible {
        let t = on_air.entry(*id).or_default();
        *t += dt;
        if *t >= PLAYED_AFTER && played.insert(*id) {
            new = true;
        }
    }
    new
}

/// Counts air time of the audible decks' tracks; a track that passes the
/// mark gets its tick in the browser.
fn mark_played(app: &App, state: &EngineState, dt: f32) {
    if dt <= 0.0 {
        return;
    }
    let audible: Vec<TrackId> = {
        let decks = app.decks.borrow();
        if decks.source_is_stick() {
            return;
        }
        (0..crate::decks::DECKS)
            .filter(|&d| state.audible.audible.get(d).copied().unwrap_or(false))
            .filter_map(|d| decks.loaded(d).map(|m| m.id))
            .collect()
    };
    if audible.is_empty() {
        return;
    }
    let new = {
        let mut rs = app.recorder.borrow_mut();
        let rs = &mut *rs;
        let before = rs.played.len();
        let new = add_air_time(&mut rs.on_air, &mut rs.played, &audible, dt);
        if rs.played.len() > before {
            let now = clock_now(app);
            for id in &audible {
                if rs.played.contains(id) {
                    rs.played_at.entry(*id).or_insert_with(|| now.clone());
                }
            }
        }
        new
    };
    if new {
        let played = app.recorder.borrow().played.clone();
        let reload = {
            let mut st = app.state.borrow_mut();
            st.browser.played_tonight = played;
            st.browser.filter.unplayed
        };
        if reload {
            app.state.borrow_mut().load_rows();
        }
        crate::browser::refresh_tracks(app);
    }
}

/// The local time of day ("22:41") from the library clock.
fn clock_now(app: &App) -> String {
    let now = app.state.borrow().lib.now().unwrap_or_default();
    now.get(11..16).unwrap_or_default().to_string()
}

/// "Mix 2026-11-06 18-30-00" from a library timestamp.
pub(crate) fn mix_name(now: &str) -> String {
    format!(
        "Mix {}",
        now.replace(':', "-")
            .trim_end_matches('Z')
            .replace('T', " ")
    )
}

/// "2026-11-06 18:30:00  ·  1:02:03" for the recordings list.
pub(crate) fn recording_detail(started: &str, duration: f64) -> String {
    format!(
        "{}  ·  {}",
        started.replace('T', " ").trim_end_matches('Z'),
        format::time(duration)
    )
}

/// Marks `track` as played tonight at `at` (UI tests: the 30 s on air
/// without waiting for them).
#[cfg(test)]
pub(crate) fn mark_played_now(app: &App, track: TrackId, at: &str) {
    {
        let mut rs = app.recorder.borrow_mut();
        rs.played.insert(track);
        rs.played_at.insert(track, at.to_string());
    }
    let played = app.recorder.borrow().played.clone();
    let reload = {
        let mut st = app.state.borrow_mut();
        st.browser.played_tonight = played;
        st.browser.filter.unplayed
    };
    if reload {
        app.state.borrow_mut().load_rows();
    }
    crate::browser::refresh_tracks(app);
}

/// Shorter silence detection times (UI tests).
#[cfg(test)]
pub(crate) fn set_silence_times(app: &App, start_after: f32, stop_after: f32) {
    app.recorder.borrow_mut().detector = SilenceDetector::new(SILENCE_DB, start_after, stop_after);
}

/// Arms (or disarms) the automatic start.
pub(crate) fn arm(app: &App, on: bool) {
    app.recorder.borrow_mut().armed = on;
}

/// The extra files a recording format produces after the WAV.
pub(crate) fn conversions(format: &str) -> Vec<&'static str> {
    match format {
        "flac" => vec!["flac"],
        "wav+mp3" => vec!["mp3"],
        _ => Vec::new(),
    }
}

/// After a recording stops: split into tracks and convert, off the UI
/// thread. A FLAC recording replaces its WAV file.
fn finish(
    rec: od_recorder::Recording,
    settings: crate::settings::RecordingSettings,
    library: PathBuf,
    events: crossbeam_channel::Sender<crate::workers::WorkerEvent>,
) {
    use crate::workers::WorkerEvent::Message;
    std::thread::spawn(move || {
        if settings.auto_split {
            match od_recorder::split(&rec) {
                Ok(files) => {
                    let _ = events.send(Message(format!(
                        "Split into {} track files next to the recording",
                        files.len()
                    )));
                }
                Err(e) => {
                    let _ = events.send(Message(format!("Split failed: {e}")));
                }
            }
        }
        for ext in conversions(&settings.format) {
            let out = rec.path.with_extension(ext);
            let result = if ext == "flac" {
                od_recorder::wav_to_flac(&rec.path, &out)
            } else {
                od_recorder::wav_to_mp3(&rec.path, &out)
            };
            if let Err(e) = result {
                tracing::warn!(error = %e, "encoding the recording failed");
                let _ = events.send(Message(
                    "Converting the recording failed — the WAV file is kept".into(),
                ));
                continue;
            }
            if let Err(e) = od_recorder::write_chapters(&out, &od_recorder::chapters_of(&rec.tracklist)) {
                tracing::warn!(error = %e, "chapters not written into the converted recording");
            }
            if ext == "flac" {
                if let Some(mut info) = od_recorder::load_info(&rec.path) {
                    let _ = std::fs::remove_file(od_recorder::info_path(&rec.path));
                    info.voice = rec.voice.clone();
                    let _ = od_recorder::save_info(&out, &info);
                }
                if let Ok(lib) = od_library::Library::open(&library) {
                    let _ = lib.conn().execute(
                        "UPDATE recording SET path = ?1 WHERE path = ?2",
                        (out.display().to_string(), rec.path.display().to_string()),
                    );
                }
                let _ = std::fs::remove_file(&rec.path);
            }
            let _ = events.send(Message(format!("Recording saved as {}", out.display())));
        }
    });
}

fn toggle(app: &App) {
    if !engine::ensure_engine(app) {
        return;
    }
    if app.recorder.borrow().is_recording() {
        // Stopped by hand: no automatic restart.
        app.recorder.borrow_mut().armed = false;
        stop(app);
    } else {
        start(app);
    }
}

/// Starts a recording of the master into the recordings folder.
fn start(app: &App) {
    let dir = app.state.borrow().settings.recordings_dir();
    let name = {
        let st = app.state.borrow();
        let now = st.lib.now().unwrap_or_default();
        recording_name(
            &st.settings.recording.name_pattern,
            &now,
            recordings_today(&st.lib, &now) + 1,
        )
    };
    let want_voice = app.state.borrow().settings.recording.voice;
    let voice = want_voice
        && app
            .recorder
            .borrow()
            .recorder
            .as_ref()
            .is_some_and(|r| r.has_voice());
    engine::send(app, Command::SetVoiceTap(voice));
    engine::send(app, Command::SetRecordTap(true));
    let r = app
        .recorder
        .borrow_mut()
        .recorder
        .as_mut()
        .map(|r| r.start_with(&dir, &name, voice));
    match r {
        Some(Ok(path)) => {
            let mut rs = app.recorder.borrow_mut();
            rs.path = Some(path);
            rs.tracks.clear();
            rs.last_dominant = None;
            rs.candidate = None;
        }
        Some(Err(e)) => app.status(format!("Recording could not start: {e}")),
        None => {}
    }
}

/// Stops the recording, stores it and starts the split / conversion.
fn stop(app: &App) {
    engine::send(app, Command::SetRecordTap(false));
    engine::send(app, Command::SetVoiceTap(false));
    let result = app
        .recorder
        .borrow_mut()
        .recorder
        .as_mut()
        .map(|r| r.stop());
    match result {
        Some(Ok(rec)) => {
            let _ = app.state.borrow().lib.conn().execute(
                "INSERT INTO recording (path, started, duration, tracklist) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%SZ','now'), ?2, ?3)",
                (rec.path.display().to_string(), rec.duration, serde_json::to_string(&rec.tracklist).unwrap_or_default()),
            );
            app.status(format!(
                "Recording saved: {} ({} tracks)",
                rec.path.display(),
                rec.tracklist.len()
            ));
            let l = rec.loudness;
            if l.clips > 0 || l.true_peak.is_some_and(|p| p > od_recorder::TRUE_PEAK_LIMIT) {
                app.status(format!(
                    "Recording saved: {} · true peak {} · {} clips",
                    od_recorder::lufs_text(l.integrated),
                    od_recorder::true_peak_text(l.true_peak),
                    l.clips
                ));
            }
            let (settings, library) = {
                let st = app.state.borrow();
                (
                    st.settings.recording.clone(),
                    st.config.library_path.clone(),
                )
            };
            if settings.auto_split || settings.format != "wav" {
                finish(rec, settings, library, app.worker_tx.clone());
            }
            refresh_list(app);
        }
        Some(Err(e)) => app.status(format!("Recording failed: {e}")),
        None => {}
    }
    let mut rs = app.recorder.borrow_mut();
    rs.last_dominant = None;
    rs.candidate = None;
    rs.path = None;
}

/// Frame tick: automatic start / stop, tracklist marks, play history, the
/// REC clock and the recording panel.
pub(crate) fn frame(app: &App, state: &EngineState) {
    log_play(app, state);
    let now = Instant::now();
    let dt = app
        .recorder
        .borrow_mut()
        .last_frame
        .replace(now)
        .map(|t| (now - t).as_secs_f32().min(0.5))
        .unwrap_or(0.0);
    mark_played(app, state, dt);
    let (armed, stop_on) = (
        app.recorder.borrow().armed,
        app.state.borrow().settings.recording.silence_stop,
    );
    let recording = app.recorder.borrow().is_recording();
    let action = {
        let mut rs = app.recorder.borrow_mut();
        auto_step(
            &mut rs.detector,
            state.master_meter.peak,
            dt,
            recording,
            armed,
            stop_on,
        )
    };
    match action {
        Some(AutoAction::Start) => {
            start(app);
            app.status("Recording started automatically");
        }
        Some(AutoAction::Stop) => {
            // Stays armed: the next sound starts a new recording.
            stop(app);
        }
        None => {}
    }
    let recording = app.recorder.borrow().is_recording();
    if recording {
        let dom = state.audible.dominant;
        let now = app.recorder.borrow().recorder.as_ref().map(|r| r.elapsed()).unwrap_or(0.0);
        let settled = {
            let mut rs = app.recorder.borrow_mut();
            let first = rs.tracks.is_empty();
            let last = rs.last_dominant;
            settle(&mut rs.candidate, dom, now, last, first)
        };
        if let Some((dom, since)) = settled {
            app.recorder.borrow_mut().last_dominant = dom;
            if let Some(d) = dom {
                let meta = app
                    .decks
                    .borrow()
                    .loaded(d)
                    .map(|m| (m.id, m.title.clone(), m.artist.clone()));
                if let Some((id, title, artist)) = meta {
                    let mut rs = app.recorder.borrow_mut();
                    let elapsed = since;
                    if let Some(r) = rs.recorder.as_mut() {
                        r.mark_track_at(since, Some(id), &title, &artist);
                    }
                    if rs
                        .tracks
                        .last()
                        .is_none_or(|l| l.1 != title || l.2 != artist)
                    {
                        rs.tracks.push((elapsed, title, artist));
                    }
                }
            }
        }
        let elapsed = app
            .recorder
            .borrow()
            .recorder
            .as_ref()
            .map(|r| r.elapsed())
            .unwrap_or(0.0);
        app.ui.set_recording_time(format::deck_time(elapsed).into());
    }
    app.ui.set_recording(recording);
    project_panel(app, state);
}

/// The recording panel (`Rec` global): level, meter, elapsed time, file
/// size and the tracklist so far.
fn project_panel(app: &App, state: &EngineState) {
    let g = app.ui.global::<crate::Rec>();
    let rs = app.recorder.borrow();
    let recording = rs.is_recording();
    let gain = 10f32.powf(app.state.borrow().settings.recording.gain_db / 20.0);
    let peak = if recording {
        rs.recorder
            .as_ref()
            .map(|r| r.level())
            .unwrap_or(0.0)
            .max(state.master_meter.peak * gain)
    } else {
        state.master_meter.peak * gain
    };
    g.set_level(meter(peak));
    g.set_clip(peak >= 1.0);
    g.set_recording(recording);
    g.set_armed(rs.armed);
    if recording {
        let r = rs.recorder.as_ref();
        let l = r.map(|r| r.loudness()).unwrap_or_default();
        set_if!(g, get_integrated, set_integrated, od_recorder::lufs_text(l.integrated));
        set_if!(g, get_short_term, set_short_term, od_recorder::lufs_text(l.short_term));
        set_if!(g, get_true_peak, set_true_peak, od_recorder::true_peak_text(l.true_peak));
        g.set_peak_over(l.true_peak.is_some_and(|p| p > od_recorder::TRUE_PEAK_LIMIT));
        g.set_clips(l.clips.min(i32::MAX as u64) as i32);
        let voice = r
            .and_then(|r| r.voice_path())
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        set_if!(g, get_voice_file, set_voice_file, voice);
        g.set_markers(
            r.map(|r| r.tracklist().iter().filter(|e| e.marker).count())
                .unwrap_or(0) as i32,
        );
    }
    let elapsed = rs.recorder.as_ref().map(|r| r.elapsed()).unwrap_or(0.0);
    let new_elapsed: slint::SharedString = if recording {
        format::deck_time(elapsed).into()
    } else {
        slint::SharedString::new()
    };
    if g.get_elapsed() != new_elapsed {
        g.set_elapsed(new_elapsed);
        let size = rs
            .path
            .as_ref()
            .and_then(|p| std::fs::metadata(p).ok())
            .map(|m| m.len())
            .unwrap_or(0);
        g.set_size(if recording {
            format::bytes(size).into()
        } else {
            slint::SharedString::new()
        });
        g.set_file(
            rs.path
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
                .into(),
        );
    }
    if g.get_tracks().row_count() != rs.tracks.len()
        || (!recording && rs.tracks.is_empty() && g.get_tracks().row_count() > 0)
    {
        let rows: Vec<RecTrack> = rs
            .tracks
            .iter()
            .enumerate()
            .map(|(i, (t, title, artist))| RecTrack {
                index: i as i32 + 1,
                time: format::deck_time(*t).into(),
                title: title.into(),
                artist: artist.into(),
            })
            .collect();
        g.set_tracks(model(rows));
    }
}


/// "−14.2 LUFS  ·  −0.8 dBTP  ·  0 CLIPS" and whether it calls for a look
/// (clips, or a true peak over −1 dBTP).
pub(crate) fn loudness_line(l: &od_recorder::Loudness) -> (String, bool) {
    let over = l.clips > 0 || l.true_peak.is_some_and(|p| p > od_recorder::TRUE_PEAK_LIMIT);
    (
        od_i18n::trf(
            "{}  ·  TRUE PEAK {}  ·  {} CLIPS",
            &[
                &od_recorder::lufs_text(l.integrated),
                &od_recorder::true_peak_text(l.true_peak),
                &l.clips,
            ],
        ),
        over,
    )
}

/// A chapter marker at the current time of the recording.
fn mark(app: &App) {
    let (n, at) = {
        let mut rs = app.recorder.borrow_mut();
        let Some(r) = rs.recorder.as_mut().filter(|r| r.is_recording()) else {
            drop(rs);
            app.status("Markers are set while recording");
            return;
        };
        let count = r.tracklist().iter().filter(|e| e.marker).count() + 1;
        let title = od_i18n::trf("Marker {}", &[&count]);
        let at = r.elapsed();
        let n = r.mark_chapter(&title);
        rs.tracks.push((at, title, String::new()));
        (n, at)
    };
    if let Some(n) = n {
        app.status(format!("Marker {n} set at {}", format::deck_time(at)));
    }
}

/// "12:34" or "1:02:03" (whole seconds) for the editor.
pub(crate) fn edit_time(secs: f64) -> String {
    let t = secs.max(0.0).round() as u64;
    if t >= 3600 {
        format!("{}:{:02}:{:02}", t / 3600, t / 60 % 60, t % 60)
    } else {
        format!("{}:{:02}", t / 60, t % 60)
    }
}

/// Reads "12:34", "1:02:03", "754" or "12:34.5" as seconds.
pub(crate) fn parse_time(text: &str) -> Option<f64> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    let mut total = 0.0;
    let parts: Vec<&str> = t.split(':').collect();
    if parts.len() > 3 {
        return None;
    }
    for (i, p) in parts.iter().enumerate() {
        let v: f64 = p.trim().parse().ok()?;
        if v < 0.0 || (i > 0 && v >= 60.0) {
            return None;
        }
        total = total * 60.0 + v;
    }
    Some(total)
}

/// Inserts a new entry after `after` (`None`: before the first), halfway
/// to the next one (or 30 s on, within the recording); returns its index
/// (pure).
pub(crate) fn insert_entry(
    list: &mut Vec<TracklistEntry>,
    after: Option<usize>,
    duration: f64,
    title: &str,
) -> usize {
    let (at, idx) = match after {
        None => (0.0, 0),
        Some(i) => {
            let i = i.min(list.len().saturating_sub(1));
            let from = list.get(i).map(|e| e.offset).unwrap_or(0.0);
            let to = list
                .get(i + 1)
                .map(|e| e.offset)
                .unwrap_or_else(|| (from + 60.0).min(duration.max(from)));
            ((from + to) / 2.0, i + 1)
        }
    };
    let idx = idx.min(list.len());
    list.insert(
        idx,
        TracklistEntry {
            offset: at,
            track: None,
            title: title.into(),
            artist: String::new(),
            marker: true,
        },
    );
    idx
}

/// Puts the entries in time order (stable, pure).
pub(crate) fn sort_entries(list: &mut [TracklistEntry]) {
    list.sort_by(|a, b| a.offset.total_cmp(&b.offset));
}

/// Documentation screenshots: the editor on a made-up mix (in memory).
pub(crate) fn demo_edit(app: &App, path: PathBuf, duration: f64, tracklist: Vec<TracklistEntry>) {
    app.recorder.borrow_mut().edit = Some(Editing {
        id: -1,
        path,
        duration,
        tracklist,
    });
    project_edit(app);
}

/// Opens the tracklist editor on a recording.
fn edit_open(app: &App, id: i64) {
    let row: Option<(String, f64, String)> = app
        .state
        .borrow()
        .lib
        .conn()
        .query_row(
            "SELECT path, duration, tracklist FROM recording WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok();
    let Some((path, duration, tracklist)) = row else {
        return;
    };
    let path = PathBuf::from(path);
    let tracklist = od_recorder::load_info(&path)
        .map(|i| i.tracklist)
        .unwrap_or_else(|| serde_json::from_str(&tracklist).unwrap_or_default());
    app.recorder.borrow_mut().edit = Some(Editing {
        id,
        path,
        duration,
        tracklist,
    });
    app.ui.global::<crate::RecEdit>().set_error(SharedString::new());
    project_edit(app);
}

/// The editor's rows from the open recording.
fn project_edit(app: &App) {
    let g = app.ui.global::<crate::RecEdit>();
    let rs = app.recorder.borrow();
    let Some(e) = rs.edit.as_ref() else {
        g.set_open(false);
        return;
    };
    g.set_open(true);
    g.set_name(
        e.path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
            .into(),
    );
    g.set_summary(
        od_i18n::trf(
            "{} ENTRIES  ·  {}",
            &[&e.tracklist.len(), &format::time(e.duration)],
        )
        .into(),
    );
    let rows: Vec<RecEditRow> = e
        .tracklist
        .iter()
        .enumerate()
        .map(|(i, t)| RecEditRow {
            index: i as i32 + 1,
            time: edit_time(if i == 0 { 0.0 } else { t.offset }).into(),
            title: t.title.clone().into(),
            artist: t.artist.clone().into(),
            marker: t.marker,
        })
        .collect();
    g.set_rows(model(rows));
}

/// Saves the open tracklist to the library and the info file.
fn edit_save(app: &App) {
    let rs = app.recorder.borrow();
    let Some(e) = rs.edit.as_ref() else { return };
    let _ = app.state.borrow().lib.conn().execute(
        "UPDATE recording SET tracklist = ?1 WHERE id = ?2",
        (
            serde_json::to_string(&e.tracklist).unwrap_or_default(),
            e.id,
        ),
    );
    let mut info = od_recorder::load_info(&e.path).unwrap_or_default();
    info.tracklist = e.tracklist.clone();
    if info.duration <= 0.0 {
        info.duration = e.duration;
    }
    if let Err(err) = od_recorder::save_info(&e.path, &info) {
        tracing::warn!(error = %err, "recording info not saved");
    }
}

fn edit_change(app: &App, f: impl FnOnce(&mut Editing)) {
    {
        let mut rs = app.recorder.borrow_mut();
        let Some(e) = rs.edit.as_mut() else { return };
        f(e);
    }
    edit_save(app);
}

/// Writes the CUE sheet and the text next to the recording, and the
/// chapters into its files (the WAV or FLAC, an MP3 copy), off the UI
/// thread.
fn edit_export(app: &App, what: &str) {
    let Some((path, tracklist)) = app
        .recorder
        .borrow()
        .edit
        .as_ref()
        .map(|e| (e.path.clone(), e.tracklist.clone()))
    else {
        return;
    };
    let (cue, text) = match od_recorder::export_tracklist(&path, &tracklist) {
        Ok(p) => p,
        Err(e) => {
            app.status(format!("Export failed: {e}"));
            return;
        }
    };
    let shown = if what == "cue" { cue } else { text };
    app.status(format!("Tracklist written to {}", shown.display()));
    let events = app.worker_tx.clone();
    std::thread::spawn(move || {
        let chapters = od_recorder::chapters_of(&tracklist);
        let mut files = vec![path.clone()];
        for ext in ["mp3", "flac"] {
            let p = path.with_extension(ext);
            if p.exists() && p != path {
                files.push(p);
            }
        }
        for f in files.iter().filter(|f| f.exists()) {
            if let Err(e) = od_recorder::write_chapters(f, &chapters) {
                let _ = events.send(crate::workers::WorkerEvent::Message(format!(
                    "Chapters could not be written into {}: {e}",
                    f.display()
                )));
            }
        }
    });
}

pub(crate) fn refresh_list(app: &App) {
    let st = app.state.borrow();
    let mut rows = Vec::new();
    if let Ok(mut stmt) = st
        .lib
        .conn()
        .prepare("SELECT id, path, started, duration FROM recording ORDER BY started DESC")
    {
        if let Ok(iter) = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, f64>(3)?,
            ))
        }) {
            for (id, path, started, duration) in iter.flatten() {
                let name = PathBuf::from(&path)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or(path.clone());
                let info = od_recorder::load_info(std::path::Path::new(&path));
                let (loudness, over) = info
                    .as_ref()
                    .map(|i| loudness_line(&i.loudness))
                    .unwrap_or_default();
                rows.push(RecordingRow {
                    id: id as i32,
                    name: name.into(),
                    detail: recording_detail(&started, duration).into(),
                    path: path.into(),
                    loudness: loudness.into(),
                    over,
                    voice: info.is_some_and(|i| i.voice.is_some_and(|v| v.exists())),
                });
            }
        }
    }
    app.ui.set_recordings(model(rows));
}

pub(crate) fn wire(app: &Rc<App>) {
    let ui = &app.ui;
    let armed = app.state.borrow().settings.recording.silence_start;
    arm(app, armed);
    ui.on_toggle_recording(handler!(app, |a| toggle(&a)));
    let g = ui.global::<crate::Rec>();
    g.on_toggle(handler!(app, |a| toggle(&a)));
    g.on_mark(handler!(app, |a| mark(&a)));
    let e = ui.global::<crate::RecEdit>();
    e.on_edit(handler!(app, |a, id: i32| edit_open(&a, id as i64)));
    e.on_close(handler!(app, |a| {
        a.recorder.borrow_mut().edit = None;
        project_edit(&a);
        refresh_list(&a);
    }));
    e.on_set_title(handler!(app, |a, i: i32, t: SharedString| edit_change(&a, |e| {
        if let Some(x) = e.tracklist.get_mut(i as usize) {
            x.title = t.to_string();
        }
    })));
    e.on_set_artist(handler!(app, |a, i: i32, t: SharedString| edit_change(&a, |e| {
        if let Some(x) = e.tracklist.get_mut(i as usize) {
            x.artist = t.to_string();
        }
    })));
    e.on_set_time(handler!(app, |a, i: i32, t: SharedString| {
        let g = a.ui.global::<crate::RecEdit>();
        let duration = a.recorder.borrow().edit.as_ref().map(|e| e.duration).unwrap_or(0.0);
        match parse_time(&t).filter(|s| *s <= duration.max(0.0) + 0.5) {
            Some(secs) => {
                g.set_error(SharedString::new());
                edit_change(&a, |e| {
                    if let Some(x) = e.tracklist.get_mut(i as usize) {
                        x.offset = secs;
                    }
                    sort_entries(&mut e.tracklist);
                });
            }
            None => g.set_error(
                od_i18n::trf(
                    "\"{}\" is not a time in the recording: type minutes and seconds such as 12:34 (up to {})",
                    &[&t, &edit_time(duration)],
                )
                .into(),
            ),
        }
        project_edit(&a);
    }));
    e.on_remove(handler!(app, |a, i: i32| {
        edit_change(&a, |e| {
            if (i as usize) < e.tracklist.len() {
                e.tracklist.remove(i as usize);
            }
        });
        project_edit(&a);
    }));
    e.on_insert(handler!(app, |a, i: i32| {
        let title = od_i18n::tr("New entry").to_string();
        edit_change(&a, |e| {
            let after = (i >= 0).then_some(i as usize);
            insert_entry(&mut e.tracklist, after, e.duration, &title);
        });
        project_edit(&a);
    }));
    e.on_export(handler!(app, |a, what: SharedString| edit_export(&a, &what)));
    g.on_set_gain(handler!(app, |a, i: i32| {
        let db = crate::settings::REC_GAINS
            .get(i.max(0) as usize)
            .copied()
            .unwrap_or(0.0);
        a.state
            .borrow_mut()
            .change_settings(|s| s.recording.gain_db = db);
        apply_gain(&a);
    }));
    g.on_step_gain(handler!(app, |a, d: i32| {
        let gains = crate::settings::REC_GAINS;
        let cur = a.state.borrow().settings.recording.gain_db;
        let i = gains
            .iter()
            .position(|x| (*x - cur).abs() < 0.01)
            .unwrap_or(3) as i32;
        let db = gains[(i + d).clamp(0, gains.len() as i32 - 1) as usize];
        a.state
            .borrow_mut()
            .change_settings(|s| s.recording.gain_db = db);
        apply_gain(&a);
    }));
    apply_gain(app);
    ui.on_reveal_recording(handler!(app, |a, id: i32| {
        let path: Option<String> = a
            .state
            .borrow()
            .lib
            .conn()
            .query_row("SELECT path FROM recording WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .ok();
        if let Some(p) = path {
            crate::preferences::reveal(std::path::Path::new(&p));
        }
    }));
    ui.on_split_recording(handler!(app, |a, id: i32| {
        let row: Option<(String, f64, String)> = a
            .state
            .borrow()
            .lib
            .conn()
            .query_row(
                "SELECT path, duration, tracklist FROM recording WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .ok();
        let Some((path, duration, tracklist)) = row else {
            return;
        };
        let path = PathBuf::from(path);
        let tracklist = od_recorder::load_info(&path)
            .map(|i| i.tracklist)
            .unwrap_or_else(|| serde_json::from_str(&tracklist).unwrap_or_default());
        let rec = od_recorder::Recording {
            cue_path: path.with_extension("cue"),
            text_path: path.with_extension("txt"),
            tracklist,
            duration,
            path,
            voice: None,
            loudness: Default::default(),
        };
        match od_recorder::split(&rec) {
            Ok(files) => a.status(format!(
                "Split into {} track files next to the recording",
                files.len()
            )),
            Err(e) => a.status(format!("Split failed: {e}")),
        }
    }));
}

/// Controller REC button.
pub(crate) fn toggle_from_controller(app: &App) {
    toggle(app);
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_track_counts_as_played_after_its_time_on_air() {
        use super::{PLAYED_AFTER, add_air_time};
        let (mut air, mut played) = (Default::default(), Default::default());
        let id = od_core::TrackId(7);
        assert!(!add_air_time(&mut air, &mut played, &[id], PLAYED_AFTER - 1.0), "a short listen is not a play");
        assert!(add_air_time(&mut air, &mut played, &[id], 1.0), "past the mark");
        assert!(!add_air_time(&mut air, &mut played, &[id], 5.0), "only once");
        assert!(played.contains(&id));
    }

    use super::*;

    #[test]
    fn formats_convert_with_ffmpeg() {
        assert!(conversions("wav").is_empty());
        assert_eq!(conversions("flac"), vec!["flac"]);
        assert_eq!(conversions("wav+mp3"), vec!["mp3"]);
    }

    #[test]
    fn file_name_pattern() {
        let now = "2026-11-07T21:30:05Z";
        assert_eq!(recording_name("", now, 1), "Mix 2026-11-07 21-30-05");
        assert_eq!(
            recording_name("Club {date} #{n}", now, 3),
            "Club 2026-11-07 #3"
        );
        assert_eq!(recording_name("set/{time}", now, 1), "set-21-30-05");
        assert_eq!(recording_name("   ", now, 1), "Mix 2026-11-07 21-30-05");
        assert_eq!(gain_text(0.0), "0 dB");
        assert_eq!(gain_text(3.0), "+3 dB");
        assert_eq!(gain_text(-6.0), "−6 dB");
    }

    #[test]
    fn meter_scale() {
        assert_eq!(meter(0.0), 0.0);
        assert_eq!(meter(1.0), 1.0);
        assert!((meter(10f32.powf(-24.0 / 20.0)) - 0.5).abs() < 1e-3);
        assert_eq!(meter(4.0), 1.0);
    }

    #[test]
    fn silence_starts_and_stops_only_when_enabled() {
        let mut d = SilenceDetector::new(SILENCE_DB, START_AFTER, STOP_AFTER);
        // Not armed: nothing happens, however loud.
        assert_eq!(auto_step(&mut d, 0.5, 5.0, false, false, true), None);
        // Armed: two seconds of sound start a recording.
        assert_eq!(auto_step(&mut d, 0.5, 1.0, false, true, true), None);
        assert_eq!(
            auto_step(&mut d, 0.5, 1.0, false, true, true),
            Some(AutoAction::Start)
        );
        // Silence while recording: stop after 30 s, only with automatic stop on.
        assert_eq!(auto_step(&mut d, 0.0, 29.0, true, true, true), None);
        assert_eq!(
            auto_step(&mut d, 0.0, 1.0, true, true, true),
            Some(AutoAction::Stop)
        );
        let mut d = SilenceDetector::new(SILENCE_DB, START_AFTER, STOP_AFTER);
        assert_eq!(
            auto_step(&mut d, 0.001, 60.0, true, true, false),
            None,
            "no automatic stop without the setting"
        );
        // Quiet (−60 dB) does not count as sound.
        let mut d = SilenceDetector::new(SILENCE_DB, START_AFTER, STOP_AFTER);
        assert_eq!(auto_step(&mut d, 0.001, 10.0, false, true, true), None);
    }

    #[test]
    fn the_tracklist_waits_until_a_deck_keeps_the_lead() {
        let mut c = None;
        // The first deck enters at once.
        assert_eq!(settle(&mut c, Some(0), 0.0, None, true), Some((Some(0), 0.0)));
        // Two decks swapping within a mix: nothing until one stays.
        assert_eq!(settle(&mut c, Some(1), 60.0, Some(0), false), None);
        assert_eq!(settle(&mut c, Some(0), 60.5, Some(0), false), None);
        assert_eq!(settle(&mut c, Some(1), 61.0, Some(0), false), None);
        assert_eq!(settle(&mut c, Some(1), 64.9, Some(0), false), None);
        assert_eq!(settle(&mut c, Some(1), 65.1, Some(0), false), Some((Some(1), 61.0)), "with the time it took the lead");
        assert_eq!(settle(&mut c, None, 70.0, Some(1), false), None, "silence enters nothing");
    }

    #[test]
    fn tracklist_editing_helpers() {
        assert_eq!(parse_time("12:34"), Some(754.0));
        assert_eq!(parse_time("1:02:03"), Some(3723.0));
        assert_eq!(parse_time(" 90 "), Some(90.0));
        assert_eq!(parse_time("12:34.5"), Some(754.5));
        assert_eq!(parse_time("1:75"), None);
        assert_eq!(parse_time("x"), None);
        assert_eq!(parse_time(""), None);
        assert_eq!(edit_time(754.4), "12:34");
        assert_eq!(edit_time(3723.0), "1:02:03");
        let e = |o: f64, t: &str| TracklistEntry { offset: o, track: Some(TrackId(1)), title: t.into(), artist: String::new(), marker: false };
        let mut l = vec![e(0.0, "a"), e(100.0, "b")];
        assert_eq!(insert_entry(&mut l, Some(0), 300.0, "n"), 1);
        assert_eq!(l[1].offset, 50.0);
        assert_eq!(insert_entry(&mut l, Some(2), 130.0, "m"), 3);
        assert_eq!(l[3].offset, 115.0, "halfway to the end of the recording");
        assert_eq!(insert_entry(&mut l, None, 300.0, "s"), 0);
        l[0].offset = 200.0;
        sort_entries(&mut l);
        assert_eq!(l.iter().map(|x| x.title.as_str()).collect::<Vec<_>>(), ["a", "n", "b", "m", "s"]);
        let (line, over) = loudness_line(&od_recorder::Loudness { integrated: Some(-9.84), true_peak: Some(0.2), clips: 3, ..Default::default() });
        assert_eq!(line, "−9.8 LUFS  ·  TRUE PEAK +0.2 dBTP  ·  3 CLIPS");
        assert!(over);
    }

    #[test]
    fn names_and_details() {
        assert_eq!(mix_name("2026-11-06T18:30:00Z"), "Mix 2026-11-06 18-30-00");
        assert_eq!(
            session_name("2026-11-07T21:30:12Z"),
            "Session 2026-11-07 21:30"
        );
        assert_eq!(
            recording_detail("2026-11-06T18:30:00Z", 3723.0),
            "2026-11-06 18:30:00  ·  1:02:03"
        );
    }
}
