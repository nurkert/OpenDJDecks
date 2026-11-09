//! Maximum compatibility for Club player USB sticks: tracks in formats older
//! club players cannot read are written to the stick as 16-bit WAV. The
//! audio is decoded with `od-decode`; the source file is never modified.

use crate::sync::part_file;
use od_core::AudioFormat;
use od_decode::{DecodeError, Layout};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Highest sample rate written. Anything above is reduced (88.2 kHz to
/// 44.1 kHz, the rest to 48 kHz); lower rates are kept as they are.
const MAX_SAMPLE_RATE: u32 = 48_000;

/// Whether `format` is written as WAV when the option is on.
pub(crate) fn needs_wav(format: Option<AudioFormat>) -> bool {
    matches!(
        format,
        Some(AudioFormat::Flac | AudioFormat::Aac | AudioFormat::Alac)
    )
}

/// The sample rate the WAV copy gets for a source at `rate`.
fn output_rate(rate: u32) -> u32 {
    match rate {
        0..=MAX_SAMPLE_RATE => rate,
        r if r % 44_100 == 0 => 44_100,
        _ => MAX_SAMPLE_RATE,
    }
}

/// Upper estimate of the size of a WAV copy (for the space check).
pub(crate) fn estimated_size(duration: f64, sample_rate: u32) -> u64 {
    (duration.max(0.0) * output_rate(sample_rate) as f64 * 4.0) as u64 + 44
}

#[derive(Debug)]
pub(crate) enum ConvertError {
    Cancelled,
    /// The stick could not be written.
    Io(std::io::Error),
    /// The source could not be decoded.
    Decode(DecodeError),
}

/// What was written for a WAV copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Written {
    pub size: u64,
    pub sample_rate: u32,
}

/// Decodes `src` and writes it to `dst` as stereo 16-bit WAV (via a part
/// file, so an interrupted sync leaves no broken track behind).
pub(crate) fn write_wav(
    src: &Path,
    dst: &Path,
    cancel: &AtomicBool,
) -> Result<Written, ConvertError> {
    let audio = od_decode::decode_with(src, Layout::Stereo, |_| !cancel.load(Ordering::Relaxed))
        .map_err(|e| {
            if cancel.load(Ordering::Relaxed) {
                ConvertError::Cancelled
            } else {
                ConvertError::Decode(e)
            }
        })?;
    let sample_rate = output_rate(audio.sample_rate);
    let samples = od_decode::resample(&audio.samples, 2, audio.sample_rate, sample_rate);
    if cancel.load(Ordering::Relaxed) {
        return Err(ConvertError::Cancelled);
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(ConvertError::Io)?;
    }
    let tmp = part_file(dst);
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let result = (|| -> Result<u64, hound::Error> {
        let mut w = hound::WavWriter::create(&tmp, spec)?;
        for s in &samples {
            w.write_sample((s.clamp(-1.0, 1.0) * 32767.0).round() as i16)?;
        }
        w.finalize()?;
        Ok(std::fs::metadata(&tmp)?.len())
    })();
    let size = match result {
        Ok(size) => size,
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            return Err(ConvertError::Io(match e {
                hound::Error::IoError(e) => e,
                e => std::io::Error::other(e.to_string()),
            }));
        }
    };
    std::fs::rename(&tmp, dst).map_err(ConvertError::Io)?;
    Ok(Written { size, sample_rate })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(path: &Path, rate: u32, frames: usize) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(path, spec).unwrap();
        for i in 0..frames {
            let v = (i as f32 / rate as f32 * 440.0 * std::f32::consts::TAU).sin() * 0.5;
            w.write_sample((v * 32767.0) as i16).unwrap();
            w.write_sample((-v * 32767.0) as i16).unwrap();
        }
        w.finalize().unwrap();
    }

    #[test]
    fn writes_a_16_bit_wav_and_leaves_the_source_alone() {
        let dir = tempfile::tempdir().unwrap();
        // The decoder goes by content; a WAV stands in for a lossless source.
        let src = dir.path().join("in.flac");
        sine(&src, 44_100, 44_100);
        let before = std::fs::read(&src).unwrap();
        let dst = dir.path().join("out/in.wav");
        let w = write_wav(&src, &dst, &AtomicBool::new(false)).unwrap();
        assert_eq!(std::fs::read(&src).unwrap(), before, "source untouched");
        let r = hound::WavReader::open(&dst).unwrap();
        assert_eq!(r.spec().bits_per_sample, 16);
        assert_eq!(r.spec().sample_rate, 44_100);
        assert_eq!(w.size, std::fs::metadata(&dst).unwrap().len());
        assert_eq!(r.duration(), 44_100);
        assert!(!dir.path().join("out/.in.wav.part").exists());
    }

    #[test]
    fn high_sample_rates_are_reduced_and_cancel_is_honoured() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("in.flac");
        sine(&src, 96_000, 9_600);
        let dst = dir.path().join("out.wav");
        let w = write_wav(&src, &dst, &AtomicBool::new(false)).unwrap();
        assert_eq!(w.sample_rate, 48_000);
        assert_eq!(output_rate(88_200), 44_100);
        assert_eq!(output_rate(44_100), 44_100);
        assert!(matches!(
            write_wav(&src, &dst, &AtomicBool::new(true)),
            Err(ConvertError::Cancelled)
        ));
    }
}
