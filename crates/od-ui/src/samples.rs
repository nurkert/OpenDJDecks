//! Sample tracks for the first start, synthesised here (own material, no
//! third-party audio, nothing in the repository): a kick, offbeat hats, a
//! bassline and chord stabs in two tempos and keys that mix with each other.

use crate::workers::{WorkerEvent, import_file};
use crossbeam_channel::Sender;
use od_library::Library;
use std::f32::consts::TAU;
use std::path::{Path, PathBuf};

const RATE: u32 = 44_100;
/// Length of a sample track in bars (about a minute and a half).
const BARS: u32 = 48;

/// Sample tracks: title, tempo, root note of the bass (Hz), seed of the noise.
const TRACKS: [(&str, f32, f32, u64); 2] = [
    ("Sample Groove A", 124.0, 55.0, 1),
    ("Sample Groove B", 126.0, 65.41, 2),
];

fn bars() -> u32 {
    od_core::env::var("SAMPLE_BARS")
        .and_then(|v| v.parse().ok())
        .filter(|b| *b > 0)
        .unwrap_or(BARS)
}

/// One track as interleaved stereo samples.
fn render(bpm: f32, root: f32, seed: u64, bars: u32) -> Vec<i16> {
    let beat = RATE as f32 * 60.0 / bpm;
    let total = (bars as f32 * 4.0 * beat) as usize;
    let mut out = vec![0f32; total * 2];
    let mut noise = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    let mut last_noise = 0.0f32;
    let fade = RATE as f32 * 0.8;
    for n in 0..total {
        let t = n as f32 / RATE as f32;
        let pos = n as f32 / beat;
        let bar = (pos / 4.0).floor() as u32;
        let in_beat = (pos.fract()) * beat / RATE as f32;
        // Sections: intro (first sixth), build (to third), main, break, main.
        let frac = bar as f32 / bars as f32;
        let (kick_on, bass_on, stab_on) = (
            !(0.66..0.8).contains(&frac),
            frac >= 0.17,
            frac >= 0.34 && !(0.66..0.72).contains(&frac),
        );
        let mut s = 0.0f32;
        if kick_on {
            let f = 45.0 + 105.0 * (-in_beat / 0.03).exp();
            s += (TAU * f * in_beat).sin() * (-in_beat / 0.1).exp() * 0.7;
        }
        // Hats on the offbeats (a little to the right), noise through a
        // simple high-pass.
        noise = noise
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let raw = ((noise >> 41) as f32 / (1u64 << 23) as f32) - 1.0;
        let high = raw - last_noise;
        last_noise = raw;
        let off = (pos + 0.5).fract() * beat / RATE as f32;
        let hat = high * (-off / 0.025).exp() * 0.12;
        // A short bass note on every sixteenth except the beat itself.
        let sixteenth = (pos * 4.0).fract() * beat / 4.0 / RATE as f32;
        let bass = if bass_on && (pos * 4.0).floor() as u32 % 4 != 0 {
            let note = root
                * match (pos * 4.0).floor() as u32 % 32 {
                    6 | 22 => 1.5,
                    14 | 30 => 1.2,
                    _ => 1.0,
                };
            (TAU * note * t).sin() * (1.0 - 0.7 * sixteenth / 0.11).max(0.0) * 0.22
        } else {
            0.0
        };
        // A minor triad stab on the offbeat of beat 2 of every bar.
        let stab_t = ((pos - 1.5).rem_euclid(4.0)) * beat / RATE as f32;
        let mut chord = 0.0;
        if stab_on {
            for m in [4.0, 4.76, 6.0] {
                chord += (TAU * root * m * t).sin();
            }
            chord *= (-stab_t / 0.18).exp() * 0.08;
        }
        s += bass + chord + hat * 0.8;
        // Fade in and out over the last bar so no track starts with a jump.
        let edge = ((n as f32 / fade).min(1.0)).min(((total - n) as f32 / fade).min(1.0));
        out[n * 2] = (s + hat * 0.35) * edge;
        out[n * 2 + 1] = s * edge;
    }
    out.into_iter()
        .map(|x| (x.clamp(-1.0, 1.0) * i16::MAX as f32 * 0.9) as i16)
        .collect()
}

/// Writes the sample tracks into `dir`; returns their paths.
pub(crate) fn write_tracks(dir: &Path, bars: u32) -> std::io::Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut paths = Vec::new();
    for (title, bpm, root, seed) in TRACKS {
        let path = dir.join(format!("{title}.wav"));
        let mut w = hound::WavWriter::create(&path, spec)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        for s in render(bpm, root, seed, bars) {
            w.write_sample(s)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
        }
        w.finalize().map_err(|e| std::io::Error::other(e.to_string()))?;
        paths.push(path);
    }
    Ok(paths)
}

/// Writes and adds the sample tracks on a thread; analysis starts by itself.
pub(crate) fn spawn(
    library_path: PathBuf,
    events: Sender<WorkerEvent>,
    wake_analysis: Sender<()>,
) {
    std::thread::spawn(move || {
        let dir = library_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("samples");
        let added = (|| -> anyhow::Result<usize> {
            let paths = write_tracks(&dir, bars())?;
            let mut lib = Library::open(&library_path)?;
            let mut added = 0;
            for path in &paths {
                if lib.track_by_path(path)?.is_some() {
                    continue;
                }
                let id = import_file(&mut lib, path)?;
                let mut tags = lib.track(id)?.tags;
                tags.artist = "OpenDJDecks".into();
                lib.set_tags(id, tags)?;
                added += 1;
            }
            Ok(added)
        })();
        match added {
            Ok(added) => {
                let _ = events.send(WorkerEvent::ImportFinished { added, failed: 0 });
            }
            Err(e) => {
                tracing::warn!(error = %e, "sample tracks failed");
                let _ = events.send(WorkerEvent::Message(
                    "Sample tracks could not be written".into(),
                ));
            }
        }
        let _ = events.send(WorkerEvent::SamplesDone);
        let _ = wake_analysis.send(());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_tracks_are_written_and_stay_in_range() {
        let dir = tempfile::tempdir().unwrap();
        let paths = write_tracks(dir.path(), 4).unwrap();
        assert_eq!(paths.len(), TRACKS.len());
        for path in &paths {
            let mut r = hound::WavReader::open(path).unwrap();
            let spec = r.spec();
            assert_eq!((spec.channels, spec.sample_rate), (2, RATE));
            let peak = r
                .samples::<i16>()
                .map(|s| (s.unwrap() as i32).abs())
                .max()
                .unwrap();
            assert!(peak > 3000, "audible ({peak})");
            assert!(peak < i16::MAX as i32, "no clipping ({peak})");
        }
    }
}
