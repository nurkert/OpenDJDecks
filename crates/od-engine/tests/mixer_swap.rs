//! EQ kills and the one-tap EQ swap between channels, now and quantised to
//! the master clock's beat or bar.

mod common;
use common::*;
use od_engine::*;

fn swap(rig: &mut Rig, at: SwapAt) {
    rig.send(Command::Mixer(MixerCommand::SwapEq {
        a: 0,
        b: 1,
        band: 0,
        at,
    }));
}

/// Deck 1 plays at 120 BPM (one beat = 0.5 s); deck 2's LOW is killed.
fn playing_rig() -> Rig {
    let mut rig = Rig::new(2);
    rig.deck(0, DeckCommand::Load(sine_track(1, 30.0, 50.0, 0.3, 120.0)));
    rig.deck(1, DeckCommand::Load(sine_track(2, 30.0, 50.0, 0.3, 120.0)));
    rig.send(Command::Channel(
        1,
        ChannelCommand::SetEqKill { band: 0, on: true },
    ));
    rig.deck(0, DeckCommand::Play);
    rig
}

/// Renders block by block until the swap has happened; returns the master
/// beat of the block it happened in.
fn beat_of_swap(rig: &mut Rig) -> f64 {
    for _ in 0..2_000 {
        let before = rig.state().decks[0].beat;
        rig.render(1);
        let st = rig.state();
        if st.channels[0].eq_kill[0] {
            assert!(st.pending_swap.is_none());
            assert!(!st.channels[1].eq_kill[0], "deck 2 got its LOW back");
            return before;
        }
        assert!(st.pending_swap.is_some(), "the swap waits");
    }
    panic!("the swap never happened");
}

#[test]
fn kill_holds_the_band_down_and_releases_to_the_knob() {
    let mut rig = Rig::new(2);
    rig.deck(0, DeckCommand::Load(sine_track(1, 10.0, 50.0, 0.5, 120.0)));
    rig.deck(0, DeckCommand::Play);
    let reference = rms(&rig.render(400)[..]);
    rig.send(Command::Channel(
        0,
        ChannelCommand::SetEqKill { band: 0, on: true },
    ));
    rig.render(100);
    let killed = rms(&rig.render(400)[..]);
    assert!(20.0 * (killed / reference).log10() < -40.0);
    let st = rig.state();
    assert_eq!(st.channels[0].eq_kill, [true, false, false]);
    assert_eq!(st.channels[0].eq[0], 0.0, "the knob stays where it was");
    rig.send(Command::Channel(
        0,
        ChannelCommand::SetEqKill { band: 0, on: false },
    ));
    rig.render(100);
    let back = rms(&rig.render(400)[..]);
    assert!((20.0 * (back / reference).log10()).abs() < 0.5);
}

#[test]
fn swap_now_exchanges_at_once() {
    let mut rig = playing_rig();
    rig.send(Command::Channel(0, ChannelCommand::SetEq { band: 0, value: 0.4 }));
    rig.render(2);
    swap(&mut rig, SwapAt::Now);
    rig.render(1);
    let st = rig.state();
    assert_eq!(st.channels[0].eq_kill[0], true);
    assert_eq!(st.channels[1].eq_kill[0], false);
    assert_eq!(st.channels[1].eq[0], 0.4, "the knob value moves with it");
    assert_eq!(st.channels[0].eq[0], 0.0);
    assert!(st.pending_swap.is_none());
}

#[test]
fn swap_on_the_next_beat() {
    let mut rig = playing_rig();
    rig.render(60); // beat ≈ 0.32
    swap(&mut rig, SwapAt::Beat);
    let beat = beat_of_swap(&mut rig);
    let block = BLOCK as f64 / SR as f64 * 2.0; // beats per block at 120 BPM
    assert!(beat <= 1.0 && beat >= 1.0 - block - 1e-9, "swapped at beat {beat}");
}

#[test]
fn swap_on_the_next_bar() {
    let mut rig = playing_rig();
    rig.render(250); // beat ≈ 1.33, the 2 of the first bar
    swap(&mut rig, SwapAt::Bar);
    let beat = beat_of_swap(&mut rig);
    let block = BLOCK as f64 / SR as f64 * 2.0;
    assert!(beat <= 4.0 && beat >= 4.0 - block - 1e-9, "swapped at beat {beat}");
}

#[test]
fn a_second_tap_cancels_a_waiting_swap() {
    let mut rig = playing_rig();
    rig.render(10);
    swap(&mut rig, SwapAt::Bar);
    rig.render(1);
    assert!(rig.state().pending_swap.is_some());
    swap(&mut rig, SwapAt::Bar);
    rig.render(800);
    let st = rig.state();
    assert!(st.pending_swap.is_none());
    assert_eq!(st.channels[1].eq_kill[0], true, "nothing was swapped");
}

#[test]
fn without_a_playing_master_a_quantised_swap_happens_at_once() {
    let mut rig = playing_rig();
    rig.deck(0, DeckCommand::Pause);
    rig.render(2);
    swap(&mut rig, SwapAt::Bar);
    rig.render(1);
    assert!(rig.state().channels[0].eq_kill[0]);
}
