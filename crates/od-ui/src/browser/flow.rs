//! Set Flow as a chart ([47]): energy, tempo and key per position, and a
//! warning on each transition that needs attention.
//!
//! A transition warns about a tempo jump beyond [`TEMPO_WARN`] percent
//! (half and double time count as the same tempo), a key clash (the key
//! relation of [62], [`KeyRelation::Clash`]) and an energy drop or jump of
//! at least [`ENERGY_STEP`] of the set's range that is also a quarter of
//! the louder track. A tempo step above [`TEMPO_CAUTION`] or a key that
//! changes the energy is a caution: dashed, without a warning.

use crate::{FlowPoint, FlowWarning, format};
use od_core::{KeyNotation, KeyRelation, MusicalKey};
use od_library::sets::SetFlow;

/// Above this tempo change (percent) a transition warns.
pub(crate) const TEMPO_WARN: f64 = 6.0;
/// Above this it is a caution.
pub(crate) const TEMPO_CAUTION: f64 = 3.0;
/// Energy step (in the set's range, 0..1) that warns.
pub(crate) const ENERGY_STEP: f32 = 0.4;

/// Why a transition needs attention.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Reason {
    Tempo(f64),
    KeyClash(MusicalKey, MusicalKey),
    EnergyDrop,
    EnergyJump,
}

/// How one transition fits.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Fit {
    pub warnings: Vec<Reason>,
    /// A step worth rehearsing (dashed).
    pub caution: bool,
    /// Tempo or key unknown on one side.
    pub unknown: bool,
    pub tempo_pct: Option<f64>,
    pub key: Option<(MusicalKey, MusicalKey, KeyRelation)>,
}

impl Fit {
    /// The line form id of the chart: "warning", "caution", "unknown", "good".
    pub fn id(&self) -> &'static str {
        if !self.warnings.is_empty() {
            "warning"
        } else if self.caution {
            "caution"
        } else if self.unknown {
            "unknown"
        } else {
            "good"
        }
    }
}

/// The fit of a transition from `a` to `b`. Energies are raw (`raw`) and
/// scaled to the set's range (`scaled`, 0..1).
pub(crate) fn fit(
    bpm: (Option<f64>, Option<f64>),
    key: (Option<MusicalKey>, Option<MusicalKey>),
    raw: (Option<f32>, Option<f32>),
    scaled: (f32, f32),
) -> Fit {
    let mut f = Fit::default();
    match bpm {
        (Some(a), Some(b)) if a > 0.0 && b > 0.0 => {
            let p = od_library::sets::bpm_change_pct(a, b);
            f.tempo_pct = Some(p);
            if p.abs() > TEMPO_WARN {
                f.warnings.push(Reason::Tempo(p));
            } else if p.abs() > TEMPO_CAUTION {
                f.caution = true;
            }
        }
        _ => f.unknown = true,
    }
    match key {
        (Some(a), Some(b)) => {
            let r = a.relation(b);
            f.key = Some((a, b, r));
            match r {
                KeyRelation::Clash => f.warnings.push(Reason::KeyClash(a, b)),
                KeyRelation::Energy => f.caution = true,
                KeyRelation::Fits => {}
            }
        }
        _ => f.unknown = true,
    }
    if let (Some(ra), Some(rb)) = raw {
        let step = scaled.1 - scaled.0;
        let louder = ra.max(rb);
        let relative = if louder > 0.0 { (rb - ra).abs() / louder } else { 0.0 };
        if step.abs() >= ENERGY_STEP && relative >= 0.25 {
            f.warnings.push(if step < 0.0 { Reason::EnergyDrop } else { Reason::EnergyJump });
        }
    }
    f
}

/// One reason in caps, as the chart and its warnings write it.
pub(crate) fn reason_text(r: &Reason, notation: KeyNotation) -> String {
    match r {
        Reason::Tempo(p) => od_i18n::trf("TEMPO {} %", &[&format!("{p:+.1}")]),
        Reason::KeyClash(a, b) => od_i18n::trf(
            "KEY CLASH {} → {}",
            &[&a.format(notation), &b.format(notation)],
        ),
        Reason::EnergyDrop => od_i18n::tr("ENERGY DROP").to_string(),
        Reason::EnergyJump => od_i18n::tr("ENERGY JUMP").to_string(),
    }
}

/// The transition in a sentence (the marker's tooltip).
fn detail(f: &Fit, notation: KeyNotation) -> String {
    let mut parts = Vec::new();
    if let Some(p) = f.tempo_pct {
        parts.push(od_i18n::trf("tempo {} %", &[&format!("{p:+.1}")]));
    }
    if let Some((a, b, r)) = f.key {
        let (a, b) = (a.format(notation), b.format(notation));
        parts.push(match r {
            KeyRelation::Fits => od_i18n::trf("key {} → {} fits", &[&a, &b]),
            KeyRelation::Energy => od_i18n::trf("key {} → {} changes the energy", &[&a, &b]),
            KeyRelation::Clash => od_i18n::trf("key {} → {} clashes", &[&a, &b]),
        });
    }
    if f.warnings.contains(&Reason::EnergyDrop) {
        parts.push(od_i18n::tr("energy drops").to_string());
    }
    if f.warnings.contains(&Reason::EnergyJump) {
        parts.push(od_i18n::tr("energy jumps").to_string());
    }
    let head = match f.id() {
        "warning" => od_i18n::tr("Warning"),
        "caution" => od_i18n::tr("Worth rehearsing"),
        "unknown" => od_i18n::tr("Not analysed"),
        _ => od_i18n::tr("Fits"),
    };
    if parts.is_empty() {
        head.to_string()
    } else {
        format!("{head}: {}", parts.join(", "))
    }
}

/// Rescales energies to 0.15..1 across the set (unchanged when flat).
pub(crate) fn normalize_energy(e: &mut [f32]) {
    let (lo, hi) = e
        .iter()
        .fold((f32::MAX, f32::MIN), |(a, b), x| (a.min(*x), b.max(*x)));
    if hi > lo {
        for x in e.iter_mut() {
            *x = 0.15 + 0.85 * (*x - lo) / (hi - lo);
        }
    }
}

/// The chart: positions, the BPM axis (low, high) and the warnings.
pub(crate) struct Chart {
    pub points: Vec<FlowPoint>,
    pub bpm_low: String,
    pub bpm_high: String,
    pub warnings: Vec<FlowWarning>,
}

pub(crate) fn chart(flow: &SetFlow, notation: KeyNotation) -> Chart {
    let raw: Vec<Option<f32>> = flow.points.iter().map(|p| p.energy).collect();
    let mut scaled: Vec<f32> = raw.iter().map(|e| e.unwrap_or(0.3)).collect();
    normalize_energy(&mut scaled);
    let bpms: Vec<f64> = flow.points.iter().filter_map(|p| p.bpm).filter(|b| *b > 0.0).collect();
    let (lo, hi) = bpms
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(*x), b.max(*x)));
    // A little air around the line; a flat set sits in the middle.
    let (lo, hi) = if bpms.is_empty() {
        (0.0, 1.0)
    } else if hi - lo < 1.0 {
        (lo - 2.0, hi + 2.0)
    } else {
        (lo.floor(), hi.ceil())
    };
    let n = flow.points.len();
    let mut warnings = Vec::new();
    let points = flow
        .points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let next = flow.points.get(i + 1);
            let f = next.map(|q| {
                fit(
                    (p.bpm, q.bpm),
                    (p.key, q.key),
                    (raw[i], raw[i + 1]),
                    (scaled[i], scaled[i + 1]),
                )
            });
            let reason = f
                .as_ref()
                .map(|f| {
                    f.warnings
                        .iter()
                        .map(|r| reason_text(r, notation))
                        .collect::<Vec<_>>()
                        .join("  ·  ")
                })
                .unwrap_or_default();
            if !reason.is_empty() {
                warnings.push(FlowWarning {
                    index: i as i32,
                    text: format!("{} → {}  {reason}", i + 1, i + 2).into(),
                });
            }
            FlowPoint {
                title: p.track.tags.title.clone().into(),
                pos: (i + 1).to_string().into(),
                bpm: p.bpm.map(|b| format!("{b:.0}")).unwrap_or_default().into(),
                bpm_level: p
                    .bpm
                    .filter(|b| *b > 0.0)
                    .map(|b| ((b - lo) / (hi - lo)).clamp(0.0, 1.0) as f32)
                    .unwrap_or(-1.0),
                key: p.key.map(|k| k.format(notation)).unwrap_or_default().into(),
                energy: scaled[i],
                start: format::time(p.start_offset).into(),
                fit: f.as_ref().map(Fit::id).unwrap_or("none").into(),
                reason: reason.into(),
                detail: f.as_ref().map(|f| detail(f, notation)).unwrap_or_default().into(),
            }
        })
        .collect();
    debug_assert!(n == 0 || warnings.len() < n);
    Chart {
        points,
        bpm_low: if bpms.is_empty() { String::new() } else { format!("{lo:.0}") },
        bpm_high: if bpms.is_empty() { String::new() } else { format!("{hi:.0}") },
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(s: &str) -> Option<MusicalKey> {
        MusicalKey::parse(s)
    }

    #[test]
    fn tempo_jumps_beyond_the_threshold_warn() {
        let f = fit((Some(120.0), Some(122.0)), (k("8A"), k("8A")), (None, None), (0.5, 0.5));
        assert_eq!(f.id(), "good");
        let f = fit((Some(120.0), Some(125.0)), (k("8A"), k("8A")), (None, None), (0.5, 0.5));
        assert_eq!(f.id(), "caution", "4 % is worth rehearsing");
        let f = fit((Some(120.0), Some(130.0)), (k("8A"), k("8A")), (None, None), (0.5, 0.5));
        assert!(matches!(f.warnings[..], [Reason::Tempo(p)] if (p - 8.333).abs() < 0.01));
        let f = fit((Some(87.0), Some(174.0)), (k("8A"), k("8A")), (None, None), (0.5, 0.5));
        assert_eq!(f.id(), "good", "double time is the same tempo");
    }

    #[test]
    fn key_clashes_follow_the_key_relation() {
        let f = fit((Some(124.0), Some(124.0)), (k("8A"), k("9A")), (None, None), (0.5, 0.5));
        assert_eq!(f.id(), "good", "neighbours fit");
        let f = fit((Some(124.0), Some(124.0)), (k("8A"), k("10A")), (None, None), (0.5, 0.5));
        assert_eq!(f.id(), "caution", "two steps change the energy");
        let f = fit((Some(124.0), Some(124.0)), (k("8A"), k("3B")), (None, None), (0.5, 0.5));
        assert_eq!(f.warnings, [Reason::KeyClash(k("8A").unwrap(), k("3B").unwrap())]);
        assert_eq!(
            reason_text(&f.warnings[0], KeyNotation::Camelot),
            "KEY CLASH 8A → 3B"
        );
        let f = fit((Some(124.0), None), (k("8A"), None), (None, None), (0.5, 0.5));
        assert_eq!(f.id(), "unknown");
    }

    #[test]
    fn energy_drops_and_jumps_warn() {
        let base = ((Some(124.0), Some(124.0)), (k("8A"), k("8A")));
        let f = fit(base.0, base.1, (Some(0.6), Some(0.3)), (1.0, 0.2));
        assert_eq!(f.warnings, [Reason::EnergyDrop]);
        let f = fit(base.0, base.1, (Some(0.3), Some(0.6)), (0.2, 1.0));
        assert_eq!(f.warnings, [Reason::EnergyJump]);
        let f = fit(base.0, base.1, (Some(0.50), Some(0.52)), (0.15, 1.0));
        assert!(f.warnings.is_empty(), "a flat set does not warn on small steps");
        let f = fit(base.0, base.1, (Some(0.6), Some(0.45)), (1.0, 0.8));
        assert!(f.warnings.is_empty(), "a small step in the set's range");
    }

    #[test]
    fn several_reasons_are_named_together() {
        let f = fit((Some(120.0), Some(132.0)), (k("8A"), k("2B")), (Some(0.6), Some(0.2)), (1.0, 0.15));
        assert_eq!(f.warnings.len(), 3);
        assert_eq!(f.id(), "warning");
        let d = detail(&f, KeyNotation::Camelot);
        assert!(d.contains("+10.0") && d.contains("8A → 2B") && d.contains("energy drops"), "{d}");
    }

    #[test]
    fn energy_rescales_across_the_set() {
        let mut e = [0.2f32, 0.4, 0.6];
        normalize_energy(&mut e);
        assert!((e[0] - 0.15).abs() < 1e-6 && (e[2] - 1.0).abs() < 1e-6);
        let mut flat = [0.5f32, 0.5];
        normalize_energy(&mut flat);
        assert_eq!(flat, [0.5, 0.5]);
    }
}
