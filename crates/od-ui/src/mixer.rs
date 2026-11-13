//! Mixer: channel strips, master, headphones and crossfader.

use crate::app::{App, handler};
use crate::models::Rows;
use crate::{ChannelView, decks, engine};
use od_engine::{
    ChannelCommand, ChannelFxKind, ChannelState, Command, DeckCommand, DeckState, EngineState,
    MixerCommand, Stem, SwapAt,
};
use slint::ComponentHandle;
use slint::Model as _;
use std::cell::Cell;
use std::rc::Rc;

/// Parts on the EQ knobs in STEM mode, by EQ band (0 low, 1 mid, 2 high).
pub(crate) const PART_BANDS: [Stem; 3] = [Stem::Drums, Stem::Inst, Stem::Vocals];

/// Part level onto an EQ knob (−1..1): centre and above = full level.
pub(crate) fn part_to_knob(level: f32) -> f32 {
    (level - 1.0).clamp(-1.0, 0.0)
}

/// EQ knob onto a part level.
pub(crate) fn knob_to_part(v: f32) -> f32 {
    (v + 1.0).clamp(0.0, 1.0)
}

#[derive(Default)]
pub(crate) struct MixerView {
    /// Bound to `channels`; rows are written only when a value changed.
    pub channels: Rows<ChannelView>,
    /// STEM per channel.
    pub parts: Cell<[bool; decks::DECKS]>,
    /// Auto Fader on, and the low EQ it last set per channel.
    pub smart: Cell<bool>,
    smart_low: Cell<[Option<f32>; decks::DECKS]>,
    /// When a swap happens.
    pub swap_at: Cell<SwapAt>,
}

/// The next swap timing: now → beat → bar → now.
pub(crate) fn next_swap_at(at: SwapAt) -> SwapAt {
    match at {
        SwapAt::Now => SwapAt::Beat,
        SwapAt::Beat => SwapAt::Bar,
        SwapAt::Bar => SwapAt::Now,
    }
}

/// The key legend of a swap timing.
pub(crate) fn swap_at_label(at: SwapAt) -> String {
    match at {
        SwapAt::Now => od_i18n::tr("NOW"),
        SwapAt::Beat => od_i18n::tr("BEAT"),
        SwapAt::Bar => od_i18n::tr("BAR"),
    }
    .into()
}

/// The two decks (0-based, in order) an EQ swap acts on among the first
/// `count`: the two loudest playing decks; with one playing, it and the
/// next deck with a track (the one being mixed in); else decks 1 and 2.
pub(crate) fn swap_pair(state: &EngineState, count: usize) -> (usize, usize) {
    let count = count.clamp(2, decks::DECKS);
    let mut playing: Vec<usize> = (0..count)
        .filter(|&d| state.decks[d].playing && state.decks[d].track.is_some())
        .collect();
    playing.sort_by(|&a, &b| {
        state.audible.level[b]
            .partial_cmp(&state.audible.level[a])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });
    let pair = match playing.as_slice() {
        [a, b, ..] => (*a, *b),
        [a] => {
            let other = (1..count)
                .map(|i| (a + i) % count)
                .find(|&d| state.decks[d].track.is_some())
                .unwrap_or(if *a == 0 { 1 } else { 0 });
            (*a, other)
        }
        [] => (0, 1),
    };
    (pair.0.min(pair.1), pair.0.max(pair.1))
}

/// A dB value for a readout: one decimal below 10 dB, a true minus sign.
fn db_text(db: f32) -> String {
    if db.abs() < 0.05 {
        return "0".into();
    }
    let t = if db.abs() >= 9.95 {
        format!("{:+.0}", db)
    } else {
        format!("{:+.1}", db)
    };
    t.replace('-', "\u{2212}")
}

/// The value under an EQ knob: its gain in dB, `OFF` at the isolator's
/// full cut and `KILL` while the band's kill is on.
pub(crate) fn eq_text(v: f32, kill: bool, classic: bool) -> String {
    if kill {
        return od_i18n::tr("KILL").into();
    }
    let v = v.clamp(-1.0, 1.0);
    if classic {
        return db_text(if v < 0.0 { v * 26.0 } else { v * 6.0 });
    }
    if v <= -0.999 {
        return od_i18n::tr("OFF").into();
    }
    db_text(20.0 * od_engine::dsp::eq::knob_to_gain(v).log10())
}

/// The value under the trim knob, in dB.
pub(crate) fn trim_text(db: f32) -> String {
    db_text(db)
}

/// The value under the CH FX knob: the filter's side and depth in percent
/// (`LP40`, `HP25`), else the signed amount; `0` in the centre.
pub(crate) fn fx_text(v: f32, kind: ChannelFxKind) -> String {
    let pct = (v.abs().min(1.0) * 100.0).round() as i32;
    if pct == 0 {
        return "0".into();
    }
    match kind {
        ChannelFxKind::Filter if v < 0.0 => format!("LP{pct}"),
        ChannelFxKind::Filter => format!("HP{pct}"),
        _ if v < 0.0 => format!("\u{2212}{pct}"),
        _ => format!("+{pct}"),
    }
}

/// Auto Fader bass swap: the low EQ of the A side (`a = true`) or B side
/// for crossfader position `u` (0 = A … 1 = B). The basses cross between
/// 40 % and 60 %; the side being faded out loses its bass.
pub(crate) fn smart_low(u: f32, a: bool) -> f32 {
    let t = ((u - 0.4) / 0.2).clamp(0.0, 1.0);
    if a { -t } else { -(1.0 - t) }
}

/// Trim in dB (−24..+12) onto the knob's −1..1.
pub(crate) fn trim_to_knob(db: f32) -> f32 {
    if db < 0.0 { db / 24.0 } else { db / 12.0 }
}

/// Knob −1..1 onto trim in dB (−24..+12).
pub(crate) fn knob_to_trim(v: f32) -> f32 {
    if v < 0.0 { v * 24.0 } else { v * 12.0 }
}

/// A channel in STEM mode: the EQ knobs show the deck's part levels (pure).
pub(crate) fn parts_view(channel: usize, c: &ChannelState, d: &DeckState) -> ChannelView {
    let part = |band: usize| part_to_knob(d.stem_levels[PART_BANDS[band].index()]);
    let pct = |band: usize| format!("{}", (knob_to_part(part(band)) * 100.0).round() as i32);
    ChannelView {
        high: part(2),
        mid: part(1),
        low: part(0),
        high_text: pct(2).into(),
        mid_text: pct(1).into(),
        low_text: pct(0).into(),
        // The legends stay kill keys: a killed part is a muted one.
        kill_high: d.stem_levels[PART_BANDS[2].index()] < 0.01,
        kill_mid: d.stem_levels[PART_BANDS[1].index()] < 0.01,
        kill_low: d.stem_levels[PART_BANDS[0].index()] < 0.01,
        parts: true,
        ..channel_view(channel, c, false)
    }
}

/// A channel's row; `classic`: the EQ is in classic mode (its dB range).
pub(crate) fn channel_view(channel: usize, c: &ChannelState, classic: bool) -> ChannelView {
    ChannelView {
        kill_high: c.eq_kill[2],
        kill_mid: c.eq_kill[1],
        kill_low: c.eq_kill[0],
        trim_text: trim_text(c.trim_db).into(),
        high_text: eq_text(c.eq[2], c.eq_kill[2], classic).into(),
        mid_text: eq_text(c.eq[1], c.eq_kill[1], classic).into(),
        low_text: eq_text(c.eq[0], c.eq_kill[0], classic).into(),
        fx_text: fx_text(c.channel_fx_amount, c.channel_fx).into(),
        clip: c.meter.clipped,
        parts: false,
        channel: channel as i32 + 1,
        trim: trim_to_knob(c.trim_db),
        high: c.eq[2],
        mid: c.eq[1],
        low: c.eq[0],
        channel_fx: c.channel_fx_amount,
        fader: c.fader,
        cue: c.cue,
        level: c.meter.rms.min(1.0),
        peak: c.meter.peak,
        // Peak hold: the highest recent peak, falling back after ~1.5 s.
        hold: c.meter.peak_hold.min(1.0),
    }
}

/// Frame tick: engine state → channel rows and master section.
pub(crate) fn project(app: &App, state: &EngineState) {
    let count = app.decks.borrow().deck_count;
    let parts = app.mixer.parts.get();
    let classic = app.state.borrow().settings.mixer.eq_mode == "classic";
    app.mixer.channels.sync_eq(
        (0..count)
            .map(|d| {
                if parts[d] {
                    parts_view(d, &state.channels[d], &state.decks[d])
                } else {
                    channel_view(d, &state.channels[d], classic)
                }
            })
            .collect(),
    );
    auto_fader(app, state);
    let ui = &app.ui;
    let g = ui.global::<crate::MixerParts>();
    let (a, b) = swap_pair(state, count);
    let pair = format!("{}/{}", a + 1, b + 1);
    if g.get_swap_pair() != pair.as_str() {
        g.set_swap_pair(pair.into());
    }
    g.set_swap_pending(
        state
            .pending_swap
            .filter(|p| (p.a.min(p.b), p.a.max(p.b)) == (a, b))
            .map_or(-1, |p| p.band as i32),
    );
    let at = swap_at_label(app.mixer.swap_at.get());
    if g.get_swap_at() != at.as_str() {
        g.set_swap_at(at.into());
    }
    ui.set_master_level(state.master_level);
    ui.set_headphone_mix(state.headphone_mix);
    ui.set_headphone_level(state.headphone_level);
    ui.set_master_bpm(
        if state.master_deck.is_some() {
            format!("{:.1}", state.master_bpm)
        } else {
            String::new()
        }
        .into(),
    );
    ui.set_crossfader((state.crossfader + 1.0) / 2.0);
    let xf: Vec<slint::SharedString> = state
        .channels
        .iter()
        .take(decks::DECKS)
        .map(|c| match c.xf_assign {
            od_engine::XfAssign::A => "A",
            od_engine::XfAssign::B => "B",
            od_engine::XfAssign::Thru => "T",
        })
        .map(Into::into)
        .collect();
    let now = ui.get_xf_assign();
    if now.row_count() != xf.len()
        || xf
            .iter()
            .enumerate()
            .any(|(i, x)| now.row_data(i).as_ref() != Some(x))
    {
        ui.set_xf_assign(crate::models::model(xf));
    }
}

/// Auto Fader: moves the low EQs of the A and B channels with the
/// crossfader (only when they change).
fn auto_fader(app: &App, state: &EngineState) {
    if !app.mixer.smart.get() {
        return;
    }
    let u = (state.crossfader + 1.0) / 2.0;
    let mut last = app.mixer.smart_low.get();
    let count = app.decks.borrow().deck_count.min(decks::DECKS);
    for c in 0..count {
        let side = match state.channels[c].xf_assign {
            od_engine::XfAssign::A => true,
            od_engine::XfAssign::B => false,
            od_engine::XfAssign::Thru => continue,
        };
        let low = smart_low(u, side);
        if last[c].is_none_or(|l| (l - low).abs() > 0.005) {
            engine::send(
                app,
                Command::Channel(
                    c,
                    ChannelCommand::SetEq {
                        band: 0,
                        value: low,
                    },
                ),
            );
            last[c] = Some(low);
        }
    }
    app.mixer.smart_low.set(last);
}

/// Auto Fader on / off: on, the decks on the crossfader follow the master
/// tempo; off, their basses return to flat.
pub(crate) fn toggle_auto_fader(app: &App) {
    engine::ensure_engine(app);
    let on = !app.mixer.smart.get();
    app.mixer.smart.set(on);
    app.ui.global::<crate::MixerParts>().set_auto_fader(on);
    let Some(state) = engine::snapshot(app) else {
        return;
    };
    let count = app.decks.borrow().deck_count.min(decks::DECKS);
    for c in 0..count {
        if state.channels[c].xf_assign == od_engine::XfAssign::Thru {
            continue;
        }
        if on {
            if state.master_deck != Some(c) && state.decks[c].track.is_some() {
                engine::deck_cmd(app, c, DeckCommand::SetSync(true));
            }
        } else if app.mixer.smart_low.get()[c].is_some() {
            engine::send(
                app,
                Command::Channel(
                    c,
                    ChannelCommand::SetEq {
                        band: 0,
                        value: 0.0,
                    },
                ),
            );
        }
    }
    app.mixer.smart_low.set([None; decks::DECKS]);
}

/// Kill of EQ band `band` (0 low, 1 mid, 2 high) on channel `c`
/// (0-based) on or off.
pub(crate) fn toggle_kill(app: &App, c: usize, band: usize) {
    engine::ensure_engine(app);
    let c = c.min(decks::DECKS - 1);
    let band = band.min(2);
    // STEM mode: the kill key mutes the band's part (and brings it back).
    if app.mixer.parts.get()[c] {
        let stem = PART_BANDS[band];
        let level = engine::snapshot(app).map_or(1.0, |s| s.decks[c].stem_levels[stem.index()]);
        engine::deck_cmd(
            app,
            c,
            DeckCommand::SetStemLevel {
                stem,
                level: if level < 0.01 { 1.0 } else { 0.0 },
            },
        );
        return;
    }
    let on = engine::snapshot(app).is_some_and(|s| s.channels[c].eq_kill[band]);
    engine::send(
        app,
        Command::Channel(c, ChannelCommand::SetEqKill { band, on: !on }),
    );
}

/// Split cue on / off (cue left, master right on the headphones).
pub(crate) fn toggle_split_cue(app: &App) {
    engine::ensure_engine(app);
    let on = !engine::snapshot(app).is_some_and(|s| s.split_cue);
    engine::send(app, Command::Mixer(MixerCommand::SetSplitCue(on)));
    app.ui.global::<crate::MixerParts>().set_split_cue(on);
}

/// Crossfader in use or not (off: every channel plays as if THRU).
pub(crate) fn toggle_crossfader(app: &App) {
    engine::ensure_engine(app);
    let on = !engine::snapshot(app).is_none_or(|s| s.crossfader_on);
    engine::send(app, Command::Mixer(MixerCommand::SetCrossfaderOn(on)));
    app.ui.global::<crate::MixerParts>().set_xf_on(on);
    app.status(if on {
        od_i18n::tr("Crossfader on")
    } else {
        od_i18n::tr("Crossfader off: every channel plays")
    });
}

/// The spoken name of a crossfader curve setting.
pub(crate) fn curve_name(curve: &str) -> String {
    match curve {
        "linear" => od_i18n::tr("LINEAR"),
        "cut" => od_i18n::tr("CUT"),
        _ => od_i18n::tr("CONSTANT POWER"),
    }
    .into()
}

/// The curve after `curve`: constant power → linear → cut → constant power.
pub(crate) fn next_curve(curve: &str) -> &'static str {
    let all = crate::settings::CROSSFADER_CURVES;
    let i = all.iter().position(|c| *c == curve).unwrap_or(0);
    all[(i + 1) % all.len()]
}

/// Sets the crossfader curve (`None`: the next one) in the settings and
/// the engine, as Preferences › Controller does.
pub(crate) fn set_crossfader_curve(app: &App, curve: Option<&str>) {
    let now = app.state.borrow().settings.mixer.crossfader_curve.clone();
    let curve = curve
        .filter(|c| crate::settings::CROSSFADER_CURVES.contains(c))
        .unwrap_or_else(|| next_curve(&now))
        .to_string();
    crate::preferences::set_crossfader_curve(app, &curve);
    show_curve(app);
    app.status(format!("{} {}", od_i18n::tr("Crossfader curve:"), curve_name(&curve)));
}

/// The curve key shows the curve in the settings.
pub(crate) fn show_curve(app: &App) {
    let curve = app.state.borrow().settings.mixer.crossfader_curve.clone();
    let g = app.ui.global::<crate::MixerParts>();
    g.set_xf_curve_name(curve_name(&curve).into());
    g.set_xf_curve(curve.into());
}

/// Cut-in on deck `d`'s channel (0-based) while `on`.
pub(crate) fn cut_in(app: &App, d: usize, on: bool) {
    engine::ensure_engine(app);
    let c = d.min(decks::DECKS - 1);
    engine::send(app, Command::Channel(c, ChannelCommand::SetCutIn(on)));
}

/// One-tap swap of EQ band `band` between the pair of decks, at the set
/// timing; a second tap while it waits cancels it (the engine decides).
pub(crate) fn swap(app: &App, band: usize) {
    engine::ensure_engine(app);
    let Some(state) = engine::snapshot(app) else {
        return;
    };
    let count = app.decks.borrow().deck_count;
    let (a, b) = swap_pair(&state, count);
    engine::send(
        app,
        Command::Mixer(MixerCommand::SwapEq {
            a,
            b,
            band: band.min(2),
            at: app.mixer.swap_at.get(),
        }),
    );
}

pub(crate) fn cycle_swap_at(app: &App) {
    let at = next_swap_at(app.mixer.swap_at.get());
    app.mixer.swap_at.set(at);
    app.ui
        .global::<crate::MixerParts>()
        .set_swap_at(swap_at_label(at).into());
}

/// Documentation screenshots: `OPENDJDECKS_DEMO_PROOF=mixer` stages a bass
/// swap: deck 2's LOW killed, a few EQ and filter values, a clip on
/// channel 1 and a LOW swap waiting for the next bar (re-armed while the
/// demo runs).
fn demo(app: &Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else { return };
    if !v.split(',').any(|p| p == "mixer") {
        return;
    }
    let weak = Rc::downgrade(app);
    slint::Timer::single_shot(std::time::Duration::from_secs(4), move || {
        let Some(a) = weak.upgrade() else { return };
        let ch = |c: usize, cmd| engine::send(&a, Command::Channel(c, cmd));
        ch(1, ChannelCommand::SetEqKill { band: 0, on: true });
        ch(0, ChannelCommand::SetEq { band: 2, value: -0.3 });
        ch(0, ChannelCommand::SetEq { band: 1, value: 0.25 });
        ch(1, ChannelCommand::SetChannelFxAmount(-0.35));
        ch(0, ChannelCommand::SetTrim(12.0));
        a.mixer.swap_at.set(SwapAt::Bar);
        let w = Rc::downgrade(&a);
        slint::Timer::single_shot(std::time::Duration::from_secs(1), move || {
            if let Some(a) = w.upgrade() {
                engine::send(&a, Command::Channel(0, ChannelCommand::SetTrim(0.0)));
            }
        });
        let w = Rc::downgrade(&a);
        let rearm = Rc::new(slint::Timer::default());
        let keep = rearm.clone();
        rearm.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(250),
            move || {
                let _ = &keep;
                let Some(a) = w.upgrade() else { return };
                if engine::snapshot(&a).is_some_and(|s| s.pending_swap.is_none()) {
                    swap(&a, 0);
                }
            },
        );
    });
}

pub(crate) fn wire(app: &Rc<App>) {
    demo(app);
    let ui = &app.ui;
    let g = ui.global::<crate::MixerParts>();
    g.on_kill(handler!(app, |a, c: i32, band: i32| toggle_kill(
        &a,
        (c as usize).saturating_sub(1),
        band.max(0) as usize
    )));
    g.on_swap(handler!(app, |a, band: i32| swap(&a, band.max(0) as usize)));
    g.on_toggle_split_cue(handler!(app, |a| toggle_split_cue(&a)));
    g.on_cycle_xf_curve(handler!(app, |a| set_crossfader_curve(&a, None)));
    show_curve(app);
    g.on_cycle_swap_at(handler!(app, |a| cycle_swap_at(&a)));
    g.on_reset_clip(handler!(app, |a| engine::send(
        &a,
        Command::Mixer(MixerCommand::ResetClipIndicators)
    )));
    // Channel callbacks carry 1-based channel numbers and start audio.
    fn ch(app: &App, c: i32, cmd: ChannelCommand) {
        engine::ensure_engine(app);
        let c = (c as usize).saturating_sub(1).min(decks::DECKS - 1);
        engine::send(app, Command::Channel(c, cmd));
    }
    ui.on_mixer_trim(handler!(app, |a, c: i32, v: f32| ch(
        &a,
        c,
        ChannelCommand::SetTrim(knob_to_trim(v))
    )));
    ui.on_mixer_eq(handler!(app, |a, c: i32, band: i32, v: f32| {
        let i = (c as usize).saturating_sub(1).min(decks::DECKS - 1);
        if a.mixer.parts.get()[i] {
            engine::ensure_engine(&a);
            let stem = PART_BANDS[(band as usize).min(2)];
            engine::deck_cmd(
                &a,
                i,
                DeckCommand::SetStemLevel {
                    stem,
                    level: knob_to_part(v),
                },
            );
        } else {
            ch(
                &a,
                c,
                ChannelCommand::SetEq {
                    band: band as usize,
                    value: v,
                },
            )
        }
    }));
    ui.global::<crate::MixerParts>()
        .on_toggle_auto_fader(handler!(app, |a| toggle_auto_fader(&a)));
    ui.global::<crate::MixerParts>()
        .on_toggle_xf(handler!(app, |a| toggle_crossfader(&a)));
    ui.global::<crate::MixerParts>()
        .on_toggle(handler!(app, |a, c: i32| {
            let i = (c as usize).saturating_sub(1).min(decks::DECKS - 1);
            let mut p = a.mixer.parts.get();
            p[i] = !p[i];
            a.mixer.parts.set(p);
            // STEM on a deck without parts: the switch asks for them.
            if p[i]
                && !engine::snapshot(&a)
                    .is_some_and(|s| s.decks[i].stems_available)
            {
                crate::stems_model::part_pressed(&a, i);
            }
        }));
    ui.on_mixer_channel_fx(handler!(app, |a, c: i32, v: f32| ch(
        &a,
        c,
        ChannelCommand::SetChannelFxAmount(v)
    )));
    ui.on_mixer_fader(handler!(app, |a, c: i32, v: f32| ch(
        &a,
        c,
        ChannelCommand::SetFader(v)
    )));
    ui.on_mixer_cue(handler!(app, |a, c: i32| {
        engine::ensure_engine(&a);
        let i = (c as usize).saturating_sub(1).min(decks::DECKS - 1);
        let on = engine::snapshot(&a)
            .map(|s| s.channels[i].cue)
            .unwrap_or(false);
        ch(&a, c, ChannelCommand::SetCue(!on));
    }));
    // Crossfader assign keys: a deck goes to the pressed side, or back to THRU.
    ui.on_mixer_xf_assign(handler!(app, |a, d: i32, side: slint::SharedString| {
        engine::ensure_engine(&a);
        let i = (d as usize).saturating_sub(1).min(decks::DECKS - 1);
        let want = if side.as_str() == "A" {
            od_engine::XfAssign::A
        } else {
            od_engine::XfAssign::B
        };
        let now = engine::snapshot(&a)
            .map(|s| s.channels[i].xf_assign)
            .unwrap_or_default();
        let next = if now == want {
            od_engine::XfAssign::Thru
        } else {
            want
        };
        ch(&a, d, ChannelCommand::SetCrossfaderAssign(next));
    }));
    ui.on_set_crossfader(handler!(app, |a, v: f32| engine::send(
        &a,
        Command::Mixer(MixerCommand::SetCrossfader(v * 2.0 - 1.0))
    )));
    ui.on_set_master_level(handler!(app, |a, v: f32| engine::send(
        &a,
        Command::Mixer(MixerCommand::SetMasterLevel(v))
    )));
    ui.on_set_headphone_mix(handler!(app, |a, v: f32| engine::send(
        &a,
        Command::Mixer(MixerCommand::SetHeadphoneMix(v))
    )));
    ui.on_set_headphone_level(handler!(app, |a, v: f32| engine::send(
        &a,
        Command::Mixer(MixerCommand::SetHeadphoneLevel(v))
    )));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_mapping_round_trips() {
        for db in [-24.0, -12.0, 0.0, 6.0, 12.0] {
            assert!((knob_to_trim(trim_to_knob(db)) - db).abs() < 1e-5);
        }
        assert_eq!(trim_to_knob(-24.0), -1.0);
        assert_eq!(trim_to_knob(12.0), 1.0);
    }

    #[test]
    fn channel_projection() {
        let c = ChannelState {
            fader: 0.7,
            cue: true,
            ..Default::default()
        };
        let v = channel_view(0, &c, false);
        assert_eq!(v.channel, 1);
        assert_eq!(v.fader, 0.7);
        assert!(v.cue);
        assert_eq!(v, channel_view(0, &c, false));
    }

    #[test]
    fn readouts_show_values_kills_and_clip() {
        let mut c = ChannelState::default();
        c.eq = [-1.0, -0.5, 1.0];
        c.eq_kill = [false, true, false];
        c.trim_db = -6.0;
        c.channel_fx_amount = -0.4;
        c.meter.clipped = true;
        let v = channel_view(0, &c, false);
        assert_eq!(v.low_text.as_str(), "OFF");
        assert_eq!(v.mid_text.as_str(), "KILL");
        assert!(v.kill_mid && !v.kill_low && !v.kill_high);
        assert_eq!(v.high_text.as_str(), "+6.0");
        assert_eq!(v.trim_text.as_str(), "\u{2212}6.0");
        assert_eq!(v.fx_text.as_str(), "LP40");
        assert!(v.clip);
        // Classic EQ: −1 is −26 dB.
        assert_eq!(channel_view(0, &c, true).low_text.as_str(), "\u{2212}26");
        // Isolator: −0.5 is about −18 dB.
        assert_eq!(eq_text(-0.5, false, false), "\u{2212}18");
        assert_eq!(eq_text(0.0, false, false), "0");
        assert_eq!(fx_text(0.25, ChannelFxKind::Filter), "HP25");
        assert_eq!(fx_text(-0.3, ChannelFxKind::Echo), "\u{2212}30");
        assert_eq!(fx_text(0.001, ChannelFxKind::Filter), "0");
    }

    #[test]
    fn swap_timing_cycles() {
        assert_eq!(next_swap_at(SwapAt::Now), SwapAt::Beat);
        assert_eq!(next_swap_at(SwapAt::Beat), SwapAt::Bar);
        assert_eq!(next_swap_at(SwapAt::Bar), SwapAt::Now);
    }

    #[test]
    fn swap_pair_follows_the_playing_decks() {
        let mut s = EngineState::default();
        assert_eq!(swap_pair(&s, 4), (0, 1), "nothing plays: decks 1 and 2");
        let track = Some(od_core::TrackId(1));
        for d in [1, 2, 3] {
            s.decks[d].track = track;
        }
        s.decks[3].playing = true;
        // One plays: it and the next deck with a track.
        assert_eq!(swap_pair(&s, 4), (1, 3));
        s.decks[2].playing = true;
        s.decks[1].playing = true;
        s.audible.level = [0.0, 0.1, 0.5, 0.4];
        // The two loudest of three.
        assert_eq!(swap_pair(&s, 4), (2, 3));
        // Only the decks shown count.
        assert_eq!(swap_pair(&s, 2), (0, 1));
    }
}
