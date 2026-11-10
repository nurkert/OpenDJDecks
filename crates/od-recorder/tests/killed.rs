//! A recording survives a killed process (roadmap [9]): a child process
//! records the engine's master output and is killed with SIGKILL; the file
//! it leaves plays, and `repair_wav` makes its header match all that reached
//! the disk.

use od_engine::{Command, Engine, EngineConfig, MixerCommand, Outputs};
use od_recorder::{Recorder, repair_wav};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};
use std::time::{Duration, Instant};

const RATE: u32 = 48_000;
const CHILD_DIR: &str = "OD_RECORDER_CHILD_DIR";

/// The child: records a test tone until it is killed.
#[test]
#[ignore = "run by killed_recording_plays_after_repair in a child process"]
fn child_records_until_killed() {
    let Some(dir) = std::env::var_os(CHILD_DIR).map(PathBuf::from) else {
        return;
    };
    let (mut engine, mut processor) = Engine::new(EngineConfig {
        sample_rate: RATE,
        max_block: 512,
        decks: 2,
        ..Default::default()
    });
    let tap = engine.take_record_tap().unwrap();
    engine.send(Command::SetRecordTap(true)).unwrap();
    engine.send(Command::Mixer(MixerCommand::SetTestTone(true))).unwrap();
    let mut rec = Recorder::new(tap, RATE);
    let path = rec.start(&dir, "set").unwrap();
    println!("PATH {}", path.display());
    let mut master = vec![0.0f32; 1024];
    let started = Instant::now();
    let mut said = false;
    let mut rendered = 0u64;
    loop {
        // Real time: one 512-frame block every ~10.7 ms.
        processor.process(512, &mut Outputs { master: &mut master, headphones: None, booth: None });
        rendered += 512;
        let due = Duration::from_secs_f64(rendered as f64 / RATE as f64);
        if let Some(wait) = due.checked_sub(started.elapsed()) {
            std::thread::sleep(wait);
        }
        // Never outlive a parent that failed to kill us.
        if started.elapsed() > Duration::from_secs(120) {
            std::process::exit(2);
        }
        if !said && rec.elapsed() > 2.5 {
            println!("RECORDING");
            said = true;
        }
    }
}

/// Kills the child if the test fails before it does.
struct Child(std::process::Child);

impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wav_seconds(path: &Path) -> f64 {
    let r = hound::WavReader::open(path).expect("a readable WAV");
    r.duration() as f64 / r.spec().sample_rate as f64
}

#[test]
fn killed_recording_plays_after_repair() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = Child(Process::new(std::env::current_exe().unwrap())
        .args(["child_records_until_killed", "--exact", "--ignored", "--nocapture", "--test-threads=1"])
        .env(CHILD_DIR, dir.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("child starts"));
    let mut lines = BufReader::new(child.0.stdout.take().unwrap()).lines();
    let mut path = None;
    let deadline = Instant::now() + Duration::from_secs(60);
    for line in lines.by_ref() {
        let line = line.unwrap();
        // The test harness may print its own text before ours on a line.
        if let Some(i) = line.find("PATH ") {
            path = Some(PathBuf::from(&line[i + 5..]));
        }
        if line.contains("RECORDING") || Instant::now() > deadline {
            break;
        }
    }
    let path = path.expect("the child names its file");
    // Mid-write, between header updates.
    std::thread::sleep(Duration::from_millis(450));
    // Child::kill is SIGKILL on Unix (TerminateProcess on Windows): no
    // destructor, no finalize.
    child.0.kill().unwrap();
    child.0.wait().unwrap();

    let before = wav_seconds(&path);
    assert!(before >= 1.0, "the file plays even unrepaired: {before:.2} s");
    let repaired = repair_wav(&path).unwrap();
    let after = wav_seconds(&path);
    assert!((after - repaired).abs() < 0.01);
    assert!(after >= before, "repair keeps everything: {before:.2} -> {after:.2}");
    assert!(after >= 2.4, "close to all that was recorded: {after:.2} s");
    // The tone is in it.
    let mut r = hound::WavReader::open(&path).unwrap();
    let peak = r
        .samples::<i32>()
        .skip(RATE as usize)
        .take(RATE as usize)
        .map(|s| s.unwrap().unsigned_abs())
        .max()
        .unwrap();
    assert!(peak > 100_000, "audio, not silence: {peak}");
    // A whole file is left as it is.
    assert!((repair_wav(&path).unwrap() - after).abs() < 1e-9);
}

#[test]
fn repair_cuts_a_torn_frame_and_fixes_the_sizes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("torn.wav");
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: RATE,
        bits_per_sample: 24,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(&path, spec).unwrap();
    for i in 0..RATE * 2 {
        w.write_sample((i as i32 % 1000) * 100).unwrap();
    }
    w.finalize().unwrap();
    // A crash before any header update: sizes say nothing, a frame is torn.
    let mut bytes = std::fs::read(&path).unwrap();
    bytes.extend_from_slice(&[1, 2, 3, 4]);
    let data = bytes.windows(4).position(|w| w == b"data").unwrap();
    bytes[4..8].copy_from_slice(&0u32.to_le_bytes());
    bytes[data + 4..data + 8].copy_from_slice(&0u32.to_le_bytes());
    std::fs::write(&path, &bytes).unwrap();
    let secs = repair_wav(&path).unwrap();
    assert!((secs - 1.0).abs() < 1e-6, "{secs}");
    assert!((wav_seconds(&path) - 1.0).abs() < 1e-6);
}
