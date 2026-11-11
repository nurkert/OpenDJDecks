//! [67] Output routing (master, headphones and each deck to chosen
//! outputs, the headphones on their own device), the external-mixer mode,
//! split cue on one stereo output, and MIDI clock out from the master deck.
//!
//! Preferences › Audio holds the settings ([`crate::settings::AudioSettings`]);
//! this module turns them into the device routing, the engine commands and
//! the lines the views show, and runs the clock.

use crate::app::App;
use crate::settings::{AudioSettings, Settings};
use od_audio_io::{OutputRouting, Route};
use od_engine::{Command, EngineState, MixerCommand};
use slint::ComponentHandle;
use std::cell::RefCell;

/// The pairs offered, as stored ("12" = outputs 1/2).
pub(crate) const PAIRS: [&str; 4] = ["12", "34", "56", "78"];

/// Device channels of a stored pair.
pub(crate) fn pair_of(code: &str) -> Option<[u16; 2]> {
    PAIRS
        .iter()
        .position(|p| *p == code)
        .map(|i| OutputRouting::pair(i as u16))
}

/// "1/2" for a pair of channels.
pub(crate) fn pair_text(p: [u16; 2]) -> String {
    format!("{}/{}", p[0] + 1, p[1] + 1)
}

/// The settings as they come out of the box (the device decides).
fn is_default(a: &AudioSettings) -> bool {
    a.master == "12" && !a.split_one && a.mixer != "external" && a.cue_device.is_empty()
        && matches!(a.headphones.as_str(), "auto" | "34" | "off")
}

pub(crate) fn external(a: &AudioSettings) -> bool {
    a.mixer == "external"
}

/// The routing to ask the device for; `None` = automatic (master on 1/2,
/// headphones on 3/4 where there are four outputs).
pub(crate) fn routing(a: &AudioSettings) -> Option<OutputRouting> {
    if is_default(a) && a.headphones == "auto" {
        return None;
    }
    let mut r = if a.split_one {
        OutputRouting::split_on_one()
    } else {
        OutputRouting {
            master: pair_of(&a.master),
            headphones: match a.headphones.as_str() {
                "off" => None,
                "auto" => Some([2, 3]),
                p => pair_of(p),
            },
            decks: [None; od_engine::MAX_DECKS],
        }
    };
    if cue_device(a).is_some() {
        r.headphones = None;
    }
    if external(a) {
        for (d, p) in a.deck_outputs.iter().enumerate().take(od_engine::MAX_DECKS) {
            r.decks[d] = pair_of(p);
        }
    }
    Some(r)
}

/// The headphones' own device and its pair (0 = 1/2), if set.
pub(crate) fn cue_device(a: &AudioSettings) -> Option<(String, u16)> {
    if a.cue_device.is_empty() || a.headphones == "off" || a.split_one {
        return None;
    }
    let pair = PAIRS.iter().position(|p| *p == a.headphones).unwrap_or(0) as u16;
    Some((a.cue_device.clone(), pair))
}

/// Engine commands of the routing settings: the external mixer always,
/// split cue when one stereo output carries both.
pub(crate) fn engine_commands(s: &Settings) -> Vec<Command> {
    let mut out = vec![Command::Mixer(MixerCommand::SetExternalMixer(external(&s.audio)))];
    if s.audio.split_one {
        out.push(Command::Mixer(MixerCommand::SetSplitCue(true)));
    }
    out
}

fn route_name(r: Route) -> String {
    match r {
        Route::Master => od_i18n::tr("master").to_string(),
        Route::Headphones => od_i18n::tr("headphones").to_string(),
        Route::Deck(d) => od_i18n::trf("deck {}", &[&(d + 1)]),
    }
}

/// What runs where: "master 1/2 · headphones 3/4 · deck 1 5/6", what the
/// device could not take, and the headphone device.
pub(crate) fn summary(n: &od_audio_io::Negotiated) -> String {
    let r = &n.routing;
    let mut parts: Vec<String> = Vec::new();
    match (r.master, r.headphones) {
        (None, Some(h)) if r.decks.iter().all(Option::is_none) && h == [0, 1] => {
            parts.push(od_i18n::tr("split cue on 1/2: headphones left, master right").to_string());
        }
        _ => {
            if let Some(m) = r.master {
                parts.push(od_i18n::trf("master {}", &[&pair_text(m)]));
            }
            if let Some(h) = r.headphones {
                parts.push(od_i18n::trf("headphones {}", &[&pair_text(h)]));
            }
        }
    }
    for (d, p) in r.decks.iter().enumerate() {
        if let Some(p) = p {
            parts.push(od_i18n::trf("deck {} {}", &[&(d + 1), &pair_text(*p)]));
        }
    }
    if let Some(c) = &n.cue_device {
        parts.push(od_i18n::trf("headphones on {}", &[c]));
    }
    let mut line = parts.join(" · ");
    if !n.dropped.is_empty() {
        let names: Vec<String> = n.dropped.iter().map(|r| route_name(*r)).collect();
        line.push_str(" · ");
        line.push_str(&od_i18n::trf(
            "left out (the device has {} outputs): {}",
            &[&n.channels, &names.join(", ")],
        ));
    }
    if let Some(e) = &n.cue_error {
        line.push_str(" · ");
        line.push_str(&od_i18n::trf("headphone device did not start: {}", &[e]));
    }
    line
}

/// The pair label of each deck with the external mixer ("OUT 1/2", "OFF"),
/// as the mixer column shows them.
pub(crate) fn deck_routes(a: &AudioSettings, decks: usize) -> Vec<String> {
    (0..decks)
        .map(|d| {
            a.deck_outputs
                .get(d)
                .and_then(|p| pair_of(p))
                .map(|p| od_i18n::trf("OUT {}", &[&pair_text(p)]))
                .unwrap_or_else(|| od_i18n::tr("OFF").to_string())
        })
        .collect()
}

/// Keeps the mixer column's external-mixer state in step with the settings.
pub(crate) fn refresh_mixer(app: &App) {
    let (ext, routes) = {
        let st = app.state.borrow();
        (
            external(&st.settings.audio),
            deck_routes(&st.settings.audio, od_engine::MAX_DECKS),
        )
    };
    let g = app.ui.global::<crate::MixerParts>();
    if g.get_external() != ext {
        g.set_external(ext);
    }
    let model: Vec<slint::SharedString> = routes.into_iter().map(Into::into).collect();
    g.set_deck_routes(crate::models::model(model));
}

// ----- MIDI clock -------------------------------------------------------

thread_local! {
    /// The running clock and the port it was opened for.
    static CLOCK: RefCell<Option<od_controller::clock::MidiClockOut>> = const { RefCell::new(None) };
    /// Why the clock's port did not open.
    static CLOCK_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

/// MIDI output ports to offer.
pub(crate) fn midi_outputs() -> Vec<String> {
    od_controller::io::list_ports()
        .map(|p| p.outputs)
        .unwrap_or_default()
}

/// Opens or closes the clock to match Preferences.
pub(crate) fn apply_clock(app: &App) {
    let want = app.state.borrow().settings.audio.clock_out.clone();
    let running = CLOCK.with(|c| c.borrow().as_ref().map(|c| c.port().to_string()));
    if running.as_deref() == Some(want.as_str()) || (want.is_empty() && running.is_none()) {
        return;
    }
    CLOCK.with(|c| *c.borrow_mut() = None);
    CLOCK_ERROR.with(|e| e.borrow_mut().clear());
    if want.is_empty() {
        return;
    }
    match od_controller::clock::MidiClockOut::open(&want) {
        Ok(clock) => CLOCK.with(|c| *c.borrow_mut() = Some(clock)),
        Err(e) => {
            tracing::warn!(port = %want, error = %e, "MIDI clock port did not open");
            CLOCK_ERROR.with(|x| *x.borrow_mut() = e.clone());
            app.status(od_i18n::trf("MIDI clock: {} did not open ({})", &[&want, &e]));
        }
    }
}

/// Test hook: the clock into `sink` instead of a MIDI port.
#[cfg(test)]
pub(crate) fn start_clock_into(port: &str, sink: Box<dyn od_controller::clock::ClockSink>) {
    CLOCK.with(|c| {
        *c.borrow_mut() = Some(od_controller::clock::MidiClockOut::start(port, sink));
    });
}

/// Test hook: the clock off.
#[cfg(test)]
pub(crate) fn stop_clock() {
    CLOCK.with(|c| *c.borrow_mut() = None);
}

/// The clock's state for Preferences: off, the port and pulses, or why not.
pub(crate) fn clock_line() -> String {
    let err = CLOCK_ERROR.with(|e| e.borrow().clone());
    CLOCK.with(|c| match c.borrow().as_ref() {
        Some(c) => od_i18n::trf("MIDI clock on {}: {} pulses sent", &[&c.port(), &c.pulses()]),
        None if !err.is_empty() => od_i18n::trf("MIDI clock did not open: {}", &[&err]),
        None => od_i18n::tr("MIDI clock off").to_string(),
    })
}

/// Frame tick: tells the clock where the master deck stands.
pub(crate) fn frame(state: &EngineState) {
    CLOCK.with(|c| {
        let c = c.borrow();
        let Some(clock) = c.as_ref() else { return };
        let src = match state.master_deck {
            Some(m) => {
                let d = &state.decks[m];
                od_controller::clock::ClockSource {
                    playing: d.playing && d.track.is_some(),
                    bpm: d.bpm,
                    beat: d.beat,
                }
            }
            None => od_controller::clock::ClockSource {
                playing: false,
                bpm: state.master_bpm,
                beat: 0.0,
            },
        };
        clock.update(src);
    });
}

/// Documentation screenshots for roadmap 0.9.0 block H
/// (`OPENDJDECKS_DEMO_PROOF`, comma separated), on the demo library with
/// the decks loaded by `OPENDJDECKS_AUTOLOAD`. The routing settings they
/// set are kept in memory only.
///
/// - `routing-stem-cue`: deck 1 (its own stem files next to the track)
///   with CUE on the stem strip and VOCAL and BASS on the headphones.
/// - `routing-external`: the external mixer, deck 1 on 3/4 and deck 2 on
///   7/8, the mixer column's plate.
/// - `routing-prefs`: the same with Preferences › Audio open; the device
///   and port lists show one anonymous interface instead of the machine's
///   own devices (the picture never names real hardware).
pub(crate) fn demo(app: &std::rc::Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else { return };
    let parts: Vec<String> = v.split(',').map(str::to_string).collect();
    if !parts.iter().any(|p| p.starts_with("routing-")) {
        return;
    }
    let weak = std::rc::Rc::downgrade(app);
    slint::Timer::single_shot(std::time::Duration::from_secs(8), move || {
        let Some(a) = weak.upgrade() else { return };
        a.ui.invoke_set_mode("perform".into());
        for p in &parts {
            match p.as_str() {
                "routing-stem-cue" => {
                    crate::stem_mix::toggle_cue_mode(&a, 0);
                    crate::engine::deck_cmd(&a, 0, od_engine::DeckCommand::SetStemAudition(0b101));
                }
                "routing-external" | "routing-prefs" => {
                    {
                        let mut st = a.state.borrow_mut();
                        st.settings.audio.mixer = "external".into();
                        st.settings.audio.deck_outputs = vec!["34".into(), "78".into(), "off".into(), "off".into()];
                        st.settings.audio.master = "12".into();
                        st.settings.audio.headphones = "56".into();
                    }
                    for c in [Command::Mixer(MixerCommand::SetExternalMixer(true))] {
                        crate::engine::send(&a, c);
                    }
                    refresh_mixer(&a);
                    if p == "routing-prefs" {
                        crate::preferences::open(&a);
                        if let Some(w) = a.prefs.borrow().window.as_ref() {
                            w.set_page("audio".into());
                            w.window().set_size(slint::LogicalSize::new(900.0, 1000.0));
                            // The machine's own devices and MIDI ports stay
                            // out of the picture: one anonymous interface.
                            let names = |v: &[&str]| -> slint::ModelRc<slint::SharedString> {
                                crate::models::model(v.iter().map(|s| (*s).into()).collect())
                            };
                            let any = od_i18n::tr("Eight-output interface").to_string();
                            w.set_audio_devices(names(&[od_i18n::tr("Automatic"), &any]));
                            w.set_audio_device(1);
                            w.set_cue_devices(names(&[od_i18n::tr("The output device"), &any]));
                            w.set_cue_device(0);
                            w.set_clock_ports(names(&[od_i18n::tr("Off")]));
                            w.set_clock_port(0);
                            w.set_audio_status(format!("{any} · 48000 Hz · master 1/2 · headphones 5/6 · deck 1 3/4 · deck 2 7/8").into());
                        }
                    }
                }
                _ => {}
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_to_routing() {
        let mut a = AudioSettings::default();
        assert_eq!(routing(&a), None, "out of the box the device decides");
        a.headphones = "56".into();
        let r = routing(&a).unwrap();
        assert_eq!((r.master, r.headphones), (Some([0, 1]), Some([4, 5])));
        a.split_one = true;
        let r = routing(&a).unwrap();
        assert_eq!((r.master, r.headphones), (None, Some([0, 1])));
        assert_eq!(cue_device(&a), None, "split keeps the headphones on 1/2");
        a.split_one = false;
        a.mixer = "external".into();
        a.master = "off".into();
        a.headphones = "off".into();
        a.deck_outputs = vec!["12".into(), "34".into(), "off".into(), "78".into()];
        let r = routing(&a).unwrap();
        assert_eq!(r.master, None);
        assert_eq!(r.decks, [Some([0, 1]), Some([2, 3]), None, Some([6, 7])]);
        a.mixer = "internal".into();
        assert_eq!(routing(&a).unwrap().decks, [None; 4], "deck pairs only with the external mixer");
        a.headphones = "34".into();
        a.cue_device = "Phones".into();
        assert_eq!(cue_device(&a), Some(("Phones".into(), 1)));
        assert_eq!(routing(&a).unwrap().headphones, None);
    }

    #[test]
    fn engine_commands_follow_the_settings() {
        let mut s = Settings::default();
        assert!(matches!(
            engine_commands(&s)[..],
            [Command::Mixer(MixerCommand::SetExternalMixer(false))]
        ));
        s.audio.mixer = "external".into();
        s.audio.split_one = true;
        assert!(matches!(
            engine_commands(&s)[..],
            [
                Command::Mixer(MixerCommand::SetExternalMixer(true)),
                Command::Mixer(MixerCommand::SetSplitCue(true))
            ]
        ));
    }

    #[test]
    fn the_summary_names_every_route() {
        let n = od_audio_io::Negotiated {
            backend: od_audio_io::Backend::Default,
            device_name: "Interface".into(),
            sample_rate: 48_000,
            channels: 4,
            buffer_frames: None,
            sample_format: "F32".into(),
            routing: OutputRouting {
                master: None,
                headphones: None,
                decks: [Some([0, 1]), Some([2, 3]), None, None],
            },
            dropped: vec![Route::Deck(2)],
            fell_back: false,
            cue_device: Some("Phones".into()),
            cue_error: None,
        };
        assert_eq!(
            summary(&n),
            "deck 1 1/2 · deck 2 3/4 · headphones on Phones · left out (the device has 4 outputs): deck 3"
        );
        let mut s = n.clone();
        s.routing = OutputRouting::split_on_one();
        s.dropped.clear();
        s.cue_device = None;
        assert_eq!(summary(&s), "split cue on 1/2: headphones left, master right");
    }
}
