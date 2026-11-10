//! The performance lock and what the app does while decks play (roadmap
//! [11]).
//!
//! Locked, nothing can happen by accident: no track is loaded onto a deck
//! that plays, nothing is deleted (lists, list entries, tracks, cues, stick
//! registrations) and no beatgrid changes. Every blocked action says so in
//! the status line.
//!
//! While a deck plays (locked or not), the computer is kept awake
//! ([`crate::keep_awake`]) and background work waits: automatic analysis
//! (a track asked for by hand still runs), the stem separation queue and
//! automatic stick sync. It goes on when the decks stop.

use crate::app::{App, handler};
use crate::keep_awake::KeepAwake;
use slint::ComponentHandle;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Decks must have stopped this long before background work resumes (a
/// track change is not a stop).
const RESUME_AFTER: Duration = Duration::from_secs(5);

/// What the lock blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Blocked {
    /// Loading onto deck N (0-based), which plays.
    Load(usize),
    Delete,
    Grid,
}

#[derive(Default)]
struct Busy {
    awake: Option<KeepAwake>,
    /// Background work is paused.
    paused: bool,
    last_playing: Option<Instant>,
}

thread_local! {
    static LOCKED: Cell<bool> = const { Cell::new(false) };
    static BUSY: RefCell<Busy> = RefCell::new(Busy::default());
}

pub(crate) fn locked() -> bool {
    LOCKED.with(|l| l.get())
}

pub(crate) fn set_locked(app: &App, on: bool) {
    LOCKED.with(|l| l.set(on));
    app.ui.global::<crate::PerformLock>().set_locked(on);
    app.status(if on {
        od_i18n::tr("Locked for the set: no loading onto a playing deck, no deleting, no grid edits")
    } else {
        od_i18n::tr("Unlocked")
    });
}

pub(crate) fn toggle(app: &App) {
    set_locked(app, !locked());
}

/// The message for a blocked action.
pub(crate) fn message(what: Blocked) -> String {
    match what {
        Blocked::Load(d) => od_i18n::trf(
            "Locked: deck {} is playing. Load onto a stopped deck, or unlock",
            &[&(d + 1)],
        ),
        Blocked::Delete => od_i18n::tr("Locked: nothing is deleted until you unlock").into(),
        Blocked::Grid => od_i18n::tr("Locked: the beatgrid stays as it is until you unlock").into(),
    }
}

impl App {
    /// True (and says why) when the lock blocks `what`.
    pub(crate) fn guard_locked(&self, what: Blocked) -> bool {
        if !locked() {
            return false;
        }
        self.status(message(what));
        true
    }
}

/// True when the lock keeps a track off `deck`: locked, and the deck plays.
pub(crate) fn blocks_load(app: &App, deck: usize) -> bool {
    if !locked() || deck >= crate::decks::DECKS {
        return false;
    }
    if deck == crate::decks::PREVIEW_DECK && app.decks.borrow().deck_count <= deck {
        return false; // the Prepare preview player
    }
    crate::engine::snapshot(app).is_some_and(|s| s.decks[deck].playing)
}

/// A deck of Perform plays (the Prepare preview does not count).
fn decks_playing(app: &App, state: &od_engine::EngineState) -> bool {
    let count = app.decks.borrow().deck_count;
    (0..count.min(crate::decks::DECKS))
        .filter(|d| *d != crate::decks::PREVIEW_DECK || count > crate::decks::PREVIEW_DECK)
        .any(|d| state.decks[d].playing)
}

/// Pauses or resumes background work.
fn set_background(app: &App, pause: bool) {
    app.analysis.set_paused(pause);
    crate::stems_batch::set_paused(pause);
    app.device_manager
        .send(od_devices::Command::SetAutoSync(!pause));
    if !pause {
        let _ = app.analysis.wake.send(());
    }
}

/// Frame tick: keep awake and pause background work while decks play.
pub(crate) fn frame(app: &App, state: &od_engine::EngineState) {
    let playing = decks_playing(app, state);
    let change = BUSY.with(|b| {
        let mut b = b.borrow_mut();
        if playing {
            b.last_playing = Some(Instant::now());
        }
        let busy = playing
            || b
                .last_playing
                .is_some_and(|t| t.elapsed() < RESUME_AFTER);
        if busy == b.paused {
            return None;
        }
        b.paused = busy;
        b.awake = busy.then(KeepAwake::acquire);
        let note = b
            .awake
            .as_ref()
            .map(|a| a.note().to_string())
            .unwrap_or_default();
        Some((busy, note))
    });
    let Some((busy, note)) = change else { return };
    set_background(app, busy);
    let g = app.ui.global::<crate::PerformLock>();
    g.set_playing(busy);
    g.set_awake_note(note.into());
    tracing::info!(busy, "decks playing: keep awake and pause background work");
}

/// Ends everything this module holds (the app quits).
pub(crate) fn release() {
    BUSY.with(|b| {
        let mut b = b.borrow_mut();
        b.awake = None;
        b.paused = false;
    });
}

/// Whether the computer is being kept awake right now (tests).
#[cfg(test)]
pub(crate) fn awake() -> bool {
    BUSY.with(|b| b.borrow().awake.as_ref().is_some_and(KeepAwake::held))
}

/// Whether background work is paused right now.
#[cfg(test)]
pub(crate) fn paused() -> bool {
    BUSY.with(|b| b.borrow().paused)
}

pub(crate) fn wire(app: &Rc<App>) {
    app.ui
        .global::<crate::PerformLock>()
        .on_toggle(handler!(app, |a| toggle(&a)));
    if od_core::env::var("DEMO_SAFETY").is_some_and(|v| v.split(',').any(|p| p == "lock")) {
        set_locked(app, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_block_explains_itself() {
        assert_eq!(
            message(Blocked::Load(1)),
            "Locked: deck 2 is playing. Load onto a stopped deck, or unlock"
        );
        for s in [
            "Locked: deck {} is playing. Load onto a stopped deck, or unlock",
            "Locked: nothing is deleted until you unlock",
            "Locked: the beatgrid stays as it is until you unlock",
            "Locked for the set: no loading onto a playing deck, no deleting, no grid edits",
            "Unlocked",
        ] {
            assert!(od_i18n::has_german(s), "no German for {s:?}");
        }
    }
}
