//! Dragging the large Perform waveform lanes with the mouse (roadmap
//! [50]). A paused deck scrubs: the waveform follows the pointer 1:1 at
//! the current zoom and stays where it is put. A playing deck never jumps:
//! the drag's speed becomes a temporary pitch bend (the engine's nudge),
//! like a hand on a platter, and the deck returns to its tempo on release;
//! with SLIP on, release returns to where the track would have been. The
//! performance lock does not block either (nudging and scrubbing a paused
//! deck are both allowed under it).

use crate::app::{App, handler};
use crate::engine;
use od_engine::DeckCommand;
use slint::ComponentHandle;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Largest bend a drag gives (±25 % speed).
const MAX_BEND: f64 = 0.25;
/// Without a move for this long the bend falls back to 0 (a held, still
/// hand lets the record run).
const STILL: Duration = Duration::from_millis(80);

/// One drag in progress.
#[derive(Clone, Debug)]
struct Drag {
    deck: usize,
    playing: bool,
    /// Position at the start (seconds).
    from: f64,
    /// SLIP was on: release returns to `from` plus the time at tempo.
    slip: bool,
    tempo: f64,
    started: Instant,
    /// Last offset (seconds of audio) and when it came.
    last: (f64, Instant),
}

#[derive(Default)]
pub(crate) struct LaneDragState {
    drag: RefCell<Option<Drag>>,
    still: RefCell<Option<slint::Timer>>,
}

/// The bend for a drag that moved `d_secs` of audio in `dt` (positive:
/// later in the track, so faster), clamped.
pub(crate) fn bend_for(d_secs: f64, dt: Duration) -> f64 {
    let t = dt.as_secs_f64().max(0.004);
    (d_secs / t).clamp(-MAX_BEND, MAX_BEND)
}

/// Where a slipped deck is on release: where it started plus the time
/// played at its tempo.
pub(crate) fn slip_return(from: f64, tempo: f64, elapsed: Duration) -> f64 {
    from + elapsed.as_secs_f64() * tempo
}

/// The readout: the scrub offset in seconds, or the bend in percent.
pub(crate) fn readout(playing: bool, value: f64) -> String {
    let s = if playing {
        format!("{:+.1} %", value * 100.0)
    } else {
        format!("{:+.2} s", value)
    };
    s.replace('-', "\u{2212}")
}

fn set_text(app: &App, deck: usize, text: String) {
    let g = app.ui.global::<crate::LaneDrag>();
    g.set_deck(deck as i32 + 1);
    g.set_text(text.into());
}

fn start(app: &Rc<App>, deck: usize) {
    let Some(s) = engine::snapshot(app).map(|s| s.decks[deck]) else { return };
    if s.track.is_none() {
        return;
    }
    let now = Instant::now();
    *app.lane_drag.drag.borrow_mut() = Some(Drag {
        deck,
        playing: s.playing,
        from: s.position,
        slip: s.slip,
        tempo: s.tempo,
        started: now,
        last: (0.0, now),
    });
    set_text(app, deck, readout(s.playing, 0.0));
}

fn to(app: &Rc<App>, deck: usize, secs: f64) {
    let Some(mut d) = app.lane_drag.drag.borrow().clone().filter(|d| d.deck == deck) else { return };
    let now = Instant::now();
    if !d.playing {
        engine::deck_cmd(app, deck, DeckCommand::Seek { seconds: (d.from + secs).max(0.0) });
        set_text(app, deck, readout(false, secs));
        return;
    }
    let bend = bend_for(secs - d.last.0, now.duration_since(d.last.1));
    d.last = (secs, now);
    engine::deck_cmd(app, deck, DeckCommand::SetPitchBend(bend as f32));
    set_text(app, deck, readout(true, bend));
    *app.lane_drag.drag.borrow_mut() = Some(d);
    // A still hand lets the deck run at its tempo again.
    let timer = slint::Timer::default();
    let weak = Rc::downgrade(app);
    timer.start(slint::TimerMode::SingleShot, STILL, move || {
        if let Some(a) = weak.upgrade() {
            if a.lane_drag.drag.borrow().as_ref().is_some_and(|d| d.deck == deck) {
                engine::deck_cmd(&a, deck, DeckCommand::SetPitchBend(0.0));
                set_text(&a, deck, readout(true, 0.0));
            }
        }
    });
    *app.lane_drag.still.borrow_mut() = Some(timer);
}

fn end(app: &Rc<App>, deck: usize) {
    let Some(d) = app.lane_drag.drag.borrow_mut().take().filter(|d| d.deck == deck) else { return };
    app.lane_drag.still.borrow_mut().take();
    if d.playing {
        engine::deck_cmd(app, deck, DeckCommand::SetPitchBend(0.0));
        if d.slip {
            let at = slip_return(d.from, d.tempo, d.started.elapsed());
            engine::deck_cmd(app, deck, DeckCommand::Seek { seconds: at });
        }
    }
    app.ui.global::<crate::LaneDrag>().set_deck(0);
}

pub(crate) fn wire(app: &Rc<App>) {
    let g = app.ui.global::<crate::LaneDrag>();
    g.on_start(handler!(app, |a, deck: i32| start(&a, (deck - 1).max(0) as usize)));
    g.on_to(handler!(app, |a, deck: i32, secs: f32| to(&a, (deck - 1).max(0) as usize, secs as f64)));
    g.on_end(handler!(app, |a, deck: i32| end(&a, (deck - 1).max(0) as usize)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_speed_becomes_a_bounded_bend() {
        // 0.05 s of audio in 0.1 s: half again as fast as the record runs.
        assert!((bend_for(0.005, Duration::from_millis(100)) - 0.05).abs() < 1e-9);
        assert!(bend_for(-0.005, Duration::from_millis(100)) < 0.0, "dragging back slows");
        assert_eq!(bend_for(10.0, Duration::from_millis(16)), MAX_BEND);
        assert_eq!(bend_for(-10.0, Duration::from_millis(16)), -MAX_BEND);
        assert!(bend_for(0.001, Duration::ZERO).is_finite());
    }

    #[test]
    fn slip_returns_to_where_the_track_would_be() {
        assert!((slip_return(10.0, 1.0, Duration::from_secs(2)) - 12.0).abs() < 1e-9);
        assert!((slip_return(10.0, 1.1, Duration::from_secs(2)) - 12.2).abs() < 1e-9);
    }

    #[test]
    fn readout_shows_seconds_or_percent() {
        assert_eq!(readout(false, 0.42), "+0.42 s");
        assert_eq!(readout(false, -1.5), "\u{2212}1.50 s");
        assert_eq!(readout(true, 0.031), "+3.1 %");
    }
}
