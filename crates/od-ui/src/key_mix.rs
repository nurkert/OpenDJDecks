//! [62] Key relation on the deck and how sure key and grid analysis are.
//!
//! Each deck's key is rated against the other playing decks in three steps
//! (fits, energy change, clash; [`KeyRelation`]) using the key you hear:
//! the track's key moved by key shift and, without keylock, by the tempo.
//! The deck draws the relation as a line under its KEY in the status-line
//! grammar (solid green fits, dashed sodium energy, struck deep red clash),
//! so the form carries it as well as the colour.
//!
//! The analysis says how sure it is of key and grid; an unsure value shows
//! its percentage on the deck and, in Prepare's INFO tab, the likely
//! alternative to take with one press or a fresh analysis.

use crate::app::{App, handler};
use od_core::{AnalysisConfidence, KeyNotation, KeyRelation, MusicalKey};
use od_engine::DeckState;
use slint::{ComponentHandle, SharedString};
use std::rc::Rc;

/// The key you hear on a deck: the track's key moved by the deck's pitch
/// (key shift, and the tempo when keylock is off), to the nearest semitone.
pub(crate) fn heard_key(s: &DeckState) -> Option<MusicalKey> {
    s.track.and(s.track_key).map(|k| k.transpose(s.pitch_semitones.round() as i32))
}

/// How bad a relation is (the deck shows the worst against all others).
fn rank(r: KeyRelation) -> u8 {
    match r {
        KeyRelation::Fits => 0,
        KeyRelation::Energy => 1,
        KeyRelation::Clash => 2,
    }
}

/// Deck `i`'s key against the other playing decks among the first
/// `count`: the worst relation and the deck it is against (`None` when no
/// other deck plays or a key is unknown).
pub(crate) fn relation(decks: &[DeckState], count: usize, i: usize) -> Option<(KeyRelation, usize, MusicalKey)> {
    let mine = decks.get(i).and_then(heard_key)?;
    (0..count.min(decks.len()))
        .filter(|&j| j != i && decks[j].playing)
        .filter_map(|j| heard_key(&decks[j]).map(|k| (mine.relation(k), j, k)))
        .max_by_key(|(r, j, _)| (rank(*r), std::cmp::Reverse(*j)))
}

/// The id the deck view uses ("fits", "energy", "clash"; "" = none).
pub(crate) fn relation_id(r: Option<KeyRelation>) -> &'static str {
    match r {
        Some(KeyRelation::Fits) => "fits",
        Some(KeyRelation::Energy) => "energy",
        Some(KeyRelation::Clash) => "clash",
        None => "",
    }
}

/// The deck's spelled-out relation ("Fits deck 2 (8A)").
pub(crate) fn relation_text(rel: Option<(KeyRelation, usize, MusicalKey)>, notation: KeyNotation) -> String {
    let Some((r, j, k)) = rel else { return String::new() };
    let deck = (j + 1).to_string();
    let key = k.format(notation);
    match r {
        KeyRelation::Fits => od_i18n::trf("Fits deck {} ({})", &[&deck, &key]),
        KeyRelation::Energy => od_i18n::trf("Energy change against deck {} ({})", &[&deck, &key]),
        KeyRelation::Clash => od_i18n::trf("Clashes with deck {} ({})", &[&deck, &key]),
    }
}

/// "41 %" for an unsure value, "" when sure enough (or unknown).
pub(crate) fn unsure_text(value: Option<f32>) -> String {
    match value {
        Some(v) if v < AnalysisConfidence::UNSURE => percent(v),
        _ => String::new(),
    }
}

/// "92 %".
pub(crate) fn percent(v: f32) -> String {
    format!("{:.0} %", (v.clamp(0.0, 1.0) * 100.0).round())
}

/// What Prepare's INFO tab shows for a track's analysis.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SureView {
    /// "92 %" ("" = no value: an older analysis).
    pub key: String,
    pub key_unsure: bool,
    /// The likely other key ("5A", "" = none).
    pub key_alt: String,
    pub grid: String,
    pub grid_unsure: bool,
    /// The likely other tempo ("62.0", "" = none).
    pub bpm_alt: String,
}

/// Projects a track's confidence (pure).
pub(crate) fn sure_view(c: Option<&AnalysisConfidence>, notation: KeyNotation) -> SureView {
    let Some(c) = c else { return SureView::default() };
    SureView {
        key: percent(c.key),
        key_unsure: c.key < AnalysisConfidence::UNSURE,
        key_alt: c.key_alt.map(|k| k.format(notation)).unwrap_or_default(),
        grid: percent(c.grid),
        grid_unsure: c.grid < AnalysisConfidence::UNSURE,
        bpm_alt: c.bpm_alt.map(|b| format!("{b:.1}")).unwrap_or_default(),
    }
}

/// Shows the confidence of the Prepare player's track.
pub(crate) fn show(app: &App, c: Option<&AnalysisConfidence>) {
    let v = sure_view(c, app.state.borrow().settings.key_notation());
    let g = app.ui.global::<crate::AnalysisSure>();
    g.set_key(v.key.into());
    g.set_key_unsure(v.key_unsure);
    g.set_key_alt(v.key_alt.into());
    g.set_grid(v.grid.into());
    g.set_grid_unsure(v.grid_unsure);
    g.set_bpm_alt(v.bpm_alt.into());
}

/// USE (the likely other key): sets it as your key (one undo step).
pub(crate) fn use_alt_key(app: &App) {
    if app.guard_read_only() {
        return;
    }
    let Some(id) = app.state.borrow().player.current else { return };
    let alt = {
        let st = app.state.borrow();
        st.lib.analysis(id).ok().and_then(|a| a.confidence).and_then(|c| c.key_alt)
    };
    let Some(alt) = alt else { return };
    let notation = app.state.borrow().settings.key_notation();
    let done = app.state.borrow_mut().lib.set_key(id, Some(alt));
    match done {
        Ok(()) => {
            app.state.borrow_mut().load_rows();
            crate::browser::refresh_tracks(app);
            crate::player::refresh_detail(app);
            crate::decks::refresh_key(app, id, alt);
            app.status(od_i18n::trf("Key set to {}", &[&alt.format(notation)]));
        }
        Err(e) => app.status(od_i18n::trf("Could not set the key: {}", &[&e])),
    }
}

/// USE (the likely other tempo): the grid at half or double the BPM.
pub(crate) fn use_alt_bpm(app: &App) {
    let Some(id) = app.state.borrow().player.current else { return };
    let (alt, bpm) = {
        let st = app.state.borrow();
        let a = st.lib.analysis(id).unwrap_or_default();
        (a.confidence.and_then(|c| c.bpm_alt), a.beatgrid.first_bpm())
    };
    let (Some(alt), Some(bpm)) = (alt, bpm) else { return };
    crate::grid_edit::act(app, "scale", if alt < bpm { "0.5" } else { "2" });
}

/// Documentation screenshots (`OPENDJDECKS_DEMO_PROOF`): `key-relation`
/// shifts deck 2 a semitone up so its key clashes with deck 1 (both decks
/// play); `key-sure` opens Prepare on the first row's INFO tab and analyses
/// that track again, so its real confidence shows.
fn demo(app: &Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else { return };
    let relation = v.split(',').any(|p| p == "key-relation");
    let sure = v.split(',').any(|p| p == "key-sure");
    if sure {
        app.ui.global::<crate::AnalysisSure>().set_first_tab("info".into());
    }
    if !relation && !sure {
        return;
    }
    let weak = Rc::downgrade(app);
    slint::Timer::single_shot(std::time::Duration::from_secs(5), move || {
        let Some(a) = weak.upgrade() else { return };
        if relation {
            crate::engine::deck_cmd(&a, 1, od_engine::DeckCommand::SetKeyShift(1.0));
            crate::engine::deck_cmd(&a, 1, od_engine::DeckCommand::Play);
            crate::engine::deck_cmd(&a, 0, od_engine::DeckCommand::Play);
        }
        if sure {
            a.ui.invoke_row_clicked(0, false, false);
            a.ui.invoke_analyse_selected();
        }
    });
}

pub(crate) fn wire(app: &Rc<App>) {
    demo(app);
    crate::stem_mix::demo(app);
    let g = app.ui.global::<crate::AnalysisSure>();
    g.on_fix(handler!(app, |a, what: SharedString| match what.as_str() {
        "key" => use_alt_key(&a),
        "bpm" => use_alt_bpm(&a),
        _ => a.ui.invoke_analyse_selected(),
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck(key: &str, playing: bool, pitch: f32) -> DeckState {
        DeckState {
            track: Some(od_core::TrackId(1)),
            track_key: MusicalKey::parse(key),
            playing,
            pitch_semitones: pitch,
            ..Default::default()
        }
    }

    #[test]
    fn worst_relation_against_the_playing_decks() {
        let decks = [deck("8A", true, 0.0), deck("9A", false, 0.0), deck("3B", true, 0.0)];
        // Deck 2 (cued): fits deck 1 but clashes with deck 3.
        let r = relation(&decks, 3, 1).unwrap();
        assert_eq!((r.0, r.1), (KeyRelation::Clash, 2));
        // Deck 1 only against playing deck 3 (deck 2 is paused).
        assert_eq!(relation(&decks, 3, 0).unwrap().1, 2);
        // Only two decks counted: deck 2 fits deck 1.
        assert_eq!(relation(&decks, 2, 1).unwrap().0, KeyRelation::Fits);
        // Nobody else plays.
        assert_eq!(relation(&decks, 2, 0), None);
        assert_eq!(relation_id(None), "");
    }

    #[test]
    fn the_heard_key_follows_pitch() {
        // 8A (Am) pitched up a semitone is Bbm (3A): an energy change.
        let decks = [deck("8A", true, 0.0), deck("8A", true, 1.0)];
        assert_eq!(heard_key(&decks[1]), MusicalKey::parse("3A"));
        assert_eq!(relation(&decks, 2, 0).unwrap().0, KeyRelation::Energy);
        // A small tempo change rounds to no change.
        let decks = [deck("8A", true, 0.0), deck("8A", true, 0.3)];
        assert_eq!(relation(&decks, 2, 0).unwrap().0, KeyRelation::Fits);
        let unloaded = DeckState::default();
        assert_eq!(heard_key(&unloaded), None);
    }

    #[test]
    fn confidence_texts() {
        assert_eq!(unsure_text(Some(0.41)), "41 %");
        assert_eq!(unsure_text(Some(0.8)), "");
        assert_eq!(unsure_text(None), "");
        let c = AnalysisConfidence {
            key: 0.3,
            key_alt: MusicalKey::parse("5A"),
            grid: 0.9,
            bpm_alt: Some(62.0),
        };
        let v = sure_view(Some(&c), KeyNotation::Camelot);
        assert_eq!((v.key.as_str(), v.key_unsure, v.key_alt.as_str()), ("30 %", true, "5A"));
        assert_eq!((v.grid.as_str(), v.grid_unsure, v.bpm_alt.as_str()), ("90 %", false, "62.0"));
        assert_eq!(sure_view(None, KeyNotation::Camelot), SureView::default());
        let tip = relation_text(Some((KeyRelation::Clash, 1, MusicalKey::parse("3B").unwrap())), KeyNotation::Camelot);
        assert!(tip.contains('2') && tip.contains("3B"), "{tip}");
    }
}
