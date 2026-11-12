//! Offline-render tests for the sampler: modes, quantised triggering, tempo
//! sync and the step sequencer.

mod common;

use std::sync::Arc;

use common::*;
use od_engine::*;

fn sample(frames: usize, f: impl Fn(usize) -> f32) -> Arc<SampleData> {
    SampleData::new(
        od_decode::DecodedAudio {
            sample_rate: SR,
            channels: 2,
            samples: (0..frames).flat_map(|i| [f(i), f(i)]).collect(),
        },
        SR,
    )
}

fn dc(frames: usize, v: f32) -> Arc<SampleData> {
    sample(frames, |_| v)
}

fn smp(rig: &mut Rig, c: SamplerCommand) {
    rig.send(Command::Sampler(c));
}

/// Master deck 0 playing silence on a 120 BPM grid from frame 0.
fn clock_rig() -> Rig {
    let mut rig = Rig::new(2);
    let n = 60 * SR as usize;
    rig.deck(
        0,
        DeckCommand::Load(track_from(1, vec![0.0; n * 2], 120.0, 0.0)),
    );
    rig.deck(0, DeckCommand::Play);
    rig
}

/// Frames where the left channel turns on (|x| crosses 1e-6 from below).
fn onsets(out: &[f32]) -> Vec<usize> {
    let mut v = Vec::new();
    let mut prev = 0.0f32;
    for (i, f) in out.chunks_exact(2).enumerate() {
        if prev.abs() <= 1e-6 && f[0].abs() > 1e-6 {
            v.push(i);
        }
        prev = f[0];
    }
    v
}

#[test]
fn quantised_trigger_starts_on_the_grid() {
    let mut rig = clock_rig();
    smp(
        &mut rig,
        SamplerCommand::Load {
            slot: 0,
            sample: dc(48_000, 0.5),
        },
    );
    smp(&mut rig, SamplerCommand::SetQuantize(Some(0.25)));
    rig.render(235); // 30 080 frames
    smp(&mut rig, SamplerCommand::Trigger { slot: 0 });
    rig.render(1);
    assert!(rig.state().sampler[0].pending);
    let out = rig.render(100);
    // Next 1/4-beat boundary: 36 000 = 30 080 + 128 + 5 792.
    assert_eq!(onsets(&out), vec![5792]);
    assert_eq!(out[5792 * 2], 0.5);
    let st = rig.state();
    assert!(st.sampler[0].playing && !st.sampler[0].pending);
    assert_eq!(st.sequencer.quantize, Some(0.25));

    // Without quantize the trigger starts with the next block.
    smp(&mut rig, SamplerCommand::SetQuantize(None));
    smp(&mut rig, SamplerCommand::Stop { slot: 0 });
    rig.render(100);
    smp(&mut rig, SamplerCommand::Trigger { slot: 0 });
    let out = rig.render(1);
    assert_eq!(out[0], 0.5);
}

/// [77] The MELODY bank is a bank like the others: its slots (the notes
/// CREATE sends) load, start on the master grid with QUANT and keep their
/// pitch (no tempo, so SYNC does not change their speed).
#[test]
fn melody_bank_slots_trigger_on_the_grid() {
    assert_eq!(MELODY_BANK, SAMPLER_BANKS - 2);
    let first = MELODY_BANK * SAMPLER_SLOTS;
    let octave = first + 8;
    assert!(octave < SAMPLER_TOTAL);
    let mut rig = clock_rig();
    for slot in [first, octave] {
        smp(&mut rig, SamplerCommand::Load { slot, sample: sample(48_000, |i| 0.25 + i as f32 / 96_000.0) });
        smp(&mut rig, SamplerCommand::SetSync { slot, on: true });
    }
    smp(&mut rig, SamplerCommand::SetQuantize(Some(1.0)));
    rig.render(235); // 30 080 frames
    smp(&mut rig, SamplerCommand::Trigger { slot: octave });
    rig.render(1);
    let st = rig.state();
    assert!(st.sampler[first].loaded && st.sampler[octave].pending && !st.sampler[octave].playing);
    let out = rig.render(150);
    // The next beat at 120 BPM: 48 000 = 30 080 + 128 + 17 792.
    assert_eq!(onsets(&out), vec![17_792]);
    assert_eq!(out[17_792 * 2], 0.25);
    let later = out[(17_792 + 1000) * 2];
    assert!((later - (0.25 + 1000.0 / 96_000.0)).abs() < 1e-6, "played at its own speed: {later}");
    assert!(rig.state().sampler[octave].playing);
}

#[test]
fn one_shot_loop_and_gate_modes() {
    let mut rig = clock_rig();
    smp(
        &mut rig,
        SamplerCommand::Load {
            slot: 3,
            sample: dc(1000, 0.5),
        },
    );
    smp(&mut rig, SamplerCommand::Trigger { slot: 3 });
    let out = rig.render(20);
    assert_eq!(out[999 * 2], 0.5);
    assert_eq!(out[1000 * 2], 0.0, "one-shot ends");
    assert!(!rig.state().sampler[3].playing);

    smp(
        &mut rig,
        SamplerCommand::SetMode {
            slot: 3,
            mode: SampleMode::Loop,
        },
    );
    smp(&mut rig, SamplerCommand::Trigger { slot: 3 });
    smp(&mut rig, SamplerCommand::Release { slot: 3 });
    let out = rig.render(20);
    assert!(
        out.iter().all(|&v| v == 0.5),
        "loop keeps playing, release ignored"
    );

    smp(&mut rig, SamplerCommand::Stop { slot: 3 });
    rig.render(2);
    smp(
        &mut rig,
        SamplerCommand::SetMode {
            slot: 3,
            mode: SampleMode::Gate,
        },
    );
    smp(&mut rig, SamplerCommand::Trigger { slot: 3 });
    let out = rig.render(20);
    assert!(out.iter().all(|&v| v == 0.5), "gate loops while held");
    smp(&mut rig, SamplerCommand::Release { slot: 3 });
    let out = rig.render(2);
    // Faded out over 128 frames, monotonically.
    assert!(out[0] <= 0.5 && out[64 * 2] < 0.3);
    assert!(
        out.chunks_exact(2)
            .zip(out.chunks_exact(2).skip(1))
            .all(|(a, b)| b[0] <= a[0])
    );
    assert!(out[BLOCK * 2..].iter().all(|&v| v == 0.0));
    assert!(!rig.state().sampler[3].playing);

    // Gain is ramped to the new value.
    smp(
        &mut rig,
        SamplerCommand::SetMode {
            slot: 3,
            mode: SampleMode::Loop,
        },
    );
    smp(&mut rig, SamplerCommand::SetGain { slot: 3, gain: 0.5 });
    smp(&mut rig, SamplerCommand::Trigger { slot: 3 });
    let out = rig.render(20);
    assert!(out[0] > 0.25 && (out[19 * BLOCK * 2] - 0.25).abs() < 1e-6);
}

#[test]
fn retrigger_is_click_free() {
    let mut rig = clock_rig();
    let s = sample(48_000, |i| {
        (std::f64::consts::TAU * 200.0 * i as f64 / SR as f64).sin() as f32 * 0.5
    });
    smp(&mut rig, SamplerCommand::Load { slot: 0, sample: s });
    let mut all = Vec::new();
    for _ in 0..10 {
        smp(&mut rig, SamplerCommand::Trigger { slot: 0 });
        all.extend(rig.render(13));
    }
    let step = all
        .chunks_exact(2)
        .zip(all.chunks_exact(2).skip(1))
        .map(|(a, b)| (a[0] - b[0]).abs())
        .fold(0.0f32, f32::max);
    // The sine alone moves up to 0.013 per frame.
    assert!(step < 0.03, "step {step}");
}

#[test]
fn sync_follows_the_master_tempo() {
    let mut rig = clock_rig();
    let s = sample(96_000, |i| {
        (std::f64::consts::TAU * 1000.0 * i as f64 / SR as f64).sin() as f32 * 0.5
    });
    smp(&mut rig, SamplerCommand::Load { slot: 5, sample: s });
    smp(
        &mut rig,
        SamplerCommand::SetMode {
            slot: 5,
            mode: SampleMode::Loop,
        },
    );
    smp(
        &mut rig,
        SamplerCommand::SetBpm {
            slot: 5,
            bpm: Some(100.0),
        },
    );
    smp(&mut rig, SamplerCommand::Trigger { slot: 5 });
    let f = estimate_freq(&rig.render(200));
    assert!((f - 1000.0).abs() < 1.0, "unsynced {f}");
    smp(&mut rig, SamplerCommand::SetSync { slot: 5, on: true });
    let f = estimate_freq(&rig.render(200));
    assert!((f - 1200.0).abs() < 1.0, "synced to 120 BPM: {f}");
    let st = rig.state().sampler[5];
    assert!(st.sync && st.bpm == Some(100.0));
}

#[test]
fn sequencer_plays_steps_on_the_master_beat() {
    let mut rig = clock_rig();
    smp(
        &mut rig,
        SamplerCommand::Load {
            slot: 0,
            sample: dc(100, 0.5),
        },
    );
    smp(
        &mut rig,
        SamplerCommand::SeqStep {
            slot: 0,
            step: 0,
            on: true,
        },
    );
    smp(
        &mut rig,
        SamplerCommand::SeqStep {
            slot: 0,
            step: 8,
            on: true,
        },
    );
    smp(&mut rig, SamplerCommand::SeqPlay(true));
    let out = rig.render(1000); // 128 000 frames = 5.33 beats
    // Steps are 16ths: step 0 at beat 0, step 8 at beat 2, step 0 at beat 4.
    assert_eq!(onsets(&out), vec![0, 48_000, 96_000]);
    let st = rig.state();
    assert!(st.sequencer.playing);
    assert_eq!(st.sampler[0].pattern, 0b1_0000_0001);
    assert_eq!(st.sequencer.step, ((128_000.0 / 6000.0) as u8) % 16);
    smp(&mut rig, SamplerCommand::SeqPlay(false));
    let out = rig.render(400);
    assert!(onsets(&out).is_empty(), "stopped");
}

#[test]
fn sequencer_pattern_recording() {
    let mut rig = clock_rig();
    smp(
        &mut rig,
        SamplerCommand::Load {
            slot: 1,
            sample: dc(100, 0.5),
        },
    );
    smp(&mut rig, SamplerCommand::SeqPlay(true));
    smp(&mut rig, SamplerCommand::SeqRecord(true));
    rig.render(191); // beat 1.019: nearest 16th is step 4 (behind)
    smp(&mut rig, SamplerCommand::Trigger { slot: 1 });
    let out = rig.render(1);
    assert_eq!(out[0], 0.5, "played at once");
    assert_eq!(rig.state().sampler[1].pattern, 1 << 4);
    rig.render(33); // block start 28 800 = beat 1.2: nearest is step 5 (ahead)
    smp(&mut rig, SamplerCommand::Trigger { slot: 1 });
    let out = rig.render(20);
    assert_eq!(rig.state().sampler[1].pattern, (1 << 4) | (1 << 5));
    // Not doubled: only the sequencer plays it, exactly at 30 000.
    assert_eq!(onsets(&out), vec![30_000 - 28_800]);
}

/// Loads a short DC one-shot on `slot` with the given steps set.
fn stepped(rig: &mut Rig, slot: usize, steps: &[usize]) {
    smp(rig, SamplerCommand::Load { slot, sample: dc(100, 0.5) });
    let mask = steps.iter().fold(0u16, |m, s| m | 1 << s);
    smp(rig, SamplerCommand::SeqPattern { slot, mask });
}

/// [80] Swing moves every off-beat step (the second 16th of each eighth)
/// later by its share of a step; the on-beat steps stay on the grid.
#[test]
fn swing_delays_the_off_beat_steps_by_its_share_of_a_step() {
    let mut rig = clock_rig();
    stepped(&mut rig, 0, &[0, 1, 2, 3]);
    smp(&mut rig, SamplerCommand::SeqSwing(0.5));
    smp(&mut rig, SamplerCommand::SeqPlay(true));
    let out = rig.render(200); // 25 600 frames: a step is 6 000 at 120 BPM
    // Steps 1 and 3 half a step (3 000 frames) late.
    assert_eq!(onsets(&out), vec![0, 9_000, 12_000, 21_000]);
    let st = rig.state().sequencer;
    assert!((st.swing - 0.5).abs() < 1e-6 && !st.half_time);
    // Out of range is clamped to the most swing there is.
    smp(&mut rig, SamplerCommand::SeqSwing(3.0));
    rig.render(1);
    assert_eq!(rig.state().sequencer.swing, SEQ_SWING_MAX);
    // Without swing the steps are straight again.
    let mut rig = clock_rig();
    stepped(&mut rig, 0, &[0, 1, 2, 3]);
    smp(&mut rig, SamplerCommand::SeqPlay(true));
    assert_eq!(onsets(&rig.render(200)), vec![0, 6_000, 12_000, 18_000]);
}

/// [80] Half time: a step lasts an eighth note, so the bar of steps takes
/// two bars of the master and the swing scales with it.
#[test]
fn half_time_doubles_the_step_length() {
    let mut rig = clock_rig();
    stepped(&mut rig, 0, &[0, 1, 2, 8]);
    smp(&mut rig, SamplerCommand::SeqHalfTime(true));
    smp(&mut rig, SamplerCommand::SeqPlay(true));
    let out = rig.render(1600); // 204 800 frames = 8.53 beats
    // Steps are 12 000 frames apart; step 8 on beat 4, steps 0 and 1 again
    // from beat 8.
    assert_eq!(onsets(&out), vec![0, 12_000, 24_000, 96_000, 192_000, 204_000]);
    assert!(rig.state().sequencer.half_time);
    assert_eq!(rig.state().sequencer.step, 1);
    // With swing a quarter of the (longer) step: 3 000 frames late.
    let mut rig = clock_rig();
    stepped(&mut rig, 0, &[0, 1]);
    smp(&mut rig, SamplerCommand::SeqHalfTime(true));
    smp(&mut rig, SamplerCommand::SeqSwing(0.25));
    smp(&mut rig, SamplerCommand::SeqPlay(true));
    assert_eq!(onsets(&rig.render(200)), vec![0, 15_000]);
}

/// [80] PLAY from a beat: nothing sounds before it, step 1 lands on it
/// and the steps count from there; from "now" step 1 is the next block.
/// Recording while waiting lands on step 1 at the earliest.
#[test]
fn play_from_a_beat_starts_step_one_there() {
    let mut rig = clock_rig();
    stepped(&mut rig, 0, &[0, 4]);
    rig.render(100); // 12 800 frames: beat 0.53
    smp(&mut rig, SamplerCommand::SeqPlayFrom(Some(4.0)));
    rig.render(1);
    let st = rig.state().sequencer;
    assert!(st.playing && st.waiting, "waits for its beat");
    let out = rig.render(800); // to frame 115 328
    // Beat 4 = frame 96 000 (step 0), beat 5 = 120 000 (step 4) is later.
    assert_eq!(onsets(&out), vec![96_000 - 12_928]);
    assert!(!rig.state().sequencer.waiting);
    // From now: step 1 on the next block, wherever the beat is.
    let mut rig = clock_rig();
    stepped(&mut rig, 0, &[0, 4]);
    rig.render(77); // 9 856 frames
    smp(&mut rig, SamplerCommand::SeqPlayFrom(None));
    let out = rig.render(200);
    assert_eq!(onsets(&out), vec![0, 6_000 * 4]);
    // Recording while step 1 waits: a press lands on step 1.
    let mut rig = clock_rig();
    smp(&mut rig, SamplerCommand::Load { slot: 2, sample: dc(100, 0.5) });
    rig.render(10);
    smp(&mut rig, SamplerCommand::SeqPlayFrom(Some(4.0)));
    smp(&mut rig, SamplerCommand::SeqRecord(true));
    rig.render(1);
    smp(&mut rig, SamplerCommand::Trigger { slot: 2 });
    rig.render(1);
    assert_eq!(rig.state().sampler[2].pattern, 1);
}

/// [80] The DRUMS bank's slots hold the synthesised kit; the sequencer
/// plays their patterns on the master grid (a four-to-the-floor kick, the
/// snare on 2 and 4), each sound starting on its step.
#[test]
fn the_sequencer_plays_the_kit_slots_on_the_master_grid() {
    let mut rig = clock_rig();
    let base = DRUMS_BANK * SAMPLER_SLOTS;
    assert!(base + SAMPLER_SLOTS <= SAMPLER_TOTAL);
    let kit = drums::kit("tight").unwrap();
    for (i, voice) in [(0, drums::Voice::ALL[0]), (1, drums::Voice::ALL[1])] {
        let mono = drums::render(voice, kit.pads[i], SR);
        let data = SampleData::new(
            od_decode::DecodedAudio { sample_rate: SR, channels: 2, samples: mono.iter().flat_map(|&x| [x, x]).collect() },
            SR,
        );
        smp(&mut rig, SamplerCommand::Load { slot: base + i, sample: data });
    }
    smp(&mut rig, SamplerCommand::SeqPattern { slot: base, mask: 0b0001_0001_0001_0001 });
    smp(&mut rig, SamplerCommand::SeqPattern { slot: base + 1, mask: 0b0001_0000_0001_0000 });
    smp(&mut rig, SamplerCommand::SeqPlay(true));
    let out = rig.render(750); // 96 000 frames: one bar
    // Energy right after each beat: the kick on every beat, louder where
    // the snare joins it (beats 2 and 4).
    let energy = |at: usize| out[at * 2..(at + 2_000) * 2].iter().map(|x| x * x).sum::<f32>();
    let beats: Vec<f32> = (0..4).map(|b| energy(b * 24_000)).collect();
    assert!(beats.iter().all(|e| *e > 1.0), "a kick on every beat: {beats:?}");
    assert!(beats[1] > beats[0] && beats[3] > beats[2], "the snare on 2 and 4: {beats:?}");
    // Silent just before each beat, sounding within a few frames of it.
    for b in 1..4 {
        let at = b * 24_000;
        assert!(out[(at - 200) * 2..at * 2].iter().all(|x| x.abs() < 1e-3), "quiet before beat {b}");
        assert!(out[at * 2..(at + 40) * 2].iter().any(|x| x.abs() > 1e-3), "sounds on beat {b}");
    }
    let st = rig.state();
    assert!(st.sampler[base].loaded && st.sampler[base + 1].loaded);
}

#[test]
fn sampler_cue_to_headphones() {
    let mut rig = clock_rig();
    smp(
        &mut rig,
        SamplerCommand::Load {
            slot: 0,
            sample: dc(48_000, 0.25),
        },
    );
    smp(&mut rig, SamplerCommand::Trigger { slot: 0 });
    rig.send(Command::Mixer(MixerCommand::SetHeadphoneMix(0.0)));
    rig.render(5);
    assert!(rig.phones[..BLOCK * 2].iter().all(|&v| v == 0.0));
    rig.send(Command::Mixer(MixerCommand::SetSamplerCue(true)));
    rig.render(5);
    assert!(
        rig.phones[..BLOCK * 2]
            .iter()
            .all(|&v| (v - 0.25).abs() < 1e-6)
    );
    assert!(rig.state().sampler_cue);
}

/// A Tempo FX unit inserted on the sampler only changes the sampler: a
/// short sample echoes after it has ended.
#[test]
fn tempo_fx_on_the_sampler() {
    let run = |target: FxTarget| {
        let mut rig = clock_rig();
        smp(
            &mut rig,
            SamplerCommand::Load {
                slot: 0,
                sample: dc(2_400, 0.5),
            },
        );
        for c in [
            TempoFxCommand::SetKind(TempoFxKind::Echo),
            TempoFxCommand::SetBeats(0.5),
            TempoFxCommand::SetLevel(1.0),
            TempoFxCommand::SetTarget(target),
            TempoFxCommand::SetOn(true),
        ] {
            rig.send(Command::TempoFx(0, c));
        }
        smp(&mut rig, SamplerCommand::Trigger { slot: 0 });
        let out = rig.render(200); // 25 600 frames ≈ 1 beat at 120 BPM
        // Half a beat (12 000 frames) after the sample: the echo.
        let tail: f32 = (12_000..14_000).map(|i| out[i * 2].abs()).sum();
        (tail, rig.state().tempo_fx[0].target)
    };
    let (echo, target) = run(FxTarget::Sampler);
    let (dry, _) = run(FxTarget::Channel(1));
    assert_eq!(target, FxTarget::Sampler);
    assert!(echo > 10.0, "sampler echo: {echo}");
    assert!(
        dry < 1e-3,
        "no echo when the unit sits on another channel: {dry}"
    );
}

/// [71] The audition voice (CREATE's HEAR) plays a sample once beside the
/// slots: to the master at unity, ignoring the sampler level, or to the cue
/// only; a stop fades it out.
#[test]
fn audition_plays_once_on_master_or_cue() {
    let mut rig = clock_rig();
    rig.send(Command::Mixer(MixerCommand::SetSamplerLevel(0.0)));
    rig.send(Command::Mixer(MixerCommand::SetHeadphoneMix(0.0)));
    smp(&mut rig, SamplerCommand::Audition { sample: dc(1000, 0.5), cue: false });
    let out = rig.render(20);
    assert_eq!(out[0], 0.5, "on the master at once, at unity");
    assert_eq!(out[999 * 2], 0.5);
    assert_eq!(out[1000 * 2], 0.0, "once");
    assert!(!rig.state().auditioning);
    assert!(rig.state().sampler.iter().all(|s| !s.loaded), "no slot was used");
    // On the cue: the master stays silent.
    smp(&mut rig, SamplerCommand::Audition { sample: dc(48_000, 0.25), cue: true });
    let out = rig.render(5);
    assert!(out.iter().all(|&v| v == 0.0), "not on the master");
    assert!(rig.phones[..BLOCK * 2].iter().all(|&v| (v - 0.25).abs() < 1e-6), "on the cue");
    assert!(rig.state().auditioning);
    smp(&mut rig, SamplerCommand::StopAudition);
    rig.render(3);
    assert!(rig.phones[..BLOCK * 2].iter().all(|&v| v == 0.0), "stopped after its fade");
    assert!(!rig.state().auditioning);
}

/// [74] A note waits for its beat of the master clock and stops on its own
/// beat; the next one starts on its beat while the sounding one fades out;
/// a stop drops a waiting note.
#[test]
fn audition_at_starts_and_stops_on_the_master_beat() {
    let mut rig = clock_rig();
    rig.send(Command::Mixer(MixerCommand::SetSamplerLevel(0.0)));
    rig.send(Command::Mixer(MixerCommand::SetHeadphoneMix(0.0)));
    rig.render(10); // 1 280 frames
    // Beat 1 is frame 24 000 at 120 BPM; beat 1.5 frame 36 000.
    smp(&mut rig, SamplerCommand::AuditionAt { sample: dc(48_000, 0.5), cue: false, beat: 1.0, until: 1.5 });
    let out = rig.render(300); // to 39 680
    assert!(rig.state().clock_beat > 1.6, "the clock follows the master: {}", rig.state().clock_beat);
    assert_eq!(onsets(&out), vec![24_000 - 1_280], "on beat 1");
    assert_eq!(out[(24_000 - 1_280 + 100) * 2], 0.5);
    assert_eq!(out[(36_000 - 1_280 - 1) * 2], 0.5, "until beat 1.5");
    assert_eq!(out[(36_000 - 1_280 + 200) * 2], 0.0, "then faded out");
    assert!(!rig.state().auditioning);
    // Two notes in a row: the second starts on beat 2 and the first fades.
    smp(&mut rig, SamplerCommand::AuditionAt { sample: dc(96_000, 0.5), cue: false, beat: 1.6, until: 4.0 });
    rig.render(1);
    assert!(rig.state().auditioning, "a past beat starts it at once");
    smp(&mut rig, SamplerCommand::AuditionAt { sample: dc(96_000, 0.25), cue: false, beat: 2.0, until: 4.0 });
    let from = 39_680 + 128;
    let out = rig.render(100);
    assert_eq!(out[(48_000 - from - 1) * 2], 0.5, "the first until beat 2");
    assert_eq!(out[(48_000 - from + 200) * 2], 0.25, "the second from beat 2, alone after the fade");
    // A stop drops a note still waiting.
    smp(&mut rig, SamplerCommand::AuditionAt { sample: dc(1_000, 0.5), cue: false, beat: 100.0, until: 101.0 });
    smp(&mut rig, SamplerCommand::StopAudition);
    rig.render(3);
    assert!(!rig.state().auditioning);
}

// ── [81] A pattern as a loop ────────────────────────────────────────────

fn one_shot(sample: Arc<SampleData>, steps: &[usize]) -> LoopPad {
    LoopPad { sample, pattern: steps.iter().fold(0u16, |m, s| m | 1 << s), mode: SampleMode::OneShot, gain: 1.0, speed: 1.0 }
}

/// [81] A loop is exactly its bars long at the tempo (rounded to a frame),
/// each pad first sounds on its first step, and a sound ringing past the
/// end wraps to the start: alone it sounds on there, and where the pad
/// hits step 1 again that hit cuts it with a retrigger's fade.
#[test]
fn a_loop_is_its_bars_long_and_a_tail_wraps_to_its_start() {
    let none = render_loop(&[], 1, 120.0, SR, 0.0, false);
    assert_eq!(none.audio.len() / 2, 96_000, "one bar at 120 BPM: 2 s");
    assert_eq!(loop_frames(4, 124.0, SR), (16.0 * 60.0 / 124.0 * f64::from(SR)).round() as usize);
    assert_eq!(render_loop(&[], 4, 124.0, SR, 0.0, false).audio.len() / 2, 371_613);
    assert_eq!(loop_frames(2, 0.0, SR), 0, "no tempo, no loop");
    // 10 000 frames on step 16 (frame 90 000): 6 000 before the end, the
    // other 4 000 at the start.
    let long = dc(10_000, 0.5);
    let l = render_loop(&[one_shot(long.clone(), &[15])], 1, 120.0, SR, 0.0, false);
    assert_eq!(l.first, [Some(90_000)]);
    assert_eq!((l.audio[89_999 * 2], l.audio[90_000 * 2], l.audio[95_999 * 2]), (0.0, 0.5, 0.5));
    assert_eq!((l.audio[0], l.audio[3_999 * 2], l.audio[4_000 * 2]), (0.5, 0.5, 0.0), "the tail at the start");
    assert!(l.audio.chunks_exact(2).all(|f| f[0] == f[1]));
    // Step 1 as well: its hit cuts the tail, which fades over 128 frames.
    let l = render_loop(&[one_shot(long, &[0, 15])], 1, 120.0, SR, 0.0, false);
    assert_eq!(l.first, [Some(0)]);
    assert_eq!(l.audio[0], 1.0, "the new hit and the tail at full");
    assert!((l.audio[64 * 2] - (0.5 + 0.5 * 64.0 / 128.0)).abs() < 1e-6, "half faded");
    assert_eq!(l.audio[128 * 2], 0.5, "only the new hit after the fade");
    // Two bars repeat the 16 steps; every pad's first hit is reported.
    let l = render_loop(&[one_shot(dc(100, 0.5), &[4]), one_shot(dc(100, 0.25), &[]), one_shot(dc(100, 0.25), &[1])], 2, 120.0, SR, 0.0, false);
    assert_eq!(onsets(&l.audio), [6_000, 24_000, 102_000, 120_000]);
    assert_eq!(l.first, [Some(24_000), None, Some(6_000)]);
}

/// [81] Swing and half time put the steps where the sequencer plays them;
/// half time takes two bars for the 16 steps.
#[test]
fn a_loop_swings_and_plays_half_time_like_the_sequencer() {
    let l = render_loop(&[one_shot(dc(100, 0.5), &[0, 1, 2, 3])], 1, 120.0, SR, 0.5, false);
    assert_eq!(onsets(&l.audio), [0, 9_000, 12_000, 21_000]);
    let l = render_loop(&[one_shot(dc(100, 0.5), &[0, 1, 8])], 2, 120.0, SR, 0.25, true);
    assert_eq!(onsets(&l.audio), [0, 15_000, 96_000]);
    let l = render_loop(&[one_shot(dc(100, 0.5), &[0, 8])], 4, 120.0, SR, 0.0, true);
    assert_eq!(onsets(&l.audio), [0, 96_000, 192_000, 288_000], "the steps twice in four bars");
}

/// [81] The loop is the sequencer's own sound: a bar of the kit played by
/// the engine, once its first bar's tails ring into the second, is the
/// rendered bar; the same pattern renders the same samples every time.
#[test]
fn a_loop_sounds_as_the_sequencer_plays_it_and_renders_alike_every_time() {
    let kit = drums::kit("tight").unwrap();
    let voice = |i: usize| {
        let mono = drums::render(drums::Voice::ALL[i], kit.pads[i], SR);
        SampleData::new(od_decode::DecodedAudio { sample_rate: SR, channels: 2, samples: mono.iter().flat_map(|&x| [x * 0.3, x * 0.3]).collect() }, SR)
    };
    // Kick on the beats, open hat on the off-beats (rings long), clap on
    // step 16 (rings over the end), a loop-mode tone on step 3.
    let tone = sample(5_000, |i| (i as f32 * 0.01).sin() * 0.1);
    let pads = vec![
        one_shot(voice(0), &[0, 4, 8, 12]),
        one_shot(voice(3), &[2, 6, 10, 14]),
        one_shot(voice(4), &[15]),
        LoopPad { mode: SampleMode::Loop, gain: 0.5, ..one_shot(tone, &[3]) },
    ];
    let swing = 0.25;
    let l = render_loop(&pads, 1, 120.0, SR, swing, false);
    assert_eq!(l, render_loop(&pads, 1, 120.0, SR, swing, false), "the same every time");
    let mut rig = clock_rig();
    for (i, p) in pads.iter().enumerate() {
        smp(&mut rig, SamplerCommand::Load { slot: i, sample: p.sample.clone() });
        smp(&mut rig, SamplerCommand::SeqPattern { slot: i, mask: p.pattern });
        smp(&mut rig, SamplerCommand::SetGain { slot: i, gain: p.gain });
        smp(&mut rig, SamplerCommand::SetMode { slot: i, mode: p.mode });
    }
    smp(&mut rig, SamplerCommand::SeqSwing(swing as f32));
    smp(&mut rig, SamplerCommand::SeqPlay(true));
    let played = rig.render(1_500); // 192 000 frames: two bars
    let bar2 = &played[96_000 * 2..192_000 * 2];
    let worst = bar2.iter().zip(&l.audio).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
    assert!(worst < 1e-4, "the second bar played is the loop: off by {worst}");
    assert!(l.audio.iter().map(|x| x.abs()).fold(0.0f32, f32::max) > 0.05, "and it sounds");
}
