//! The worker's model check, on its own: it sets the data dir of this test
//! binary.
#![cfg(feature = "separation")]

use od_core::{ContentHash, TrackId};
use od_stems::{MODEL, StemCache, StemEvent, StemJob, StemWorker, StemWorkerOptions};

#[test]
fn a_damaged_model_is_reported_and_never_loaded() {
    let dir = tempfile::tempdir().unwrap();
    // SAFETY: the only test of this binary; set before the worker starts.
    unsafe { std::env::set_var("OPENDJDECKS_DATA_DIR", dir.path()) };
    let models = dir.path().join("models");
    std::fs::create_dir_all(&models).unwrap();
    // A file of the wrong size, the way an interrupted copy looks.
    std::fs::write(models.join(MODEL.file_name), b"not a model").unwrap();
    assert!(matches!(
        od_stems::model_status(),
        od_stems::ModelStatus::Damaged { .. }
    ));

    let worker = StemWorker::spawn(
        StemCache::new(dir.path().join("stems")),
        StemWorkerOptions::default(),
    );
    worker.enqueue(StemJob {
        track: TrackId(7),
        path: "/nonexistent.wav".into(),
        hash: ContentHash("dd".into()),
    });
    let ev = worker
        .events()
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    assert_eq!(ev, StemEvent::ModelDamaged { track: TrackId(7) });
}
