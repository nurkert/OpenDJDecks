//! The stem model workflow in the real window: a part key asks, nothing
//! downloads by itself, the download reports and fails in plain words, and
//! Preferences repairs it. The HTTP layer is mocked; no real model.

use super::harness::{Fixture, Ui};
use crate::stems_model::Phase;
use i_slint_backend_testing::ElementHandle;
use od_stems::{FetchFailure, ModelInfo, ModelStatus, ModelStore};
use od_update::http::Response;
use sha2::{Digest, Sha256};
use slint::ComponentHandle;
use std::sync::{Arc, Mutex};

const URL: &str = "https://models.test/stem.onnx";

fn payload() -> Vec<u8> {
    (0..300_000u32).map(|i| (i % 251) as u8).collect()
}

fn sha(data: &[u8]) -> &'static str {
    let h: String = Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Box::leak(h.into_boxed_str())
}

fn info(data: &[u8]) -> ModelInfo {
    ModelInfo {
        file_name: "stem.onnx",
        url: URL,
        sha256: sha(data),
        size: data.len() as u64,
        license: "test",
    }
}

#[derive(Clone)]
struct Mock {
    body: Arc<Vec<u8>>,
    /// Answered in turn; empty = the file.
    answers: Arc<Mutex<Vec<u16>>>,
    requests: Arc<Mutex<u32>>,
}

impl Mock {
    fn new(answers: &[u16]) -> Self {
        Self {
            body: Arc::new(payload()),
            answers: Arc::new(Mutex::new(answers.iter().rev().copied().collect())),
            requests: Arc::default(),
        }
    }

    fn requests(&self) -> u32 {
        *self.requests.lock().unwrap()
    }
}

impl od_update::Http for Mock {
    fn get(&self, _url: &str, range_from: Option<u64>) -> Result<Response, od_update::Error> {
        *self.requests.lock().unwrap() += 1;
        let from = range_from.unwrap_or(0) as usize;
        if let Some(status) = self.answers.lock().unwrap().pop() {
            return Ok(Response {
                status,
                content_length: Some(0),
                body: Box::new(std::io::empty()),
            });
        }
        Ok(Response {
            status: if from > 0 { 206 } else { 200 },
            content_length: None,
            body: Box::new(std::io::Cursor::new(self.body[from..].to_vec())),
        })
    }
}

/// Opens the app with the model store on a temporary folder and a mock server.
fn open(ui: &Ui, mock: &Mock) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let data = payload();
    let store = ModelStore::new(dir.path(), info(&data))
        .with_http(Box::new(mock.clone()))
        .with_free_space(Box::new(|_| Some(u64::MAX)));
    ui.app.stems_model.borrow_mut().set_store(store);
    crate::stems_model::refresh(&ui.app);
    dir
}

fn perform(ui: &Ui) {
    ui.app.ui.invoke_set_mode("perform".into());
    crate::engine::ensure_engine(&ui.app);
    ui.pump(300);
}

fn load_first(ui: &Ui) -> od_core::TrackId {
    let (id, title) = ui.fx.tracks[0].clone();
    crate::decks::load_track(&ui.app, 0, id, None);
    assert!(ui.wait_until(30.0, |u| {
        super::flows::deck_views(u)
            .first()
            .is_some_and(|d| d.title == title.as_str())
    }));
    id
}

fn status(ui: &Ui) -> ModelStatus {
    ui.app.stems_model.borrow().store().status()
}

fn phase(ui: &Ui) -> Phase {
    ui.app.stems_model.borrow().phase
}

fn press_part_key(ui: &Ui) {
    ui.click("deck1.stem.vocal.request");
    ui.pump(50);
}

#[test]
fn loading_a_track_never_asks() {
    let ui = Ui::open((1440, 900));
    let mock = Mock::new(&[]);
    let _dir = open(&ui, &mock);
    perform(&ui);
    load_first(&ui);
    ui.pump(500);
    assert!(!ui.has("app.stems.prompt"), "no dialog on load");
    assert!(!ui.has("app.stems"), "no notice on load");
    assert_eq!(mock.requests(), 0, "and no download");
    assert!(!ui.app.decks.borrow().stems_requested());
}

#[test]
fn a_part_key_opens_the_dialog_and_not_now_is_respected() {
    let ui = Ui::open((1440, 900));
    let mock = Mock::new(&[]);
    let _dir = open(&ui, &mock);
    perform(&ui);
    press_part_key(&ui);
    assert!(ui.has("app.stems.prompt"), "the dialog asks");
    assert!(ui.has("app.stems.prompt.download") && ui.has("app.stems.prompt.later"));
    let text = ElementHandle::find_by_accessible_label(&ui.app.ui, "app.stems.prompt.text")
        .next()
        .and_then(|e| e.accessible_value())
        .unwrap_or_default();
    assert!(
        text.contains("once") && text.contains("MB"),
        "says what it needs: {text}"
    );
    assert_eq!(mock.requests(), 0, "nothing has been downloaded");

    ui.click("app.stems.prompt.later");
    ui.pump(50);
    assert!(!ui.has("app.stems.prompt"));
    assert!(ui.app.stems_model.borrow().declined);
    // Pressing again: no dialog for the rest of the session, only the notice.
    press_part_key(&ui);
    assert!(!ui.has("app.stems.prompt"), "NOT NOW holds for the session");
    assert!(
        ui.has("app.stems.later") && ui.has("app.stems.download"),
        "the download stays one press away"
    );
    assert_eq!(mock.requests(), 0);
}

#[test]
fn while_music_plays_only_a_notice_then_the_dialog() {
    let ui = Ui::open((1440, 900));
    let mock = Mock::new(&[]);
    let _dir = open(&ui, &mock);
    perform(&ui);
    load_first(&ui);
    ui.click("deck1.play");
    assert!(ui.wait_until(10.0, |u| {
        crate::engine::snapshot(&u.app).is_some_and(|s| s.decks[0].playing)
    }));
    press_part_key(&ui);
    assert!(
        !ui.has("app.stems.prompt"),
        "no modal dialog while music plays"
    );
    assert!(
        ui.wait_until(5.0, |u| u.has("app.stems.download")),
        "a notice offers it"
    );
    assert!(ui.wait_until(5.0, |u| u.app.stems_model.borrow().offer_later));
    ui.click("deck1.play");
    assert!(ui.wait_until(10.0, |u| {
        crate::engine::snapshot(&u.app).is_some_and(|s| !s.decks[0].playing)
    }));
    assert!(
        ui.wait_until(5.0, |u| u.has("app.stems.prompt")),
        "the dialog follows once playback has stopped"
    );
    assert_eq!(mock.requests(), 0);
}

#[test]
fn download_runs_in_the_background_and_ends_installed() {
    let ui = Ui::open((1440, 900));
    let mock = Mock::new(&[]);
    let _dir = open(&ui, &mock);
    perform(&ui);
    press_part_key(&ui);
    ui.click("app.stems.prompt.download");
    assert!(!ui.has("app.stems.prompt"), "the dialog closes at once");
    assert!(
        ui.wait_until(5.0, |u| matches!(phase(u), Phase::Downloading(..))
            || phase(u) == Phase::Idle)
    );
    assert!(ui.wait_until(30.0, |u| status(u) == ModelStatus::Installed
        && phase(u) == Phase::Idle));
    assert!(!ui.has("app.stems"), "the notice is gone");
    assert_eq!(mock.requests(), 1);
}

#[test]
fn a_failed_download_offers_retry_and_help() {
    let ui = Ui::open((1440, 900));
    let mock = Mock::new(&[503]);
    let _dir = open(&ui, &mock);
    perform(&ui);
    press_part_key(&ui);
    ui.click("app.stems.prompt.download");
    assert!(ui.wait_until(30.0, |u| phase(u)
        == Phase::Failed(FetchFailure::Server(503))));
    assert!(ui.wait_until(5.0, |u| u.has("app.stems.retry") && u.has("app.stems.help")));
    let text = ui.value("app.stems");
    assert!(!text.contains("503"), "no codes in the sentence: {text}");
    ui.click("app.stems.help");
    ui.pump(50);
    let opened = ui
        .app
        .stems_model
        .borrow()
        .help_opened
        .clone()
        .unwrap_or_default();
    assert!(opened.contains("/wiki/stems/#"), "{opened}");
    ui.click("app.stems.retry");
    assert!(ui.wait_until(30.0, |u| status(u) == ModelStatus::Installed
        && phase(u) == Phase::Idle));
    assert_eq!(mock.requests(), 2);
}

/// Preferences > Analysis > Stems: every state, delete-and-redownload from a
/// damaged file, REMOVE MODEL.
#[test]
fn preferences_show_and_repair_the_model() {
    let ui = Ui::open((1440, 900));
    let mock = Mock::new(&[]);
    let dir = open(&ui, &mock);
    ui.app.ui.invoke_open_preferences();
    ui.pump(200);
    let w = ui
        .app
        .prefs
        .borrow()
        .window
        .as_ref()
        .unwrap()
        .clone_strong();
    w.set_page("analysis".into());
    crate::stems_model::refresh(&ui.app);
    ui.pump(100);
    let el = |label: &str| ElementHandle::find_by_accessible_label(&w, label).next();
    let click = |label: &str| {
        el(label)
            .unwrap_or_else(|| panic!("no `{label}`"))
            .mock_single_click(slint::platform::PointerEventButton::Left);
        ui.pump(100);
    };
    assert_eq!(w.get_stems_state(), "missing");
    assert!(el("prefs.analysis.stems-status").is_some());
    assert!(el("prefs.analysis.stems-download").is_some());
    assert!(
        el("prefs.analysis.stems-remove").is_none(),
        "nothing to remove yet"
    );
    // Damaged: a file of the wrong size.
    std::fs::write(ui.app.stems_model.borrow().store().model_path(), b"damaged").unwrap();
    crate::stems_model::refresh(&ui.app);
    ui.pump(50);
    assert_eq!(w.get_stems_state(), "damaged");
    assert!(
        el("prefs.analysis.stems-help").is_some(),
        "a damaged model gets HELP"
    );
    click("prefs.analysis.stems-redownload");
    assert!(ui.wait_until(30.0, |u| status(u) == ModelStatus::Installed
        && phase(u) == Phase::Idle));
    crate::stems_model::refresh(&ui.app);
    ui.pump(50);
    assert_eq!(w.get_stems_state(), "installed");
    assert!(el("prefs.analysis.stems-remove").is_some());
    // REMOVE MODEL leaves separated stems alone.
    let cache = od_stems::StemCache::open_default();
    let hash = od_core::ContentHash("prefs-remove".into());
    let stems = od_stems::Stems {
        sample_rate: 44_100,
        vocals: vec![0.0; 20],
        drums: vec![0.0; 20],
        bass: vec![0.0; 20],
    };
    cache.store(&hash, &stems).unwrap();
    click("prefs.analysis.stems-remove");
    assert_eq!(status(&ui), ModelStatus::Missing);
    assert!(cache.exists(&hash), "separated stems stay");
    let _ = cache.remove(&hash);
    assert_eq!(w.get_stems_state(), "missing");
    drop(dir);
    let _ = w.hide();
}

/// Batch separation asks, and the batch waits for the model.
#[test]
fn batch_separation_asks_and_goes_ahead_after_the_download() {
    let ui = Ui::open((1440, 900));
    let mock = Mock::new(&[]);
    let _dir = open(&ui, &mock);
    let id = ui.fx.tracks[0].0;
    crate::stems_batch::separate(&ui.app, &[id]);
    ui.pump(50);
    assert!(ui.has("app.stems.prompt"), "batch separation asks too");
    assert_eq!(
        crate::stems_batch::count().total,
        0,
        "nothing queued without the model"
    );
    ui.click("app.stems.prompt.download");
    assert!(ui.wait_until(30.0, |u| status(u) == ModelStatus::Installed));
    assert!(
        ui.wait_until(5.0, |_| crate::stems_batch::count().total == 1),
        "the waiting track is queued"
    );
}

#[test]
fn the_kiosk_never_shows_a_dialog() {
    let ui = Ui::start(Fixture::new(), (800, 480), true);
    let mock = Mock::new(&[]);
    let _dir = open(&ui, &mock);
    crate::stems_model::request(&ui.app, &[]);
    ui.pump(100);
    assert!(
        !ui.has("app.stems.prompt"),
        "nothing to answer on the kiosk"
    );
    assert!(!ui.has("app.stems"), "and no notice either");
    assert_eq!(mock.requests(), 0);
}
