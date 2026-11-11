//! The test tone reaches the master and headphone outputs, and stops.

use od_engine::{Command, Engine, EngineConfig, MixerCommand, Outputs};

fn peak(buf: &[f32]) -> f32 {
    buf.iter().fold(0.0, |p, s| p.max(s.abs()))
}

#[test]
fn test_tone_on_master_and_headphones_and_it_stops() {
    let cfg = EngineConfig::default();
    let (mut engine, mut processor) = Engine::new(cfg);
    let mut master = vec![0.0; 2048];
    let mut phones = vec![0.0; 2048];
    let render = |p: &mut od_engine::EngineProcessor, m: &mut [f32], h: &mut [f32]| {
        for _ in 0..4 {
            p.process(1024, &mut Outputs { master: m, headphones: Some(h), booth: None, decks: None });
        }
    };
    render(&mut processor, &mut master, &mut phones);
    assert!(peak(&master) < 1e-6, "silent at first");
    engine
        .send(Command::Mixer(MixerCommand::SetTestTone(true)))
        .unwrap();
    render(&mut processor, &mut master, &mut phones);
    assert!(peak(&master) > 0.05, "tone on the master");
    assert!(peak(&master) < 0.2, "and quiet");
    assert!(peak(&phones) > 0.01, "and on the headphones");
    engine
        .send(Command::Mixer(MixerCommand::SetTestTone(false)))
        .unwrap();
    render(&mut processor, &mut master, &mut phones);
    render(&mut processor, &mut master, &mut phones);
    assert!(peak(&master) < 1e-6, "stops");
}
