//! The design rules of `DESIGN.md` as checks over the Slint sources:
//! the type scale and its floor, the contrast table, spacing on the scale,
//! no dashed outlines, one glossary.

use std::collections::BTreeMap;
use std::path::PathBuf;

fn ui_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui")
}

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

/// Every `.slint` file with its source (comments removed).
fn sources() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(ui_dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "slint") {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let code: String = std::fs::read_to_string(&path)
                .unwrap()
                .lines()
                .map(strip_comment)
                .collect::<Vec<_>>()
                .join("\n");
            out.insert(name, code);
        }
    }
    out
}

fn strip_comment(line: &str) -> &str {
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}

/// `prop: value;` pairs (one occurrence each, in source order). `name` must
/// not be preceded by a word character or `-`.
fn values<'a>(src: &'a str, name: &str) -> Vec<&'a str> {
    let mut out = vec![];
    let needle = format!("{name}:");
    let mut at = 0;
    while let Some(i) = src[at..].find(&needle) {
        let start = at + i;
        at = start + needle.len();
        let prev = src[..start].chars().next_back().unwrap_or(' ');
        if prev.is_alphanumeric() || prev == '-' || prev == '_' {
            continue;
        }
        let rest = &src[at..];
        let end = rest.find(';').unwrap_or(rest.len());
        out.push(rest[..end].trim());
    }
    out
}

/// Pixel lengths written as literals (`12px`), not comparisons such as
/// `root.size > 120px`.
fn px_literals(value: &str) -> Vec<String> {
    let bytes = value.as_bytes();
    let mut found = vec![];
    let mut i = 0;
    while let Some(off) = value[i..].find("px") {
        let end = i + off;
        let mut s = end;
        while s > 0 && (bytes[s - 1].is_ascii_digit() || bytes[s - 1] == b'.') {
            s -= 1;
        }
        i = end + 2;
        if s == end {
            continue;
        }
        let mut before = s;
        while before > 0 && bytes[before - 1] == b' ' {
            before -= 1;
        }
        let cmp = before > 0 && matches!(bytes[before - 1], b'<' | b'>' | b'=');
        if !cmp {
            found.push(value[s..end + 2].to_string());
        }
    }
    found
}

fn rgb(hex: &str) -> (f64, f64, f64) {
    let h = hex.trim_start_matches('#');
    let p = |i| u8::from_str_radix(&h[i..i + 2], 16).unwrap() as f64 / 255.0;
    (p(0), p(2), p(4))
}

fn luminance(hex: &str) -> f64 {
    let lin = |c: f64| {
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = rgb(hex);
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

fn contrast(a: &str, b: &str) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Evaluates a colour expression of `tokens.slint` for the given switches:
/// `#HEX`, `name` (another token) or `cond ? a : b` with parentheses.
fn eval(expr: &str, flags: &BTreeMap<&str, bool>, tokens: &BTreeMap<String, String>) -> String {
    let e = expr.trim();
    if let Some(inner) = unwrap_parens(e) {
        return eval(inner, flags, tokens);
    }
    if let Some((cond, a, b)) = split_ternary(e) {
        return if truth(cond, flags) {
            eval(a, flags, tokens)
        } else {
            eval(b, flags, tokens)
        };
    }
    if e.starts_with('#') {
        return e[..7].to_string();
    }
    let e = e.strip_prefix("root.").unwrap_or(e);
    eval(
        tokens.get(e).unwrap_or_else(|| panic!("unknown token {e}")),
        flags,
        tokens,
    )
}

fn truth(cond: &str, flags: &BTreeMap<&str, bool>) -> bool {
    let c = cond.trim();
    if let Some(rest) = c.strip_prefix('!') {
        return !truth(rest, flags);
    }
    flags[c]
}

fn unwrap_parens(e: &str) -> Option<&str> {
    if !e.starts_with('(') || !e.ends_with(')') {
        return None;
    }
    let mut depth = 0;
    for (i, ch) in e.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && i != e.len() - 1 {
                    return None;
                }
            }
            _ => {}
        }
    }
    Some(&e[1..e.len() - 1])
}

fn split_ternary(e: &str) -> Option<(&str, &str, &str)> {
    let mut depth = 0;
    let mut q = None;
    let mut nest = 0;
    for (i, ch) in e.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            '?' if depth == 0 => {
                if q.is_none() {
                    q = Some(i);
                } else {
                    nest += 1;
                }
            }
            ':' if depth == 0 => {
                if nest > 0 {
                    nest -= 1;
                } else {
                    let q = q?;
                    return Some((&e[..q], &e[q + 1..i], &e[i + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

/// The colour tokens of `tokens.slint`.
fn colour_tokens() -> BTreeMap<String, String> {
    let src = std::fs::read_to_string(ui_dir().join("tokens.slint")).unwrap();
    let mut out = BTreeMap::new();
    for line in src.lines() {
        let line = strip_comment(line).trim();
        if let Some(rest) = line.strip_prefix("out property <color> ") {
            if let Some((name, value)) = rest.split_once(':') {
                out.insert(
                    name.trim().to_string(),
                    value.trim().trim_end_matches(';').to_string(),
                );
            }
        }
    }
    out
}

fn palettes() -> Vec<BTreeMap<&'static str, bool>> {
    let mut all = vec![];
    for light in [false, true] {
        for hc in [false, true] {
            for cb in [false, true] {
                all.push(BTreeMap::from([
                    ("light", light),
                    ("high-contrast", hc),
                    ("colorblind-safe", cb),
                ]));
            }
        }
    }
    all
}

const SURFACES: [&str; 4] = ["ground", "ground-band", "plate", "plate-raised"];
const TONES: [&str; 7] = ["w405", "w436", "w486", "w546", "w589", "w615", "w656"];

/// Each ink reaches 4.5:1 on every surface it may sit on, in every
/// palette; so does the ink on anything filled in a tone.
#[test]
fn contrast_table_holds_in_every_palette() {
    let tokens = colour_tokens();
    let mut problems = vec![];
    for flags in palettes() {
        for ink in ["bone", "bone-2", "bone-3"] {
            for surface in SURFACES {
                let c = contrast(&eval(ink, &flags, &tokens), &eval(surface, &flags, &tokens));
                if c < 4.5 {
                    problems.push(format!("{ink} on {surface} {flags:?}: {c:.2}"));
                }
            }
        }
        for tone in TONES {
            let c = contrast(&eval("ground", &flags, &tokens), &eval(tone, &flags, &tokens));
            if c < 4.5 {
                problems.push(format!("ground on {tone} {flags:?}: {c:.2}"));
            }
        }
        for fill in ["bone", "bone-2"] {
            let c = contrast(&eval("ground", &flags, &tokens), &eval(fill, &flags, &tokens));
            if c < 4.5 {
                problems.push(format!("ground on {fill} {flags:?}: {c:.2}"));
            }
        }
    }
    assert!(problems.is_empty(), "contrast below 4.5:\n{}", problems.join("\n"));
}

/// Five sizes, two weights, no font size written as a number anywhere but
/// the token file; nothing below its floor.
#[test]
fn type_scale_and_floors_hold() {
    let tokens = std::fs::read_to_string(ui_dir().join("tokens.slint")).unwrap();
    let want = [
        ("hero", "32px"),
        ("heading", "22px"),
        ("body", "15px"),
        ("control", "13px"),
        ("label", "11px"),
    ];
    let mut problems = vec![];
    for (name, size) in want {
        let line = tokens
            .lines()
            .find(|l| l.contains(&format!("out property <length> {name}:")))
            .unwrap_or_else(|| panic!("token {name} missing"));
        if !line.contains(size) {
            problems.push(format!("{name} should be {size}: {line}"));
        }
    }
    // Floors: label 11, what is touched or read live 12 (the kiosk included).
    let label = tokens.lines().find(|l| l.contains("out property <length> label:")).unwrap();
    if !label.contains("root.touch ? 12px : 11px") {
        problems.push(format!("label floor is not 11 / touch 12: {label}"));
    }
    let control = tokens.lines().find(|l| l.contains("out property <length> control:")).unwrap();
    if !control.contains("max(12px,") {
        problems.push(format!("control floor is not 12: {control}"));
    }
    for (name, src) in sources() {
        if name == "tokens.slint" {
            continue;
        }
        for v in values(&src, "font-size") {
            if !px_literals(v).is_empty() {
                problems.push(format!("{name}: font-size: {v}"));
            }
        }
        for v in values(&src, "font-weight") {
            if !v.contains("Theme.weight-") {
                if v.starts_with(|c: char| c.is_ascii_digit()) {
                    problems.push(format!("{name}: font-weight: {v}"));
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// Milliseconds written as literals (`90ms`); `1ms` and `0ms` are unit
/// conversions and the off value, not timings.
fn ms_literals(src: &str) -> Vec<String> {
    let bytes = src.as_bytes();
    let mut found = vec![];
    let mut i = 0;
    while let Some(off) = src[i..].find("ms") {
        let end = i + off;
        i = end + 2;
        let after = bytes.get(end + 2).copied().unwrap_or(b' ');
        if after.is_ascii_alphanumeric() || after == b'-' || after == b'_' {
            continue;
        }
        let mut s = end;
        while s > 0 && (bytes[s - 1].is_ascii_digit() || bytes[s - 1] == b'.') {
            s -= 1;
        }
        if s == end || (s > 0 && (bytes[s - 1].is_ascii_alphabetic() || bytes[s - 1] == b'-')) {
            continue;
        }
        let lit = &src[s..end + 2];
        if lit != "1ms" && lit != "0ms" {
            found.push(lit.to_string());
        }
    }
    found
}

/// The motion language (DESIGN.md, Motion): four named timings and one
/// curve in the token file, all 0 ms with reduced motion; every `animate`
/// uses them, no timing is written as a number anywhere else, and the
/// audio-linked visuals (waveform lanes, phase meter, booth) never animate.
#[test]
fn motion_uses_the_named_timings_and_one_curve() {
    let tokens = std::fs::read_to_string(ui_dir().join("tokens.slint")).unwrap();
    let mut problems = vec![];
    for name in ["t-press", "t-toggle", "t-panel", "t-mode"] {
        let line = tokens
            .lines()
            .find(|l| l.contains(&format!("out property <duration> {name}:")))
            .unwrap_or_else(|| panic!("token {name} missing"));
        if !line.contains("root.motion ?") || !line.contains(": 0ms") {
            problems.push(format!("{name} is not 0 ms with reduced motion: {line}"));
        }
    }
    let curves = tokens.lines().filter(|l| l.contains("out property <easing>")).count();
    if curves != 1 {
        problems.push(format!("{curves} easing curves in the tokens, want one"));
    }
    for (name, src) in sources() {
        if name == "tokens.slint" {
            continue;
        }
        for lit in ms_literals(&src) {
            problems.push(format!("{name}: hard-coded timing {lit}"));
        }
        let mut at = 0;
        while let Some(i) = src[at..].find("animate ") {
            let start = at + i;
            at = start + 8;
            let block = &src[start..start + src[start..].find('}').unwrap_or(src.len() - start)];
            let duration = block.split("duration:").nth(1).map(|v| v.split(';').next().unwrap().trim());
            if !duration.is_some_and(|d| ["Theme.t-press", "Theme.t-toggle", "Theme.t-panel", "Theme.t-mode"].contains(&d)) {
                problems.push(format!("{name}: animation without a named timing: {block}"));
            }
            if !block.contains("easing: Theme.ease;") {
                problems.push(format!("{name}: animation without the one curve: {block}"));
            }
        }
        if ["wave.slint", "phase.slint", "booth.slint"].contains(&name.as_str()) && src.contains("animate ") {
            problems.push(format!("{name}: audio-linked visuals are smoothed"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The calmer track table and the control anatomy (DESIGN.md, Track Table
/// and Control anatomy): the selected row is filled in ash plate, the title
/// leads in bone at a heavier weight, secondary columns recede to bone 2 /
/// bone 3 (never fainter), the loaded mark is the deck's own colour (no
/// sodium double); knobs carry a value arc and a centre mark, faders a cap
/// with a centre line, and both a drag readout.
#[test]
fn table_and_controls_follow_their_anatomy() {
    let table = sources().remove("table.slint").unwrap();
    let controls = sources().remove("controls.slint").unwrap();
    let line = table
        .lines()
        .find(|l| l.trim_start().starts_with("background: root.item.selected"))
        .expect("selection fill");
    let mut problems = vec![];
    if !line.contains("Theme.plate") {
        problems.push(format!("selected row not filled in plate: {line}"));
    }
    let ink = &table[table.find("pure function ink-of").expect("ink-of")..];
    let ink = &ink[..ink.find("\n    }").unwrap()];
    if !ink.contains(r#"if (id == "title") { return Theme.bone; }"#) {
        problems.push("the title does not lead in bone".into());
    }
    for faint in ["Theme.rule", "with-alpha"] {
        if ink.contains(faint) {
            problems.push(format!("a column recedes below bone 3: {faint}"));
        }
    }
    if !table.contains(r#"font-weight: c.id == "title" ? (root.item.selected ? Theme.weight-semi : Theme.weight-medium)"#) {
        problems.push("the title is not the heaviest cell".into());
    }
    let loaded = &table[table.find("if root.item.deck > 0:").expect("loaded mark")..];
    let loaded = &loaded[..loaded.find("\n    }").unwrap()];
    if loaded.contains("Theme.attention") || !loaded.contains("Theme.deck-color(root.item.deck)") {
        problems.push("the loaded mark is not in the deck colour".into());
    }
    for needed in ["ArcTo", "knob.centre", "knob.arc", "fader.cap", "fader.detent", "component DragReadout"] {
        if !controls.contains(needed) {
            problems.push(format!("controls.slint lacks {needed}"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// Spacing goes through the scale, keys and fields never use dashed
/// outlines, and text colours are readable tokens.
#[test]
fn spacing_outlines_and_text_colours_follow_the_rules() {
    let mut problems = vec![];
    for (name, src) in sources() {
        if name == "tokens.slint" {
            continue;
        }
        for prop in ["spacing", "padding", "padding-left", "padding-right", "padding-top", "padding-bottom"] {
            for v in values(&src, prop) {
                let bad: Vec<String> = px_literals(v)
                    .into_iter()
                    .filter(|p| !p.starts_with("0px") && *p != "0")
                    .collect();
                if !bad.is_empty() {
                    problems.push(format!("{name}: {prop}: {v}"));
                }
            }
        }
        for v in values(&src, "color") {
            if v.contains("with-alpha") || v.contains("Theme.rule") {
                problems.push(format!("{name}: text colour too faint: {v}"));
            }
            if let Some(hex) = v.split_whitespace().find(|t| t.starts_with('#')) {
                let c = hex.trim_end_matches(';');
                let worst = ["#151514", "#1A1A19", "#262624", "#32322F"]
                    .iter()
                    .map(|s| contrast(c, s))
                    .fold(f64::INFINITY, f64::min);
                if worst < 4.5 {
                    problems.push(format!("{name}: hard-coded text colour {c}: {worst:.2}"));
                }
            }
        }
        if src.contains("dashed: true") || src.contains("component Outline") {
            problems.push(format!("{name}: dashed control outline"));
        }
    }
    let controls = std::fs::read_to_string(ui_dir().join("controls.slint")).unwrap();
    for needed in ["component FocusRing", "Theme.focus-width", "ArmedOutline"] {
        if !controls.contains(needed) {
            problems.push(format!("controls.slint lacks {needed}"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// One glossary: retired words stay out of the labels, and the guides
/// record each term in English and German.
#[test]
fn one_glossary_in_app_and_guides() {
    let mut problems = vec![];
    for (name, src) in sources() {
        for retired in [
            r#"label: "MT""#,
            r#"@tr("PARTS")"#,
            r#"@tr("LOAD → FREE")"#,
            r#"label: "LIGHT""#,
        ] {
            if src.contains(retired) {
                problems.push(format!("{name}: {retired}"));
            }
        }
    }
    for (path, heading, words) in [
        ("docs/guide/en/getting-started.md", "## Words used in OpenDJDecks", ["Keylock", "Stems", "Light strip", "Lighting"]),
        ("docs/guide/de/getting-started.md", "## Begriffe in OpenDJDecks", ["Keylock", "Stems", "Lichtleiste", "Lighting"]),
    ] {
        let text = std::fs::read_to_string(workspace().join(path)).unwrap();
        if !text.contains(heading) {
            problems.push(format!("{path}: no glossary"));
        }
        for w in words {
            if !text.contains(&format!("**{w}**")) {
                problems.push(format!("{path}: {w} missing"));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
