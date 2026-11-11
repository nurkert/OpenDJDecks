//! Mix recording (SPEC §11): the master output from the engine's record tap
//! is written to a WAV file by a background thread; the tracklist is built
//! from the engine's "audible track" logic and written as a `.cue` sheet and
//! plain text, and the recording can be split into one file per track.
//!
//! A recording survives a crash: the writer brings the WAV header up to date
//! every second ([`FLUSH_EVERY`]), so a killed process leaves a file that
//! plays up to the last second, and [`repair_wav`] makes the header match
//! everything that reached the disk after that.

mod chapters;
mod encode;
mod loudness;
pub use chapters::{
    Chapter, chapters_of, read_flac_chapters, read_mp3_chapters, read_wav_chapters,
    write_chapters,
};
pub use encode::{AutoAction, SilenceDetector, wav_to_flac, wav_to_mp3};
pub use loudness::{
    ABSOLUTE_GATE, Loudness, LoudnessMeter, TRUE_PEAK_LIMIT, lufs_text, true_peak_text,
};

use od_core::TrackId;
use od_engine::RecordTap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum RecordError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("wav error: {0}")]
    Wav(#[from] hound::Error),
    #[error("already recording")]
    Busy,
    #[error("not recording")]
    Idle,
    #[error("encoding: {0}")]
    Encode(String),
}

pub type Result<T> = std::result::Result<T, RecordError>;

/// How often the writer makes the file on disk a complete WAV.
pub const FLUSH_EVERY: Duration = Duration::from_secs(1);

/// One tracklist entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TracklistEntry {
    /// Seconds from the start of the recording.
    pub offset: f64,
    pub track: Option<TrackId>,
    pub title: String,
    pub artist: String,
    /// A chapter marker set by hand while recording (no track behind it).
    #[serde(default)]
    pub marker: bool,
}

/// A finished recording.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recording {
    pub path: PathBuf,
    pub duration: f64,
    pub tracklist: Vec<TracklistEntry>,
    pub cue_path: PathBuf,
    pub text_path: PathBuf,
    /// The microphone on its own, sample-aligned with `path`.
    #[serde(default)]
    pub voice: Option<PathBuf>,
    #[serde(default)]
    pub loudness: Loudness,
}

/// What travels beside a recording in `<name>.json`: the tracklist as
/// edited, the voice track and the loudness.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RecordingInfo {
    #[serde(default)]
    pub tracklist: Vec<TracklistEntry>,
    #[serde(default)]
    pub voice: Option<PathBuf>,
    #[serde(default)]
    pub loudness: Loudness,
    #[serde(default)]
    pub duration: f64,
}

/// The info file of the recording at `audio`.
pub fn info_path(audio: &Path) -> PathBuf {
    audio.with_extension("json")
}

/// Reads the info file of a recording (`None` for recordings made before
/// it existed).
pub fn load_info(audio: &Path) -> Option<RecordingInfo> {
    let text = std::fs::read_to_string(info_path(audio)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Writes the info file of a recording.
pub fn save_info(audio: &Path, info: &RecordingInfo) -> Result<()> {
    let text = serde_json::to_string_pretty(info).map_err(|e| RecordError::Encode(e.to_string()))?;
    std::fs::write(info_path(audio), text)?;
    Ok(())
}

/// Writes the CUE sheet and the text tracklist next to the recording (the
/// names it got when it stopped) and returns their paths.
pub fn export_tracklist(audio: &Path, tracklist: &[TracklistEntry]) -> Result<(PathBuf, PathBuf)> {
    let cue = audio.with_extension("cue");
    let text = audio.with_extension("txt");
    std::fs::write(&cue, cue_sheet(audio, tracklist))?;
    std::fs::write(&text, text_tracklist(tracklist))?;
    Ok((cue, text))
}

struct Active {
    path: PathBuf,
    voice: Option<PathBuf>,
    loudness: Arc<std::sync::Mutex<Loudness>>,
    stop: Arc<AtomicBool>,
    frames: Arc<AtomicU64>,
    /// Peak of the last block written, as f32 bits (for the level meter).
    peak: Arc<std::sync::atomic::AtomicU32>,
    thread: JoinHandle<Result<()>>,
    tracklist: Vec<TracklistEntry>,
}

/// Owns the record tap for the lifetime of the app and records on demand.
pub struct Recorder {
    tap: Arc<parking::Tap>,
    /// The microphone alone (see `od_engine::Engine::take_voice_tap`).
    voice: Option<Arc<parking::Tap>>,
    sample_rate: u32,
    active: Option<Active>,
    /// Recording gain (linear), applied before writing.
    gain: Arc<std::sync::atomic::AtomicU32>,
}

mod parking {
    use od_engine::RecordTap;
    use std::sync::Mutex;

    /// The tap moves between recording threads; only one uses it at a time.
    pub struct Tap(pub Mutex<RecordTap>);
}

impl Recorder {
    pub fn new(tap: RecordTap, sample_rate: u32) -> Self {
        Self {
            tap: Arc::new(parking::Tap(std::sync::Mutex::new(tap))),
            voice: None,
            sample_rate,
            active: None,
            gain: Arc::new(std::sync::atomic::AtomicU32::new(1.0f32.to_bits())),
        }
    }

    /// Gives the recorder the engine's voice tap, for a separate voice track.
    pub fn with_voice(mut self, voice: RecordTap) -> Self {
        self.voice = Some(Arc::new(parking::Tap(std::sync::Mutex::new(voice))));
        self
    }

    /// Whether a voice track can be recorded.
    pub fn has_voice(&self) -> bool {
        self.voice.is_some()
    }

    /// The voice track being written, while recording one.
    pub fn voice_path(&self) -> Option<&Path> {
        self.active.as_ref().and_then(|a| a.voice.as_deref())
    }

    /// Loudness, true peak and clips of the recording so far.
    pub fn loudness(&self) -> Loudness {
        self.active
            .as_ref()
            .map(|a| *a.loudness.lock().unwrap_or_else(|e| e.into_inner()))
            .unwrap_or_default()
    }

    /// Sets the recording gain in dB (applies immediately, also while recording).
    pub fn set_gain_db(&self, db: f32) {
        let g = 10f32.powf(db.clamp(-24.0, 12.0) / 20.0);
        self.gain.store(g.to_bits(), Ordering::Relaxed);
    }

    /// Peak level (0..=1+) of the most recent audio written.
    pub fn level(&self) -> f32 {
        self.active
            .as_ref()
            .map(|a| f32::from_bits(a.peak.load(Ordering::Relaxed)))
            .unwrap_or(0.0)
    }

    pub fn is_recording(&self) -> bool {
        self.active.is_some()
    }

    /// Seconds recorded so far.
    pub fn elapsed(&self) -> f64 {
        self.active
            .as_ref()
            .map(|a| a.frames.load(Ordering::Relaxed) as f64 / self.sample_rate as f64)
            .unwrap_or(0.0)
    }

    /// Starts writing `dir/<name>.wav` (24-bit stereo). The caller must also
    /// enable the engine's record tap.
    pub fn start(&mut self, dir: &Path, name: &str) -> Result<PathBuf> {
        self.start_with(dir, name, false)
    }

    /// Like [`Recorder::start`]; with `voice` (and a voice tap) the
    /// microphone is written on its own to `dir/<name> voice.wav` as well,
    /// sample for sample beside the mix. The caller enables the engine's
    /// voice tap too.
    pub fn start_with(&mut self, dir: &Path, name: &str, voice: bool) -> Result<PathBuf> {
        if self.active.is_some() {
            return Err(RecordError::Busy);
        }
        std::fs::create_dir_all(dir)?;
        let path = unique_path(dir, name, "wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: self.sample_rate,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec)?;
        let voice_tap = self.voice.clone().filter(|_| voice);
        let voice_path = voice_tap.as_ref().map(|_| {
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            unique_path(dir, &format!("{stem} voice"), "wav")
        });
        let mut voice_writer = match &voice_path {
            Some(p) => Some(hound::WavWriter::create(p, spec)?),
            None => None,
        };
        let stop = Arc::new(AtomicBool::new(false));
        let frames = Arc::new(AtomicU64::new(0));
        let tap = self.tap.clone();
        let (stop2, frames2) = (stop.clone(), frames.clone());
        let peak = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let (peak2, gain) = (peak.clone(), self.gain.clone());
        let loudness = Arc::new(std::sync::Mutex::new(Loudness::default()));
        let loudness2 = loudness.clone();
        let mut meter = LoudnessMeter::new(self.sample_rate);
        // Discard whatever accumulated before the start.
        {
            let mut scratch = Vec::new();
            tap.0
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .drain_into(&mut scratch);
            if let Some(v) = &voice_tap {
                v.0.lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .drain_into(&mut scratch);
            }
        }
        let thread = std::thread::Builder::new()
            .name("od-recorder".into())
            .spawn(move || -> Result<()> {
                let mut buf: Vec<f32> = Vec::with_capacity(48_000);
                let mut vbuf: Vec<f32> = Vec::with_capacity(48_000);
                let mut flushed = std::time::Instant::now();
                loop {
                    let done = stop2.load(Ordering::Relaxed);
                    buf.clear();
                    vbuf.clear();
                    // The voice first: whatever the mix holds then, the
                    // voice holds at least as far (both fill in one block).
                    if let Some(v) = &voice_tap {
                        v.0.lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .drain_into(&mut vbuf);
                    }
                    tap.0
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .drain_into(&mut buf);
                    let g = f32::from_bits(gain.load(Ordering::Relaxed));
                    let mut p = 0.0f32;
                    for s in buf.iter_mut() {
                        *s *= g;
                        p = p.max(s.abs());
                    }
                    meter.process(&buf);
                    for s in &buf {
                        writer.write_sample((s.clamp(-1.0, 1.0) * 8_388_607.0) as i32)?;
                    }
                    if let Some(w) = voice_writer.as_mut() {
                        for s in &vbuf {
                            w.write_sample((s.clamp(-1.0, 1.0) * 8_388_607.0) as i32)?;
                        }
                    }
                    if !buf.is_empty() {
                        peak2.store(p.to_bits(), Ordering::Relaxed);
                        *loudness2.lock().unwrap_or_else(|e| e.into_inner()) = meter.reading();
                    }
                    frames2.fetch_add(buf.len() as u64 / 2, Ordering::Relaxed);
                    if done {
                        break;
                    }
                    if flushed.elapsed() >= FLUSH_EVERY {
                        // Header sizes and buffered samples to the disk: a
                        // crash from here on leaves a playable file.
                        writer.flush()?;
                        if let Some(w) = voice_writer.as_mut() {
                            w.flush()?;
                        }
                        flushed = std::time::Instant::now();
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                writer.finalize()?;
                if let Some(w) = voice_writer {
                    w.finalize()?;
                }
                Ok(())
            })?;
        self.active = Some(Active {
            path: path.clone(),
            voice: voice_path,
            loudness,
            stop,
            frames,
            peak,
            thread,
            tracklist: Vec::new(),
        });
        Ok(path)
    }

    /// The file being written, while recording.
    pub fn path(&self) -> Option<&Path> {
        self.active.as_ref().map(|a| a.path.as_path())
    }

    /// The tracklist so far, while recording.
    pub fn tracklist(&self) -> &[TracklistEntry] {
        self.active.as_ref().map_or(&[], |a| a.tracklist.as_slice())
    }

    /// Records that `track` became the audible track now.
    pub fn mark_track(&mut self, track: Option<TrackId>, title: &str, artist: &str) {
        let elapsed = self.elapsed();
        self.mark_track_at(elapsed, track, title, artist);
    }

    /// Records that `track` became the audible track at `elapsed` seconds
    /// (when it was found to stay there later).
    pub fn mark_track_at(&mut self, elapsed: f64, track: Option<TrackId>, title: &str, artist: &str) {
        let Some(a) = self.active.as_mut() else {
            return;
        };
        if a.tracklist
            .last()
            .is_some_and(|l| l.track == track && track.is_some())
        {
            return;
        }
        a.tracklist.push(TracklistEntry {
            offset: elapsed,
            track,
            title: title.into(),
            artist: artist.into(),
            marker: false,
        });
    }

    /// Sets a chapter marker named `title` at the current time; returns
    /// its number among the markers (from 1).
    pub fn mark_chapter(&mut self, title: &str) -> Option<usize> {
        let elapsed = self.elapsed();
        let a = self.active.as_mut()?;
        a.tracklist.push(TracklistEntry {
            offset: elapsed,
            track: None,
            title: title.into(),
            artist: String::new(),
            marker: true,
        });
        Some(a.tracklist.iter().filter(|e| e.marker).count())
    }

    /// Stops the recording and writes the tracklist files.
    pub fn stop(&mut self) -> Result<Recording> {
        let a = self.active.take().ok_or(RecordError::Idle)?;
        a.stop.store(true, Ordering::Relaxed);
        a.thread
            .join()
            .map_err(|_| std::io::Error::other("recorder thread panicked"))??;
        let duration = a.frames.load(Ordering::Relaxed) as f64 / self.sample_rate as f64;
        let tracklist: Vec<TracklistEntry> = a
            .tracklist
            .into_iter()
            .filter(|e| e.track.is_some() || e.marker)
            .collect();
        let (cue_path, text_path) = export_tracklist(&a.path, &tracklist)?;
        if let Err(e) = write_chapters(&a.path, &chapters_of(&tracklist)) {
            tracing::warn!(error = %e, "chapters not written into the recording");
        }
        let loudness = *a.loudness.lock().unwrap_or_else(|e| e.into_inner());
        let rec = Recording {
            path: a.path,
            duration,
            tracklist,
            cue_path,
            text_path,
            voice: a.voice,
            loudness,
        };
        save_info(
            &rec.path,
            &RecordingInfo {
                tracklist: rec.tracklist.clone(),
                voice: rec.voice.clone(),
                loudness,
                duration,
            },
        )?;
        Ok(rec)
    }
}

fn unique_path(dir: &Path, name: &str, ext: &str) -> PathBuf {
    let mut p = dir.join(format!("{name}.{ext}"));
    let mut n = 2;
    while p.exists() {
        p = dir.join(format!("{name} ({n}).{ext}"));
        n += 1;
    }
    p
}

fn cue_time(secs: f64) -> String {
    // CUE frames are 1/75 s.
    let total_frames = (secs.max(0.0) * 75.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02}",
        total_frames / 75 / 60,
        total_frames / 75 % 60,
        total_frames % 75
    )
}

fn escape(s: &str) -> String {
    s.replace('"', "'")
}

/// A CUE sheet for the recording.
pub fn cue_sheet(audio: &Path, tracklist: &[TracklistEntry]) -> String {
    let file = audio
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut s = format!(
        "PERFORMER \"OpenDJDecks\"\nTITLE \"{}\"\nFILE \"{}\" WAVE\n",
        escape(&file),
        escape(&file)
    );
    for (i, e) in tracklist.iter().enumerate() {
        s.push_str(&format!(
            "  TRACK {:02} AUDIO\n    TITLE \"{}\"\n    PERFORMER \"{}\"\n    INDEX 01 {}\n",
            i + 1,
            escape(&e.title),
            escape(&e.artist),
            cue_time(if i == 0 { 0.0 } else { e.offset })
        ));
    }
    s
}

/// Plain tracklist in the "MM:SS Artist - Title" form used by Mixcloud and
/// SoundCloud descriptions.
pub fn text_tracklist(tracklist: &[TracklistEntry]) -> String {
    tracklist
        .iter()
        .map(|e| {
            let t = e.offset.max(0.0).round() as u64;
            let stamp = if t >= 3600 {
                format!("{}:{:02}:{:02}", t / 3600, t / 60 % 60, t % 60)
            } else {
                format!("{:02}:{:02}", t / 60, t % 60)
            };
            if e.artist.is_empty() {
                format!("{stamp} {}", e.title)
            } else {
                format!("{stamp} {} - {}", e.artist, e.title)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// Splits a recording into one WAV per tracklist entry next to it, in a
/// folder named after the recording. Returns the written files.
pub fn split(recording: &Recording) -> Result<Vec<PathBuf>> {
    let mut reader = hound::WavReader::open(&recording.path)?;
    let spec = reader.spec();
    let rate = spec.sample_rate as f64;
    let dir = recording.path.with_extension("");
    std::fs::create_dir_all(&dir)?;
    let samples: Vec<i32> = reader
        .samples::<i32>()
        .collect::<std::result::Result<_, _>>()?;
    let frames = samples.len() / spec.channels as usize;
    let mut out = Vec::new();
    for (i, e) in recording.tracklist.iter().enumerate() {
        let start = if i == 0 {
            0
        } else {
            (e.offset * rate) as usize
        };
        let end = recording
            .tracklist
            .get(i + 1)
            .map(|n| (n.offset * rate) as usize)
            .unwrap_or(frames)
            .min(frames);
        if end <= start {
            continue;
        }
        let name: String = format!("{:02} {} - {}", i + 1, e.artist, e.title)
            .chars()
            .map(|c| {
                if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        let path = dir.join(format!("{name}.wav"));
        let mut w = hound::WavWriter::create(&path, spec)?;
        for s in &samples[start * spec.channels as usize..end * spec.channels as usize] {
            w.write_sample(*s)?;
        }
        w.finalize()?;
        out.push(path);
    }
    Ok(out)
}

/// Makes a WAV left by a killed process whole: the RIFF and data sizes
/// are set to what is on the disk, and a partly written last frame is cut
/// off. Returns the playable duration in seconds. A file that is already
/// whole is left as it is.
pub fn repair_wav(path: &Path) -> Result<f64> {
    use std::io::{Read, Seek, SeekFrom, Write};
    let mut f = std::fs::OpenOptions::new().read(true).write(true).open(path)?;
    let len = f.metadata()?.len();
    let mut head = [0u8; 12];
    f.read_exact(&mut head)?;
    if &head[0..4] != b"RIFF" || &head[8..12] != b"WAVE" {
        return Err(RecordError::Encode("not a WAV file".into()));
    }
    // Walk the chunks to "fmt " (block size) and "data".
    let mut pos = 12u64;
    let mut block_align = 0u64;
    let mut byte_rate = 0u64;
    let data_at = loop {
        if pos + 8 > len {
            return Err(RecordError::Encode("no data chunk".into()));
        }
        f.seek(SeekFrom::Start(pos))?;
        let mut ch = [0u8; 8];
        f.read_exact(&mut ch)?;
        let size = u32::from_le_bytes([ch[4], ch[5], ch[6], ch[7]]) as u64;
        if &ch[0..4] == b"fmt " {
            let mut fmt = [0u8; 16];
            f.read_exact(&mut fmt)?;
            byte_rate = u32::from_le_bytes([fmt[8], fmt[9], fmt[10], fmt[11]]) as u64;
            block_align = u16::from_le_bytes([fmt[12], fmt[13]]) as u64;
        }
        if &ch[0..4] == b"data" {
            break pos;
        }
        pos += 8 + size + (size & 1);
    };
    if block_align == 0 || byte_rate == 0 {
        return Err(RecordError::Encode("no format chunk".into()));
    }
    let data_start = data_at + 8;
    // Chapters after the audio ([27]): the file was finished, leave it.
    {
        f.seek(SeekFrom::Start(data_at + 4))?;
        let mut sz = [0u8; 4];
        f.read_exact(&mut sz)?;
        let declared = u32::from_le_bytes(sz) as u64;
        let after = data_start + declared + (declared & 1);
        if declared > 0 && after + 8 <= len {
            let mut id = [0u8; 4];
            f.seek(SeekFrom::Start(after))?;
            f.read_exact(&mut id)?;
            if &id == b"cue " || &id == b"LIST" {
                return Ok(declared as f64 / byte_rate as f64);
            }
        }
    }
    let data_len = (len.saturating_sub(data_start) / block_align) * block_align;
    let end = data_start + data_len;
    if end < len {
        f.set_len(end)?;
    }
    let riff = u32::try_from(end - 8).unwrap_or(u32::MAX);
    let data = u32::try_from(data_len).unwrap_or(u32::MAX);
    f.seek(SeekFrom::Start(4))?;
    f.write_all(&riff.to_le_bytes())?;
    f.seek(SeekFrom::Start(data_at + 4))?;
    f.write_all(&data.to_le_bytes())?;
    f.sync_all()?;
    Ok(data_len as f64 / byte_rate as f64)
}

/// Re-export for callers that only need the tap type.
pub type Tap = RecordTap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cue_and_text_format() {
        let tl = vec![
            TracklistEntry {
                offset: 0.4,
                track: Some(TrackId(1)),
                title: "One".into(),
                artist: "A".into(),
                marker: false,
            },
            TracklistEntry {
                offset: 3725.5,
                track: Some(TrackId(2)),
                title: "Two \"x\"".into(),
                artist: "B".into(),
                marker: false,
            },
        ];
        let cue = cue_sheet(Path::new("/x/mix.wav"), &tl);
        assert!(cue.contains("INDEX 01 00:00:00"));
        assert!(cue.contains("INDEX 01 62:05:38"));
        assert!(cue.contains("TITLE \"Two 'x'\""));
        assert_eq!(
            text_tracklist(&tl),
            "00:00 A - One\n1:02:06 B - Two \"x\"\n"
        );
    }

    #[test]
    fn records_engine_output_and_splits() {
        use od_engine::{Command, Engine, EngineConfig, Outputs};
        let (mut engine, mut proc) = Engine::new(EngineConfig {
            sample_rate: 48_000,
            max_block: 256,
            decks: 2,
            ..Default::default()
        });
        let tap = engine.take_record_tap().unwrap();
        engine.send(Command::SetRecordTap(true)).unwrap();
        let mut rec = Recorder::new(tap, 48_000);
        let dir = tempfile::tempdir().unwrap();
        rec.start(dir.path(), "mix").unwrap();
        rec.mark_track(Some(TrackId(1)), "One", "A");
        // Wait until the writer thread has caught up with `frames` rendered
        // frames (CI machines can be slow; never rely on fixed sleeps).
        let wait_for = |rec: &Recorder, frames: usize| {
            let deadline = std::time::Instant::now() + Duration::from_secs(20);
            while rec.elapsed() * 48_000.0 < frames as f64 && std::time::Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        let mut master = vec![0.0; 512];
        for i in 0..400 {
            proc.process(
                256,
                &mut Outputs {
                    master: &mut master,
                    headphones: None,
                    booth: None,
                    decks: None,
                },
            );
            if i % 50 == 49 {
                wait_for(&rec, (i + 1) * 256);
            }
            if i == 199 {
                rec.mark_track(Some(TrackId(2)), "Two", "B");
            }
        }
        wait_for(&rec, 400 * 256);
        let r = rec.stop().unwrap();
        assert!(r.duration > 1.0, "duration {}", r.duration);
        assert_eq!(r.tracklist.len(), 2);
        assert!(r.cue_path.exists() && r.text_path.exists());
        let parts = split(&r).unwrap();
        assert_eq!(parts.len(), 2);
        assert!(matches!(rec.stop(), Err(RecordError::Idle)));
    }

    /// [27] A voice track beside the mix: the microphone alone, as long as
    /// the mix (sample-aligned), chapter markers in the file, the loudness
    /// and the info file.
    #[test]
    fn records_a_voice_track_markers_and_loudness() {
        use od_engine::{Command, Engine, EngineConfig, InputCommand, Outputs};
        let (mut engine, mut proc) = Engine::new(EngineConfig {
            sample_rate: 48_000,
            max_block: 256,
            decks: 2,
            ..Default::default()
        });
        let tap = engine.take_record_tap().unwrap();
        let voice = engine.take_voice_tap().unwrap();
        let (mut mic, mic_rx) = rtrb::RingBuffer::<f32>::new(48_000 * 4);
        engine.send(Command::Input(InputCommand::Attach(mic_rx, 1))).unwrap();
        engine.send(Command::Input(InputCommand::SetOn(true))).unwrap();
        engine.send(Command::SetRecordTap(true)).unwrap();
        engine.send(Command::SetVoiceTap(true)).unwrap();
        let mut rec = Recorder::new(tap, 48_000).with_voice(voice);
        assert!(rec.has_voice());
        let dir = tempfile::tempdir().unwrap();
        rec.start_with(dir.path(), "mix", true).unwrap();
        assert!(rec.voice_path().unwrap().ends_with("mix voice.wav"));
        rec.mark_track(Some(TrackId(1)), "One", "A");
        let mut master = vec![0.0; 512];
        let mut phase = 0.0f32;
        for i in 0..800 {
            for _ in 0..256 {
                phase += 440.0 * std::f32::consts::TAU / 48_000.0;
                let _ = mic.push(phase.sin() * 0.3);
            }
            proc.process(256, &mut Outputs { master: &mut master, headphones: None, booth: None, decks: None });
            if i == 400 {
                let deadline = std::time::Instant::now() + Duration::from_secs(20);
                while rec.elapsed() < 1.0 && std::time::Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(5));
                }
                assert_eq!(rec.mark_chapter("Shout-out"), Some(1));
            }
            if i % 50 == 49 {
                let deadline = std::time::Instant::now() + Duration::from_secs(20);
                while rec.elapsed() * 48_000.0 < ((i + 1) * 256) as f64 && std::time::Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        }
        let live = rec.loudness();
        assert!(live.integrated.is_some(), "{live:?}");
        let r = rec.stop().unwrap();
        let voice = r.voice.clone().unwrap();
        let mix_frames = hound::WavReader::open(&r.path).unwrap().duration();
        let voice_frames = hound::WavReader::open(&voice).unwrap().duration();
        assert_eq!(mix_frames, voice_frames, "aligned");
        // The voice holds the mic alone: a 440 Hz tone at about −10 dBFS.
        let peak = hound::WavReader::open(&voice)
            .unwrap()
            .samples::<i32>()
            .map(|s| s.unwrap().abs())
            .max()
            .unwrap() as f32
            / 8_388_607.0;
        assert!(peak > 0.1 && peak < 0.35, "{peak}");
        assert_eq!(r.tracklist.len(), 2);
        assert!(r.tracklist[1].marker && r.tracklist[1].track.is_none());
        let chapters = read_wav_chapters(&r.path).unwrap();
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[1].title, "Shout-out");
        assert!(r.loudness.integrated.unwrap() < -5.0);
        assert!(r.loudness.true_peak.is_some());
        assert_eq!(r.loudness.clips, 0);
        let info = load_info(&r.path).unwrap();
        assert_eq!(info.tracklist, r.tracklist);
        assert_eq!(info.voice.as_deref(), Some(voice.as_path()));
        assert!(std::fs::read_to_string(&r.text_path).unwrap().contains("Shout-out"));
    }

    fn sine_wav(path: &std::path::Path, secs: f32) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(path, spec).unwrap();
        for i in 0..(48_000.0 * secs) as usize {
            let v = ((i as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin()
                * 0.5
                * 8_388_607.0) as i32;
            w.write_sample(v).unwrap();
            w.write_sample(-v).unwrap();
        }
        w.finalize().unwrap();
    }

    #[test]
    fn flac_is_lossless_and_mp3_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("mix.wav");
        sine_wav(&wav, 1.3);
        let flac = dir.path().join("mix.flac");
        wav_to_flac(&wav, &flac).unwrap();
        let bytes = std::fs::read(&flac).unwrap();
        assert_eq!(&bytes[..4], b"fLaC");
        // Decode with an independent decoder (symphonia via od-decode) and
        // compare every sample: FLAC must be bit-exact.
        let decoded = od_decode::decode(&flac, od_decode::Layout::Stereo).unwrap();
        let original: Vec<i32> = hound::WavReader::open(&wav)
            .unwrap()
            .samples::<i32>()
            .map(|s| s.unwrap())
            .collect();
        assert_eq!(decoded.samples.len(), original.len());
        for (d, o) in decoded.samples.iter().zip(&original) {
            assert!((d * 8_388_608.0 - *o as f32).abs() < 0.5, "{d} vs {o}");
        }
        assert!(bytes.len() < std::fs::metadata(&wav).unwrap().len() as usize);

        let mp3 = dir.path().join("mix.mp3");
        wav_to_mp3(&wav, &mp3).unwrap();
        let m = std::fs::read(&mp3).unwrap();
        // 1.3 s at 320 kbit/s is about 52 kB; an MPEG frame sync starts it.
        assert!(m.len() > 40_000 && m.len() < 70_000, "{}", m.len());
        assert!(m.windows(2).any(|w| w[0] == 0xFF && w[1] & 0xE0 == 0xE0));
    }

    #[test]
    fn silence_detector_starts_and_stops() {
        let mut d = SilenceDetector::new(-50.0, 1.0, 10.0);
        assert_eq!(d.update(-20.0, 0.5, false), None);
        assert_eq!(d.update(-20.0, 0.6, false), Some(AutoAction::Start));
        assert_eq!(d.update(-70.0, 9.0, true), None);
        assert_eq!(d.update(-20.0, 0.1, true), None);
        assert_eq!(d.update(-70.0, 9.9, true), None);
        assert_eq!(d.update(-70.0, 0.2, true), Some(AutoAction::Stop));
    }
}
