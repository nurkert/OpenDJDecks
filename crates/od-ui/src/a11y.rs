//! Readable names for screen readers and the help overlay.
//!
//! Every control carries two accessibility strings: a stable test id
//! (`accessible-id`, the convention label of docs/parity.md such as
//! `deck1.stem.vocal`) and a spoken name (`accessible-label`). The shared
//! controls compute the name from what the person already sees: the keymap
//! title when the id is a keymap action ("Deck 1: Play or pause"), else the
//! legend or the first clause of the tooltip, with the deck, channel or unit
//! the control belongs to. So the names follow the legends and tooltips and
//! are translated with them.

use crate::keymap;

/// Whether `s` looks like a test id (`deck1.stem.vocal`, `prefs.view`),
/// never a readable name.
pub(crate) fn id_like(s: &str) -> bool {
    let s = s.trim();
    s.contains('.')
        && !s.contains(' ')
        && s.split('.').all(|p| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '\\')
        })
}

/// The first clause of a tooltip: up to a colon, a bracket or the end of the
/// first sentence, at most about 60 characters.
fn first_clause(tip: &str) -> String {
    let mut end = tip.len();
    for stop in [": ", " (", ". ", "; ", " — ", " – "] {
        if let Some(i) = tip.find(stop) {
            end = end.min(i);
        }
    }
    let mut s = tip[..end].trim().trim_end_matches('.').to_string();
    if s.chars().count() > 60 {
        let cut: String = s.chars().take(60).collect();
        s = match cut.rfind(' ') {
            Some(i) => cut[..i].to_string(),
            None => cut,
        };
    }
    s
}

/// `zoom-in` → `zoom in` (the last resort: a part of the id in words).
fn words_of(part: &str) -> String {
    part.replace(['-', '_'], " ")
}

/// The number glued to an id part: `deck2` → 2.
fn instance(part: &str, noun: &str) -> Option<u32> {
    part.strip_prefix(noun)?.parse().ok()
}

/// "Deck 1", "Channel 2", "FX 1", "Player 2" for an id, if it names one.
fn owner(id: &str) -> Option<String> {
    let mut parts = id.split('.');
    let first = parts.next()?;
    if let Some(n) = instance(first, "deck") {
        return Some(od_i18n::trf("Deck {}", &[&n]));
    }
    if let Some(n) = instance(first, "player").filter(|n| *n > 1) {
        return Some(od_i18n::trf("Player {}", &[&n]));
    }
    let second = parts.next().unwrap_or("");
    match first {
        "mixer" => instance(second, "ch").map(|n| od_i18n::trf("Channel {}", &[&n])),
        "fx" => instance(second, "unit").map(|n| od_i18n::trf("FX {}", &[&n])),
        _ => None,
    }
}

/// The accessible description: the tooltip, and the shortcut in brackets
/// when there is one ("Undo the last change [⌘Z]").
pub(crate) fn describe(tip: &str, keys: &str) -> String {
    match (tip.trim().is_empty(), keys.is_empty()) {
        (_, true) => tip.to_string(),
        (true, false) => format!("[{keys}]"),
        (false, false) => format!("{tip} [{keys}]"),
    }
}

/// Splits a description back into the tooltip and the shortcut.
pub(crate) fn split_description(d: &str) -> (String, String) {
    if let Some(open) = d.rfind(" [").or_else(|| d.starts_with('[').then_some(0))
        && d.ends_with(']')
    {
        let start = if d[open..].starts_with(" [") { open + 2 } else { open + 1 };
        return (d[..open].trim().to_string(), d[start..d.len() - 1].to_string());
    }
    (d.to_string(), String::new())
}

/// The spoken name of a control from its test id, its legend and its
/// tooltip (any of them may be empty).
pub(crate) fn name_for(id: &str, label: &str, tip: &str) -> String {
    let id = id.trim();
    if let Some(a) = keymap::action(id) {
        return a.title();
    }
    let label = label.trim();
    let letters = label.chars().filter(|c| c.is_alphabetic()).count();
    let core = if letters >= 2 && !id_like(label) {
        label.to_string()
    } else if !tip.trim().is_empty() {
        first_clause(tip.trim())
    } else if !label.is_empty() && !id_like(label) {
        label.to_string()
    } else if let Some(last) = id.rsplit('.').next().filter(|p| !p.is_empty() && !p.starts_with("cmd:")) {
        words_of(last)
    } else {
        String::new()
    };
    if core.is_empty() {
        return core;
    }
    match owner(id) {
        Some(o) if !core.starts_with(&o) => od_i18n::trf("{}: {}", &[&o, &core]),
        _ => core,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_readable() {
        od_i18n::set_language(od_i18n::Language::English);
        assert!(id_like("deck1.stem.vocal"));
        assert!(id_like("prefs.view"));
        assert!(!id_like("Deck 1: Play"));
        assert!(!id_like("LOAD SELECTED"));
        // A keymap action reads its title.
        assert!(name_for("deck1.play", "", "").starts_with("Deck 1"));
        // A legend with words, owned by a deck.
        assert_eq!(name_for("deck2.stem.vocal", "VOCAL", ""), "Deck 2: VOCAL");
        // A sign or symbol reads the tooltip's first clause.
        assert_eq!(
            name_for("deck1.bpm-half", "½", "Half the tempo: the track's grid at half the BPM"),
            "Deck 1: Half the tempo"
        );
        assert_eq!(name_for("mixer.ch3.trim", "TRIM", ""), "Channel 3: TRIM");
        assert_eq!(name_for("wave.zoom-in", "", ""), "zoom in");
        assert_eq!(name_for("", "", ""), "");
        assert_eq!(describe("Undo", "⌘Z"), "Undo [⌘Z]");
        assert_eq!(split_description("Undo [⌘Z]"), ("Undo".into(), "⌘Z".into()));
        assert_eq!(split_description("[F1]"), ("".into(), "F1".into()));
        assert_eq!(split_description("Plain"), ("Plain".into(), "".into()));
        for (id, l, t) in [("deck1.stem.vocal", "", ""), ("browser.rows", "", "")] {
            assert!(!id_like(&name_for(id, l, t)), "{id}");
        }
    }
}
