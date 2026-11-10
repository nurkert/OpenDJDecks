//! A cued start that waits for a beat of the master clock (start on the
//! master's next phrase, [56]).

mod common;
use common::*;
use od_engine::*;

/// Deck 1 plays at 120 BPM (one beat = 0.5 s); deck 2 is cued and armed.
fn rig_with_master() -> Rig {
    let mut rig = Rig::new(2);
    rig.deck(0, DeckCommand::Load(sine_track(1, 60.0, 50.0, 0.3, 120.0)));
    rig.deck(1, DeckCommand::Load(sine_track(2, 60.0, 80.0, 0.3, 120.0)));
    rig.deck(0, DeckCommand::Play);
    rig.render(4);
    rig
}

#[test]
fn armed_deck_starts_on_the_master_beat() {
    let mut rig = rig_with_master();
    let target = rig.state().decks[0].beat.floor() + 6.0;
    rig.send(Command::StartOnBeat { deck: 1, beat: Some(target) });
    let mut started_at = None;
    for _ in 0..4_000 {
        let before = rig.state().decks[0].beat;
        rig.render(1);
        let st = rig.state();
        if st.decks[1].playing {
            assert!(st.decks[1].start_on_beat.is_none());
            started_at = Some(before);
            break;
        }
        assert_eq!(st.decks[1].start_on_beat, Some(target), "it waits");
    }
    let before = started_at.expect("the deck started");
    // It starts in the block that holds the beat: at most one block early.
    let block_beats = BLOCK as f64 / SR as f64 * 2.0;
    assert!(
        before <= target && target - before <= block_beats + 1e-9,
        "started at master beat {before}, target {target}"
    );
}

#[test]
fn play_cue_or_a_cancel_drop_the_wait() {
    let mut rig = rig_with_master();
    let far = rig.state().decks[0].beat + 100.0;
    rig.send(Command::StartOnBeat { deck: 1, beat: Some(far) });
    rig.render(2);
    assert_eq!(rig.state().decks[1].start_on_beat, Some(far));
    rig.send(Command::StartOnBeat { deck: 1, beat: None });
    rig.render(2);
    assert!(rig.state().decks[1].start_on_beat.is_none());
    assert!(!rig.state().decks[1].playing);

    rig.send(Command::StartOnBeat { deck: 1, beat: Some(far) });
    rig.render(2);
    rig.deck(1, DeckCommand::Cue(true));
    rig.deck(1, DeckCommand::Cue(false));
    rig.render(2);
    assert!(rig.state().decks[1].start_on_beat.is_none(), "CUE cancels");
}

#[test]
fn without_a_playing_master_it_starts_at_once() {
    let mut rig = Rig::new(2);
    rig.deck(0, DeckCommand::Load(sine_track(1, 30.0, 50.0, 0.3, 120.0)));
    rig.deck(1, DeckCommand::Load(sine_track(2, 30.0, 80.0, 0.3, 120.0)));
    rig.render(2);
    rig.send(Command::StartOnBeat { deck: 1, beat: Some(64.0) });
    rig.render(2);
    let st = rig.state();
    assert!(st.decks[1].playing);
    assert!(st.decks[1].start_on_beat.is_none());
}
