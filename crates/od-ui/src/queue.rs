//! The queue in Perform ([29]): what comes next (added from the browser,
//! reordered, removed, LOAD NEXT to a free deck), requests typed in and
//! matched to the library, the do-not-play tag (a warning when such a
//! track is loaded; auto-play leaves it out), auto-play (the queue plays
//! on by itself and mixes each track into the next over a number of beats)
//! and the dead-air warning (the music is about to run out and no deck is
//! cued).
//!
//! The queue is kept in the library meta (`queue`), so it survives a
//! restart. Auto-play keeps one deck cued with the next playable track and
//! starts it when the playing track is that many beats from its end: the
//! incoming deck's channel fader comes up while the outgoing one goes down
//! (or the crossfader moves, when the two decks sit on its two sides), with
//! SYNC on for the incoming deck when both tracks have a beatgrid.

use crate::app::{App, handler};
use crate::models::model;
use crate::{QueueRow, decks, engine, format};
use od_core::TrackId;
use od_engine::{ChannelCommand, Command, DeckCommand, EngineState, MixerCommand, XfAssign};
use od_library::UserTag;
use serde::{Deserialize, Serialize};
use slint::{ComponentHandle, SharedString};
use std::cell::RefCell;
use std::rc::Rc;

/// Library meta key of the queue.
pub(crate) const META_QUEUE: &str = "queue";
/// Auto-play mix lengths offered (beats; 0 = a cut).
pub(crate) const MIXES: [u32; 4] = [0, 8, 16, 32];
/// Dead-air warning times offered (seconds before the music runs out).
pub(crate) const WARNS: [u32; 3] = [15, 30, 60];

/// One entry: a track, or a request (with the track it was matched to).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Item {
    pub track: Option<TrackId>,
    /// The request as typed ("" = not a request).
    #[serde(default)]
    pub request: String,
}

/// A running auto-play mix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Mix {
    pub from: usize,
    pub to: usize,
    /// Outgoing position when the mix started and its end (seconds).
    pub start: f64,
    pub end: f64,
    /// The crossfader moves from `from`'s side to `to`'s.
    pub crossfader: Option<(f32, f32)>,
    /// Last values sent (to send only changes).
    pub sent: f32,
}

#[derive(Default)]
pub(crate) struct QueueState {
    pub items: Vec<Item>,
    pub auto: bool,
    pub mix: Option<Mix>,
    /// Track per deck at the last frame (a new load is checked for the
    /// do-not-play tag).
    seen: [Option<TrackId>; decks::DECKS],
    /// The alert shown: ("dnp", deck) or ("dead-air", deck).
    pub alert: Option<(&'static str, usize)>,
    /// A do-not-play warning that was kept (per deck and track).
    kept: Vec<(usize, TrackId)>,
    /// Seconds left on the deck running out (dead air).
    left: f64,
    loaded_once: bool,
}

thread_local! {
    pub(crate) static STATE: RefCell<QueueState> = RefCell::new(QueueState::default());
    /// Tests and documentation screenshots: what the panel and the alert
    /// show is set by hand and kept.
    pub(crate) static HOLD_ALERT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn with<T>(f: impl FnOnce(&mut QueueState) -> T) -> T {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

/// Whether `track` carries the do-not-play tag.
pub(crate) fn is_dnp(app: &App, track: TrackId) -> bool {
    app.state
        .borrow()
        .lib
        .user_tags(track)
        .map(|t| t.iter().any(UserTag::is_do_not_play))
        .unwrap_or(false)
}

/// The first item auto-play and LOAD NEXT take: a track that is not tagged
/// do not play (pure).
pub(crate) fn next_playable(items: &[Item], dnp: impl Fn(TrackId) -> bool) -> Option<usize> {
    items
        .iter()
        .position(|i| i.track.is_some_and(|t| !dnp(t)))
}

/// Moves item `i` by `delta` within the list; returns the new index (pure).
pub(crate) fn move_item(items: &mut [Item], i: usize, delta: i32) -> Option<usize> {
    let j = i as i64 + delta as i64;
    if i >= items.len() || j < 0 || j as usize >= items.len() {
        return None;
    }
    items.swap(i, j as usize);
    Some(j as usize)
}

/// The fader and crossfader values `p` (0..1) through a mix: (outgoing,
/// incoming) channel faders, and the crossfader when it moves (pure).
pub(crate) fn mix_levels(p: f32, crossfader: Option<(f32, f32)>) -> (f32, f32, Option<f32>) {
    let p = p.clamp(0.0, 1.0);
    match crossfader {
        Some((a, b)) => (1.0, 1.0, Some(a + (b - a) * p)),
        None => (1.0 - p, p, None),
    }
}

/// Seconds a mix over `beats` takes at `bpm` (0 beats or no tempo: a cut).
pub(crate) fn mix_secs(beats: u32, bpm: f64) -> f64 {
    if beats == 0 || bpm <= 1.0 {
        0.0
    } else {
        beats as f64 * 60.0 / bpm
    }
}

/// The deck that runs out first among those on air, with its seconds
/// left, when no other deck is cued (pure): `(deck, left)`.
pub(crate) fn dead_air(
    on_air: &[(usize, f64)],
    cued: &[usize],
    warn: f64,
) -> Option<(usize, f64)> {
    if on_air.is_empty() {
        return None;
    }
    let (d, left) = on_air
        .iter()
        .copied()
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    // The music goes on as long as another deck is on air with time left.
    let others = on_air.iter().any(|(o, l)| *o != d && *l > warn);
    if left > warn || others || cued.iter().any(|c| *c != d) {
        return None;
    }
    Some((d, left))
}

fn save(app: &App) {
    let json = with(|s| serde_json::to_string(&s.items).unwrap_or_default());
    if app.state.borrow().read_only() {
        return;
    }
    if let Err(e) = app.state.borrow_mut().lib.set_meta(META_QUEUE, &json) {
        tracing::warn!(error = %e, "queue not saved");
    }
}

fn load(app: &App) {
    let items: Vec<Item> = app
        .state
        .borrow()
        .lib
        .meta(META_QUEUE)
        .ok()
        .flatten()
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default();
    with(|s| {
        s.items = items;
        s.loaded_once = true;
    });
}

/// Adds tracks to the end of the queue.
pub(crate) fn add(app: &App, ids: &[TrackId]) {
    if ids.is_empty() {
        app.status("Select tracks in the browser to add them to the queue");
        return;
    }
    with(|s| {
        s.items.extend(ids.iter().map(|t| Item {
            track: Some(*t),
            request: String::new(),
        }))
    });
    save(app);
    let dnp = ids.iter().filter(|t| is_dnp(app, **t)).count();
    if dnp > 0 {
        app.status(format!(
            "Added {} to the queue; {dnp} tagged do not play (auto-play leaves them out)",
            ids.len()
        ));
    } else {
        app.status(format!("Added {} to the queue", ids.len()));
    }
    project(app);
}

/// The selected browser tracks (library tracks only).
fn selected(app: &App) -> Vec<TrackId> {
    let st = app.state.borrow();
    if st.read_only() {
        return Vec::new();
    }
    st.selected_ids()
}

/// The library track a request names, if one matches every word of it
/// (the best by the fuzzy search's score).
pub(crate) fn match_request(lib: &od_library::Library, text: &str) -> Option<TrackId> {
    // "Artist - Title", "Artist – Title" and "Title by Artist" alike.
    let q = crate::browser::fuzzy::Query::new(&text.replace(['-', '–', '—'], " "));
    if q.is_empty() {
        return None;
    }
    lib.tracks(od_library::SortKey::default(), false)
        .ok()?
        .into_iter()
        .filter(|t| !t.missing)
        .filter_map(|t| crate::browser::fuzzy::score(&q, &t).map(|s| (s, t.id)))
        .max_by_key(|(s, _)| *s)
        .map(|(_, id)| id)
}

/// A request typed into the queue: matched to the library when it can be.
pub(crate) fn request(app: &App, text: &str) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    let track = match_request(&app.state.borrow().lib, text);
    with(|s| {
        s.items.push(Item {
            track,
            request: text.to_string(),
        })
    });
    save(app);
    match track {
        Some(t) => {
            let title = app
                .state
                .borrow()
                .lib
                .track(t)
                .map(|t| t.tags.title)
                .unwrap_or_default();
            app.status(format!("Request queued: {title}"));
        }
        None => app.status("Request noted: no track in the library matches it yet"),
    }
    project(app);
}

/// Ties request `i` to the track selected in the browser.
fn match_selected(app: &App, i: usize) {
    let Some(t) = selected(app).first().copied() else {
        app.status("Select the requested track in the browser first");
        return;
    };
    with(|s| {
        if let Some(it) = s.items.get_mut(i) {
            it.track = Some(t);
        }
    });
    save(app);
    project(app);
}

/// Decks free for the next track: not playing, and empty or played out.
fn free_decks(app: &App, state: &EngineState) -> Vec<usize> {
    let d = app.decks.borrow();
    (0..d.deck_count.min(decks::DECKS))
        .filter(|&i| {
            let s = &state.decks[i];
            !s.playing
                && !d.loading_on(i)
                && (d.loaded(i).is_none()
                    || crate::mix_preview::free_for_next(true, s.playing, s.position, s.duration))
        })
        .collect()
}

/// Decks cued: a track loaded and not played out, not on air (pure inputs
/// from the engine state).
fn cued_decks(app: &App, state: &EngineState) -> Vec<usize> {
    let d = app.decks.borrow();
    (0..d.deck_count.min(decks::DECKS))
        .filter(|&i| {
            let s = &state.decks[i];
            let audible = state.audible.audible.get(i).copied().unwrap_or(false);
            (d.loaded(i).is_some() || d.loading_on(i))
                && s.position < s.duration - 1.0
                && !(s.playing && audible)
        })
        .collect()
}

/// Loads the next playable track to a free deck; returns the deck.
pub(crate) fn load_next(app: &App) -> Option<usize> {
    if !engine::ensure_engine(app) {
        return None;
    }
    let state = engine::snapshot(app)?;
    let Some(deck) = free_decks(app, &state).first().copied() else {
        app.status("No free deck: every deck is playing or holds a track");
        return None;
    };
    let items = with(|s| s.items.clone());
    let Some(i) = next_playable(&items, |t| is_dnp(app, t)) else {
        app.status("The queue holds no track to load");
        return None;
    };
    let track = items[i].track?;
    with(|s| {
        s.items.remove(i);
    });
    save(app);
    decks::load_track(app, deck, track, None);
    project(app);
    Some(deck)
}

fn ch(app: &App, d: usize, cmd: ChannelCommand) {
    engine::send(app, Command::Channel(d, cmd));
}

/// Starts the cued deck `to` and the mix from `from` into it.
fn start_mix(app: &App, state: &EngineState, from: usize, to: usize, secs: f64) {
    let out = &state.decks[from];
    let xf = |d: usize| match state.channels[d].xf_assign {
        XfAssign::A => Some(-1.0f32),
        XfAssign::B => Some(1.0f32),
        _ => None,
    };
    let crossfader = match (xf(from), xf(to)) {
        (Some(a), Some(b)) if a != b && secs > 0.0 => Some((a, b)),
        _ => None,
    };
    if secs > 0.0 {
        if crossfader.is_none() {
            ch(app, to, ChannelCommand::SetFader(0.0));
        } else {
            ch(app, to, ChannelCommand::SetFader(1.0));
        }
        if out.track_bpm > 0.0 && state.decks[to].track_bpm > 0.0 && !state.decks[to].sync {
            engine::deck_cmd(app, to, DeckCommand::SetSync(true));
        }
    } else {
        ch(app, to, ChannelCommand::SetFader(1.0));
    }
    engine::deck_cmd(app, to, DeckCommand::Play);
    if secs > 0.0 {
        with(|s| {
            s.mix = Some(Mix {
                from,
                to,
                start: out.position,
                end: out.duration.max(out.position + 0.1),
                crossfader,
                sent: -1.0,
            })
        });
    }
}

/// Moves a running mix on by the outgoing deck's position.
fn step_mix(app: &App, state: &EngineState) {
    let Some(m) = with(|s| s.mix) else { return };
    let out = &state.decks[m.from];
    let done = !out.playing || out.position >= m.end - 0.05;
    let p = if done {
        1.0
    } else {
        ((out.position - m.start) / (m.end - m.start)) as f32
    };
    if (p - m.sent).abs() >= 0.01 || done {
        let (a, b, x) = mix_levels(p, m.crossfader);
        match x {
            Some(x) => engine::send(app, Command::Mixer(MixerCommand::SetCrossfader(x))),
            None => {
                ch(app, m.from, ChannelCommand::SetFader(a));
                ch(app, m.to, ChannelCommand::SetFader(b));
            }
        }
        with(|s| {
            if let Some(mm) = s.mix.as_mut() {
                mm.sent = p;
            }
        });
    }
    if done {
        with(|s| s.mix = None);
    }
}

/// Auto-play: keep a deck cued with the next track, start it when the
/// track on air reaches the mix, start the queue when nothing plays.
fn auto_play(app: &App, state: &EngineState) {
    if with(|s| s.mix.is_some()) {
        step_mix(app, state);
        return;
    }
    let cued = cued_decks(app, state);
    let has_next = with(|s| s.items.clone());
    let playable = next_playable(&has_next, |t| is_dnp(app, t)).is_some();
    if cued.is_empty() && playable && !free_decks(app, state).is_empty() {
        load_next(app);
        return;
    }
    let n = app.decks.borrow().deck_count.min(decks::DECKS);
    let playing: Vec<usize> = (0..n).filter(|&d| state.decks[d].playing).collect();
    let ready: Vec<usize> = cued
        .iter()
        .copied()
        .filter(|&d| app.decks.borrow().loaded(d).is_some() && !state.decks[d].playing)
        .collect();
    let Some(&to) = ready.first() else { return };
    if playing.is_empty() {
        // Nothing plays: the queue starts.
        ch(app, to, ChannelCommand::SetFader(1.0));
        engine::deck_cmd(app, to, DeckCommand::Play);
        return;
    }
    let from = state
        .audible
        .dominant
        .filter(|d| playing.contains(d))
        .unwrap_or(playing[0]);
    let out = &state.decks[from];
    let beats = app.state.borrow().settings.queue.mix_beats;
    let secs = mix_secs(beats, out.bpm);
    let left = out.duration - out.position;
    if left <= secs.max(0.15) && left > 0.0 {
        start_mix(app, state, from, to, secs.min(left));
    }
}

/// Frame tick (every mode): do-not-play on load, auto-play, dead air and
/// the panel.
pub(crate) fn frame(app: &App, state: &EngineState) {
    if !with(|s| s.loaded_once) {
        load(app);
    }
    let n = app.decks.borrow().deck_count.min(decks::DECKS);
    // A new track on a deck: warn when it is tagged do not play.
    for d in 0..n {
        let now = app.decks.borrow().loaded_id(d);
        let was = with(|s| std::mem::replace(&mut s.seen[d], now));
        if now != was
            && let Some(t) = now
            && !app.decks.borrow().source_is_stick()
            && is_dnp(app, t)
            && !with(|s| s.kept.contains(&(d, t)))
        {
            with(|s| s.alert = Some(("dnp", d)));
            app.status(format!("Deck {}: this track is tagged do not play", d + 1));
        }
        // A do-not-play alert goes when its deck is emptied or loads
        // another track.
        if now != was && (now.is_none() || now.is_some_and(|t| !is_dnp(app, t))) {
            with(|s| {
                if s.alert == Some(("dnp", d)) {
                    s.alert = None;
                }
            });
        }
    }
    if with(|s| s.auto) && !app.state.borrow().read_only() {
        auto_play(app, state);
    }
    if HOLD_ALERT.with(|h| h.get()) {
        return;
    }
    // Dead air: the deck on air runs out and nothing is cued.
    let warn = app.state.borrow().settings.queue.warn_secs as f64;
    let on_air: Vec<(usize, f64)> = (0..n)
        .filter(|&d| state.decks[d].playing && state.audible.audible.get(d).copied().unwrap_or(false))
        .map(|d| {
            let s = &state.decks[d];
            (d, ((s.duration - s.position) / s.speed.abs().max(0.05)).max(0.0))
        })
        .collect();
    let cued = cued_decks(app, state);
    let auto_covers = with(|s| s.auto && (s.mix.is_some() || next_playable(&s.items, |t| is_dnp(app, t)).is_some()));
    let dead = if auto_covers { None } else { dead_air(&on_air, &cued, warn) };
    with(|s| {
        match dead {
            Some((d, left)) => {
                if s.alert.is_none_or(|a| a.0 == "dead-air") {
                    s.alert = Some(("dead-air", d));
                }
                s.left = left;
            }
            None => {
                if s.alert.is_some_and(|a| a.0 == "dead-air") {
                    s.alert = None;
                }
            }
        }
    });
    project_alert(app, state);
    project_live(app, state);
}

/// The alert band's words.
fn project_alert(app: &App, _state: &EngineState) {
    let g = app.ui.global::<crate::Queue>();
    let (kind, title, detail) = match with(|s| s.alert.map(|a| (a, s.left))) {
        None => ("", String::new(), String::new()),
        Some((("dead-air", d), left)) => (
            "dead-air",
            od_i18n::trf("Dead air in {}", &[&format::time(left.ceil())]),
            if with(|s| s.items.iter().any(|i| i.track.is_some())) {
                od_i18n::trf("Deck {} runs out and no deck is cued. LOAD NEXT takes the queue's next track.", &[&(d + 1)])
            } else {
                od_i18n::trf("Deck {} runs out and no deck is cued, and the queue is empty: load a track.", &[&(d + 1)])
            },
        ),
        Some(((_, d), _)) => {
            let title = app
                .decks
                .borrow()
                .loaded(d)
                .map(|m| m.title.clone())
                .unwrap_or_default();
            (
                "dnp",
                od_i18n::trf("Do not play: {}", &[&title]),
                od_i18n::trf("Deck {} holds a track tagged do not play.", &[&(d + 1)]),
            )
        }
    };
    if g.get_alert() != kind {
        g.set_alert(kind.into());
    }
    if g.get_alert_title() != title.as_str() {
        g.set_alert_title(title.into());
    }
    if g.get_alert_detail() != detail.as_str() {
        g.set_alert_detail(detail.into());
    }
}

/// What changes from frame to frame: LOAD NEXT, ADD and the mix line.
fn project_live(app: &App, state: &EngineState) {
    let g = app.ui.global::<crate::Queue>();
    let items = with(|s| s.items.clone());
    let can_load = !free_decks(app, state).is_empty()
        && next_playable(&items, |t| is_dnp(app, t)).is_some()
        && !app.state.borrow().read_only();
    if g.get_can_load() != can_load {
        g.set_can_load(can_load);
    }
    let can_add = !selected(app).is_empty();
    if g.get_can_add() != can_add {
        g.set_can_add(can_add);
    }
    let mixing = with(|s| s.mix).map(|m| {
        od_i18n::trf(
            "MIXING DECK {} › DECK {} · {} BEATS",
            &[&(m.from + 1), &(m.to + 1), &app.state.borrow().settings.queue.mix_beats],
        )
    });
    let mixing = mixing.unwrap_or_default();
    if g.get_mixing() != mixing.as_str() {
        g.set_mixing(mixing.into());
    }
}

/// The panel's rows and choices.
pub(crate) fn project(app: &App) {
    let g = app.ui.global::<crate::Queue>();
    let items = with(|s| s.items.clone());
    let next = next_playable(&items, |t| is_dnp(app, t));
    let rows: Vec<QueueRow> = {
        let st = app.state.borrow();
        items
            .iter()
            .enumerate()
            .map(|(i, it)| {
                let t = it.track.and_then(|t| st.lib.track(t).ok());
                let dnp = it.track.is_some_and(|t| {
                    st.lib
                        .user_tags(t)
                        .map(|v| v.iter().any(UserTag::is_do_not_play))
                        .unwrap_or(false)
                });
                QueueRow {
                    title: t.as_ref().map(|t| t.tags.title.clone()).unwrap_or_default().into(),
                    artist: t.as_ref().map(|t| t.tags.artist.clone()).unwrap_or_default().into(),
                    detail: t
                        .as_ref()
                        .map(|t| {
                            let mut v = Vec::new();
                            if let Some(b) = t.bpm {
                                v.push(format!("{b:.1} BPM"));
                            }
                            let k = st.key_text(t);
                            if !k.is_empty() {
                                v.push(k);
                            }
                            v.push(format::time(t.duration));
                            v.join("  ·  ")
                        })
                        .unwrap_or_default()
                        .into(),
                    request: it.request.clone().into(),
                    matched: it.track.is_some(),
                    dnp,
                    next: next == Some(i),
                }
            })
            .collect()
    };
    g.set_rows(model(rows));
    g.set_auto(with(|s| s.auto));
    let q = app.state.borrow().settings.queue.clone();
    g.set_mixes(model(
        MIXES
            .iter()
            .map(|m| {
                SharedString::from(if *m == 0 {
                    od_i18n::tr("CUT").to_string()
                } else {
                    m.to_string()
                })
            })
            .collect(),
    ));
    g.set_mix(MIXES.iter().position(|m| *m == q.mix_beats).unwrap_or(2) as i32);
    g.set_warns(model(
        WARNS
            .iter()
            .map(|w| SharedString::from(od_i18n::trf("{} S", &[w])))
            .collect(),
    ));
    g.set_warn(WARNS.iter().position(|w| *w == q.warn_secs).unwrap_or(1) as i32);
}

/// Toggles the do-not-play tag on the selected tracks (off when all have
/// it, else on).
pub(crate) fn toggle_dnp(app: &App) {
    let ids = selected(app);
    if ids.is_empty() {
        return;
    }
    let all = ids.iter().all(|t| is_dnp(app, *t));
    set_dnp(app, !all);
}

/// Sets or clears the do-not-play tag on the selected tracks, in one undo
/// step.
pub(crate) fn set_dnp(app: &App, on: bool) {
    if app.guard_read_only() {
        return;
    }
    let ids = selected(app);
    if ids.is_empty() {
        return;
    }
    let tag = UserTag::do_not_play();
    let states: Vec<od_library::EntityState> = ids
        .iter()
        .map(|id| {
            let mut tags = app.state.borrow().lib.user_tags(*id).unwrap_or_default();
            tags.retain(|t| !t.is_do_not_play());
            if on {
                tags.push(tag.clone());
            }
            od_library::EntityState::UserTags { track: *id, tags }
        })
        .collect();
    let r = app.state.borrow_mut().lib.edit("Edit Tags", states);
    if app.report(r).is_none() {
        return;
    }
    app.after_edit(if on {
        "Tagged do not play: auto-play leaves these out"
    } else {
        "Do-not-play tag removed"
    });
    project(app);
}

pub(crate) fn wire(app: &Rc<App>) {
    load(app);
    let g = app.ui.global::<crate::Queue>();
    g.on_add(handler!(app, |a| {
        let ids = selected(&a);
        add(&a, &ids);
    }));
    g.on_load_next(handler!(app, |a| {
        load_next(&a);
    }));
    g.on_remove(handler!(app, |a, i: i32| {
        with(|s| {
            if (i as usize) < s.items.len() {
                s.items.remove(i as usize);
            }
        });
        save(&a);
        project(&a);
    }));
    g.on_move(handler!(app, |a, i: i32, d: i32| {
        with(|s| move_item(&mut s.items, i.max(0) as usize, d));
        save(&a);
        project(&a);
    }));
    g.on_request(handler!(app, |a, t: SharedString| request(&a, &t)));
    g.on_match(handler!(app, |a, i: i32| match_selected(&a, i.max(0) as usize)));
    g.on_set_auto(handler!(app, |a, on: bool| {
        // Off in the middle of a mix: the mix ends at once where it was
        // going (the incoming track up, the outgoing one down).
        let running = with(|s| {
            s.auto = on;
            if on { None } else { s.mix.take() }
        });
        if let Some(m) = running {
            let (out, inc, x) = mix_levels(1.0, m.crossfader);
            match x {
                Some(x) => engine::send(&a, Command::Mixer(MixerCommand::SetCrossfader(x))),
                None => {
                    ch(&a, m.from, ChannelCommand::SetFader(out));
                    ch(&a, m.to, ChannelCommand::SetFader(inc));
                }
            }
        }
        a.status(if on {
            "Auto-play on: the queue plays on by itself"
        } else {
            "Auto-play off"
        });
        project(&a);
    }));
    g.on_set_mix(handler!(app, |a, i: i32| {
        let beats = MIXES.get(i.max(0) as usize).copied().unwrap_or(16);
        a.state.borrow_mut().change_settings(|s| s.queue.mix_beats = beats);
        project(&a);
    }));
    g.on_set_warn(handler!(app, |a, i: i32| {
        let secs = WARNS.get(i.max(0) as usize).copied().unwrap_or(30);
        a.state.borrow_mut().change_settings(|s| s.queue.warn_secs = secs);
        project(&a);
    }));
    g.on_alert_action(handler!(app, |a, what: SharedString| match what.as_str() {
        "load" => {
            load_next(&a);
        }
        "unload" | "keep" => {
            let alert = with(|s| s.alert);
            if let Some(("dnp", d)) = alert {
                let track = a.decks.borrow().loaded_id(d);
                with(|s| {
                    s.alert = None;
                    if let Some(t) = track
                        && what.as_str() == "keep"
                    {
                        s.kept.push((d, t));
                    }
                });
                if what.as_str() == "unload" {
                    decks::unload(&a, d);
                }
            }
        }
        _ => {}
    }));
    project(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn it(t: i64) -> Item {
        Item { track: Some(TrackId(t)), request: String::new() }
    }

    #[test]
    fn next_skips_do_not_play_and_unmatched_requests() {
        let items = vec![
            Item { track: None, request: "x".into() },
            it(1),
            it(2),
        ];
        assert_eq!(next_playable(&items, |_| false), Some(1));
        assert_eq!(next_playable(&items, |t| t == TrackId(1)), Some(2));
        assert_eq!(next_playable(&items, |_| true), None);
    }

    #[test]
    fn moves_within_the_list() {
        let mut v = vec![it(1), it(2), it(3)];
        assert_eq!(move_item(&mut v, 2, -1), Some(1));
        assert_eq!(v.iter().map(|i| i.track.unwrap().0).collect::<Vec<_>>(), [1, 3, 2]);
        assert_eq!(move_item(&mut v, 0, -1), None);
        assert_eq!(move_item(&mut v, 2, 1), None);
    }

    #[test]
    fn mix_levels_and_lengths() {
        assert_eq!(mix_levels(0.0, None), (1.0, 0.0, None));
        assert_eq!(mix_levels(0.25, None), (0.75, 0.25, None));
        assert_eq!(mix_levels(2.0, None), (0.0, 1.0, None));
        assert_eq!(mix_levels(0.5, Some((-1.0, 1.0))), (1.0, 1.0, Some(0.0)));
        assert!((mix_secs(16, 120.0) - 8.0).abs() < 1e-9);
        assert_eq!(mix_secs(0, 120.0), 0.0);
        assert_eq!(mix_secs(16, 0.0), 0.0);
    }

    #[test]
    fn dead_air_only_when_nothing_else_carries_on() {
        // Deck 0 has 20 s left, warn at 30 s.
        assert_eq!(dead_air(&[(0, 20.0)], &[], 30.0), Some((0, 20.0)));
        assert_eq!(dead_air(&[(0, 40.0)], &[], 30.0), None, "not yet");
        assert_eq!(dead_air(&[(0, 20.0)], &[1], 30.0), None, "deck 2 is cued");
        assert_eq!(dead_air(&[(0, 20.0), (1, 200.0)], &[], 30.0), None, "deck 2 plays on");
        assert_eq!(dead_air(&[], &[], 30.0), None, "nothing on air");
        assert_eq!(dead_air(&[(0, 20.0), (1, 10.0)], &[], 30.0), Some((1, 10.0)));
    }
}
