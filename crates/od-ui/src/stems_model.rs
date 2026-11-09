//! The stem separation model as the user meets it (SPEC §10.3): asked for
//! once, downloaded in the background, repaired from Preferences.
//!
//! Nothing downloads without the user asking. A part key, the STEM switch
//! or batch separation asks through [`request`]: a dialog when the room is
//! quiet, a slim notice while music plays or a mix records (the dialog then
//! follows once playback has stopped), nothing at all in the kiosk or on
//! builds that cannot separate. The work happens in [`od_stems::ModelStore`]
//! on a thread; this module projects its state into the notice bar, the
//! dialog (`ui/stems_model.slint`) and Preferences › Analysis › Stems.

use crate::app::{App, handler};
use crossbeam_channel::{Receiver, Sender, unbounded};
use od_core::TrackId;
use od_i18n::Language;
use od_stems::{FetchFailure, ModelStatus, ModelStore};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "separation")]
use std::time::{Duration, Instant};

/// The wiki section the HELP action opens; stable, checked against the
/// guides in the tests below.
pub(crate) const HELP_ANCHOR_EN: &str = "the-download-stopped-and-now-stems-do-not-work";
pub(crate) const HELP_ANCHOR_DE: &str =
    "der-download-ist-abgebrochen-und-stems-funktionieren-nicht";

/// A few progress messages per second are plenty.
#[cfg(feature = "separation")]
const PROGRESS_INTERVAL: Duration = Duration::from_millis(150);

/// What the download machinery is doing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Phase {
    #[default]
    Idle,
    /// Bytes done, bytes total.
    Downloading(u64, u64),
    /// The last download failed; the notice offers RETRY and HELP.
    Failed(FetchFailure),
    /// Separation found the model damaged; RETRY replaces it.
    Damaged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(feature = "separation"), allow(dead_code))]
enum Ended {
    Installed,
    Cancelled,
    Failed(FetchFailure),
}

#[cfg_attr(not(feature = "separation"), allow(dead_code))]
enum Msg {
    Progress(u64, u64),
    Ended(Ended),
    Verified(ModelStatus),
}

pub(crate) struct StemsModel {
    store: Arc<ModelStore>,
    pub phase: Phase,
    /// NOT NOW was chosen: no dialog for the rest of the session.
    pub declined: bool,
    /// The notice offering the download is up.
    pub notice: bool,
    /// The dialog is open.
    pub prompt_open: bool,
    /// Asked while music played: offer the dialog once it has stopped.
    pub offer_later: bool,
    /// A check found the file damaged although its size and mark looked fine.
    known_bad: bool,
    /// Tracks of a batch waiting for the model.
    wanted: Vec<TrackId>,
    /// Target of the last HELP (headless runs open nothing).
    pub help_opened: Option<String>,
    #[cfg_attr(not(feature = "separation"), allow(dead_code))]
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    cancel: Arc<AtomicBool>,
}

impl Default for StemsModel {
    fn default() -> Self {
        let (tx, rx) = unbounded();
        Self {
            store: Arc::new(ModelStore::open_default()),
            phase: Phase::Idle,
            declined: false,
            notice: false,
            prompt_open: false,
            offer_later: false,
            known_bad: false,
            wanted: Vec::new(),
            help_opened: None,
            tx,
            rx,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl StemsModel {
    /// What is on disk, corrected by what a later check found.
    fn status(&self) -> ModelStatus {
        let status = self.store.status();
        if self.known_bad && status.usable() {
            ModelStatus::Damaged {
                bytes: self.store.info().size,
            }
        } else {
            status
        }
    }

    fn model_size(&self) -> u64 {
        self.store.info().size
    }
}

/// `n` bytes in whole megabytes, as the app states file sizes.
pub(crate) fn mb(n: u64) -> u64 {
    (n + (1 << 19)) >> 20
}

/// One short, human sentence for a failed download.
pub(crate) fn failure_text(f: FetchFailure) -> String {
    match f {
        FetchFailure::NoNetwork => {
            od_i18n::tr("No internet connection. Check your network and try again.").to_string()
        }
        FetchFailure::Server(_) => {
            od_i18n::tr("The download server did not answer properly. Try again in a few minutes.")
                .to_string()
        }
        FetchFailure::DiskFull { needed } => od_i18n::trf(
            "There is not enough free disk space. The stem model needs {} MB free.",
            &[&mb(needed)],
        ),
        FetchFailure::Checksum => {
            od_i18n::tr("The download was damaged on the way and was thrown away. Please try again.")
                .to_string()
        }
        FetchFailure::Interrupted => {
            od_i18n::tr("The download stopped before it was finished. It continues where it left off.")
                .to_string()
        }
        FetchFailure::Storage => {
            od_i18n::tr("The stem model could not be saved on this computer. Check that the disk is writable and try again.")
                .to_string()
        }
    }
}

fn download_text(done: u64, total: u64) -> String {
    let pct = if total > 0 { done * 100 / total } else { 0 };
    od_i18n::trf(
        "Downloading the stem model — {} % ({} of {} MB)",
        &[&pct, &mb(done), &mb(total)],
    )
}

/// The notice bar: state ("" hides it), text, progress (pure).
pub(crate) fn bar(phase: Phase, notice: bool, model_mb: u64) -> (&'static str, String, f32) {
    match phase {
        Phase::Downloading(done, total) => {
            let p = if total > 0 {
                done as f32 / total as f32
            } else {
                0.0
            };
            ("downloading", download_text(done, total), p.clamp(0.0, 1.0))
        }
        Phase::Failed(f) => ("error", failure_text(f), 0.0),
        Phase::Damaged => (
            "error",
            od_i18n::tr("The stem model is damaged. RETRY deletes it and downloads it again.")
                .to_string(),
            0.0,
        ),
        Phase::Idle if notice => (
            "offer",
            od_i18n::trf(
                "Stems need a one-time download of {} MB. Your music stays on this computer.",
                &[&model_mb],
            ),
            0.0,
        ),
        Phase::Idle => ("", String::new(), 0.0),
    }
}

/// How to put the question (pure).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Offer {
    /// The room is quiet: the dialog.
    Dialog,
    /// No modal dialog over playing music, a recording, or after NOT NOW.
    Notice,
}

pub(crate) fn offer_kind(declined: bool, playing: bool, recording: bool) -> Offer {
    if declined || playing || recording {
        Offer::Notice
    } else {
        Offer::Dialog
    }
}

/// The title of the dialog for what is on disk (pure).
pub(crate) fn prompt_title(status: ModelStatus) -> String {
    match status {
        ModelStatus::Partial { .. } => od_i18n::tr("The stem model is incomplete").to_string(),
        ModelStatus::Damaged { .. } => od_i18n::tr("The stem model is damaged").to_string(),
        _ => od_i18n::tr("Stems need a one-time download").to_string(),
    }
}

/// Preferences › Analysis › Stems: the state for its buttons and its line.
pub(crate) struct PrefsView {
    pub state: &'static str,
    pub text: String,
    /// Offer HELP next to the status (a failure or a damaged model).
    pub help: bool,
}

/// What Preferences shows (pure).
pub(crate) fn prefs_view(
    status: ModelStatus,
    phase: Phase,
    on_disk: u64,
    model_mb: u64,
    supported: bool,
) -> PrefsView {
    if !supported {
        return PrefsView {
            state: "unsupported",
            text: od_i18n::tr("Stems cannot be separated with this build. Stems separated on another computer still play.")
                .to_string(),
            help: false,
        };
    }
    match phase {
        Phase::Downloading(done, total) => PrefsView {
            state: "downloading",
            text: download_text(done, total),
            help: false,
        },
        Phase::Failed(f) => PrefsView {
            state: match status {
                ModelStatus::Partial { .. } => "partial",
                ModelStatus::Damaged { .. } => "damaged",
                _ => "missing",
            },
            text: failure_text(f),
            help: true,
        },
        Phase::Damaged => PrefsView {
            state: "damaged",
            text: od_i18n::trf("Damaged, {} MB on disk", &[&mb(on_disk)]),
            help: true,
        },
        Phase::Idle => match status {
            ModelStatus::Missing => PrefsView {
                state: "missing",
                text: od_i18n::trf("Not installed. The download is {} MB.", &[&model_mb]),
                help: false,
            },
            ModelStatus::Partial { bytes } => PrefsView {
                state: "partial",
                text: od_i18n::trf("Incomplete, {} of {} MB", &[&mb(bytes), &model_mb]),
                help: true,
            },
            ModelStatus::Unverified | ModelStatus::Installed => PrefsView {
                state: "installed",
                text: od_i18n::trf("Installed, {} MB on disk", &[&mb(on_disk)]),
                help: false,
            },
            ModelStatus::Damaged { .. } => PrefsView {
                state: "damaged",
                text: od_i18n::trf("Damaged, {} MB on disk", &[&mb(on_disk)]),
                help: true,
            },
        },
    }
}

/// The wiki section behind HELP (pure).
pub(crate) fn help_url(lang: Language) -> String {
    match lang {
        Language::German => format!("https://opendjdecks.de/wiki/stems/#{HELP_ANCHOR_DE}"),
        Language::English => format!("https://opendjdecks.de/en/wiki/stems/#{HELP_ANCHOR_EN}"),
    }
}

/// Whether separation can run right now.
pub(crate) fn ready(app: &App) -> bool {
    app.stems_model.borrow().status().usable()
}

/// Where the user wants parts that need the model: `tracks` wait for it
/// when it is missing. Returns whether the model can be used right away.
pub(crate) fn request(app: &App, tracks: &[TrackId]) -> bool {
    let lite = app.state.borrow().config.lite;
    if lite || !od_stems::separation_supported() {
        app.status("Parts are separated on a desktop computer; the stick brings them along");
        return false;
    }
    if ready(app) {
        return true;
    }
    // Nothing the kiosk screen would have to answer: the separation stays
    // on the desktop, stems arrive on the stick.
    if app.state.borrow().config.kiosk {
        app.status("Parts are separated on a desktop computer; the stick brings them along");
        return false;
    }
    let phase = {
        let mut m = app.stems_model.borrow_mut();
        for t in tracks {
            if !m.wanted.contains(t) {
                m.wanted.push(*t);
            }
        }
        m.phase
    };
    match phase {
        Phase::Downloading(..) => app.status("The stem model is still downloading"),
        // The notice already says what happened.
        Phase::Failed(_) | Phase::Damaged => refresh(app),
        Phase::Idle => {
            let offer = offer_kind(
                app.stems_model.borrow().declined,
                playing(app),
                app.recorder.borrow().is_recording(),
            );
            {
                let mut m = app.stems_model.borrow_mut();
                match offer {
                    Offer::Dialog => {
                        m.prompt_open = true;
                        m.notice = false;
                    }
                    Offer::Notice => {
                        m.notice = true;
                        m.offer_later = !m.declined;
                    }
                }
            }
            refresh(app);
        }
    }
    false
}

/// A part key or the STEM switch of `deck` was pressed without parts.
pub(crate) fn part_pressed(app: &App, deck: usize) {
    if app.decks.borrow().stems_pending(deck) {
        return;
    }
    if request(app, &[]) {
        crate::decks::request_loaded_stems(app, Some(deck));
    }
}

/// Separation found the model damaged (from the workers' events).
pub(crate) fn damaged(app: &App) {
    {
        let mut m = app.stems_model.borrow_mut();
        m.known_bad = true;
        if m.phase == Phase::Idle {
            m.phase = Phase::Damaged;
        }
    }
    refresh(app);
}

fn playing(app: &App) -> bool {
    crate::engine::snapshot(app).is_some_and(|s| s.decks.iter().any(|d| d.playing))
}

/// Pushes the state into the notice, the dialog and Preferences.
pub(crate) fn refresh(app: &App) {
    let (state, text, progress, prompt, title) = {
        let m = app.stems_model.borrow();
        let (state, text, progress) = bar(m.phase, m.notice, mb(m.model_size()));
        (
            state,
            text,
            progress,
            m.prompt_open,
            prompt_title(m.status()),
        )
    };
    app.ui.set_stems_model_state(state.into());
    app.ui.set_stems_model_text(text.into());
    app.ui.set_stems_model_progress(progress);
    app.ui.set_stems_model_prompt(prompt);
    app.ui.set_stems_model_prompt_title(title.into());
    app.ui
        .set_stems_model_size(mb(app.stems_model.borrow().model_size()) as i32);
    crate::preferences::refresh_stems(app);
}

/// Preferences › Analysis › Stems for the current state.
pub(crate) fn current_prefs_view(app: &App) -> PrefsView {
    let m = app.stems_model.borrow();
    let supported = !app.state.borrow().config.lite && od_stems::separation_supported();
    prefs_view(
        m.status(),
        m.phase,
        m.store.size_on_disk(),
        mb(m.model_size()),
        supported,
    )
}

/// Starts the download; `fresh` removes every leftover first.
#[cfg(feature = "separation")]
fn start(app: &App, fresh: bool) {
    if app.state.borrow().config.lite || !od_stems::separation_supported() {
        return;
    }
    if matches!(app.stems_model.borrow().phase, Phase::Downloading(..)) {
        return;
    }
    if fresh {
        // Stop anything still using the model before it goes.
        stop_jobs(app);
    }
    let (store, tx, cancel) = {
        let mut m = app.stems_model.borrow_mut();
        m.cancel.store(false, Ordering::Relaxed);
        m.notice = false;
        m.prompt_open = false;
        m.offer_later = false;
        m.known_bad = false;
        m.phase = Phase::Downloading(0, m.model_size());
        (m.store.clone(), m.tx.clone(), m.cancel.clone())
    };
    refresh(app);
    let spawned = std::thread::Builder::new()
        .name("stem-model".into())
        .spawn(move || {
            if fresh && let Err(e) = store.remove_all() {
                tracing::warn!(error = %e, "could not remove the stem model");
                let _ = tx.send(Msg::Ended(Ended::Failed(FetchFailure::Storage)));
                return;
            }
            let ptx = tx.clone();
            let mut last = Instant::now() - PROGRESS_INTERVAL;
            let result = store.fetch(
                &mut |done, total| {
                    if last.elapsed() >= PROGRESS_INTERVAL || done >= total {
                        last = Instant::now();
                        let _ = ptx.send(Msg::Progress(done, total));
                    }
                },
                &cancel,
            );
            let ended = match result {
                Ok(_) => Ended::Installed,
                Err(od_stems::Error::Cancelled) => Ended::Cancelled,
                Err(od_stems::Error::Fetch(f)) => Ended::Failed(f),
                Err(e) => {
                    tracing::warn!(error = %e, "stem model download failed");
                    Ended::Failed(FetchFailure::Storage)
                }
            };
            let _ = tx.send(Msg::Ended(ended));
        });
    if spawned.is_err() {
        app.stems_model.borrow_mut().phase = Phase::Failed(FetchFailure::Storage);
        refresh(app);
    }
}

#[cfg(not(feature = "separation"))]
fn start(_app: &App, _fresh: bool) {}

/// Stops separation jobs so the model file can go.
fn stop_jobs(app: &App) {
    crate::decks::cancel_stem_jobs(app);
    crate::stems_batch::cancel_all();
}

fn cancel(app: &App) {
    app.stems_model
        .borrow()
        .cancel
        .store(true, Ordering::Relaxed);
}

/// Deletes the model and every leftover; stems already separated stay.
fn remove(app: &App) {
    if matches!(app.stems_model.borrow().phase, Phase::Downloading(..)) {
        return;
    }
    stop_jobs(app);
    let result = app.stems_model.borrow().store.remove_all();
    {
        let mut m = app.stems_model.borrow_mut();
        m.known_bad = false;
        m.phase = Phase::Idle;
        m.notice = false;
    }
    match result {
        Ok(()) => app.status("Stem model removed. Stems you already separated keep working."),
        Err(e) => {
            tracing::warn!(error = %e, "could not remove the stem model");
            app.status("The stem model could not be removed. Is it in use by another program?");
        }
    }
    refresh(app);
}

fn help(app: &App) {
    let url = help_url(od_i18n::language());
    if !app.state.borrow().config.headless
        && let Err(e) = od_update::install::open(url.as_ref())
    {
        tracing::warn!(error = %e, "could not open the wiki page");
    }
    app.stems_model.borrow_mut().help_opened = Some(url);
}

/// A button of the dialog, the notice or Preferences.
pub(crate) fn act(app: &App, what: &str) {
    match what {
        "download" | "retry" => {
            let fresh = app.stems_model.borrow().phase == Phase::Damaged
                || matches!(
                    app.stems_model.borrow().status(),
                    ModelStatus::Damaged { .. }
                );
            start(app, fresh);
        }
        "redownload" => start(app, true),
        "remove" => remove(app),
        "cancel" => cancel(app),
        "help" => help(app),
        "close" => {
            let mut m = app.stems_model.borrow_mut();
            if !matches!(m.phase, Phase::Downloading(..)) {
                m.phase = Phase::Idle;
            }
            m.notice = false;
            drop(m);
            refresh(app);
        }
        "later" => {
            {
                let mut m = app.stems_model.borrow_mut();
                m.declined = true;
                m.prompt_open = false;
                m.notice = false;
                m.offer_later = false;
                m.wanted.clear();
            }
            app.status(
                "Stems stay off for now. The download is in Preferences › Analysis › Stems.",
            );
            refresh(app);
        }
        _ => tracing::debug!(what, "stem model action ignored"),
    }
}

/// Periodic work (from the app tick): the worker's messages, and the dialog
/// that waited for the music to stop.
pub(crate) fn poll(app: &App) {
    let msgs: Vec<Msg> = app.stems_model.borrow().rx.try_iter().collect();
    for m in msgs {
        receive(app, m);
    }
    if crate::stems_batch::take_damaged() {
        damaged(app);
    }
    let waiting = {
        let m = app.stems_model.borrow();
        m.offer_later && m.phase == Phase::Idle && !m.status().usable()
    };
    if waiting && !playing(app) && !app.recorder.borrow().is_recording() {
        {
            let mut m = app.stems_model.borrow_mut();
            m.offer_later = false;
            m.notice = false;
            m.prompt_open = true;
        }
        refresh(app);
    }
}

fn receive(app: &App, msg: Msg) {
    match msg {
        Msg::Progress(done, total) => {
            app.stems_model.borrow_mut().phase = Phase::Downloading(done, total);
            refresh(app);
        }
        Msg::Verified(status) => {
            app.stems_model.borrow_mut().known_bad = !status.usable();
            refresh(app);
        }
        Msg::Ended(Ended::Installed) => {
            let wanted = {
                let mut m = app.stems_model.borrow_mut();
                m.phase = Phase::Idle;
                m.declined = false;
                std::mem::take(&mut m.wanted)
            };
            refresh(app);
            app.status("Stem model installed");
            // What was asked for while the model was missing goes ahead now.
            crate::decks::request_loaded_stems(app, None);
            if !wanted.is_empty() {
                crate::stems_batch::separate(app, &wanted);
            }
        }
        Msg::Ended(Ended::Cancelled) => {
            app.stems_model.borrow_mut().phase = Phase::Idle;
            app.status("Download paused. DOWNLOAD continues it where it stopped.");
            refresh(app);
        }
        Msg::Ended(Ended::Failed(f)) => {
            tracing::warn!("stem model download failed: {f}");
            app.stems_model.borrow_mut().phase = Phase::Failed(f);
            refresh(app);
        }
    }
}

/// Wires the callbacks and checks a model that nothing vouches for yet.
pub(crate) fn wire(app: &Rc<App>) {
    app.ui
        .on_stems_model_action(handler!(app, |a, what: slint::SharedString| {
            act(&a, what.as_str())
        }));
    refresh(app);
    #[cfg(feature = "separation")]
    verify_in_background(app);
}

/// A file from an older version, or one placed by hand: its checksum was
/// never recorded here. Reading 158 MB belongs off the UI thread.
#[cfg(feature = "separation")]
fn verify_in_background(app: &App) {
    let m = app.stems_model.borrow();
    if m.store.status() != ModelStatus::Unverified {
        return;
    }
    let (store, tx) = (m.store.clone(), m.tx.clone());
    drop(m);
    let _ = std::thread::Builder::new()
        .name("stem-model-check".into())
        .spawn(move || {
            if let Ok(status) = store.verify() {
                let _ = tx.send(Msg::Verified(status));
            }
        });
}

#[cfg(test)]
impl StemsModel {
    /// Points the runtime at a test store (temporary folder, mock HTTP).
    pub(crate) fn set_store(&mut self, store: ModelStore) {
        self.store = Arc::new(store);
    }

    pub(crate) fn store(&self) -> &ModelStore {
        &self.store
    }
}

/// Tests that need "the model is there" without a model file on disk.
#[cfg(test)]
pub(crate) fn assume_model_installed(app: &App) {
    let dir = crate::ui_tests::harness::test_env().join("assumed-model");
    std::fs::create_dir_all(&dir).unwrap();
    let info = od_stems::ModelInfo {
        file_name: "assumed.onnx",
        url: "https://models.invalid/assumed.onnx",
        sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        size: 0,
        license: "test",
    };
    std::fs::write(dir.join("assumed.onnx"), b"").unwrap();
    let store = ModelStore::new(dir, info);
    store.verify().unwrap();
    app.stems_model.borrow_mut().set_store(store);
    app.stems_model.borrow_mut().phase = Phase::Idle;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asks_with_a_dialog_only_in_silence() {
        assert_eq!(offer_kind(false, false, false), Offer::Dialog);
        assert_eq!(
            offer_kind(false, true, false),
            Offer::Notice,
            "a deck is playing"
        );
        assert_eq!(offer_kind(false, false, true), Offer::Notice, "recording");
        assert_eq!(offer_kind(true, false, false), Offer::Notice, "NOT NOW");
    }

    #[test]
    fn every_failure_is_one_short_sentence_in_both_languages() {
        od_i18n::set_language(Language::English);
        let failures = [
            FetchFailure::NoNetwork,
            FetchFailure::Server(503),
            FetchFailure::DiskFull { needed: 158 << 20 },
            FetchFailure::Checksum,
            FetchFailure::Interrupted,
            FetchFailure::Storage,
        ];
        for f in failures {
            let t = failure_text(f);
            assert!(t.len() < 120, "{t}");
            assert!(!t.contains("http") && !t.contains("503"), "{t}");
        }
        assert!(failure_text(FetchFailure::DiskFull { needed: 158 << 20 }).contains("158 MB"));
        for s in [
            "No internet connection. Check your network and try again.",
            "The download server did not answer properly. Try again in a few minutes.",
            "There is not enough free disk space. The stem model needs {} MB free.",
            "The download was damaged on the way and was thrown away. Please try again.",
            "The download stopped before it was finished. It continues where it left off.",
            "The stem model could not be saved on this computer. Check that the disk is writable and try again.",
            "Downloading the stem model — {} % ({} of {} MB)",
            "The stem model is damaged. RETRY deletes it and downloads it again.",
            "Stems need a one-time download of {} MB. Your music stays on this computer.",
            "The stem model is incomplete",
            "The stem model is damaged",
            "Stems need a one-time download",
            "Stems cannot be separated with this build. Stems separated on another computer still play.",
            "Damaged, {} MB on disk",
            "Not installed. The download is {} MB.",
            "Incomplete, {} of {} MB",
            "Installed, {} MB on disk",
            "The stem model is still downloading",
            "Stem model removed. Stems you already separated keep working.",
            "The stem model could not be removed. Is it in use by another program?",
            "Stems stay off for now. The download is in Preferences › Analysis › Stems.",
            "Stem model installed",
            "Download paused. DOWNLOAD continues it where it stopped.",
        ] {
            assert!(od_i18n::has_german(s), "no German for {s:?}");
        }
    }

    #[test]
    fn notice_texts_and_progress() {
        od_i18n::set_language(Language::English);
        assert_eq!(bar(Phase::Idle, false, 158).0, "");
        assert_eq!(bar(Phase::Idle, true, 158).0, "offer");
        assert!(bar(Phase::Idle, true, 158).1.contains("158 MB"));
        let (s, t, p) = bar(Phase::Downloading(82_806_318, 165_612_636), false, 158);
        assert_eq!(s, "downloading");
        assert_eq!(t, "Downloading the stem model — 50 % (79 of 158 MB)");
        assert_eq!(p, 0.5);
        assert_eq!(
            bar(Phase::Failed(FetchFailure::NoNetwork), false, 158).0,
            "error"
        );
        assert_eq!(bar(Phase::Damaged, false, 158).0, "error");
    }

    #[test]
    fn preferences_state_per_status() {
        od_i18n::set_language(Language::English);
        let v = |s, ph| prefs_view(s, ph, 165_612_636, 158, true);
        assert_eq!(v(ModelStatus::Missing, Phase::Idle).state, "missing");
        assert_eq!(
            v(ModelStatus::Partial { bytes: 10 << 20 }, Phase::Idle).state,
            "partial"
        );
        assert_eq!(
            v(ModelStatus::Partial { bytes: 10 << 20 }, Phase::Idle).text,
            "Incomplete, 10 of 158 MB"
        );
        assert_eq!(
            v(ModelStatus::Installed, Phase::Idle).text,
            "Installed, 158 MB on disk"
        );
        assert_eq!(v(ModelStatus::Unverified, Phase::Idle).state, "installed");
        assert_eq!(
            v(ModelStatus::Damaged { bytes: 5 }, Phase::Idle).state,
            "damaged"
        );
        assert_eq!(
            v(ModelStatus::Missing, Phase::Downloading(1, 2)).state,
            "downloading"
        );
        assert!(v(ModelStatus::Missing, Phase::Failed(FetchFailure::Checksum)).help);
        assert_eq!(
            prefs_view(ModelStatus::Missing, Phase::Idle, 0, 158, false).state,
            "unsupported"
        );
    }

    #[test]
    fn help_points_at_the_wiki_section_of_the_language() {
        assert_eq!(
            help_url(Language::German),
            format!("https://opendjdecks.de/wiki/stems/#{HELP_ANCHOR_DE}")
        );
        assert_eq!(
            help_url(Language::English),
            format!("https://opendjdecks.de/en/wiki/stems/#{HELP_ANCHOR_EN}")
        );
    }

    /// The anchor the website derives from a heading (remark's slug rule).
    fn slug(heading: &str) -> String {
        let lower = heading.to_lowercase();
        let cleaned: String = lower
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c.is_whitespace() {
                    c
                } else {
                    ' '
                }
            })
            .collect();
        cleaned.split_whitespace().collect::<Vec<_>>().join("-")
    }

    #[test]
    fn help_anchors_exist_in_the_guides() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide");
        for (lang, anchor) in [("en", HELP_ANCHOR_EN), ("de", HELP_ANCHOR_DE)] {
            let text = std::fs::read_to_string(root.join(lang).join("stems.md")).unwrap();
            assert!(
                text.lines()
                    .filter_map(|l| l.strip_prefix("## "))
                    .any(|h| slug(h) == anchor),
                "{lang}/stems.md has no section `{anchor}`"
            );
            let trouble =
                std::fs::read_to_string(root.join(lang).join("troubleshooting.md")).unwrap();
            assert!(
                trouble.contains(&format!("stems.md#{anchor}")),
                "{lang}/troubleshooting.md links there"
            );
        }
    }
}
