//! Transients of recently loaded tracks, for the grid editors (evidence on
//! the waveform, snapping, FIT, AUTO-FIX, the Perform meter).
//!
//! They are computed on a worker from the samples a deck or the preview
//! already decoded (or, for a track not loaded anywhere, from a fresh
//! decode) and kept in memory for the last four tracks, by the audio's
//! content hash (the same audio has the same transients); nothing is
//! stored.

use od_analysis::transients::{Transient, transients};
use od_core::{ContentHash, TrackId};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Tracks kept.
const KEEP: usize = 4;

static CACHE: Mutex<VecDeque<(ContentHash, Arc<Vec<Transient>>)>> = Mutex::new(VecDeque::new());
/// Tracks being computed (no second job for the same track).
static PENDING: Mutex<Vec<ContentHash>> = Mutex::new(Vec::new());
/// Bumped whenever a track's transients arrive (the UI redraws).
static VERSION: AtomicU64 = AtomicU64::new(0);

/// The transients of the audio with hash `hash`, when computed.
pub(crate) fn get(hash: &ContentHash) -> Option<Arc<Vec<Transient>>> {
    let c = CACHE.lock().ok()?;
    c.iter().find(|(t, _)| t == hash).map(|(_, v)| v.clone())
}

/// The transients of library track `id` (looks up its hash).
pub(crate) fn of_track(app: &crate::app::App, id: TrackId) -> Option<Arc<Vec<Transient>>> {
    let hash = app.state.borrow().src().track(id).ok()?.content_hash;
    get(&hash)
}

/// Changes whenever transients arrive.
pub(crate) fn version() -> u64 {
    VERSION.load(Ordering::Relaxed)
}

pub(crate) fn insert(hash: ContentHash, list: Vec<Transient>) {
    if let Ok(mut p) = PENDING.lock() {
        p.retain(|t| *t != hash);
    }
    if let Ok(mut c) = CACHE.lock() {
        c.retain(|(t, _)| *t != hash);
        c.push_back((hash, Arc::new(list)));
        while c.len() > KEEP {
            c.pop_front();
        }
    }
    VERSION.fetch_add(1, Ordering::Relaxed);
}

/// Claims the job for `hash`; false when it is done or running.
fn claim(hash: &ContentHash) -> bool {
    if get(hash).is_some() {
        return false;
    }
    let Ok(mut p) = PENDING.lock() else { return false };
    if p.contains(hash) {
        return false;
    }
    p.push(hash.clone());
    true
}

/// Mono mixdown of interleaved stereo.
fn mono(stereo: &[f32]) -> Vec<f32> {
    stereo.chunks_exact(2).map(|f| (f[0] + f[1]) * 0.5).collect()
}

/// From a deck's decoded track (call on the loading thread).
pub(crate) fn from_track_data(hash: &ContentHash, data: &od_engine::TrackData) {
    if data.samples.is_empty() || !claim(hash) {
        return;
    }
    let list = transients(&mono(&data.samples), data.sample_rate);
    insert(hash.clone(), list);
}

/// Makes sure the transients of a track that no deck has decoded get
/// computed: decodes the file on a worker.
pub(crate) fn ensure(hash: ContentHash, path: PathBuf) {
    if !claim(&hash) {
        return;
    }
    std::thread::spawn(move || match od_decode::decode(&path, od_decode::Layout::Stereo) {
        Ok(audio) => {
            let list = transients(&mono(&audio.samples), audio.sample_rate);
            insert(hash, list);
        }
        Err(e) => {
            tracing::debug!(error = %e, "transients: decode failed");
            if let Ok(mut p) = PENDING.lock() {
                p.retain(|t| *t != hash);
            }
        }
    });
}
