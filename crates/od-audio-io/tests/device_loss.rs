//! Device loss and recovery on the virtual output (roadmap [10]): the loss
//! is seen at once, and the same engine goes on on another device with its
//! decks where they were. Real interfaces cannot be unplugged in a test;
//! the virtual device fails the way an unplugged one does (callbacks stop,
//! the backend reports the device gone).

use od_audio_io::{AudioConfig, AudioHost, Health, STALL_LIMIT, VirtualOutput};
use od_core::{Beatgrid, TrackId};
use od_decode::DecodedAudio;
use od_engine::{Command, DeckCommand, Engine, EngineConfig, TrackData};
use std::time::{Duration, Instant};

const SR: u32 = 48_000;

fn engine() -> (Engine, od_engine::EngineProcessor) {
    Engine::new(EngineConfig {
        sample_rate: SR,
        max_block: 2048,
        decks: 2,
        ..EngineConfig::default()
    })
}

fn silent_track(seconds: f64) -> std::sync::Arc<TrackData> {
    let n = (seconds * SR as f64) as usize * 2;
    TrackData::new(
        TrackId(7),
        DecodedAudio {
            sample_rate: SR,
            channels: 2,
            samples: vec![0.0; n],
        },
        SR,
        Beatgrid::constant(120.0, 0.0, 1, seconds),
        &[],
        0.0,
    )
}

fn wait_for(secs: f32, mut cond: impl FnMut() -> bool) -> bool {
    let end = Instant::now() + Duration::from_secs_f32(secs);
    while Instant::now() < end {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    cond()
}

fn position(engine: &mut Engine) -> f64 {
    engine.state().decks[0].position
}

#[test]
fn loss_is_seen_at_once_and_the_engine_moves_to_another_device() {
    let (mut engine, processor) = engine();
    let host = AudioHost::start_virtual(&VirtualOutput::new("Interface A", SR), processor);
    assert!(host.is_virtual());
    assert!(wait_for(2.0, || host.health() == Health::Running));
    engine.send(Command::Deck(0, DeckCommand::Load(silent_track(60.0)))).unwrap();
    engine.send(Command::Deck(0, DeckCommand::Play)).unwrap();
    assert!(
        wait_for(3.0, || position(&mut engine) > 0.3),
        "deck 1 plays on the device"
    );

    // Pull the cable.
    host.fault_hook().lose_device();
    let stats = host.stats();
    assert_eq!(stats.health(), Health::Lost, "the loss shows at once");
    assert!(stats.device_lost);
    let at_loss = position(&mut engine);
    let callbacks = host.stats().callbacks;
    std::thread::sleep(Duration::from_millis(60));
    assert!(
        host.stats().callbacks <= callbacks + 1,
        "no callbacks from a device that is gone"
    );

    // The processor comes back with the deck as it was.
    let processor = host.stop().expect("the processor is handed back");
    let other = AudioHost::start_virtual(&VirtualOutput::new("Built-in Output", SR), processor);
    assert_eq!(other.info().device_name, "Built-in Output");
    assert!(wait_for(2.0, || other.health() == Health::Running));
    assert!(
        wait_for(3.0, || position(&mut engine) > at_loss + 0.2),
        "deck 1 goes on from where it was ({at_loss:.2} s)"
    );
    let now = position(&mut engine);
    assert!(
        now < at_loss + 2.5,
        "and does not jump: {now:.2} s after {at_loss:.2} s"
    );
    assert!(engine.state().decks[0].playing);
    assert_eq!(other.stats().dropouts(), 0);
    drop(other.stop());
}

#[test]
fn a_device_that_stops_calling_back_counts_as_lost() {
    let (_engine, processor) = engine();
    let host = AudioHost::start_virtual(&VirtualOutput::new("Interface A", SR), processor);
    assert!(wait_for(2.0, || host.health() == Health::Running));
    host.fault_hook().stall();
    std::thread::sleep(STALL_LIMIT / 2);
    assert_eq!(host.health(), Health::Running, "a short gap is not a loss");
    assert!(
        wait_for(STALL_LIMIT.as_secs_f32() + 1.0, || host.health() == Health::Lost),
        "silence longer than the limit is"
    );
    assert!(host.stop().is_some());
}

#[test]
fn dropouts_are_counted() {
    let (_engine, processor) = engine();
    let host = AudioHost::start_virtual(&VirtualOutput::new("Interface A", SR), processor);
    let hook = host.fault_hook();
    hook.xrun();
    hook.xrun();
    assert_eq!(host.stats().dropouts(), 2);
    assert!(host.stop().is_some());
}

#[test]
fn a_device_that_cannot_open_gives_the_processor_back() {
    let (_engine, processor) = engine();
    let cfg = AudioConfig {
        device: Some("No such interface 0d-test".into()),
        fallback_to_default: false,
        ..AudioConfig::default()
    };
    match AudioHost::start_or_return(&cfg, processor) {
        Ok(_) => panic!("a missing device must not open"),
        Err(e) => assert!(e.processor.is_some(), "processor lost: {e}"),
    }
}
