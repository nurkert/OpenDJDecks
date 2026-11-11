//! Your own stem files as a track's parts ([66]): stems exported from a
//! production or bought with the track, played instead of separated ones.
//!
//! # Where they are found
//!
//! Next to the track `Some Track.mp3`, either
//!
//! * a folder `Some Track.stems`, `Some Track stems`, `Some Track_stems`
//!   or `Some Track (stems)` whose files are named after their part, e.g.
//!   `vocals.wav`, `Drums.flac`, `03 bass.wav`; or
//! * files beside the track whose name starts with the track's name and
//!   then names the part: `Some Track.vocals.wav`, `Some Track - Drums.flac`,
//!   `Some Track_bass.wav`, `Some Track (other).wav`.
//!
//! Part words: `vocals` / `vocal` / `vox` / `voice`, `drums` / `drum` /
//! `percussion`, `bass`, and for the rest `other` / `inst` / `instrumental`
//! / `music` / `melody`. WAV, FLAC and AIFF files are read.
//!
//! # What is checked
//!
//! Vocals, drums and bass must all be there. Each file must have the
//! track's sample rate (no resampling: a different rate means the files
//! were made from another master) and its length within half a second;
//! shorter files are padded with silence, longer ones cut. The fourth
//! part stays the track minus the other three, as with separated parts: an
//! "other" file is checked like the rest but not played.

use std::path::{Path, PathBuf};

use crate::Stems;

/// A part of the track as its own file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OwnPart {
    Vocals,
    Drums,
    Bass,
    Other,
}

impl OwnPart {
    pub const ALL: [OwnPart; 4] = [OwnPart::Vocals, OwnPart::Drums, OwnPart::Bass, OwnPart::Other];

    pub fn name(self) -> &'static str {
        match self {
            OwnPart::Vocals => "vocals",
            OwnPart::Drums => "drums",
            OwnPart::Bass => "bass",
            OwnPart::Other => "other",
        }
    }

    /// The part a file name (without extension, any case) names, if one.
    fn of_name(name: &str) -> Option<OwnPart> {
        let words: Vec<String> = name
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(str::to_string)
            .collect();
        let has = |ws: &[&str]| words.iter().any(|w| ws.contains(&w.as_str()));
        if has(&["vocals", "vocal", "vox", "voice", "voices", "acapella"]) {
            Some(OwnPart::Vocals)
        } else if has(&["drums", "drum", "percussion", "perc"]) {
            Some(OwnPart::Drums)
        } else if has(&["bass", "bassline"]) {
            Some(OwnPart::Bass)
        } else if has(&["other", "inst", "instrumental", "music", "melody", "rest"]) {
            Some(OwnPart::Other)
        } else {
            None
        }
    }
}

/// Own stem files found for a track.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OwnStemSet {
    /// The folder they are in (for the views: where the parts come from).
    pub folder: PathBuf,
    pub vocals: Option<PathBuf>,
    pub drums: Option<PathBuf>,
    pub bass: Option<PathBuf>,
    pub other: Option<PathBuf>,
}

impl OwnStemSet {
    pub fn get(&self, p: OwnPart) -> Option<&PathBuf> {
        match p {
            OwnPart::Vocals => self.vocals.as_ref(),
            OwnPart::Drums => self.drums.as_ref(),
            OwnPart::Bass => self.bass.as_ref(),
            OwnPart::Other => self.other.as_ref(),
        }
    }

    fn slot(&mut self, p: OwnPart) -> &mut Option<PathBuf> {
        match p {
            OwnPart::Vocals => &mut self.vocals,
            OwnPart::Drums => &mut self.drums,
            OwnPart::Bass => &mut self.bass,
            OwnPart::Other => &mut self.other,
        }
    }

    fn is_empty(&self) -> bool {
        OwnPart::ALL.iter().all(|p| self.get(*p).is_none())
    }

    /// Parts the engine needs that are missing.
    pub fn missing(&self) -> Vec<OwnPart> {
        [OwnPart::Vocals, OwnPart::Drums, OwnPart::Bass]
            .into_iter()
            .filter(|p| self.get(*p).is_none())
            .collect()
    }
}

/// Why own stem files were not used.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum OwnStemError {
    #[error("parts missing: {}", .0.iter().map(|p| p.name()).collect::<Vec<_>>().join(", "))]
    Missing(Vec<OwnPart>),
    #[error("{part} is {got} Hz, the track {want} Hz")]
    SampleRate {
        part: &'static str,
        got: u32,
        want: u32,
    },
    #[error("{part} is {got:.1} s long, the track {want:.1} s")]
    Length {
        part: &'static str,
        got: f64,
        want: f64,
    },
    #[error("{part} could not be read: {why}")]
    Unreadable { part: &'static str, why: String },
}

const AUDIO: [&str; 4] = ["wav", "flac", "aif", "aiff"];
const FOLDER_SUFFIXES: [&str; 4] = [".stems", " stems", "_stems", " (stems)"];
/// Longest length difference accepted, seconds.
pub const LENGTH_TOLERANCE_S: f64 = 0.5;

fn is_audio(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| AUDIO.contains(&e.to_lowercase().as_str()))
}

/// Looks for own stem files of `track` (see the module docs). `None` when
/// there are none at all; a set with parts missing is returned so the
/// views can say what is missing.
pub fn find_own_stems(track: &Path) -> Option<OwnStemSet> {
    let dir = track.parent()?;
    let base = track.file_stem()?.to_str()?;
    // A folder named after the track.
    for suffix in FOLDER_SUFFIXES {
        let folder = dir.join(format!("{base}{suffix}"));
        if !folder.is_dir() {
            continue;
        }
        let mut set = OwnStemSet {
            folder: folder.clone(),
            ..Default::default()
        };
        let mut files: Vec<PathBuf> = std::fs::read_dir(&folder)
            .ok()?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file() && is_audio(p))
            .collect();
        files.sort();
        for f in files {
            let Some(name) = f.file_stem().and_then(|n| n.to_str()) else { continue };
            if let Some(part) = OwnPart::of_name(name) {
                set.slot(part).get_or_insert(f);
            }
        }
        if !set.is_empty() {
            return Some(set);
        }
    }
    // Files beside the track: its name, then the part.
    let mut set = OwnStemSet {
        folder: dir.to_path_buf(),
        ..Default::default()
    };
    let lower = base.to_lowercase();
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && is_audio(p) && p != track)
        .collect();
    files.sort();
    for f in files {
        let Some(name) = f.file_stem().and_then(|n| n.to_str()) else { continue };
        let Some(rest) = name.to_lowercase().strip_prefix(&lower).map(str::to_string) else {
            continue;
        };
        // The track's name must end at a separator ("Song 2" is not a part of "Song").
        if !rest.starts_with(['.', ' ', '-', '_', '(', '[']) {
            continue;
        }
        if let Some(part) = OwnPart::of_name(&rest) {
            set.slot(part).get_or_insert(f);
        }
    }
    (!set.is_empty()).then_some(set)
}

/// Reads the own stem files of `set` for a track decoded at
/// `sample_rate` with `frames` frames, checked as the module docs say.
pub fn load_own_stems(set: &OwnStemSet, sample_rate: u32, frames: usize) -> Result<Stems, OwnStemError> {
    let missing = set.missing();
    if !missing.is_empty() {
        return Err(OwnStemError::Missing(missing));
    }
    let want = frames as f64 / sample_rate.max(1) as f64;
    let read = |part: OwnPart| -> Result<Vec<f32>, OwnStemError> {
        let Some(path) = set.get(part) else {
            return Ok(Vec::new());
        };
        let audio = od_decode::decode(path, od_decode::Layout::Stereo).map_err(|e| {
            OwnStemError::Unreadable {
                part: part.name(),
                why: e.to_string(),
            }
        })?;
        if audio.sample_rate != sample_rate {
            return Err(OwnStemError::SampleRate {
                part: part.name(),
                got: audio.sample_rate,
                want: sample_rate,
            });
        }
        let got = audio.frames() as f64 / audio.sample_rate.max(1) as f64;
        if (got - want).abs() > LENGTH_TOLERANCE_S {
            return Err(OwnStemError::Length {
                part: part.name(),
                got,
                want,
            });
        }
        let mut s = audio.samples;
        s.resize(frames * 2, 0.0);
        Ok(s)
    };
    let vocals = read(OwnPart::Vocals)?;
    let drums = read(OwnPart::Drums)?;
    let bass = read(OwnPart::Bass)?;
    // Checked, not played (the fourth part is the track minus the rest).
    read(OwnPart::Other)?;
    Ok(Stems {
        sample_rate,
        vocals,
        drums,
        bass,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(path: &Path, rate: u32, seconds: f64, freq: f32) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(path, spec).unwrap();
        let n = (seconds * rate as f64) as usize;
        for i in 0..n {
            let v = ((i as f32 * freq * std::f32::consts::TAU / rate as f32).sin() * 8000.0) as i16;
            w.write_sample(v).unwrap();
            w.write_sample(v).unwrap();
        }
        w.finalize().unwrap();
    }

    #[test]
    fn part_words() {
        assert_eq!(OwnPart::of_name("03 Lead Vocals"), Some(OwnPart::Vocals));
        assert_eq!(OwnPart::of_name("DRUMS"), Some(OwnPart::Drums));
        assert_eq!(OwnPart::of_name("bass_stem"), Some(OwnPart::Bass));
        assert_eq!(OwnPart::of_name("Instrumental"), Some(OwnPart::Other));
        assert_eq!(OwnPart::of_name("bassoon"), None, "whole words only");
        assert_eq!(OwnPart::of_name("master"), None);
    }

    #[test]
    fn a_folder_named_after_the_track() {
        let dir = tempfile::tempdir().unwrap();
        let track = dir.path().join("Night Drive.mp3");
        std::fs::write(&track, b"").unwrap();
        let f = dir.path().join("Night Drive.stems");
        std::fs::create_dir(&f).unwrap();
        for n in ["Vocals.wav", "drums.flac", "03 bass.wav", "other.wav", "notes.txt"] {
            std::fs::write(f.join(n), b"").unwrap();
        }
        let set = find_own_stems(&track).unwrap();
        assert_eq!(set.folder, f);
        assert_eq!(set.vocals, Some(f.join("Vocals.wav")));
        assert_eq!(set.drums, Some(f.join("drums.flac")));
        assert_eq!(set.bass, Some(f.join("03 bass.wav")));
        assert_eq!(set.other, Some(f.join("other.wav")));
        assert!(set.missing().is_empty());
    }

    #[test]
    fn files_beside_the_track() {
        let dir = tempfile::tempdir().unwrap();
        let track = dir.path().join("Song.flac");
        std::fs::write(&track, b"").unwrap();
        for n in ["Song - Vocals.wav", "Song_drums.wav", "Song (Bass).aiff", "Song 2.wav", "Songbass.wav"] {
            std::fs::write(dir.path().join(n), b"").unwrap();
        }
        let set = find_own_stems(&track).unwrap();
        assert_eq!(set.vocals, Some(dir.path().join("Song - Vocals.wav")));
        assert_eq!(set.drums, Some(dir.path().join("Song_drums.wav")));
        assert_eq!(set.bass, Some(dir.path().join("Song (Bass).aiff")));
        assert_eq!(set.other, None);
        // Nothing for another track in the same folder.
        let other = dir.path().join("Other.flac");
        assert_eq!(find_own_stems(&other), None);
    }

    #[test]
    fn loading_checks_rate_and_length() {
        let dir = tempfile::tempdir().unwrap();
        let track = dir.path().join("T.wav");
        wav(&track, 44_100, 2.0, 220.0);
        let f = dir.path().join("T stems");
        std::fs::create_dir(&f).unwrap();
        wav(&f.join("vocals.wav"), 44_100, 2.0, 440.0);
        wav(&f.join("drums.wav"), 44_100, 1.8, 880.0);
        let set = find_own_stems(&track).unwrap();
        assert_eq!(
            load_own_stems(&set, 44_100, 88_200).unwrap_err(),
            OwnStemError::Missing(vec![OwnPart::Bass])
        );
        assert_eq!(OwnStemError::Missing(vec![OwnPart::Bass]).to_string(), "parts missing: bass");
        wav(&f.join("bass.wav"), 48_000, 2.0, 110.0);
        let set = find_own_stems(&track).unwrap();
        let e = load_own_stems(&set, 44_100, 88_200).unwrap_err();
        assert_eq!(e.to_string(), "bass is 48000 Hz, the track 44100 Hz");
        wav(&f.join("bass.wav"), 44_100, 3.0, 110.0);
        let e = load_own_stems(&set, 44_100, 88_200).unwrap_err();
        assert!(matches!(e, OwnStemError::Length { part: "bass", .. }), "{e}");
        wav(&f.join("bass.wav"), 44_100, 2.1, 110.0);
        let stems = load_own_stems(&set, 44_100, 88_200).unwrap();
        // Padded and cut to the track's length.
        assert_eq!(stems.vocals.len(), 88_200 * 2);
        assert_eq!(stems.drums.len(), 88_200 * 2);
        assert_eq!(stems.bass.len(), 88_200 * 2);
        assert!(stems.drums[80_000 * 2..].iter().all(|v| *v == 0.0), "drums padded");
        assert!(stems.vocals.iter().any(|v| v.abs() > 0.1));
    }
}
