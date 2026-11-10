//! Separating the parts of tracks ahead of time (Prepare, SPEC §10.3): the
//! selected tracks are queued on a background worker that writes into the
//! same stem cache the decks read, so a track loads with its parts ready.
//! Progress and the result show in the status line.

use crate::app::App;
use od_core::TrackId;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

/// A finished or running batch: (done, failed, total).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BatchCount {
    pub done: usize,
    pub failed: usize,
    pub total: usize,
}

impl BatchCount {
    pub fn finished(&self) -> bool {
        self.total > 0 && self.done + self.failed >= self.total
    }

    /// The status line for the batch (pure).
    pub fn message(&self, last_error: &str) -> String {
        if !self.finished() {
            od_i18n::trf(
                "Separating stems: {} of {} tracks …",
                &[&(self.done + self.failed + 1).min(self.total), &self.total],
            )
        } else if self.failed == 0 {
            od_i18n::trf("Stems ready for {} tracks", &[&self.done])
        } else {
            od_i18n::trf(
                "Stems ready for {} tracks, {} failed: {}",
                &[&self.done, &self.failed, &last_error],
            )
        }
    }
}

#[derive(Default)]
struct Batch {
    #[cfg(feature = "separation")]
    worker: Option<od_stems::StemWorker>,
    queued: Vec<TrackId>,
    count: BatchCount,
    last_error: String,
    /// The worker found the model damaged; stems_model.rs takes this up.
    damaged: bool,
    /// The track being separated now and how far.
    running: Option<(TrackId, f32)>,
    /// Finished (parts in the cache) and failed tracks of the batch.
    finished: HashSet<TrackId>,
    failed: HashSet<TrackId>,
    /// Finished since the last frame: decks holding them take the parts.
    fresh: Vec<TrackId>,
    /// Something changed that the STEMS column shows.
    dirty: bool,
}

/// What the library's STEMS column shows for a track.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TrackStems {
    /// Not separated (and not queued).
    None,
    Queued,
    /// Being separated, this far (0..1).
    Running(f32),
    Ready,
    Failed,
}

impl TrackStems {
    /// The row's state id and its text ("READY", "42 %").
    pub fn view(self) -> (&'static str, String) {
        match self {
            TrackStems::None => ("", String::new()),
            TrackStems::Queued => ("queued", od_i18n::tr("QUEUED").to_string()),
            TrackStems::Running(f) => ("running", format!("{:.0} %", (f.clamp(0.0, 1.0) * 100.0).floor())),
            TrackStems::Ready => ("ready", od_i18n::tr("READY").to_string()),
            TrackStems::Failed => ("failed", od_i18n::tr("FAILED").to_string()),
        }
    }
}

thread_local! {
    /// Whether the stem cache holds a track's parts, by content hash (one
    /// file check per track; finished batch jobs update it).
    static CACHED: RefCell<HashMap<String, bool>> = RefCell::new(HashMap::new());
    /// Decks play: the queue waits (roadmap [11]).
    static PAUSED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static BATCH: RefCell<Batch> = RefCell::new(Batch::default());
    static TIMER: slint::Timer = slint::Timer::default();
}

/// Whether the worker found the model damaged since the last call.
pub(crate) fn take_damaged() -> bool {
    BATCH.with(|b| std::mem::take(&mut b.borrow_mut().damaged))
}

/// Holds the batch while decks play (the parts of loaded tracks are
/// separated by the decks' own worker and do not wait).
pub(crate) fn set_paused(on: bool) {
    PAUSED.with(|p| p.set(on));
    #[cfg(feature = "separation")]
    BATCH.with(|b| {
        if let Some(w) = b.borrow().worker.as_ref() {
            w.set_paused(on);
        }
    });
}

/// The current batch count (tests, status).
pub(crate) fn count() -> BatchCount {
    BATCH.with(|b| b.borrow().count)
}

/// Progress of `track` while the batch separates it now.
pub(crate) fn running(track: TrackId) -> Option<f32> {
    BATCH.with(|b| b.borrow().running.filter(|r| r.0 == track).map(|r| r.1))
}

/// `track` waits in the batch.
pub(crate) fn is_queued(track: TrackId) -> bool {
    BATCH.with(|b| {
        let b = b.borrow();
        b.queued.contains(&track)
            && !b.finished.contains(&track)
            && !b.failed.contains(&track)
            && b.running.is_none_or(|r| r.0 != track)
    })
}

/// Whether the stem cache holds the parts of the file with `hash`.
pub(crate) fn cached(cache: &od_stems::StemCache, hash: &od_core::ContentHash) -> bool {
    CACHED.with(|c| {
        *c.borrow_mut()
            .entry(hash.0.clone())
            .or_insert_with(|| cache.exists(hash))
    })
}

/// Forgets what is known about the cache (parts removed in Preferences).
pub(crate) fn forget_cached() {
    CACHED.with(|c| c.borrow_mut().clear());
}

/// The STEMS column's state of a track.
pub(crate) fn track_state(track: TrackId, in_cache: bool) -> TrackStems {
    BATCH.with(|b| {
        let b = b.borrow();
        if let Some((t, f)) = b.running.filter(|r| r.0 == track) {
            let _ = t;
            return TrackStems::Running(f);
        }
        if in_cache || b.finished.contains(&track) {
            TrackStems::Ready
        } else if b.failed.contains(&track) {
            TrackStems::Failed
        } else if b.queued.contains(&track) {
            TrackStems::Queued
        } else {
            TrackStems::None
        }
    })
}

/// Frame tick: decks holding a track the batch just finished take its
/// parts, and the STEMS column follows the batch.
pub(crate) fn tick(app: &App) {
    let (fresh, dirty) = BATCH.with(|b| {
        let mut b = b.borrow_mut();
        (std::mem::take(&mut b.fresh), std::mem::take(&mut b.dirty))
    });
    #[cfg(feature = "separation")]
    for t in &fresh {
        if app.decks.borrow().loaded_ids().contains(t) {
            crate::decks::stems_ready(app, *t);
        }
    }
    #[cfg(not(feature = "separation"))]
    let _ = fresh;
    if dirty && crate::browser::stems_shown(&app.state.borrow()) {
        crate::browser::refresh_tracks(app);
    }
}

/// Documentation screenshots only: shows `ready`, `running` and `queued`
/// as if a batch ran (nothing is separated).
pub(crate) fn stage(ready: &[TrackId], running: Option<(TrackId, f32)>, queued: &[TrackId]) {
    BATCH.with(|b| {
        let mut b = b.borrow_mut();
        for t in ready.iter().chain(queued) {
            b.failed.remove(t);
        }
        for t in queued {
            b.finished.remove(t);
        }
        b.finished.extend(ready.iter().copied());
        b.queued.extend(ready.iter().chain(queued).copied());
        b.queued.extend(running.map(|r| r.0));
        b.running = running;
        b.dirty = true;
    });
}

/// "Separate stems for the whole list": every track of the list shown.
pub(crate) fn separate_list(app: &App) {
    if app.guard_read_only() {
        return;
    }
    let ids = app.state.borrow().list_ids();
    separate(app, &ids);
}

/// "Separate parts" for the selected tracks of the library.
pub(crate) fn separate_selected(app: &App) {
    if app.guard_read_only() {
        return;
    }
    let ids = app.state.borrow().selected_ids();
    separate(app, &ids);
}

#[cfg(not(feature = "separation"))]
pub(crate) fn separate(app: &App, _ids: &[TrackId]) {
    app.status("This build separates no stems (feature separation)");
}

/// Stops the running and queued batch (the model is about to go).
pub(crate) fn cancel_all() {
    #[cfg(feature = "separation")]
    BATCH.with(|b| {
        if let Some(w) = &b.borrow().worker {
            w.cancel_all();
        }
    });
}

/// Queues `ids` (tracks already queued are skipped).
#[cfg(feature = "separation")]
pub(crate) fn separate(app: &App, ids: &[TrackId]) {
    if ids.is_empty() {
        app.status("Select the tracks to separate first");
        return;
    }
    if app.state.borrow().config.lite {
        app.status("Stems are separated on a desktop computer; the stick brings them along");
        return;
    }
    // The model comes first; the tracks wait for it (nothing downloads
    // until the user has been asked, see stems_model.rs).
    if !crate::stems_model::request(app, ids) {
        return;
    }
    let jobs: Vec<od_stems::StemJob> = {
        let st = app.state.borrow();
        ids.iter()
            .filter_map(|id| st.lib.track(*id).ok())
            .map(|t| od_stems::StemJob {
                track: t.id,
                path: t.path.clone(),
                hash: t.content_hash.clone(),
            })
            .collect()
    };
    // Already separated: nothing to queue (the column shows READY).
    let jobs: Vec<od_stems::StemJob> = {
        let cache = app.decks.borrow().stem_cache();
        jobs.into_iter().filter(|j| !cached(&cache, &j.hash)).collect()
    };
    if jobs.is_empty() {
        app.status(od_i18n::tr("Every track here has its stems already"));
        return;
    }
    BATCH.with(|b| {
        let mut b = b.borrow_mut();
        if b.count.finished() {
            b.count = BatchCount::default();
            b.queued.clear();
            b.finished.clear();
            b.failed.clear();
        }
        b.dirty = true;
        let b = &mut *b;
        let worker = b.worker.get_or_insert_with(|| {
            let w = od_stems::StemWorker::spawn(
                od_stems::StemCache::open_default(),
                od_stems::StemWorkerOptions::default(),
            );
            w.set_paused(PAUSED.with(|p| p.get()));
            w
        });
        let mut added = Vec::new();
        for j in jobs {
            if !b.queued.contains(&j.track) {
                added.push(j.track);
                worker.enqueue(j);
            }
        }
        b.count.total += added.len();
        b.queued.extend(added);
    });
    app.status(count().message(""));
    start_polling(app);
}

#[cfg(feature = "separation")]
fn start_polling(app: &App) {
    let ui = slint::ComponentHandle::as_weak(&app.ui);
    TIMER.with(|t| {
        if t.running() {
            return;
        }
        t.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(200),
            move || {
                let Some(ui) = ui.upgrade() else { return };
                let (changed, message, finished) = poll();
                if changed {
                    ui.set_status_text(message.into());
                }
                if finished {
                    TIMER.with(|t| t.stop());
                }
            },
        );
    });
}

/// Takes the worker's events: (changed, status message, batch finished).
#[cfg(feature = "separation")]
fn poll() -> (bool, String, bool) {
    use od_stems::StemEvent;
    BATCH.with(|b| {
        let mut b = b.borrow_mut();
        let events: Vec<StemEvent> = b
            .worker
            .as_ref()
            .map(|w| w.events().try_iter().collect())
            .unwrap_or_default();
        let mut changed = false;
        for e in events {
            if !matches!(e, StemEvent::ModelDownload { .. }) {
                b.dirty = true;
            }
            match e {
                StemEvent::Done { track, hash, .. } | StemEvent::AlreadyCached { track, hash } => {
                    b.count.done += 1;
                    b.finished.insert(track);
                    b.fresh.push(track);
                    b.running = None;
                    CACHED.with(|c| c.borrow_mut().insert(hash.0.clone(), true));
                    changed = true;
                }
                StemEvent::Failed { error, track } => {
                    b.count.failed += 1;
                    b.failed.insert(track);
                    b.running = None;
                    b.last_error = error;
                    changed = true;
                }
                StemEvent::Cancelled { track } => {
                    b.count.failed += 1;
                    b.failed.insert(track);
                    b.running = None;
                    changed = true;
                }
                StemEvent::ModelDamaged { track } => {
                    b.count.failed += 1;
                    b.failed.insert(track);
                    b.running = None;
                    b.last_error = od_i18n::tr("The stem model is damaged").to_string();
                    b.damaged = true;
                    changed = true;
                }
                StemEvent::Started { track } => b.running = Some((track, 0.0)),
                StemEvent::Progress { track, fraction } => b.running = Some((track, fraction)),
                StemEvent::ModelDownload { .. } => {}
            }
        }
        let msg = b.count.message(&b.last_error);
        (changed, msg, b.count.finished())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_messages() {
        let mut c = BatchCount {
            done: 0,
            failed: 0,
            total: 3,
        };
        assert!(!c.finished());
        assert!(c.message("").contains('1') && c.message("").contains('3'));
        c.done = 2;
        c.failed = 1;
        assert!(c.finished());
        assert!(c.message("model missing").contains("model missing"));
        c.failed = 0;
        c.done = 3;
        assert!(c.message("").contains('3'));
    }

    #[test]
    fn column_states() {
        assert_eq!(TrackStems::Ready.view().0, "ready");
        assert_eq!(TrackStems::Running(0.426).view().1, "42 %");
        assert_eq!(TrackStems::None.view(), ("", String::new()));
        assert_eq!(TrackStems::Queued.view().0, "queued");
        assert_eq!(TrackStems::Failed.view().0, "failed");
        // Nothing queued: only the cache decides.
        assert_eq!(track_state(TrackId(987_654), true), TrackStems::Ready);
        assert_eq!(track_state(TrackId(987_654), false), TrackStems::None);
    }
}
