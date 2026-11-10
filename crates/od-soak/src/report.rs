//! Turns soak logs (`soak --out run.jsonl`) into the tables of the long-run
//! results guide (docs/guide/{en,de}/long-run-results.md), between the
//! `soak-results` markers. Only measured numbers are written: a run without
//! its closing `result` line is listed as unfinished with the minutes it
//! logged, and nothing is extrapolated.
//!
//! ```text
//! cargo run -p od-soak --bin soak-report -- docs/soak/*.jsonl
//! ```

use anyhow::{Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub const START: &str =
    "<!-- soak-results:start (generated: cargo run -p od-soak --bin soak-report -- docs/soak/*.jsonl) -->";
pub const END: &str = "<!-- soak-results:end -->";

#[derive(Debug, Default)]
struct Run {
    file: String,
    header: Value,
    minutes: Vec<Value>,
    result: Option<Value>,
}

fn load(path: &Path) -> Result<Run> {
    let text = std::fs::read_to_string(path).with_context(|| format!("{}", path.display()))?;
    let mut run = Run {
        file: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        ..Default::default()
    };
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let v: Value = serde_json::from_str(line)?;
        match v["type"].as_str() {
            Some("header") => run.header = v,
            Some("minute") => run.minutes.push(v),
            Some("result") => run.result = Some(v),
            _ => {}
        }
    }
    Ok(run)
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}

fn median(mut xs: Vec<f64>) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.sort_by(|a, b| a.total_cmp(b));
    xs[xs.len() / 2]
}

fn max(xs: impl Iterator<Item = f64>) -> f64 {
    xs.fold(0.0, f64::max)
}

fn utc(unix: u64) -> String {
    // Civil date from days since 1970 (Howard Hinnant's algorithm).
    let days = (unix / 86_400) as i64;
    let secs = unix % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02} UTC", secs / 3600, (secs % 3600) / 60)
}

struct Words {
    de: bool,
}

impl Words {
    fn t<'a>(&self, en: &'a str, de: &'a str) -> &'a str {
        if self.de { de } else { en }
    }
    fn num(&self, x: f64, digits: usize) -> String {
        let s = format!("{x:.digits$}");
        if self.de { s.replace('.', ",") } else { s }
    }
}

fn run_section(run: &Run, w: &Words) -> String {
    let h = &run.header;
    let mut s = String::new();
    let decks = h["decks"].as_u64().unwrap_or(0);
    let minutes = run.minutes.len();
    let planned = f(&h["planned_minutes"]);
    let title = if planned < 30.0 {
        format!(
            "{} {decks} {}, {} ({minutes} min)",
            w.t("Short smoke run:", "Kurzer Probelauf:"),
            w.t("decks", "Decks"),
            w.t("not a long run", "kein Langlauf")
        )
    } else {
        format!(
            "{decks} {}, {} {}",
            w.t("decks", "Decks"),
            w.num(planned / 60.0, 0),
            w.t("hours", "Stunden")
        )
    };
    s.push_str(&format!("### {title}\n\n"));
    if run.result.is_none() {
        s.push_str(&format!(
            "{} {minutes} {}\n\n",
            w.t(
                "**Unfinished:** the log ends without its closing line. Logged so far:",
                "**Nicht beendet:** Das Log endet ohne seine Schlusszeile. Bisher protokolliert:"
            ),
            w.t("minutes.", "Minuten.")
        ));
    }
    let callbacks: f64 = run.minutes.iter().map(|m| f(&m["callbacks"])).sum();
    let misses: f64 = run.minutes.iter().map(|m| f(&m["misses"])).sum();
    let overruns: f64 = run.minutes.iter().map(|m| f(&m["overruns"])).sum();
    let p50 = median(run.minutes.iter().map(|m| f(&m["render_us"]["p50"])).collect());
    let p99_med = median(run.minutes.iter().map(|m| f(&m["render_us"]["p99"])).collect());
    let p99_worst = max(run.minutes.iter().map(|m| f(&m["render_us"]["p99"])));
    let rmax = max(run.minutes.iter().map(|m| f(&m["render_us"]["max"])));
    let period_us = f(&h["period_ms"]) * 1000.0;
    let pct = |us: f64| if period_us > 0.0 { us / period_us * 100.0 } else { 0.0 };
    let rss: Vec<f64> = run.minutes.iter().map(|m| f(&m["rss_mb"])).collect();
    let load1: Vec<f64> = run.minutes.iter().map(|m| f(&m["loadavg"][0])).collect();
    let minutes_with_miss = run.minutes.iter().filter(|m| f(&m["misses"]) > 0.0).count();
    let row = |k: &str, v: String| format!("| {k} | {v} |\n");
    s.push_str(w.t("| Measure | Value |\n|---|---|\n", "| Messgröße | Wert |\n|---|---|\n"));
    s.push_str(&row(
        w.t("Started", "Beginn"),
        utc(h["started_unix"].as_u64().unwrap_or(0)),
    ));
    s.push_str(&row(w.t("Minutes measured", "Gemessene Minuten"), minutes.to_string()));
    s.push_str(&row(
        w.t("Buffer, rate, latency", "Puffer, Rate, Latenz"),
        format!(
            "{} {} · {} Hz · {} ms",
            h["buffer_frames"],
            w.t("frames", "Frames"),
            h["sample_rate"],
            w.num(f(&h["latency_ms"]), 2)
        ),
    ));
    s.push_str(&row(
        w.t("Engine callbacks", "Engine-Callbacks"),
        format!("{}", callbacks as u64),
    ));
    s.push_str(&row(
        w.t("Deadline misses (dropouts)", "Verpasste Fristen (Aussetzer)"),
        format!(
            "**{}** {} {} {}",
            misses as u64,
            w.t("in", "in"),
            minutes_with_miss,
            w.t("of the minutes", "der Minuten")
        ),
    ));
    s.push_str(&row(
        w.t("Render time longer than the buffer", "Rechenzeit länger als der Puffer"),
        format!("{}", overruns as u64),
    ));
    s.push_str(&row(
        w.t("Render time, median of the minute medians", "Rechenzeit, Median der Minuten-Mediane"),
        format!("{} ms ({} %)", w.num(p50 / 1000.0, 2), w.num(pct(p50), 0)),
    ));
    s.push_str(&row(
        w.t("Render time p99, typical minute / worst minute", "Rechenzeit p99, typische / schlechteste Minute"),
        format!(
            "{} ms ({} %) / {} ms ({} %)",
            w.num(p99_med / 1000.0, 2),
            w.num(pct(p99_med), 0),
            w.num(p99_worst / 1000.0, 2),
            w.num(pct(p99_worst), 0)
        ),
    ));
    s.push_str(&row(
        w.t("Longest single callback", "Längster einzelner Callback"),
        format!("{} ms ({} %)", w.num(rmax / 1000.0, 2), w.num(pct(rmax), 0)),
    ));
    if let (Some(first), Some(last)) = (rss.first(), rss.last()) {
        s.push_str(&row(
            w.t("Memory (RSS) first / last / highest minute", "Speicher (RSS) erste / letzte / höchste Minute"),
            format!(
                "{} / {} / {} MB",
                w.num(*first, 0),
                w.num(*last, 0),
                w.num(max(rss.iter().copied()), 0)
            ),
        ));
    }
    s.push_str(&row(
        w.t("Machine load average (1 min), median / highest", "Last des Rechners (1 min), Median / höchste"),
        format!("{} / {}", w.num(median(load1.clone()), 1), w.num(max(load1.iter().copied()), 1)),
    ));
    if let Some(r) = &run.result {
        s.push_str(&row(
            w.t("Recorded to WAV", "Als WAV aufgenommen"),
            format!("{} min", w.num(f(&r["recorded_s"]) / 60.0, 0)),
        ));
        s.push_str(&row(
            w.t("Deck reloads during the run", "Neu geladene Decks während des Laufs"),
            format!("{}", r["reloads"]),
        ));
    }
    s.push_str(&row(w.t("Log", "Log"), format!("`docs/soak/{}`", run.file)));
    // Hour by hour for long runs.
    if minutes >= 120 {
        s.push_str(&format!(
            "\n{}\n\n",
            w.t("Hour by hour:", "Stunde für Stunde:")
        ));
        s.push_str(w.t(
            "| Hour | Misses | Render p99, worst minute | Longest callback | RSS at the end | Load average (median) |\n|---|---|---|---|---|---|\n",
            "| Stunde | Aussetzer | Rechenzeit p99, schlechteste Minute | Längster Callback | RSS am Ende | Last (Median) |\n|---|---|---|---|---|---|\n",
        ));
        for (i, hour) in run.minutes.chunks(60).enumerate() {
            let m: f64 = hour.iter().map(|m| f(&m["misses"])).sum();
            let p99 = max(hour.iter().map(|m| f(&m["render_us"]["p99"])));
            let mx = max(hour.iter().map(|m| f(&m["render_us"]["max"])));
            let r = hour.last().map(|m| f(&m["rss_mb"])).unwrap_or(0.0);
            let l = median(hour.iter().map(|m| f(&m["loadavg"][0])).collect());
            let label = if hour.len() < 60 {
                format!("{} ({} min)", i + 1, hour.len())
            } else {
                format!("{}", i + 1)
            };
            s.push_str(&format!(
                "| {label} | {} | {} ms | {} ms | {} MB | {} |\n",
                m as u64,
                w.num(p99 / 1000.0, 2),
                w.num(mx / 1000.0, 2),
                w.num(r, 0),
                w.num(l, 1)
            ));
        }
    }
    s.push('\n');
    s
}

/// The generated section for `runs`.
fn section(runs: &[Run], de: bool) -> String {
    let w = Words { de };
    let mut s = String::new();
    if let Some(h) = runs.first().map(|r| &r.header) {
        s.push_str(&format!(
            "{} {} ({}), {} GB, {}. {}\n\n",
            w.t("Machine:", "Rechner:"),
            h["machine"].as_str().unwrap_or("?"),
            h["cpu"].as_str().unwrap_or("?"),
            h["memory_bytes"].as_u64().unwrap_or(0) / (1 << 30),
            h["os"].as_str().unwrap_or("?"),
            w.t(
                "Build: optimised (`--profile soak`).",
                "Build: optimiert (`--profile soak`)."
            )
        ));
    }
    let long_done = runs
        .iter()
        .any(|r| r.result.is_some() && f(&r.header["planned_minutes"]) >= 240.0);
    if !long_done {
        s.push_str(w.t(
            "**No run of four hours or more has finished yet.** Only what is listed below was measured so far.\n\n",
            "**Noch ist kein Lauf von vier Stunden oder mehr beendet.** Bisher wurde nur gemessen, was unten steht.\n\n",
        ));
    }
    for r in runs {
        s.push_str(&run_section(r, &w));
    }
    s
}

fn replace(doc: &str, body: &str) -> Option<String> {
    let a = doc.find(START)? + START.len();
    let b = doc[a..].find(END)? + a;
    Some(format!("{}\n\n{}\n{}", &doc[..a], body.trim_end(), &doc[b..]))
}

fn main() -> Result<()> {
    let paths: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    anyhow::ensure!(!paths.is_empty(), "give the soak logs (.jsonl)");
    let runs: Vec<Run> = paths.iter().map(|p| load(p)).collect::<Result<_>>()?;
    let guide = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide");
    for (lang, de) in [("en", false), ("de", true)] {
        let path = guide.join(lang).join("long-run-results.md");
        let doc = std::fs::read_to_string(&path)?;
        let new = replace(&doc, &section(&runs, de))
            .with_context(|| format!("{}: soak-results markers missing", path.display()))?;
        std::fs::write(&path, new)?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(minutes: usize, finished: bool) -> String {
        let mut s = String::from(
            r#"{"type":"header","decks":2,"planned_minutes":240.0,"period_ms":5.333,"latency_ms":5.333,"buffer_frames":256,"sample_rate":48000,"started_unix":1794292352,"machine":"M","cpu":"C","memory_bytes":34359738368,"os":"OS"}"#,
        );
        for i in 0..minutes {
            s.push_str(&format!(
                "\n{{\"type\":\"minute\",\"t\":{},\"callbacks\":11250,\"misses\":{},\"overruns\":0,\"render_us\":{{\"p50\":2000.0,\"p99\":3000.0,\"max\":4000.0}},\"rss_mb\":500.0,\"loadavg\":[2.0,2.0,2.0]}}",
                i + 1,
                if i == 3 { 2 } else { 0 }
            ));
        }
        if finished {
            s.push_str("\n{\"type\":\"result\",\"recorded_s\":14400,\"reloads\":80}");
        }
        s
    }

    fn run(text: &str) -> Run {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = tempfile_dir();
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = dir.join(format!("r{n}-{}.jsonl", text.len()));
        std::fs::write(&p, text).unwrap();
        load(&p).unwrap()
    }

    fn tempfile_dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("od-soak-report-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_finished_run_is_summed_and_split_by_hour() {
        let md = section(&[run(&log(130, true))], false);
        assert!(md.contains("### 2 decks, 4 hours"), "{md}");
        assert!(md.contains("| Minutes measured | 130 |"), "{md}");
        assert!(md.contains("**2** in 1 of the minutes"), "{md}");
        assert!(md.contains("| 3 (10 min) |"), "the last partial hour says so: {md}");
        assert!(md.contains("2026-11-10 06:32 UTC"), "{md}");
        assert!(!md.contains("Unfinished"));
        assert!(!md.contains("No run of four hours"));
        let de = section(&[run(&log(130, true))], true);
        assert!(de.contains("| Gemessene Minuten | 130 |"), "{de}");
    }

    #[test]
    fn an_unfinished_run_says_so_and_adds_nothing() {
        let md = section(&[run(&log(5, false))], false);
        assert!(md.contains("**Unfinished:**"), "{md}");
        assert!(md.contains("No run of four hours or more has finished yet"), "{md}");
        assert!(md.contains("| Minutes measured | 5 |"));
        assert!(!md.contains("Hour by hour"));
    }

    #[test]
    fn the_section_goes_between_the_markers() {
        let doc = format!("a\n{START}\nold\n{END}\nb\n");
        let new = replace(&doc, "new").unwrap();
        assert_eq!(new, format!("a\n{START}\n\nnew\n{END}\nb\n"));
    }
}
