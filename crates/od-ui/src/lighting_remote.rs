//! LIGHTING: a separate lighting operator (roadmap [34]: a controller of
//! its own, OSC in, the music sent to another instance), DJ events as
//! triggers and music sources beyond the decks ([36]), and unattended mode
//! for a fixed venue ([37]).
//!
//! What it changes is kept in the rig file (`lighting.json`): the
//! controller given to the lights, OSC, the music source, the triggers
//! (with the show) and unattended mode. The OSC socket, the audio input
//! and the controller assignment are opened again from it on every start.

use std::net::SocketAddr;
use std::rc::Rc;

use od_controller::osc::{OscEvent, OscReceiver, OscSender};
use od_lighting::events::TRIGGER_BEATS;
use od_lighting::{DjEvent, MusicSource, Trigger, TriggerTarget, Unattended, CALM_AFTER};
use slint::SharedString;

use crate::app::{App, handler};
use crate::lighting::{LightingRuntime, each, edit, refresh};
use crate::lighting_stage::{Plays, fade_label, plays_short, targets};
use crate::models::Rows;

/// Live state of the remote parts (never saved).
#[derive(Default)]
pub struct Remote {
    pub osc: Option<OscReceiver>,
    pub osc_error: String,
    /// The audio input while it is the source.
    pub input: Option<od_audio_io::AudioInput>,
    pub input_error: String,
    /// MIDI inputs to choose the lighting controller from; fixed by tests
    /// and the documentation hook.
    pub midi_inputs: Vec<String>,
    pub midi_fixed: bool,
    pub send_error: String,
    /// Listen on the loopback address only (tests and the documentation
    /// pictures: nothing from outside this computer).
    pub loopback_only: bool,
    pub(crate) trigger_rows: Rows<crate::LightTriggerRow>,
    /// From the show thread: the trigger playing, the last event, what
    /// arrives from outside the decks, silence and the calm look.
    pub trigger: Option<od_lighting::ActiveTrigger>,
    pub last_event: Option<DjEvent>,
    pub outside: Option<(bool, f64)>,
    pub input_db: f32,
    pub silent: f32,
    pub calm: f32,
    pub sent: u64,
}

/// [37] The show runs unattended and the setup is locked: the show
/// controls, the lighting controller and OSC are locked as well.
pub(crate) fn show_locked(rt: &LightingRuntime) -> bool {
    rt.config.unattended.enabled && rt.locked()
}

/// Label of a DJ event.
pub fn event_label(e: DjEvent) -> String {
    od_i18n::tr(e.label()).to_string()
}

/// Everything a trigger can play, in stepping order: nothing, RELEASE,
/// then the cue stack and the pads (as a light cue can).
fn trigger_targets(s: &od_lighting::ShowSettings) -> Vec<Option<TriggerTarget>> {
    [None, Some(TriggerTarget::Release)]
        .into_iter()
        .chain(targets(s).into_iter().map(|p| {
            Some(match p {
                Plays::Cue(i) => TriggerTarget::Cue(i),
                Plays::Pad(p) => TriggerTarget::Pad(p),
            })
        }))
        .collect()
}

fn target_label(s: &od_lighting::ShowSettings, t: Option<TriggerTarget>) -> String {
    match t {
        None => od_i18n::tr("NOTHING").to_string(),
        Some(TriggerTarget::Release) => od_i18n::tr("RELEASE").to_string(),
        Some(TriggerTarget::Cue(i)) => plays_short(s, Plays::Cue(i)),
        Some(TriggerTarget::Pad(p)) => plays_short(s, Plays::Pad(p)),
    }
}

fn beats_label(beats: f32) -> String {
    if beats <= 0.0 {
        od_i18n::tr("UNTIL NEXT").to_string()
    } else {
        fade_label(beats)
    }
}

fn trigger_rows(rt: &LightingRuntime) -> Vec<crate::LightTriggerRow> {
    let s = &rt.config.settings;
    DjEvent::ALL
        .iter()
        .map(|e| {
            let t = s.triggers.iter().find(|t| t.event == *e);
            crate::LightTriggerRow {
                event: event_label(*e).into(),
                plays: target_label(s, t.map(|t| t.target)).into(),
                beats: match t {
                    Some(t) if t.target != TriggerTarget::Release => beats_label(t.beats),
                    _ => "—".to_string(),
                }
                .into(),
                set: t.is_some(),
                active: rt.remote.trigger.is_some_and(|a| a.event == *e),
            }
        })
        .collect()
}

/// "INPUT · 126 BPM", "NETWORK · NOTHING ARRIVES", "DECKS".
fn source_text(rt: &LightingRuntime) -> String {
    let r = &rt.remote;
    match rt.config.music {
        MusicSource::Decks => od_i18n::tr("THE DECKS OF THIS APP").to_string(),
        MusicSource::Input => {
            if !r.input_error.is_empty() {
                r.input_error.clone()
            } else {
                match r.outside {
                    Some((true, bpm)) => od_i18n::trf("{} · {} BPM", &[
                        &r.input.as_ref().map_or(String::new(), |i| i.device_name.clone()),
                        &format!("{bpm:.0}"),
                    ]),
                    _ => od_i18n::trf("{} · SILENCE", &[
                        &r.input.as_ref().map_or(String::new(), |i| i.device_name.clone()),
                    ]),
                }
            }
        }
        MusicSource::Network => match r.outside {
            Some((true, bpm)) => od_i18n::trf("FROM THE NETWORK · {} BPM", &[&format!("{bpm:.0}")]),
            Some((false, _)) if rt.config.remote.osc_in => {
                od_i18n::tr("NOTHING ARRIVES · OSC IN IS ON").to_string()
            }
            _ => od_i18n::tr("TURN OSC IN ON UNDER SETUP › REMOTE").to_string(),
        },
    }
}

fn osc_note(rt: &LightingRuntime) -> String {
    let r = &rt.remote;
    if !rt.config.remote.osc_in {
        return od_i18n::tr("OFF").to_string();
    }
    match &r.osc {
        Some(o) => od_i18n::trf(
            "LISTENING ON PORT {} · {} MESSAGES",
            &[&o.local_addr().port(), &o.received()],
        ),
        None => od_i18n::trf("CANNOT LISTEN · {}", &[&r.osc_error]),
    }
}

fn send_note(rt: &LightingRuntime) -> String {
    let r = &rt.remote;
    if !rt.config.remote.send {
        return od_i18n::tr("OFF").to_string();
    }
    if !r.send_error.is_empty() {
        return r.send_error.clone();
    }
    od_i18n::trf("SENDING · {} MESSAGES", &[&r.sent])
}

fn calm_label(secs: f32) -> String {
    if secs >= 60.0 {
        od_i18n::trf("{} MIN", &[&((secs / 60.0).round() as u32)])
    } else {
        od_i18n::trf("{} S", &[&(secs.round() as u32)])
    }
}

fn unattended_text(rt: &LightingRuntime) -> String {
    let r = &rt.remote;
    if r.calm > 0.0 {
        od_i18n::tr("SILENCE · THE CALM LOOK").to_string()
    } else if r.silent >= 1.0 {
        od_i18n::trf("SILENCE FOR {} S · CALM AFTER {}", &[
            &(r.silent as u32),
            &calm_label(rt.config.unattended.calm_after),
        ])
    } else {
        od_i18n::tr("FOLLOWING THE MUSIC").to_string()
    }
}

/// Writes the properties of the remote parts to every lighting window.
pub(crate) fn sync(app: &App) {
    let rt = app.lighting.borrow();
    rt.remote.trigger_rows.sync_eq(trigger_rows(&rt));
    let inputs: Vec<SharedString> = rt.remote.midi_inputs.iter().map(|p| p.as_str().into()).collect();
    let inputs = slint::ModelRc::new(slint::VecModel::from(inputs));
    let controller_note = controller_note(app, &rt);
    each(app, |g| {
        g.set_midi_inputs(inputs.clone());
        g.set_light_controller(rt.config.remote.controller.as_str().into());
        g.set_light_controller_note(controller_note.as_str().into());
        g.set_osc_in(rt.config.remote.osc_in);
        g.set_osc_port(rt.config.remote.osc_port.to_string().into());
        g.set_osc_send(rt.config.remote.send);
        g.set_osc_send_to(rt.config.remote.send_to.as_str().into());
        g.set_source(
            match rt.config.music {
                MusicSource::Decks => "decks",
                MusicSource::Input => "input",
                MusicSource::Network => "network",
            }
            .into(),
        );
        g.set_unattended(rt.config.unattended.enabled);
        g.set_autostart(rt.config.unattended.autostart);
        g.set_calm_after(calm_label(rt.config.unattended.calm_after).into());
        g.set_show_locked(show_locked(&rt));
    });
    drop(rt);
    sync_live(app);
}

/// The texts that follow the show thread (a few times a second).
pub(crate) fn sync_live(app: &App) {
    let rt = app.lighting.borrow();
    rt.remote.trigger_rows.sync_eq(trigger_rows(&rt));
    let source = source_text(&rt);
    let osc = osc_note(&rt);
    let send = send_note(&rt);
    let last = rt.remote.last_event.map(event_label).unwrap_or_default();
    let state = unattended_text(&rt);
    each(app, |g| {
        if g.get_source_text().as_str() != source {
            g.set_source_text(source.as_str().into());
        }
        if g.get_osc_note().as_str() != osc {
            g.set_osc_note(osc.as_str().into());
        }
        if g.get_osc_send_note().as_str() != send {
            g.set_osc_send_note(send.as_str().into());
        }
        if g.get_last_event().as_str() != last {
            g.set_last_event(last.as_str().into());
        }
        if g.get_unattended_text().as_str() != state {
            g.set_unattended_text(state.as_str().into());
        }
    });
}

fn controller_note(app: &App, rt: &LightingRuntime) -> String {
    let port = &rt.config.remote.controller;
    if port.is_empty() {
        return String::new();
    }
    if crate::controller::lighting_connected(app).as_deref() == Some(port.as_str()) {
        od_i18n::tr("CONNECTED · PLAYS THE LIGHTS WITH THE LIGHTING LAYOUT").to_string()
    } else {
        od_i18n::tr("NOT CONNECTED · PLAYS THE LIGHTS ONCE IT IS").to_string()
    }
}

/// Takes the show thread's view of triggers, sources and silence.
pub(crate) fn take_snapshot(rt: &mut LightingRuntime, s: &crate::lighting::Snapshot) {
    let r = &mut rt.remote;
    r.trigger = s.trigger;
    r.last_event = s.last_event;
    r.outside = s.outside;
    r.input_db = s.input_db;
    r.silent = s.silent;
    r.calm = s.calm;
    r.sent = s.sent;
}

/// Opens what the rig asks for: the OSC socket, the sender, the music
/// source (and the audio input), unattended mode and the lighting
/// controller. Called after loading and after every change.
pub(crate) fn apply(app: &App) {
    {
        let mut rt = app.lighting.borrow_mut();
        let rt = &mut *rt;
        let cfg = rt.config.remote.clone();
        // OSC in.
        let want = cfg.osc_in.then_some(cfg.osc_port);
        let have = rt.remote.osc.as_ref().map(|o| o.local_addr().port());
        if want.is_none() {
            rt.remote.osc = None;
            rt.remote.osc_error.clear();
        } else if have != want || rt.remote.osc.is_none() {
            // Port 0 (tests) takes any free port and keeps it.
            if !(want == Some(0) && rt.remote.osc.is_some()) {
                rt.remote.osc = None;
                let host = if rt.remote.loopback_only || cfg!(test) { [127, 0, 0, 1] } else { [0, 0, 0, 0] };
                let addr = SocketAddr::from((host, cfg.osc_port));
                match OscReceiver::bind(addr) {
                    Ok(r) => {
                        rt.remote.osc = Some(r);
                        rt.remote.osc_error.clear();
                    }
                    Err(e) => rt.remote.osc_error = e.to_string(),
                }
            }
        }
        // The music sent to another instance.
        rt.remote.send_error.clear();
        let sender = if cfg.send {
            match od_controller::osc::parse_target(&cfg.send_to).and_then(|t| OscSender::new(t).map_err(|e| e.to_string())) {
                Ok(s) => Some(s),
                Err(e) => {
                    rt.remote.send_error = od_i18n::trf("CANNOT SEND · {}", &[&e]);
                    None
                }
            }
        } else {
            None
        };
        rt.runner.send_to(sender);
        // The music source; the audio input only while it is the source.
        rt.runner.source(rt.config.music);
        if rt.config.music == MusicSource::Input {
            if rt.remote.input.is_none() {
                open_input(rt);
            }
        } else if rt.remote.input.is_some() || !rt.remote.input_error.is_empty() {
            rt.remote.input = None;
            rt.remote.input_error.clear();
            rt.runner.listen(None);
        }
        rt.runner.unattended(rt.config.unattended);
    }
    let port = app.lighting.borrow().config.remote.controller.clone();
    crate::controller::assign_lighting(app, &port);
}

/// Opens the default audio input for the show (mono or stereo).
fn open_input(rt: &mut LightingRuntime) {
    if cfg!(test) || od_core::env::var("NULL_AUDIO").is_some() {
        rt.remote.input_error = od_i18n::tr("NO AUDIO INPUT (AUDIO IS OFF)").to_string();
        return;
    }
    let cfg = od_audio_io::InputConfig {
        channels: Some(2),
        ring_seconds: Some(1),
        ..Default::default()
    };
    let mono = od_audio_io::InputConfig {
        channels: Some(1),
        ..cfg.clone()
    };
    match od_audio_io::AudioInput::start(&cfg).or_else(|_| od_audio_io::AudioInput::start(&mono)) {
        Ok((input, ring)) => {
            rt.runner.listen(Some((ring, input.channels, input.sample_rate)));
            rt.remote.input = Some(input);
            rt.remote.input_error.clear();
        }
        Err(e) => {
            rt.remote.input_error = od_i18n::trf("NO AUDIO INPUT · {}", &[&e.to_string()]);
        }
    }
}

/// Per frame: what arrived over OSC.
pub(crate) fn poll(app: &App) {
    let events: Vec<OscEvent> = app
        .lighting
        .borrow()
        .remote
        .osc
        .as_ref()
        .map(|o| o.events().try_iter().collect())
        .unwrap_or_default();
    for e in events {
        match e {
            OscEvent::Light(a) => crate::lighting::controller(app, a),
            OscEvent::Music(m) => app
                .lighting
                .borrow()
                .runner
                .external(crate::lighting::network_music(&m)),
        }
    }
}

/// An edit of the remote setup (behind the setup PIN), applied at once.
fn setup_edit(app: &App, f: impl FnOnce(&mut LightingRuntime)) {
    if app.lighting.borrow().locked() {
        return;
    }
    edit(app, f);
    apply(app);
    refresh(app);
}

/// Lists the MIDI inputs (unless fixed by a test or the documentation).
pub(crate) fn refresh_midi(app: &App) {
    let list = {
        if app.lighting.borrow().remote.midi_fixed {
            return;
        }
        crate::controller::midi_inputs(app)
    };
    app.lighting.borrow_mut().remote.midi_inputs = list;
}

/// [37] On start: a show set to run unattended with autostart opens
/// LIGHTING in SHOW and sends DMX.
pub(crate) fn autostart(app: &App) {
    let start = {
        let rt = app.lighting.borrow();
        rt.config.unattended.enabled && rt.config.unattended.autostart
    };
    if !start {
        return;
    }
    {
        let mut rt = app.lighting.borrow_mut();
        rt.config.enabled = true;
        rt.reapply_outputs();
    }
    app.ui.set_mode("lighting".into());
    let g = slint::ComponentHandle::global::<crate::Lighting>(&app.ui);
    g.set_space("show".into());
    g.set_centre("stage".into());
    app.status(od_i18n::tr("Lighting: the show runs unattended"));
    refresh(app);
}

pub(crate) fn wire(app: &Rc<App>, g: &crate::Lighting<'_>) {
    g.set_triggers(app.lighting.borrow().remote.trigger_rows.handle());
    g.on_refresh_midi(handler!(app, |a| {
        refresh_midi(&a);
        refresh(&a);
    }));
    g.on_use_controller(handler!(app, |a, i: i32| {
        let port = if i < 0 {
            String::new()
        } else {
            match a.lighting.borrow().remote.midi_inputs.get(i as usize) {
                Some(p) => p.clone(),
                None => return,
            }
        };
        setup_edit(&a, |rt| rt.config.remote.controller = port);
    }));
    g.on_set_osc_in(handler!(app, |a, on: bool| setup_edit(&a, |rt| rt.config.remote.osc_in = on)));
    g.on_set_osc_port(handler!(app, |a, text: SharedString| {
        match text.trim().parse::<u16>() {
            Ok(p) if p >= 1024 => setup_edit(&a, |rt| rt.config.remote.osc_port = p),
            _ => {
                a.status(od_i18n::tr("Lighting: the OSC port is 1024 to 65535"));
                refresh(&a);
            }
        }
    }));
    g.on_set_osc_send(handler!(app, |a, on: bool, to: SharedString| {
        setup_edit(&a, |rt| {
            rt.config.remote.send = on;
            if !to.trim().is_empty() || !on {
                rt.config.remote.send_to = to.trim().to_string();
            }
        })
    }));
    g.on_set_source(handler!(app, |a, s: SharedString| {
        if show_locked(&a.lighting.borrow()) {
            return;
        }
        let src = match s.as_str() {
            "input" => MusicSource::Input,
            "network" => MusicSource::Network,
            _ => MusicSource::Decks,
        };
        edit(&a, |rt| rt.config.music = src);
        apply(&a);
        refresh(&a);
    }));
    g.on_trigger_plays(handler!(app, |a, row: i32, d: i32| {
        let Some(event) = DjEvent::ALL.get(row as usize).copied() else { return };
        if show_locked(&a.lighting.borrow()) {
            return;
        }
        edit(&a, |rt| {
            let s = &mut rt.config.settings;
            let options = trigger_targets(s);
            let now = s.triggers.iter().find(|t| t.event == event).map(|t| t.target);
            let i = options.iter().position(|o| *o == now).unwrap_or(0) as i32;
            let next = options[(i + d).rem_euclid(options.len() as i32) as usize];
            let beats = s.triggers.iter().find(|t| t.event == event).map_or(0.0, |t| t.beats);
            s.triggers.retain(|t| t.event != event);
            if let Some(target) = next {
                s.triggers.push(Trigger { event, target, beats });
                s.triggers.sort_by_key(|t| DjEvent::ALL.iter().position(|e| *e == t.event));
            }
        });
    }));
    g.on_trigger_beats(handler!(app, |a, row: i32, d: i32| {
        let Some(event) = DjEvent::ALL.get(row as usize).copied() else { return };
        if show_locked(&a.lighting.borrow()) {
            return;
        }
        edit(&a, |rt| {
            if let Some(t) = rt.config.settings.triggers.iter_mut().find(|t| t.event == event) {
                let i = TRIGGER_BEATS.iter().position(|b| *b == t.beats).unwrap_or(0) as i32;
                t.beats = TRIGGER_BEATS[(i + d).clamp(0, TRIGGER_BEATS.len() as i32 - 1) as usize];
            }
        });
    }));
    g.on_set_unattended(handler!(app, |a, on: bool| {
        {
            let rt = a.lighting.borrow();
            if rt.locked() || (on && rt.config.safety.pin.is_empty()) {
                return;
            }
        }
        setup_edit(&a, |rt| {
            rt.config.unattended.enabled = on;
            if on {
                // Fog and lasers stay locked out while unattended.
                rt.arming = od_lighting::Arming::default();
                rt.momentary = Default::default();
                rt.runner.momentary(rt.momentary);
            }
        });
        a.status(if on {
            od_i18n::tr("Lighting: the show runs unattended. LOCK closes it.")
        } else {
            od_i18n::tr("Lighting: unattended mode off")
        });
    }));
    g.on_set_autostart(handler!(app, |a, on: bool| setup_edit(&a, |rt| rt.config.unattended.autostart = on)));
    g.on_step_calm(handler!(app, |a, d: i32| setup_edit(&a, |rt| {
        let u = &mut rt.config.unattended;
        let i = CALM_AFTER
            .iter()
            .position(|s| *s >= u.calm_after)
            .unwrap_or(1) as i32;
        u.calm_after = CALM_AFTER[(i + d).clamp(0, CALM_AFTER.len() as i32 - 1) as usize];
    })));
}

/// Keeps unattended mode honest: it needs a PIN (cleared PIN, mode off).
pub(crate) fn check_pin(rt: &mut LightingRuntime) {
    if rt.config.safety.pin.is_empty() && rt.config.unattended.enabled {
        rt.config.unattended = Unattended {
            enabled: false,
            ..rt.config.unattended
        };
    }
}
