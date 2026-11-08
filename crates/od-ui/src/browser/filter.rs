//! The quick filter bar: BPM range, minimum rating, one tag value, and
//! key / tempo relative to the master deck.

use od_core::MusicalKey;
use od_library::{UserTag, TrackRecord};

/// The track the master-relative conditions compare with.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Reference {
    pub bpm: Option<f64>,
    pub key: Option<MusicalKey>,
    /// Shown in the bar, e.g. "MASTER 124.0 · 8A".
    pub label: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Filter {
    pub bpm_min: Option<f64>,
    pub bpm_max: Option<f64>,
    /// Minimum stars, 0 = any.
    pub rating_min: u8,
    pub user_tag: Option<UserTag>,
    /// Key compatible with the reference (Camelot distance ≤ 1).
    pub key_match: bool,
    /// Tempo within ± this percentage of the reference (double / half time count).
    pub bpm_pct: Option<f64>,
}

fn number(v: &str) -> Option<f64> {
    v.trim()
        .trim_start_matches('±')
        .trim_end_matches('%')
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|x| x.is_finite() && *x >= 0.0)
}

/// "Category: Value" → Tags ("" or no value = none).
pub(crate) fn parse_user_tag(v: &str) -> Option<UserTag> {
    let (c, v) = v.split_once(':')?;
    let t = UserTag::new(c, v);
    (!t.category.is_empty() && !t.value.is_empty()).then_some(t)
}

pub(crate) fn user_tag_text(t: &UserTag) -> String {
    format!("{}: {}", t.category, t.value)
}

impl Filter {
    /// Any condition set.
    pub fn active(&self) -> bool {
        self.bpm_min.is_some()
            || self.bpm_max.is_some()
            || self.rating_min > 0
            || self.user_tag.is_some()
            || self.key_match
            || self.bpm_pct.is_some()
    }

    /// Conditions that need a reference track.
    pub fn uses_reference(&self) -> bool {
        self.key_match || self.bpm_pct.is_some()
    }

    /// Applies one edit from the filter bar.
    pub fn edit(&mut self, field: &str, value: &str) {
        match field {
            "bpm-min" => self.bpm_min = number(value),
            "bpm-max" => self.bpm_max = number(value),
            "pct" => self.bpm_pct = number(value),
            "key" => self.key_match = !value.is_empty(),
            "rating" => self.rating_min = value.trim().parse::<u8>().unwrap_or(0).min(5),
            "user-tag" => self.user_tag = parse_user_tag(value),
            "clear" => *self = Filter::default(),
            _ => {}
        }
    }

    /// Whether `t` (with its tag values) passes. Master-relative
    /// conditions are skipped without a reference (or its BPM / key).
    pub fn matches(&self, t: &TrackRecord, tags: &[UserTag], reference: Option<&Reference>) -> bool {
        if let Some(lo) = self.bpm_min
            && !t.bpm.is_some_and(|b| b >= lo)
        {
            return false;
        }
        if let Some(hi) = self.bpm_max
            && !t.bpm.is_some_and(|b| b <= hi)
        {
            return false;
        }
        if t.tags.rating < self.rating_min {
            return false;
        }
        if let Some(m) = &self.user_tag
            && !tags.contains(m)
        {
            return false;
        }
        if let Some(r) = reference {
            if let (true, Some(rk)) = (self.key_match, r.key)
                && !t.key.is_some_and(|k| rk.camelot_distance(k) <= 1)
            {
                return false;
            }
            if let (Some(p), Some(rb)) = (self.bpm_pct, r.bpm)
                && !t
                    .bpm
                    .is_some_and(|b| od_library::sets::bpm_change_pct(rb, b).abs() <= p + 1e-9)
            {
                return false;
            }
        }
        true
    }

    /// The bar's view model.
    pub fn view(&self, reference: Option<&Reference>) -> crate::FilterView {
        let num = |v: Option<f64>| {
            v.map(|x| {
                if x.fract() == 0.0 {
                    format!("{x:.0}")
                } else {
                    format!("{x}")
                }
            })
            .unwrap_or_default()
        };
        crate::FilterView {
            active: self.active(),
            bpm_min: num(self.bpm_min).into(),
            bpm_max: num(self.bpm_max).into(),
            rating: self.rating_min as i32,
            user_tag: self
                .user_tag
                .as_ref()
                .map(user_tag_text)
                .unwrap_or_default()
                .into(),
            key_match: self.key_match,
            bpm_pct: num(self.bpm_pct).into(),
            reference: reference
                .map(|r| r.label.clone())
                .unwrap_or_default()
                .into(),
        }
    }
}

/// How well a track fits the reference (MATCH column): key (Camelot
/// distance) and tempo (change in percent, half / double time counted).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Match {
    /// 0..=3 filled segments; -1 without a reference.
    pub score: i32,
    pub key_distance: Option<u8>,
    pub tempo_pct: Option<f64>,
}

/// Tempo change that still counts as a good / a workable fit (percent).
pub(crate) const MATCH_TEMPO_GOOD: f64 = 3.0;
pub(crate) const MATCH_TEMPO_OK: f64 = 6.0;

/// Rates a track against `reference`. Key and tempo give 2 points when
/// they fit well (distance ≤ 1, within ±3 %), 1 when workable (two steps,
/// within ±6 %) or unknown, else 0; the sum maps to segments: 4 → 3,
/// 3 → 2, 2 → 1, less → 0.
pub(crate) fn match_with(
    reference: Option<&Reference>,
    bpm: Option<f64>,
    key: Option<MusicalKey>,
) -> Match {
    let Some(r) = reference.filter(|r| r.bpm.is_some() || r.key.is_some()) else {
        return Match {
            score: -1,
            key_distance: None,
            tempo_pct: None,
        };
    };
    let key_distance = r.key.zip(key).map(|(a, b)| a.camelot_distance(b));
    let tempo_pct = r
        .bpm
        .zip(bpm)
        .filter(|(a, b)| *a > 0.0 && *b > 0.0)
        .map(|(a, b)| od_library::sets::bpm_change_pct(a, b));
    let key_points = match key_distance {
        None => 1,
        Some(0 | 1) => 2,
        Some(2) => 1,
        Some(_) => 0,
    };
    let tempo_points = match tempo_pct.map(f64::abs) {
        None => 1,
        Some(p) if p <= MATCH_TEMPO_GOOD + 1e-9 => 2,
        Some(p) if p <= MATCH_TEMPO_OK + 1e-9 => 1,
        Some(_) => 0,
    };
    Match {
        score: (key_points + tempo_points - 1).clamp(0, 3),
        key_distance,
        tempo_pct,
    }
}

impl Match {
    /// Sort value: the score, then the closer tempo.
    pub fn rank(&self) -> f64 {
        self.score as f64 * 1000.0 - self.tempo_pct.map(|p| p.abs().min(99.0)).unwrap_or(50.0)
    }

    /// Tooltip of the MATCH cell ("vs MASTER 124.0 · 8A: adjacent key · tempo +2.1 %").
    pub fn tip(&self, reference: Option<&Reference>) -> String {
        let Some(r) = reference.filter(|_| self.score >= 0) else {
            return String::new();
        };
        let key = match self.key_distance {
            None => od_i18n::tr("key unknown").to_string(),
            Some(0) => od_i18n::tr("same key").to_string(),
            Some(1) => od_i18n::tr("adjacent key").to_string(),
            Some(2) => od_i18n::tr("key two steps away").to_string(),
            Some(_) => od_i18n::tr("key clash").to_string(),
        };
        let tempo = match self.tempo_pct {
            None => od_i18n::tr("tempo unknown").to_string(),
            Some(p) => od_i18n::trf("tempo {} %", &[&format!("{p:+.1}")]),
        };
        od_i18n::trf("vs {}: {} · {}", &[&r.label, &key, &tempo])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(bpm: Option<f64>, key: Option<&str>, rating: u8) -> TrackRecord {
        let mut t: TrackRecord = serde_json::from_value(serde_json::json!({
            "id": 1, "path": "", "content_hash": "", "file_size": 0, "format": null, "duration": 0.0,
            "sample_rate": 44100, "channels": 2, "bit_rate": null, "bits_per_sample": null,
            "tags": od_core::TrackTags::default(), "date_added": "", "play_count": 0, "last_played": null,
            "missing": false, "bpm": null, "key": null, "analysis_version": 0
        }))
        .unwrap();
        t.bpm = bpm;
        t.key = key.and_then(MusicalKey::parse);
        t.tags.rating = rating;
        t
    }

    #[test]
    fn bpm_range_rating_and_user_tag() {
        let mut f = Filter::default();
        assert!(!f.active());
        f.edit("bpm-min", "120");
        f.edit("bpm-max", "126,5");
        assert_eq!((f.bpm_min, f.bpm_max), (Some(120.0), Some(126.5)));
        assert!(f.matches(&track(Some(124.0), None, 0), &[], None));
        assert!(!f.matches(&track(Some(128.0), None, 0), &[], None));
        assert!(
            !f.matches(&track(None, None, 0), &[], None),
            "unanalysed tracks fail a BPM range"
        );
        f.edit("rating", "3");
        assert!(!f.matches(&track(Some(124.0), None, 2), &[], None));
        assert!(f.matches(&track(Some(124.0), None, 4), &[], None));
        f.edit("user-tag", "Mood: Peak");
        let peak = [UserTag::new("Mood", "Peak")];
        assert!(f.matches(&track(Some(124.0), None, 4), &peak, None));
        assert!(!f.matches(
            &track(Some(124.0), None, 4),
            &[UserTag::new("Mood", "Warm")],
            None
        ));
        assert_eq!(f.view(None).user_tag, "Mood: Peak");
        f.edit("clear", "");
        assert_eq!(f, Filter::default());
        assert_eq!(parse_user_tag("no colon"), None);
    }

    #[test]
    fn key_and_tempo_relative_to_master() {
        let mut f = Filter::default();
        f.edit("key", "1");
        f.edit("pct", "±4%");
        assert!(f.uses_reference() && f.bpm_pct == Some(4.0));
        let master = Reference {
            bpm: Some(125.0),
            key: MusicalKey::parse("8A"),
            label: "MASTER".into(),
        };
        assert!(
            f.matches(&track(Some(124.0), Some("9A"), 0), &[], Some(&master)),
            "adjacent key, 0.8 % slower"
        );
        assert!(
            f.matches(&track(Some(63.0), Some("8B"), 0), &[], Some(&master)),
            "relative key, half time"
        );
        assert!(
            !f.matches(&track(Some(124.0), Some("10A"), 0), &[], Some(&master)),
            "two steps away"
        );
        assert!(
            !f.matches(&track(Some(115.0), Some("8A"), 0), &[], Some(&master)),
            "8 % slower"
        );
        // No master deck: the relative conditions do not hide anything.
        assert!(f.matches(&track(Some(90.0), Some("1B"), 0), &[], None));
    }

    #[test]
    fn match_score_combines_key_and_tempo() {
        let master = Reference {
            bpm: Some(125.0),
            key: MusicalKey::parse("8A"),
            label: "MASTER 125.0 · 8A".into(),
        };
        let m = |bpm: Option<f64>, key: Option<&str>| {
            match_with(Some(&master), bpm, key.and_then(MusicalKey::parse))
        };
        assert_eq!(m(Some(126.0), Some("9A")).score, 3, "adjacent key, +0.8 %");
        assert_eq!(m(Some(62.5), Some("8B")).score, 3, "relative key, half time");
        assert_eq!(m(Some(130.0), Some("8A")).score, 2, "same key, +4 %");
        assert_eq!(m(Some(125.0), Some("10A")).score, 2, "two steps, same tempo");
        assert_eq!(m(Some(125.0), Some("3B")).score, 1, "key clash, same tempo");
        assert_eq!(m(Some(130.0), Some("10A")).score, 1, "both workable");
        assert_eq!(m(Some(140.0), Some("8A")).score, 1, "same key, +12 %");
        assert_eq!(m(Some(140.0), Some("3B")).score, 0, "nothing fits");
        assert_eq!(m(None, Some("8A")).score, 2, "unknown tempo counts as workable");
        assert_eq!(m(None, None).score, 1);
        assert_eq!(match_with(None, Some(125.0), None).score, -1, "no reference");
        let empty = Reference::default();
        assert_eq!(match_with(Some(&empty), Some(125.0), None).score, -1);
        // Better fits rank higher; equal scores by the closer tempo.
        assert!(m(Some(126.0), Some("9A")).rank() > m(Some(128.0), Some("9A")).rank());
        assert!(m(Some(128.0), Some("9A")).rank() > m(Some(130.0), Some("8A")).rank());
        od_i18n::set_language(od_i18n::Language::English);
        assert_eq!(
            m(Some(126.0), Some("9A")).tip(Some(&master)),
            "vs MASTER 125.0 · 8A: adjacent key · tempo +0.8 %"
        );
        assert_eq!(match_with(None, None, None).tip(None), "");
    }
}
