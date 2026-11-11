//! The check before the gig (roadmap [10]): audio outputs, controller, disk
//! space for the recording and missing files, each with a plain result.
//! Opened from the gig menu in the top bar; a band, never a dialog.

use crate::app::{App, handler};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::path::Path;
use std::rc::Rc;

/// Below this the recording may not last the night.
const DISK_WARN: u64 = 4 * 1024 * 1024 * 1024;
const DISK_FAIL: u64 = 1024 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    Ok,
    Warn,
    Fail,
}

impl Verdict {
    fn id(self) -> &'static str {
        match self {
            Verdict::Ok => "ok",
            Verdict::Warn => "warn",
            Verdict::Fail => "fail",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Row {
    pub id: &'static str,
    pub verdict: Verdict,
    pub detail: String,
}

/// What the audio output is doing.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum AudioFact {
    Running {
        device: String,
        latency: Option<String>,
        headphones: bool,
        dropouts: u64,
    },
    Lost(String),
    NotRunning(String),
}

/// Everything the check looks at, gathered first (so the verdicts are pure).
#[derive(Clone, Debug)]
pub(crate) struct Facts {
    pub audio: AudioFact,
    pub controller: Option<String>,
    pub free_bytes: Option<u64>,
    pub sample_rate: u32,
    /// Titles of tracks whose file is missing.
    pub missing: Vec<String>,
}

fn gigabytes(b: u64) -> String {
    format!("{:.1}", b as f64 / 1024.0 / 1024.0 / 1024.0)
}

/// The verdict per check.
pub(crate) fn evaluate(f: &Facts) -> Vec<Row> {
    let mut rows = Vec::new();
    rows.push(match &f.audio {
        AudioFact::Running {
            device,
            latency,
            headphones,
            dropouts,
        } => {
            let mut detail = device.clone();
            if let Some(l) = latency {
                detail.push_str(&format!(" · {l}"));
            }
            let verdict = if *dropouts > 0 {
                detail.push_str(" · ");
                detail.push_str(&od_i18n::trf("{} dropouts so far", &[dropouts]));
                Verdict::Warn
            } else if !headphones {
                detail.push_str(" · ");
                detail.push_str(&od_i18n::tr("no headphone output"));
                Verdict::Warn
            } else {
                detail.push_str(" · ");
                detail.push_str(&od_i18n::tr("master and headphones"));
                Verdict::Ok
            };
            Row {
                id: "outputs",
                verdict,
                detail,
            }
        }
        AudioFact::Lost(d) => Row {
            id: "outputs",
            verdict: Verdict::Fail,
            detail: od_i18n::trf("{} is lost", &[d]),
        },
        AudioFact::NotRunning(why) => Row {
            id: "outputs",
            verdict: Verdict::Fail,
            detail: why.clone(),
        },
    });
    rows.push(match &f.controller {
        Some(name) => Row {
            id: "controller",
            verdict: Verdict::Ok,
            detail: name.clone(),
        },
        None => Row {
            id: "controller",
            verdict: Verdict::Warn,
            detail: od_i18n::tr("No controller connected").into(),
        },
    });
    rows.push(match f.free_bytes {
        Some(free) => {
            // 24-bit stereo WAV.
            let per_hour = f.sample_rate.max(1) as u64 * 2 * 3 * 3600;
            let hours = free / per_hour;
            let detail = od_i18n::trf(
                "{} GB free, about {} hours of recording",
                &[&gigabytes(free), &hours],
            );
            Row {
                id: "disk",
                verdict: if free < DISK_FAIL {
                    Verdict::Fail
                } else if free < DISK_WARN {
                    Verdict::Warn
                } else {
                    Verdict::Ok
                },
                detail,
            }
        }
        None => Row {
            id: "disk",
            verdict: Verdict::Warn,
            detail: od_i18n::tr("Free space unknown").into(),
        },
    });
    rows.push(if f.missing.is_empty() {
        Row {
            id: "files",
            verdict: Verdict::Ok,
            detail: od_i18n::tr("Every track file is in place").into(),
        }
    } else {
        let names: Vec<&str> = f.missing.iter().take(3).map(|s| s.as_str()).collect();
        let more = f.missing.len().saturating_sub(names.len());
        let mut detail = od_i18n::trf("{} missing: {}", &[&f.missing.len(), &names.join(", ")]);
        if more > 0 {
            detail.push_str(&od_i18n::trf(" and {} more", &[&more]));
        }
        Row {
            id: "files",
            verdict: Verdict::Fail,
            detail,
        }
    });
    rows
}

/// Free bytes on the file system the recordings go to (the nearest
/// existing folder of the path).
fn free_at(path: &Path) -> Option<u64> {
    let mut p = Some(path);
    while let Some(dir) = p {
        if dir.exists() {
            return od_devices::volumes::space(dir).map(|(_, free)| free);
        }
        p = dir.parent();
    }
    None
}

fn gather(app: &App) -> Facts {
    // The check opens the output if nothing has started it yet.
    let started = crate::engine::ensure_engine(app);
    let audio = {
        let h = app.engine.borrow();
        match h.host.as_ref() {
            _ if crate::audio_health::is_lost() => {
                AudioFact::Lost(app.ui.global::<crate::AudioHealth>().get_lost_device().into())
            }
            Some(host) => AudioFact::Running {
                device: host.info().device_name.clone(),
                latency: crate::audio_health::latency(host).map(crate::audio_health::latency_text),
                headphones: host.info().routing.headphones.is_some()
                    || host.info().cue_device.is_some(),
                dropouts: 0,
            },
            None if !started => AudioFact::NotRunning(h.message.clone()),
            None => AudioFact::NotRunning(od_i18n::tr("No audio output").into()),
        }
    };
    let audio = match audio {
        AudioFact::Running {
            device,
            latency,
            headphones,
            ..
        } => AudioFact::Running {
            device,
            latency,
            headphones,
            dropouts: crate::audio_health::dropouts(app),
        },
        other => other,
    };
    let controller = app.controller.borrow().name.clone();
    let (dir, rate) = {
        let st = app.state.borrow();
        (st.settings.recordings_dir(), app.engine.borrow().sample_rate())
    };
    let missing = {
        let mut st = app.state.borrow_mut();
        let _ = st.lib.check_missing();
        st.lib
            .tracks(od_library::SortKey::Title, false)
            .unwrap_or_default()
            .into_iter()
            .filter(|t| t.missing)
            .map(|t| {
                if t.tags.title.is_empty() {
                    t.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
                } else {
                    t.tags.title.clone()
                }
            })
            .collect()
    };
    Facts {
        audio,
        controller,
        free_bytes: free_at(&dir),
        sample_rate: rate,
        missing,
    }
}

/// Runs the check and shows the band.
pub(crate) fn run(app: &App) {
    let rows = evaluate(&gather(app));
    let worst = rows.iter().map(|r| r.verdict).fold(Verdict::Ok, |a, b| match (a, b) {
        (Verdict::Fail, _) | (_, Verdict::Fail) => Verdict::Fail,
        (Verdict::Warn, _) | (_, Verdict::Warn) => Verdict::Warn,
        _ => Verdict::Ok,
    });
    let g = app.ui.global::<crate::GigCheck>();
    g.set_rows(ModelRc::new(VecModel::from(
        rows.iter()
            .map(|r| crate::GigCheckRow {
                id: r.id.into(),
                state: r.verdict.id().into(),
                detail: r.detail.clone().into(),
            })
            .collect::<Vec<_>>(),
    )));
    g.set_result(worst.id().into());
    g.set_visible(true);
    app.refresh_library();
}

pub(crate) fn wire(app: &Rc<App>) {
    let g = app.ui.global::<crate::GigCheck>();
    g.on_run(handler!(app, |a| run(&a)));
    g.on_close(handler!(app, |a| a.ui.global::<crate::GigCheck>().set_visible(false)));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> Facts {
        Facts {
            audio: AudioFact::Running {
                device: "Interface".into(),
                latency: Some("5.3 ms".into()),
                headphones: true,
                dropouts: 0,
            },
            controller: Some("Controller".into()),
            free_bytes: Some(100 * 1024 * 1024 * 1024),
            sample_rate: 48_000,
            missing: Vec::new(),
        }
    }

    fn verdicts(f: &Facts) -> Vec<(&'static str, Verdict)> {
        evaluate(f).into_iter().map(|r| (r.id, r.verdict)).collect()
    }

    #[test]
    fn a_ready_setup_passes_every_check() {
        let rows = evaluate(&facts());
        assert!(rows.iter().all(|r| r.verdict == Verdict::Ok), "{rows:?}");
        assert_eq!(
            rows.iter().map(|r| r.id).collect::<Vec<_>>(),
            ["outputs", "controller", "disk", "files"]
        );
        // 100 GB at 48 kHz, 24-bit stereo: about 103 hours.
        assert!(rows[2].detail.contains("about 103 hours"), "{}", rows[2].detail);
    }

    #[test]
    fn each_problem_is_named() {
        let mut f = facts();
        f.audio = AudioFact::Lost("Interface".into());
        f.controller = None;
        f.free_bytes = Some(512 * 1024 * 1024);
        f.missing = vec!["A".into(), "B".into(), "C".into(), "D".into()];
        assert_eq!(
            verdicts(&f),
            [
                ("outputs", Verdict::Fail),
                ("controller", Verdict::Warn),
                ("disk", Verdict::Fail),
                ("files", Verdict::Fail)
            ]
        );
        let rows = evaluate(&f);
        assert!(rows[3].detail.contains("4 missing: A, B, C and 1 more"), "{}", rows[3].detail);
        f.free_bytes = Some(2 * 1024 * 1024 * 1024);
        assert_eq!(evaluate(&f)[2].verdict, Verdict::Warn, "2 GB is tight");
    }

    #[test]
    fn dropouts_and_a_missing_headphone_output_are_warnings() {
        let mut f = facts();
        f.audio = AudioFact::Running {
            device: "Speakers".into(),
            latency: None,
            headphones: false,
            dropouts: 0,
        };
        assert_eq!(evaluate(&f)[0].verdict, Verdict::Warn);
        f.audio = AudioFact::Running {
            device: "Speakers".into(),
            latency: None,
            headphones: true,
            dropouts: 3,
        };
        let r = &evaluate(&f)[0];
        assert_eq!(r.verdict, Verdict::Warn);
        assert!(r.detail.contains("3 dropouts"), "{}", r.detail);
    }
}
