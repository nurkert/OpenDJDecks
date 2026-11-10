//! GRID panel of the Perform deck: the beatgrid fixed at the playhead
//! while the track plays, and your correction of the track's auto gain.
//!
//! Row 1 is the evidence: a meter of how far the grid is off the kicks
//! around the playhead (the median over the last eight beats, updated per
//! beat), the FIT of the whole grid, its tempo and the CLICK. Rows 2–3
//! move the grid (SET 1 HERE snaps to the kick within ±60 ms, which also
//! takes up your reaction and the output latency), change its tempo about
//! the playhead, PIN kicks (two pins solve the tempo) and AUTO-FIX it.
//! Row 4 opens the full editor, locks the grid and corrects the gain.
//!
//! Edits go to the library at once (decks follow); the same kind of edit
//! within 1.5 s joins the previous undo step, so a run of nudges is undone
//! in one go. The operations are the editor's ([`crate::grid_edit`]).

use crate::app::{App, handler};
use crate::engine;
use crate::grid_edit::{self, Cx, Op};
use od_analysis::transients as tr;
use od_core::{DataSource, TrackId};
use od_engine::DeckCommand;
use od_library::{EntityState, Library};
use slint::{ComponentHandle, Model, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Gain correction range and step (dB).
const GAIN_LIMIT: f32 = 12.0;
/// Same-kind edits closer than this join one undo step.
const MERGE: Duration = Duration::from_millis(1500);
/// Beats the meter takes the median over.
const METER_BEATS: usize = 8;

fn gain_key(id: TrackId) -> String {
    format!("track-gain.{}", id.0)
}

/// The stored gain correction of a track in dB (0 when none).
pub(crate) fn stored_gain(lib: &Library, id: TrackId) -> f32 {
    lib.meta(&gain_key(id))
        .ok()
        .flatten()
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|v| v.is_finite())
        .map(|v| v.clamp(-GAIN_LIMIT, GAIN_LIMIT))
        .unwrap_or(0.0)
}

/// "+1.5 dB", "0.0 dB", "−3.0 dB".
pub(crate) fn gain_label(db: f32) -> String {
    if db.abs() < 0.05 {
        "0.0 dB".into()
    } else if db > 0.0 {
        format!("+{db:.1} dB")
    } else {
        format!("−{:.1} dB", -db)
    }
}

/// Per deck: pins (with the track they belong to), CLICK, the last edit
/// (for merging) and what the meter last showed.
#[derive(Default)]
struct DeckEdit {
    pins: Vec<f64>,
    pins_track: Option<TrackId>,
    click: bool,
    last: Option<(TrackId, String, Instant)>,
    meter_key: Option<(TrackId, usize, u64, usize)>,
    fit_key: Option<(TrackId, u64, usize)>,
    fit: Option<f32>,
}

thread_local! {
    static DECKS: RefCell<[DeckEdit; crate::decks::DECKS]> = RefCell::new(Default::default());
    static MODEL: RefCell<Option<Rc<VecModel<crate::DeckGridView>>>> = const { RefCell::new(None) };
}

/// The kind of an action for merging ("shift" nudges join, a SET 1 never).
fn merge_kind(action: &str) -> Option<&'static str> {
    match action {
        "shift" | "beat" => Some("move"),
        "step" => Some("tempo"),
        _ => None,
    }
}

/// Runs a panel action on deck `d` (0-based).
pub(crate) fn act(app: &App, d: usize, action: &str, arg: &str) {
    let Some(id) = app.decks.borrow().loaded(d).map(|m| m.id) else {
        app.status("Load a track on this deck first");
        return;
    };
    if app.decks.borrow().source_is_stick() {
        app.status("Tracks on a stick are read-only here — edit them in your library and the stick follows");
        return;
    }
    let pos = engine::snapshot(app)
        .and_then(|s| s.decks.get(d).map(|s| s.position))
        .unwrap_or(0.0);
    let num = arg.trim().replace(',', ".").parse::<f64>().unwrap_or(0.0);
    if action == "gain" || action == "gain-reset" {
        set_gain(app, d, id, action, num);
        return;
    }
    if action == "click" {
        let on = DECKS.with(|x| {
            let mut x = x.borrow_mut();
            x[d].click = !x[d].click;
            x[d].click
        });
        engine::deck_cmd(
            app,
            d,
            DeckCommand::SetMetronome { on, level: grid_edit::CLICK_LEVEL, cue: true },
        );
        app.status(if on { "Click on (headphones)" } else { "Click off" });
        refresh_row(app, d, true, None);
        return;
    }
    if app.guard_locked(crate::perform_lock::Blocked::Grid) {
        return;
    }
    let op = match action {
        "set-one" => Op::SetOne(pos),
        "shift" => Op::Shift(num / 1000.0),
        "beat" => Op::Beats(num),
        "step" => Op::Step(num),
        "scale" if num > 0.0 => Op::Scale(num),
        "bpm" => match crate::player::parse_bpm(arg) {
            Some(b) => Op::Bpm(b),
            None => {
                app.status("Type a tempo between 30 and 300 BPM");
                return;
            }
        },
        "pin" => Op::Pin(pos),
        "clear-pins" => Op::ClearPins,
        "auto-fix" => Op::AutoFix,
        "lock" => Op::Lock,
        _ => return,
    };
    let (grid, duration) = {
        let st = app.state.borrow();
        (
            st.lib.beatgrid(id).ok().flatten().unwrap_or_default(),
            st.lib.track(id).map(|t| t.duration).unwrap_or(0.0),
        )
    };
    let pins = DECKS.with(|x| {
        let mut x = x.borrow_mut();
        if x[d].pins_track != Some(id) {
            x[d].pins.clear();
            x[d].pins_track = Some(id);
        }
        x[d].pins.clone()
    });
    let hash = app.decks.borrow().loaded(d).map(|m| m.content_hash().clone()).unwrap_or(od_core::ContentHash(String::new()));
    let transients = crate::transients::get(&hash);
    let cx = Cx {
        pivot: pos,
        duration,
        transients: transients.as_deref().map(|v| v.as_slice()),
        snap: true,
    };
    let out = match grid_edit::apply_op(&grid, &pins, &op, &cx) {
        Ok(out) => out,
        Err(why) => {
            if !why.is_empty() {
                app.status(why);
            }
            return;
        }
    };
    let now = Instant::now();
    let kind = merge_kind(action);
    let merge = DECKS.with(|x| {
        let mut x = x.borrow_mut();
        x[d].pins = out.pins.clone();
        let m = x[d].last.as_ref().is_some_and(|(t, k, at)| {
            *t == id && kind == Some(k.as_str()) && now.duration_since(*at) < MERGE
        });
        x[d].last = kind.map(|k| (id, k.to_string(), now));
        m
    });
    if out.grid != grid {
        let state = EntityState::Grid { track: id, grid: Some((out.grid.clone(), DataSource::Manual)) };
        let r = app.state.borrow_mut().lib.edit_merged("Edit beatgrid", vec![state], merge);
        if app.report(r).is_none() {
            return;
        }
        crate::decks::follow_grid(app, id, &out.grid);
        app.after_edit(&out.status);
    } else {
        app.status(out.status);
    }
    refresh_row(app, d, true, None);
}

fn set_gain(app: &App, d: usize, id: TrackId, action: &str, num: f64) {
    let now = app.decks.borrow().loaded(d).map_or(0.0, |m| m.gain_adjust);
    let db = if action == "gain-reset" {
        0.0
    } else {
        ((now + num as f32) * 10.0).round() / 10.0
    }
    .clamp(-GAIN_LIMIT, GAIN_LIMIT);
    let r = app.state.borrow_mut().lib.set_meta(&gain_key(id), &format!("{db:.1}"));
    if app.report(r).is_none() {
        return;
    }
    let mut decks = app.decks.borrow_mut();
    for i in 0..crate::decks::DECKS {
        if let Some(m) = decks.loaded_mut(i).filter(|m| m.id == id) {
            m.gain_adjust = db;
            engine::deck_cmd(app, i, DeckCommand::SetGainAdjust(db));
        }
    }
    drop(decks);
    app.status(format!("Gain {}", gain_label(db)));
}

/// Frame tick (Perform): the meters follow the playheads, once per beat.
pub(crate) fn tick(app: &App, state: &od_engine::EngineState) {
    for (d, s) in state.decks.iter().enumerate().take(crate::decks::DECKS) {
        refresh_row(app, d, false, Some(s.position));
    }
}

fn model(app: &App) -> Rc<VecModel<crate::DeckGridView>> {
    MODEL.with(|m| {
        m.borrow_mut()
            .get_or_insert_with(|| {
                let v = Rc::new(VecModel::from(vec![crate::DeckGridView::default(); crate::decks::DECKS]));
                app.ui.global::<crate::DeckGrid>().set_decks(v.clone().into());
                v
            })
            .clone()
    })
}

/// The panel readouts of deck `d`; `force` redraws even within a beat.
fn refresh_row(app: &App, d: usize, force: bool, pos: Option<f64>) {
    let model = model(app);
    let Some((id, grid, hash)) = app
        .decks
        .borrow()
        .loaded(d)
        .map(|m| (m.id, m.grid.clone(), m.content_hash().clone()))
    else {
        if model.row_data(d).is_some_and(|r| r != crate::DeckGridView::default()) {
            model.set_row_data(d, crate::DeckGridView::default());
        }
        return;
    };
    let pos = pos
        .or_else(|| engine::snapshot(app).and_then(|s| s.decks.get(d).map(|s| s.position)))
        .unwrap_or(0.0);
    let beat = grid.beat_index_at(pos).unwrap_or(0);
    let version = crate::transients::version();
    let key = (id, beat, version, grid.beats.len() ^ (grid.beats.first().map_or(0, |b| (b.time * 1e6) as usize)));
    let (fresh, pins, click) = DECKS.with(|x| {
        let mut x = x.borrow_mut();
        let e = &mut x[d];
        let fresh = force || e.meter_key != Some(key);
        e.meter_key = Some(key);
        let pins = if e.pins_track == Some(id) { e.pins.len() } else { 0 };
        (fresh, pins, e.click)
    });
    if !fresh {
        return;
    }
    let transients = crate::transients::get(&hash);
    let mut row = crate::DeckGridView {
        bpm: grid_edit::bpm_at(&grid, pos).map(grid_edit::bpm_text).unwrap_or_default().into(),
        pins: pins as i32,
        click,
        fit: SharedString::from("—"),
        err_text: SharedString::from("—"),
        ..Default::default()
    };
    if let Some(list) = transients.as_deref() {
        // Around the playhead: the last eight beats.
        let lo = beat.saturating_sub(METER_BEATS - 1);
        let errors: Vec<tr::BeatError> = (lo..=beat.min(grid.beats.len().saturating_sub(1)))
            .filter_map(|i| {
                let b = grid.beats.get(i)?;
                Some(match tr::snap(list, b.time, tr::MATCH_WINDOW) {
                    Some(t) => tr::BeatError { beat: i, err: t - b.time, matched: true },
                    None => tr::BeatError { beat: i, err: 0.0, matched: false },
                })
            })
            .collect();
        if let Some(m) = tr::median_error(&errors) {
            row.has_err = true;
            row.err = (m * 1000.0).clamp(-20.0, 20.0) as f32;
            row.err_text = grid_edit::ms_text(m).into();
            row.err_tone = grid_edit::tone_of(m);
        }
        let fit_key = (id, version, key.3);
        let fit = DECKS.with(|x| {
            let mut x = x.borrow_mut();
            let e = &mut x[d];
            if e.fit_key != Some(fit_key) {
                e.fit = Some(tr::fit(&tr::grid_error(&grid, list)));
                e.fit_key = Some(fit_key);
            }
            e.fit
        });
        if let Some(f) = fit {
            row.fit = format!("{} %", f.round() as i32).into();
        }
    }
    if model.row_data(d).as_ref() != Some(&row) {
        model.set_row_data(d, row);
    }
}

pub(crate) fn wire(app: &Rc<App>) {
    model(app);
    app.ui.global::<crate::DeckGrid>().on_act(handler!(
        app,
        |a, deck: i32, action: SharedString, arg: SharedString| {
            act(&a, (deck.max(1) - 1) as usize, action.as_str(), arg.as_str())
        }
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_reads_as_a_signed_level() {
        assert_eq!(gain_label(0.0), "0.0 dB");
        assert_eq!(gain_label(1.5), "+1.5 dB");
        assert_eq!(gain_label(-3.0), "−3.0 dB");
    }

    #[test]
    fn nudges_merge_but_set_one_does_not() {
        assert_eq!(merge_kind("shift"), Some("move"));
        assert_eq!(merge_kind("beat"), Some("move"));
        assert_eq!(merge_kind("step"), Some("tempo"));
        assert_eq!(merge_kind("set-one"), None);
        assert_eq!(merge_kind("pin"), None);
    }
}
