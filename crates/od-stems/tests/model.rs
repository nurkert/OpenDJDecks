//! The model download against a mock HTTP layer: no network, no real model.
#![cfg(feature = "separation")]

use od_stems::{Error, FetchFailure, ModelInfo, ModelStatus, ModelStore};
use od_update::http::Response;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const URL: &str = "https://models.test/stem.onnx";
const NAME: &str = "stem.onnx";

fn payload() -> Vec<u8> {
    (0..200_000u32).map(|i| (i % 251) as u8).collect()
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
        file_name: NAME,
        url: URL,
        sha256: sha(data),
        size: data.len() as u64,
        license: "test",
    }
}

/// What the next request gets.
#[derive(Clone)]
enum Answer {
    Body,
    /// The first `n` bytes, then a dropped connection.
    Cut(usize),
    Status(u16),
    NoNetwork,
    /// Ignores `Range` and sends everything.
    Full,
}

#[derive(Clone, Default)]
struct Mock {
    requests: Arc<Mutex<Vec<Option<u64>>>>,
    answers: Arc<Mutex<Vec<Answer>>>,
    body: Arc<Vec<u8>>,
}

impl Mock {
    fn new(body: &[u8], answers: &[Answer]) -> Self {
        Self {
            requests: Arc::default(),
            answers: Arc::new(Mutex::new(answers.iter().rev().cloned().collect())),
            body: Arc::new(body.to_vec()),
        }
    }

    fn requests(&self) -> Vec<Option<u64>> {
        self.requests.lock().unwrap().clone()
    }
}

/// Reads `data`, then fails.
struct Dropped(std::io::Cursor<Vec<u8>>);

impl Read for Dropped {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.0.read(buf)? {
            0 => Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "dropped",
            )),
            n => Ok(n),
        }
    }
}

impl od_update::Http for Mock {
    fn get(&self, _url: &str, range_from: Option<u64>) -> Result<Response, od_update::Error> {
        self.requests.lock().unwrap().push(range_from);
        let answer = self.answers.lock().unwrap().pop().unwrap_or(Answer::Body);
        let from = range_from.unwrap_or(0) as usize;
        let response = |status, bytes: Vec<u8>| Response {
            status,
            content_length: Some(bytes.len() as u64),
            body: Box::new(std::io::Cursor::new(bytes)),
        };
        Ok(match answer {
            Answer::NoNetwork => return Err(od_update::Error::Network("no route".into())),
            Answer::Status(s) => response(s, Vec::new()),
            Answer::Full => response(200, self.body.to_vec()),
            Answer::Body => response(if from > 0 { 206 } else { 200 }, self.body[from..].to_vec()),
            Answer::Cut(n) => Response {
                status: if from > 0 { 206 } else { 200 },
                content_length: None,
                body: Box::new(Dropped(std::io::Cursor::new(
                    self.body[from..from + n].to_vec(),
                ))),
            },
        })
    }
}

fn store(dir: &Path, data: &[u8], mock: &Mock) -> ModelStore {
    ModelStore::new(dir, info(data))
        .with_http(Box::new(mock.clone()))
        .with_free_space(Box::new(|_| Some(u64::MAX)))
}

fn fetch(s: &ModelStore) -> Result<(), Error> {
    s.fetch(&mut |_, _| {}, &AtomicBool::new(false)).map(|_| ())
}

fn failure(r: Result<(), Error>) -> FetchFailure {
    match r {
        Err(Error::Fetch(f)) => f,
        other => panic!("expected a download failure, got {other:?}"),
    }
}

#[test]
fn downloads_and_marks_the_model_verified() {
    let dir = tempfile::tempdir().unwrap();
    let data = payload();
    let mock = Mock::new(&data, &[]);
    let s = store(dir.path(), &data, &mock);
    assert_eq!(s.status(), ModelStatus::Missing);
    assert_eq!(s.size_on_disk(), 0);
    let mut seen = 0;
    s.fetch(&mut |d, _| seen = d, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(seen, data.len() as u64);
    assert_eq!(s.status(), ModelStatus::Installed, "checked in passing");
    assert_eq!(std::fs::read(s.model_path()).unwrap(), data);
    assert!(s.size_on_disk() >= data.len() as u64);
    assert_eq!(mock.requests().len(), 1);
    // A second fetch does nothing.
    fetch(&s).unwrap();
    assert_eq!(
        mock.requests().len(),
        1,
        "no request for an installed model"
    );
}

#[test]
fn an_interrupted_download_resumes_with_a_range() {
    let dir = tempfile::tempdir().unwrap();
    let data = payload();
    let mock = Mock::new(&data, &[Answer::Cut(70_000), Answer::Body]);
    let s = store(dir.path(), &data, &mock);
    assert_eq!(failure(fetch(&s)), FetchFailure::Interrupted);
    assert_eq!(
        s.status(),
        ModelStatus::Partial { bytes: 70_000 },
        "the partial file stays"
    );
    fetch(&s).unwrap();
    assert_eq!(
        mock.requests(),
        vec![None, Some(70_000)],
        "asked only for the rest"
    );
    assert_eq!(s.status(), ModelStatus::Installed);
    assert_eq!(std::fs::read(s.model_path()).unwrap(), data);
}

#[test]
fn a_connection_closed_early_counts_as_interrupted() {
    let dir = tempfile::tempdir().unwrap();
    let data = payload();
    let mock = Mock::new(&data, &[Answer::Status(200)]);
    let s = store(dir.path(), &data, &mock);
    assert_eq!(
        failure(fetch(&s)),
        FetchFailure::Interrupted,
        "an empty body ends the download"
    );
    assert!(s.status() != ModelStatus::Installed);
}

#[test]
fn cancel_keeps_the_partial_file_for_the_next_try() {
    let dir = tempfile::tempdir().unwrap();
    let data = payload();
    let mock = Mock::new(&data, &[]);
    let s = store(dir.path(), &data, &mock);
    let cancel = AtomicBool::new(false);
    let r = s.fetch(
        &mut |d, _| {
            if d > 60_000 {
                cancel.store(true, Ordering::Relaxed);
            }
        },
        &cancel,
    );
    assert!(matches!(r, Err(Error::Cancelled)), "{r:?}");
    assert!(matches!(s.status(), ModelStatus::Partial { .. }));
    fetch(&s).unwrap();
    assert_eq!(s.status(), ModelStatus::Installed);
}

#[test]
fn a_bad_checksum_discards_the_file_and_never_installs_it() {
    let dir = tempfile::tempdir().unwrap();
    let data = payload();
    let mut corrupt = data.clone();
    corrupt[1234] ^= 0xFF;
    let mock = Mock::new(&corrupt, &[]);
    let s = store(dir.path(), &data, &mock);
    assert_eq!(failure(fetch(&s)), FetchFailure::Checksum);
    assert_eq!(s.status(), ModelStatus::Missing, "the bad file is gone");
    assert!(!s.model_path().exists(), "and it never became the model");
}

#[test]
fn a_full_disk_is_found_before_anything_is_requested() {
    let dir = tempfile::tempdir().unwrap();
    let data = payload();
    let mock = Mock::new(&data, &[]);
    let s = ModelStore::new(dir.path(), info(&data))
        .with_http(Box::new(mock.clone()))
        .with_free_space(Box::new(|_| Some(1024)));
    let r = fetch(&s);
    let needed = data.len() as u64 + 16 * 1024 * 1024;
    assert_eq!(
        failure(r),
        FetchFailure::DiskFull { needed },
        "what to have free, margin included"
    );
    assert!(
        mock.requests().is_empty(),
        "no request when the model cannot fit"
    );
    assert_eq!(s.status(), ModelStatus::Missing);
}

#[test]
fn the_free_space_check_counts_only_what_is_still_needed() {
    let dir = tempfile::tempdir().unwrap();
    let data = payload();
    std::fs::write(dir.path().join(format!("{NAME}.part")), &data[..150_000]).unwrap();
    let mock = Mock::new(&data, &[]);
    // Room for the missing 50 KB and the margin, not for the whole model.
    let room = 50_000 + 16 * 1024 * 1024;
    let s = ModelStore::new(dir.path(), info(&data))
        .with_http(Box::new(mock.clone()))
        .with_free_space(Box::new(move |_| Some(room)));
    fetch(&s).unwrap();
    assert_eq!(s.status(), ModelStatus::Installed);
}

#[test]
fn network_and_server_failures_are_told_apart() {
    let dir = tempfile::tempdir().unwrap();
    let data = payload();
    let mock = Mock::new(&data, &[Answer::NoNetwork, Answer::Status(503)]);
    let s = store(dir.path(), &data, &mock);
    assert_eq!(failure(fetch(&s)), FetchFailure::NoNetwork);
    assert_eq!(failure(fetch(&s)), FetchFailure::Server(503));
}

#[test]
fn a_server_ignoring_the_range_restarts_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let data = payload();
    std::fs::write(dir.path().join(format!("{NAME}.part")), &data[..1000]).unwrap();
    let mock = Mock::new(&data, &[Answer::Full]);
    let s = store(dir.path(), &data, &mock);
    fetch(&s).unwrap();
    assert_eq!(std::fs::read(s.model_path()).unwrap(), data);
}

#[test]
fn a_range_the_server_refuses_starts_over() {
    let dir = tempfile::tempdir().unwrap();
    let data = payload();
    std::fs::write(dir.path().join(format!("{NAME}.part")), &data[..1000]).unwrap();
    let mock = Mock::new(&data, &[Answer::Status(416), Answer::Body]);
    let s = store(dir.path(), &data, &mock);
    fetch(&s).unwrap();
    assert_eq!(mock.requests(), vec![Some(1000), None]);
    assert_eq!(s.status(), ModelStatus::Installed);
}

#[test]
fn damage_shows_in_the_status_and_is_never_trusted() {
    let dir = tempfile::tempdir().unwrap();
    let data = payload();
    let mock = Mock::new(&data, &[]);
    let s = store(dir.path(), &data, &mock);
    // Too short: damaged by its size alone.
    std::fs::write(s.model_path(), &data[..100]).unwrap();
    assert_eq!(s.status(), ModelStatus::Damaged { bytes: 100 });
    assert_eq!(s.verify().unwrap(), ModelStatus::Damaged { bytes: 100 });
    // Right size, wrong bytes: unverified until hashed, then damaged.
    let mut corrupt = data.clone();
    corrupt[9] ^= 1;
    std::fs::write(s.model_path(), &corrupt).unwrap();
    assert_eq!(s.status(), ModelStatus::Unverified);
    assert!(s.verify().unwrap() != ModelStatus::Installed);
    assert_eq!(
        s.status(),
        ModelStatus::Unverified,
        "a failed check earns no mark"
    );
    // A correct copy placed by hand passes and is remembered.
    std::fs::write(s.model_path(), &data).unwrap();
    assert_eq!(s.verify().unwrap(), ModelStatus::Installed);
    assert_eq!(s.status(), ModelStatus::Installed);
    // Changing the file afterwards drops the mark again.
    std::fs::write(s.model_path(), &corrupt).unwrap();
    assert_eq!(s.status(), ModelStatus::Unverified);
}

#[test]
fn a_damaged_model_is_replaced_by_a_new_download() {
    let dir = tempfile::tempdir().unwrap();
    let data = payload();
    let mock = Mock::new(&data, &[]);
    let s = store(dir.path(), &data, &mock);
    std::fs::write(s.model_path(), b"garbage").unwrap();
    fetch(&s).unwrap();
    assert_eq!(s.status(), ModelStatus::Installed);
    assert_eq!(std::fs::read(s.model_path()).unwrap(), data);
}

#[test]
fn delete_and_redownload_removes_every_leftover_first() {
    let dir = tempfile::tempdir().unwrap();
    let data = payload();
    let mock = Mock::new(&data, &[]);
    let s = store(dir.path(), &data, &mock);
    // A damaged model, a partial download, a stale copy of another version,
    // and a file that is none of the model's business.
    std::fs::write(s.model_path(), b"broken").unwrap();
    std::fs::write(dir.path().join(format!("{NAME}.part")), &data[..1000]).unwrap();
    std::fs::write(dir.path().join("older_model.onnx"), b"stale").unwrap();
    std::fs::write(dir.path().join("notes.txt"), b"mine").unwrap();
    s.remove_all().unwrap();
    assert_eq!(s.status(), ModelStatus::Missing);
    assert_eq!(s.size_on_disk(), 0);
    assert!(dir.path().join("notes.txt").exists(), "other files stay");
    fetch(&s).unwrap();
    assert_eq!(mock.requests(), vec![None], "started from nothing");
    assert_eq!(s.status(), ModelStatus::Installed);
}

#[test]
fn removing_the_model_keeps_stems_already_separated() {
    let dir = tempfile::tempdir().unwrap();
    let cache = od_stems::StemCache::new(dir.path().join("stems"));
    let hash = od_core::ContentHash("cc".into());
    let stems = od_stems::Stems {
        sample_rate: 44_100,
        vocals: vec![0.0; 20],
        drums: vec![0.0; 20],
        bass: vec![0.0; 20],
    };
    cache.store(&hash, &stems).unwrap();
    let data = payload();
    let mock = Mock::new(&data, &[]);
    let models = dir.path().join("models");
    std::fs::create_dir_all(&models).unwrap();
    let s = store(&models, &data, &mock);
    fetch(&s).unwrap();
    s.remove_all().unwrap();
    assert_eq!(s.status(), ModelStatus::Missing);
    assert!(cache.exists(&hash), "separated stems stay");
}

#[test]
fn a_finished_part_is_moved_into_place_without_a_request() {
    let dir = tempfile::tempdir().unwrap();
    let data = payload();
    std::fs::write(dir.path().join(format!("{NAME}.part")), &data).unwrap();
    let mock = Mock::new(&data, &[]);
    let s = store(dir.path(), &data, &mock);
    fetch(&s).unwrap();
    assert!(mock.requests().is_empty());
    assert_eq!(s.status(), ModelStatus::Installed);
}
