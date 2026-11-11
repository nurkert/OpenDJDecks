//! Roadmap 0.9.0 [65]: the planned set in Perform. The last set opened in
//! the browser is the plan; a line over the list says which of its tracks
//! is on air, which one comes next with its transition note, and where you
//! stand against the plan: ahead or behind by time (with a planned start)
//! and by position.
//!
//! The arithmetic is pure; [`tick`] reads the library and the engine once
//! a second.

use crate::app::{App, handler};
use crate::engine;
use od_core::TrackId;
use od_library::{NodeId, NodeKind};
use slint::ComponentHandle;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// Where the plan is held while the app runs.
#[derive(Default)]
pub(crate) struct PlanState {
    /// The set that is the plan.
    pub node: Cell<Option<NodeId>>,
    /// The position on air last time (a track can stand twice in a set).
    pub pos: Cell<Option<usize>>,
    /// Closed with ×: hidden until another set is opened.
    pub closed: Cell<bool>,
}

/// A planned start such as "23:30", "23.30", "2330" or "23" in minutes
/// after midnight.
pub(crate) fn parse_clock(s: &str) -> Option<u32> {
    let t = s.trim();
    let (h, m) = if let Some((h, m)) = t.split_once([':', '.', 'h']) {
        (h.trim().parse::<u32>().ok()?, if m.trim().is_empty() { 0 } else { m.trim().parse::<u32>().ok()? })
    } else if t.len() == 4 && t.chars().all(|c| c.is_ascii_digit()) {
        (t[..2].parse().ok()?, t[2..].parse().ok()?)
    } else {
        (t.parse::<u32>().ok()?, 0)
    };
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// The position on air: the on-air track's place in the set, preferring
/// the one at or after the last position (a track may stand twice).
pub(crate) fn position(tracks: &[TrackId], on_air: Option<TrackId>, last: Option<usize>) -> Option<usize> {
    let t = on_air?;
    let from = last.unwrap_or(0);
    tracks
        .iter()
        .enumerate()
        .skip(from)
        .find(|(_, x)| **x == t)
        .or_else(|| tracks.iter().enumerate().find(|(_, x)| **x == t))
        .map(|(i, _)| i)
}

/// Seconds from the start of the set at which each position starts.
pub(crate) fn offsets(durations: &[f64]) -> Vec<f64> {
    let mut at = 0.0;
    durations
        .iter()
        .map(|d| {
            let o = at;
            at += d.max(0.0);
            o
        })
        .collect()
}

/// The position the plan expects at `elapsed` seconds into the set.
pub(crate) fn planned_position(offsets: &[f64], elapsed: f64) -> usize {
    offsets.iter().rposition(|o| *o <= elapsed).unwrap_or(0)
}

/// How far the set is from its plan, in seconds (positive: behind, the
/// music is later than planned). `now` and `start` are seconds after
/// midnight; a set across midnight counts the short way round.
pub(crate) fn delay(start: f64, now: f64, offset: f64, position: f64) -> f64 {
    let planned_elapsed = offset + position;
    let real_elapsed = (now - start).rem_euclid(86_400.0);
    // Within 12 h after the start, else the set has not started yet.
    let real_elapsed = if real_elapsed > 43_200.0 { real_elapsed - 86_400.0 } else { real_elapsed };
    real_elapsed - planned_elapsed
}

/// "1:20" for 80 s.
fn mm_ss(secs: f64) -> String {
    let s = secs.abs().round() as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

/// Where you stand, in words: "ON TIME", "BEHIND 1:20", "AHEAD 0:45", plus
/// the positions when they differ ("· 1 TRACK BEHIND"); and the line form
/// (solid on plan, dashed off it by more than a minute).
pub(crate) fn standing(delay: Option<f64>, pos: usize, planned: Option<usize>) -> (String, &'static str) {
    let mut parts = Vec::new();
    let mut form = "solid";
    match delay {
        Some(d) if d.abs() < 60.0 => parts.push(od_i18n::tr("ON TIME").to_string()),
        Some(d) if d > 0.0 => {
            parts.push(od_i18n::trf("BEHIND {}", &[&mm_ss(d)]).to_string());
            form = "dashed";
        }
        Some(d) => {
            parts.push(od_i18n::trf("AHEAD {}", &[&mm_ss(d)]).to_string());
            form = "dashed";
        }
        None => form = "half",
    }
    if let Some(p) = planned {
        let diff = p as i64 - pos as i64;
        match diff {
            1 => parts.push(od_i18n::tr("1 TRACK BEHIND").to_string()),
            -1 => parts.push(od_i18n::tr("1 TRACK AHEAD").to_string()),
            d if d > 1 => parts.push(od_i18n::trf("{} TRACKS BEHIND", &[&d]).to_string()),
            d if d < -1 => parts.push(od_i18n::trf("{} TRACKS AHEAD", &[&(-d)]).to_string()),
            _ => {}
        }
    }
    if delay.is_none() {
        parts.push(od_i18n::tr("NO PLANNED START").to_string());
    }
    (parts.join(" · "), form)
}

/// The local time of day in seconds (SQLite knows the local offset).
fn now_of_day(app: &App) -> f64 {
    let offset: i64 = app
        .state
        .borrow()
        .lib
        .conn()
        .query_row(
            "SELECT CAST(strftime('%s', 'now', 'localtime') AS INTEGER) - CAST(strftime('%s', 'now') AS INTEGER)",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    (unix + offset as f64).rem_euclid(86_400.0)
}

/// The plan line, from the library and the engine.
pub(crate) fn tick(app: &App) {
    let g = app.ui.global::<crate::SetPlan>();
    // The set open in the browser becomes the plan.
    let open = {
        let st = app.state.borrow();
        st.browser.node.filter(|n| st.lib.node(*n).is_ok_and(|n| n.kind == NodeKind::Set))
    };
    if open.is_some() && open != app.plan.node.get() {
        app.plan.node.set(open);
        app.plan.pos.set(None);
        app.plan.closed.set(false);
    }
    let Some(node) = app.plan.node.get().filter(|_| !app.plan.closed.get()) else {
        g.set_shown(false);
        return;
    };
    let (name, start, entries, tracks) = {
        let st = app.state.borrow();
        let Ok(n) = st.lib.node(node) else {
            drop(st);
            app.plan.node.set(None);
            g.set_shown(false);
            return;
        };
        let entries = st.lib.entries(node).unwrap_or_default();
        let tracks: Vec<_> = entries.iter().map(|e| st.lib.track(e.track).ok()).collect();
        (
            n.name,
            n.set_meta.and_then(|m| m.planned_start).and_then(|s| parse_clock(&s)),
            entries,
            tracks,
        )
    };
    if entries.is_empty() {
        g.set_shown(false);
        return;
    }
    let ids: Vec<TrackId> = entries.iter().map(|e| e.track).collect();
    let durations: Vec<f64> = tracks.iter().map(|t| t.as_ref().map_or(0.0, |t| t.duration)).collect();
    let offs = offsets(&durations);
    let state = engine::snapshot(app);
    let on_air = state
        .as_ref()
        .and_then(|s| s.audible.dominant.and_then(|d| s.decks[d].track.map(|t| (t, s.decks[d].position))));
    let pos = position(&ids, on_air.map(|x| x.0), app.plan.pos.get());
    if pos.is_some() {
        app.plan.pos.set(pos);
    }
    let shown_pos = pos.or(app.plan.pos.get());
    let notation = app.state.borrow().settings.key_notation();
    g.set_shown(true);
    g.set_name(name.into());
    g.set_off_plan(on_air.is_some() && pos.is_none());
    g.set_total(ids.len() as i32);
    g.set_now(shown_pos.map_or(0, |p| p as i32 + 1));
    g.set_now_title(
        shown_pos
            .and_then(|p| tracks[p].as_ref())
            .map(|t| t.tags.title.clone())
            .unwrap_or_default()
            .into(),
    );
    let next = shown_pos.map_or(0, |p| p + 1);
    match tracks.get(next).and_then(|t| t.as_ref()) {
        Some(t) => {
            g.set_next(next as i32 + 1);
            g.set_next_title(t.tags.title.clone().into());
            g.set_next_artist(t.tags.artist.clone().into());
            let mut detail = Vec::new();
            if let Some(b) = t.bpm {
                detail.push(format!("{b:.1} BPM"));
            }
            if let Some(k) = t.key {
                detail.push(k.format(notation));
            }
            detail.push(crate::format::time(t.duration));
            g.set_next_detail(detail.join(" · ").into());
        }
        None => {
            g.set_next(0);
            g.set_next_title(SharedText::default());
            g.set_next_artist(SharedText::default());
            g.set_next_detail(SharedText::default());
        }
    }
    // The note of the transition from the track on air into the next one.
    g.set_note(
        shown_pos
            .and_then(|p| entries.get(p))
            .map(|e| e.note.clone())
            .unwrap_or_default()
            .into(),
    );
    let (text, form) = match (pos, on_air) {
        (Some(p), Some((_, at))) => {
            let d = start.map(|s| delay(s as f64 * 60.0, now_of_day(app), offs[p], at));
            let planned = start.map(|s| {
                let elapsed = (now_of_day(app) - s as f64 * 60.0).rem_euclid(86_400.0);
                planned_position(&offs, elapsed)
            });
            standing(d, p, planned)
        }
        (None, Some(_)) => (od_i18n::tr("OFF PLAN: THE TRACK ON AIR IS NOT IN THE SET").to_string(), "struck"),
        _ => (
            if start.is_some() {
                od_i18n::tr("WAITING FOR THE FIRST TRACK").to_string()
            } else {
                od_i18n::tr("NO PLANNED START").to_string()
            },
            "half",
        ),
    };
    g.set_standing(text.into());
    g.set_form(form.into());
}

type SharedText = slint::SharedString;

pub(crate) fn wire(app: &Rc<App>) {
    let g = app.ui.global::<crate::SetPlan>();
    g.on_close(handler!(app, |a| {
        a.plan.closed.set(true);
        tick(&a);
    }));
    let weak = Rc::downgrade(app);
    let timer = Rc::new(slint::Timer::default());
    let keep = timer.clone();
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(500), move || {
        let _ = &keep;
        if let Some(a) = weak.upgrade() {
            tick(&a);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planned_starts_parse() {
        assert_eq!(parse_clock("23:30"), Some(23 * 60 + 30));
        assert_eq!(parse_clock("23.30"), Some(23 * 60 + 30));
        assert_eq!(parse_clock("2330"), Some(23 * 60 + 30));
        assert_eq!(parse_clock("22"), Some(22 * 60));
        assert_eq!(parse_clock("22h"), Some(22 * 60));
        assert_eq!(parse_clock("25:00"), None);
        assert_eq!(parse_clock("late"), None);
    }

    #[test]
    fn the_position_follows_the_track_on_air() {
        let t = |i| TrackId(i);
        let set = [t(1), t(2), t(3), t(2)];
        assert_eq!(position(&set, Some(t(2)), None), Some(1));
        // The same track later in the set: after the last position.
        assert_eq!(position(&set, Some(t(2)), Some(2)), Some(3));
        assert_eq!(position(&set, Some(t(9)), Some(1)), None);
        assert_eq!(position(&set, None, Some(1)), None);
    }

    #[test]
    fn behind_or_ahead_by_time_and_position() {
        let offs = offsets(&[300.0, 300.0, 300.0]);
        assert_eq!(offs, vec![0.0, 300.0, 600.0]);
        assert_eq!(planned_position(&offs, 650.0), 2);
        // Start 22:00; at 22:11:40 track 2 (offset 300) is 100 s in: the
        // plan has it at 22:06:40, so 300 s behind.
        let start = 22.0 * 3600.0;
        let d = delay(start, start + 700.0, 300.0, 100.0);
        assert_eq!(d, 300.0);
        // Across midnight: start 23:50, now 00:05 → 15 min into the set.
        let d = delay(23.0 * 3600.0 + 50.0 * 60.0, 5.0 * 60.0, 0.0, 600.0);
        assert_eq!(d, 300.0);
        // Before the start: negative (ahead).
        let d = delay(start, start - 120.0, 0.0, 0.0);
        assert_eq!(d, -120.0);
        let (text, form) = standing(Some(300.0), 1, Some(2));
        assert_eq!(form, "dashed");
        assert!(text.contains("BEHIND 5:00") && text.contains("1 TRACK BEHIND"), "{text}");
        let (text, form) = standing(Some(20.0), 1, Some(1));
        assert_eq!((text.as_str(), form), ("ON TIME", "solid"));
        let (text, form) = standing(None, 1, None);
        assert_eq!((text.as_str(), form), ("NO PLANNED START", "half"));
    }
}
