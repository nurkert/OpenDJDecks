//! Restore after a crash (roadmap [9]).
//!
//! While the app runs, the deck state (tracks, positions, tempo, loops,
//! the mixer and a running recording) is written to `session.json` next to
//! the library once a second, atomically (a temporary file, synced, then
//! renamed over the old one), on a background thread. A clean quit marks it
//! clean. When the app starts and finds it not clean, it keeps it as
//! `session-crashed.json` and offers it back in a band under the top bar;
//! a recording that was running is made playable at once
//! ([`od_recorder::repair_wav`]) and added to the recordings with its
//! tracklist, whatever the DJ chooses.

use crate::app::{App, handler};
use crate::decks::{self, DECKS, PREVIEW_DECK};
use od_core::TrackId;
use od_engine::{ChannelCommand, Command, DeckCommand, EngineState, MixerCommand, TempoRange, XfAssign};
use serde::{Deserialize, Serialize};
use slint::ComponentHandle;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Held while the session file is written; after a clean quit the saver
/// writes nothing more (a state queued just before must not undo it).
static WRITE: std::sync::Mutex<bool> = std::sync::Mutex::new(false);

/// How often the state is written while something is loaded.
const SAVE_EVERY: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct DeckSnap {
    pub deck: usize,
    pub track: i64,
    pub title: String,
    pub position: f64,
    pub duration: f64,
    pub playing: bool,
    pub tempo_fader: f32,
    pub tempo_range: String,
    pub keylock: bool,
    pub key_shift: f32,
    pub sync: bool,
    pub loop_start: Option<f64>,
    pub loop_end: Option<f64>,
    pub loop_active: bool,
    pub hot_cue_bank: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct ChannelSnap {
    pub fader: f32,
    pub trim_db: f32,
    pub eq: [f32; 3],
    pub xf: String,
    pub cue: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct RecordingSnap {
    pub path: PathBuf,
    /// Unix seconds when it started.
    pub started: u64,
    pub tracklist: Vec<od_recorder::TracklistEntry>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Snapshot {
    /// Unix seconds.
    pub saved: u64,
    /// Written by a clean quit: nothing to offer.
    pub clean: bool,
    pub deck_count: usize,
    pub decks: Vec<DeckSnap>,
    pub crossfader: f32,
    pub master_level: f32,
    pub channels: Vec<ChannelSnap>,
    pub recording: Option<RecordingSnap>,
}

impl Snapshot {
    /// Something worth offering back.
    pub fn worth_restoring(&self) -> bool {
        !self.clean && (!self.decks.is_empty() || self.recording.is_some())
    }
}

/// `session.json` beside the library.
pub(crate) fn session_path(library: &Path) -> PathBuf {
    library.with_file_name("session.json")
}

fn crashed_path(library: &Path) -> PathBuf {
    library.with_file_name("session-crashed.json")
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Writes `bytes` to `path` so that a crash at any moment leaves either the
/// old or the new content: a temporary file in the same folder, synced to
/// the disk, renamed over `path`.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    // The rename itself reaches the disk with the folder (best effort; not
    // possible on Windows).
    #[cfg(unix)]
    if let Some(dir) = path.parent() {
        if let Ok(d) = std::fs::File::open(dir) {
            let _ = d.sync_all();
        }
    }
    Ok(())
}

/// Reads a snapshot; a missing or damaged file is `None`.
pub(crate) fn read(path: &Path) -> Option<Snapshot> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

fn range_id(r: TempoRange) -> &'static str {
    crate::engine::tempo_range_id(r)
}

fn range_of(id: &str) -> TempoRange {
    match id {
        "six" => TempoRange::Six,
        "sixteen" => TempoRange::Sixteen,
        "wide" => TempoRange::Wide,
        _ => TempoRange::Ten,
    }
}

fn xf_id(x: XfAssign) -> &'static str {
    match x {
        XfAssign::A => "a",
        XfAssign::B => "b",
        XfAssign::Thru => "thru",
    }
}

fn xf_of(id: &str) -> XfAssign {
    match id {
        "a" => XfAssign::A,
        "b" => XfAssign::B,
        _ => XfAssign::Thru,
    }
}

/// The decks shown in Perform (the preview deck only with four decks).
fn performance_decks(count: usize) -> impl Iterator<Item = usize> {
    (0..count.min(DECKS)).filter(move |d| *d != PREVIEW_DECK || count > PREVIEW_DECK)
}

/// The state worth keeping, from the engine and the loaded tracks.
fn capture(app: &App, state: &EngineState) -> Snapshot {
    let d = app.decks.borrow();
    let decks = performance_decks(d.deck_count)
        .filter_map(|i| {
            let meta = d.loaded(i)?;
            let s = &state.decks[i];
            Some(DeckSnap {
                deck: i,
                track: meta.id.0,
                title: meta.title.clone(),
                position: s.position,
                duration: s.duration,
                playing: s.playing,
                tempo_fader: s.tempo_fader,
                tempo_range: range_id(s.tempo_range).into(),
                keylock: s.keylock,
                key_shift: s.key_shift,
                sync: s.sync,
                loop_start: s.loop_state.start,
                loop_end: s.loop_state.end,
                loop_active: s.loop_state.active,
                hot_cue_bank: s.hot_cue_bank,
            })
        })
        .collect();
    let channels = (0..d.deck_count.min(DECKS))
        .map(|i| {
            let c = &state.channels[i];
            ChannelSnap {
                fader: c.fader,
                trim_db: c.trim_db,
                eq: c.eq,
                xf: xf_id(c.xf_assign).into(),
                cue: c.cue,
            }
        })
        .collect();
    let deck_count = d.deck_count;
    drop(d);
    let recording = {
        let r = app.recorder.borrow();
        r.recording().map(|rec| RecordingSnap {
            path: rec.path().map(Path::to_path_buf).unwrap_or_default(),
            started: now_unix().saturating_sub(rec.elapsed() as u64),
            tracklist: rec.tracklist().to_vec(),
        })
    };
    Snapshot {
        saved: now_unix(),
        clean: false,
        deck_count,
        decks,
        crossfader: state.crossfader,
        master_level: state.master_level,
        channels,
        recording,
    }
}

struct Saver {
    tx: crossbeam_channel::Sender<(PathBuf, Vec<u8>)>,
    last: Option<Instant>,
    /// The last snapshot written (without its time).
    written: Option<Snapshot>,
}

/// What the start found: the snapshot offered back.
#[derive(Default)]
struct Offer {
    snapshot: Option<Snapshot>,
}

thread_local! {
    static SAVER: RefCell<Option<Saver>> = const { RefCell::new(None) };
    static OFFER: RefCell<Offer> = RefCell::new(Offer::default());
}

fn saver() -> crossbeam_channel::Sender<(PathBuf, Vec<u8>)> {
    SAVER.with(|s| {
        let mut s = s.borrow_mut();
        if s.is_none() {
            let (tx, rx) = crossbeam_channel::unbounded::<(PathBuf, Vec<u8>)>();
            let _ = std::thread::Builder::new()
                .name("od-session-save".into())
                .spawn(move || {
                    while let Ok(mut job) = rx.recv() {
                        // Only the newest state matters.
                        while let Ok(newer) = rx.try_recv() {
                            job = newer;
                        }
                        let closed = WRITE.lock().unwrap_or_else(|e| e.into_inner());
                        if *closed {
                            continue;
                        }
                        if let Err(e) = write_atomic(&job.0, &job.1) {
                            tracing::warn!(error = %e, "session state not saved");
                        }
                        drop(closed);
                    }
                });
            *s = Some(Saver {
                tx,
                last: None,
                written: None,
            });
        }
        s.as_ref().unwrap().tx.clone()
    })
}

/// Frame tick: writes the state once a second when it changed (always while
/// a deck plays, the positions move).
pub(crate) fn frame(app: &App, state: &EngineState) {
    let tx = saver();
    let due = SAVER.with(|s| {
        s.borrow()
            .as_ref()
            .is_some_and(|s| s.last.is_none_or(|t| t.elapsed() >= SAVE_EVERY))
    });
    if !due {
        return;
    }
    let mut snap = capture(app, state);
    let unchanged = SAVER.with(|s| {
        let mut s = s.borrow_mut();
        let s = s.as_mut().unwrap();
        s.last = Some(Instant::now());
        let mut cmp = snap.clone();
        cmp.saved = 0;
        if s.written.as_ref() == Some(&cmp) {
            true
        } else {
            s.written = Some(cmp);
            false
        }
    });
    if unchanged {
        return;
    }
    snap.saved = now_unix();
    let path = session_path(&app.state.borrow().config.library_path);
    if let Ok(bytes) = serde_json::to_vec_pretty(&snap) {
        let _ = tx.send((path, bytes));
    }
}

/// A clean quit: nothing to offer next time.
pub(crate) fn clean_exit(app: &App) {
    let path = session_path(&app.state.borrow().config.library_path);
    let snap = Snapshot {
        saved: now_unix(),
        clean: true,
        ..Snapshot::default()
    };
    let mut closed = WRITE.lock().unwrap_or_else(|e| e.into_inner());
    *closed = true;
    if let Ok(bytes) = serde_json::to_vec_pretty(&snap) {
        let _ = write_atomic(&path, &bytes);
    }
}

/// Keeps the recording of a crashed run: repairs the file, writes its
/// tracklist and lists it under Recordings. Returns (name, seconds).
fn keep_recording(app: &App, rec: &RecordingSnap) -> Option<(String, f64)> {
    if !rec.path.exists() {
        return None;
    }
    let secs = match od_recorder::repair_wav(&rec.path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, path = %rec.path.display(), "recording not repaired");
            return None;
        }
    };
    let tracklist: Vec<od_recorder::TracklistEntry> = rec
        .tracklist
        .iter()
        .filter(|e| e.track.is_some())
        .cloned()
        .collect();
    let _ = std::fs::write(
        rec.path.with_extension("cue"),
        od_recorder::cue_sheet(&rec.path, &tracklist),
    );
    let _ = std::fs::write(
        rec.path.with_extension("txt"),
        od_recorder::text_tracklist(&tracklist),
    );
    let path = rec.path.display().to_string();
    {
        let st = app.state.borrow();
        let known: bool = st
            .lib
            .conn()
            .query_row("SELECT COUNT(*) > 0 FROM recording WHERE path = ?1", [&path], |r| r.get(0))
            .unwrap_or(false);
        if !known {
            let _ = st.lib.conn().execute(
                "INSERT INTO recording (path, started, duration, tracklist) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%SZ', ?2, 'unixepoch'), ?3, ?4)",
                (
                    &path,
                    rec.started as i64,
                    secs,
                    serde_json::to_string(&tracklist).unwrap_or_default(),
                ),
            );
        }
    }
    let name = rec
        .path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    Some((name, secs))
}

/// "1:15".
fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// "Deck 1: Night Drive at 1:15 · Deck 2: Sunday Static at 0:43".
pub(crate) fn summary(snap: &Snapshot) -> String {
    snap.decks
        .iter()
        .map(|d| {
            od_i18n::trf(
                "Deck {}: {} at {}",
                &[&(d.deck + 1), &d.title, &clock(d.position)],
            )
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Start-up: a run that did not end cleanly is offered back.
fn check_on_start(app: &App) {
    let library = app.state.borrow().config.library_path.clone();
    let (current, crashed) = (session_path(&library), crashed_path(&library));
    if let Some(s) = read(&current).filter(Snapshot::worth_restoring) {
        // Keep it apart, so this run's saving cannot overwrite it.
        if std::fs::rename(&current, &crashed).is_err() {
            return;
        }
        tracing::warn!("the last session did not end cleanly; offering it back");
        let _ = s;
    }
    let Some(mut snap) = read(&crashed) else { return };
    let mut note = String::new();
    if let Some(rec) = snap.recording.take() {
        if let Some((name, secs)) = keep_recording(app, &rec) {
            note = od_i18n::trf("The recording {} was kept: {}", &[&name, &clock(secs)]);
        }
        // Repaired once: the file no longer names it.
        if let Ok(bytes) = serde_json::to_vec_pretty(&snap) {
            let _ = write_atomic(&crashed, &bytes);
        }
    }
    let when: String = app
        .state
        .borrow()
        .lib
        .conn()
        .query_row(
            "SELECT strftime('%H:%M', ?1, 'unixepoch', 'localtime')",
            [snap.saved as i64],
            |r| r.get(0),
        )
        .unwrap_or_default();
    let g = app.ui.global::<crate::Restore>();
    g.set_when(when.into());
    g.set_summary(summary(&snap).into());
    g.set_recording(note.into());
    g.set_has_decks(!snap.decks.is_empty());
    g.set_visible(true);
    OFFER.with(|o| o.borrow_mut().snapshot = Some(snap));
}

/// RESTORE: loads the tracks back where they were and sets the mixer; the
/// decks wait for PLAY.
pub(crate) fn restore(app: &App) {
    let Some(snap) = OFFER.with(|o| o.borrow_mut().snapshot.take()) else {
        close(app);
        return;
    };
    if snap.deck_count != app.decks.borrow().deck_count && snap.deck_count >= 2 {
        decks::set_deck_count(app, snap.deck_count);
    }
    if !crate::engine::ensure_engine(app) {
        crate::engine::show_message(app);
    }
    let mut restored = Vec::new();
    for d in &snap.decks {
        let id = TrackId(d.track);
        if d.deck >= DECKS || app.state.borrow().lib.track(id).is_err() {
            continue;
        }
        app.decks.borrow_mut().restore[d.deck] = Some(d.clone());
        decks::load_track(app, d.deck, id, None);
        restored.push((d.deck + 1).to_string());
    }
    for (i, c) in snap.channels.iter().enumerate().take(DECKS) {
        let ch = |cmd| crate::engine::send(app, Command::Channel(i, cmd));
        ch(ChannelCommand::SetFader(c.fader));
        ch(ChannelCommand::SetTrim(c.trim_db));
        for (band, v) in c.eq.iter().enumerate() {
            ch(ChannelCommand::SetEq { band, value: *v });
        }
        ch(ChannelCommand::SetCrossfaderAssign(xf_of(&c.xf)));
        ch(ChannelCommand::SetCue(c.cue));
    }
    crate::engine::send(app, Command::Mixer(MixerCommand::SetCrossfader(snap.crossfader)));
    crate::engine::send(app, Command::Mixer(MixerCommand::SetMasterLevel(snap.master_level)));
    app.ui.set_mode("perform".into());
    app.status(od_i18n::trf(
        "Restored deck {}: press PLAY to go on",
        &[&restored.join(", ")],
    ));
    close(app);
}

/// Applies the rest of a restored deck once its track is loaded (called
/// from `decks::receive_prepared`).
pub(crate) fn apply_deck(app: &App, deck: usize, snap: &DeckSnap) {
    let cmd = |c| crate::engine::deck_cmd(app, deck, c);
    cmd(DeckCommand::SetTempoRange(range_of(&snap.tempo_range)));
    cmd(DeckCommand::SetTempoFader(snap.tempo_fader));
    cmd(DeckCommand::SetKeylock(snap.keylock));
    cmd(DeckCommand::SetKeyShift(snap.key_shift));
    cmd(DeckCommand::SetHotCueBank(snap.hot_cue_bank));
    cmd(DeckCommand::Seek {
        seconds: snap.position,
    });
    if let (Some(start), Some(end)) = (snap.loop_start, snap.loop_end) {
        cmd(DeckCommand::SetLoop {
            start,
            end,
            active: snap.loop_active,
        });
    }
    if snap.sync {
        cmd(DeckCommand::SetSync(true));
    }
}

/// START EMPTY or after RESTORE: the offer goes.
fn close(app: &App) {
    let library = app.state.borrow().config.library_path.clone();
    let _ = std::fs::remove_file(crashed_path(&library));
    OFFER.with(|o| o.borrow_mut().snapshot = None);
    app.ui.global::<crate::Restore>().set_visible(false);
    crate::recorder::refresh_list(app);
}

pub(crate) fn wire(app: &Rc<App>) {
    let g = app.ui.global::<crate::Restore>();
    g.on_restore(handler!(app, |a| restore(&a)));
    g.on_dismiss(handler!(app, |a| close(&a)));
    check_on_start(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_the_whole_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("session.json");
        write_atomic(&p, b"{\"old\": true}").unwrap();
        write_atomic(&p, b"{}").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"{}");
        assert!(!p.with_extension("json.tmp").exists(), "no temporary file left");
    }

    #[test]
    fn a_clean_or_empty_session_is_not_offered() {
        let mut s = Snapshot::default();
        assert!(!s.worth_restoring());
        s.decks.push(DeckSnap {
            deck: 0,
            track: 1,
            title: "Night Drive".into(),
            position: 75.4,
            ..DeckSnap::default()
        });
        assert!(s.worth_restoring());
        assert_eq!(summary(&s), "Deck 1: Night Drive at 1:15");
        s.clean = true;
        assert!(!s.worth_restoring());
        // A damaged file reads as nothing.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("session.json");
        std::fs::write(&p, b"{\"decks\": [").unwrap();
        assert!(read(&p).is_none());
    }
}
