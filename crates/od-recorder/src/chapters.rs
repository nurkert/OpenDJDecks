//! Chapter markers inside the recording ([27]): one per tracklist entry,
//! so a player or an editor shows where each track (or a marker set while
//! recording) starts.
//!
//! - WAV: a `cue ` chunk with one cue point per chapter and a `LIST` /
//!   `adtl` chunk with a `labl` name for each, after the audio.
//! - MP3: an ID3v2.4 tag in front with a `CTOC` table of contents and a
//!   `CHAP` frame per chapter (its name as `TIT2`).
//! - FLAC: `CHAPTERnnn` / `CHAPTERnnnNAME` Vorbis comments.
//!
//! Writing again replaces what an earlier write put there.

use crate::{RecordError, Result, TracklistEntry};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// One chapter: its start (seconds from the start of the file) and name.
#[derive(Clone, Debug, PartialEq)]
pub struct Chapter {
    pub start: f64,
    pub title: String,
}

/// The chapters of a tracklist ("Artist - Title", or the title alone).
pub fn chapters_of(tracklist: &[TracklistEntry]) -> Vec<Chapter> {
    tracklist
        .iter()
        .enumerate()
        .map(|(i, e)| Chapter {
            start: if i == 0 { 0.0 } else { e.offset.max(0.0) },
            title: if e.artist.trim().is_empty() {
                e.title.clone()
            } else {
                format!("{} - {}", e.artist, e.title)
            },
        })
        .collect()
}

/// Writes `chapters` into the recording at `path` (WAV, MP3 or FLAC by its
/// extension; other files are left alone).
pub fn write_chapters(path: &Path, chapters: &[Chapter]) -> Result<()> {
    match path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .as_deref()
    {
        Some("wav") => write_wav_chapters(path, chapters),
        Some("mp3") => write_mp3_chapters(path, chapters),
        Some("flac") => write_flac_chapters(path, chapters),
        _ => Ok(()),
    }
}

fn read_u32(f: &mut impl Read) -> std::io::Result<u32> {
    let mut b = [0u8; 4];
    f.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

/// RIFF chunks of a WAV file: (id, offset of the chunk header, body size).
pub(crate) fn wav_chunks(f: &mut File) -> Result<Vec<([u8; 4], u64, u64)>> {
    let len = f.metadata()?.len();
    f.seek(SeekFrom::Start(0))?;
    let mut head = [0u8; 12];
    f.read_exact(&mut head)?;
    if &head[0..4] != b"RIFF" || &head[8..12] != b"WAVE" {
        return Err(RecordError::Encode("not a WAV file".into()));
    }
    let mut out = Vec::new();
    let mut pos = 12u64;
    while pos + 8 <= len {
        f.seek(SeekFrom::Start(pos))?;
        let mut id = [0u8; 4];
        f.read_exact(&mut id)?;
        let size = read_u32(f)? as u64;
        out.push((id, pos, size));
        pos += 8 + size + (size & 1);
    }
    Ok(out)
}

/// The frame rate and frame count of a WAV file.
fn wav_rate(f: &mut File, chunks: &[([u8; 4], u64, u64)]) -> Result<u32> {
    let (_, at, _) = chunks
        .iter()
        .find(|c| &c.0 == b"fmt ")
        .ok_or_else(|| RecordError::Encode("no format chunk".into()))?;
    f.seek(SeekFrom::Start(at + 8 + 4))?;
    Ok(read_u32(f)?)
}

fn write_wav_chapters(path: &Path, chapters: &[Chapter]) -> Result<()> {
    let mut f = std::fs::OpenOptions::new().read(true).write(true).open(path)?;
    let chunks = wav_chunks(&mut f)?;
    let rate = wav_rate(&mut f, &chunks)? as f64;
    let data_end = chunks
        .iter()
        .find(|c| &c.0 == b"data")
        .map(|(_, at, size)| at + 8 + size + (size & 1))
        .ok_or_else(|| RecordError::Encode("no data chunk".into()))?;
    // Our chunks follow the audio; drop an earlier write of them.
    let ours = chunks
        .iter()
        .filter(|(id, at, _)| *at >= data_end && (id == b"cue " || id == b"LIST"))
        .map(|c| c.1)
        .min();
    let end = ours.unwrap_or_else(|| f.metadata().map(|m| m.len()).unwrap_or(data_end));
    f.set_len(end)?;
    let mut body = Vec::new();
    if !chapters.is_empty() {
        // cue: count, then 24 bytes per point.
        let mut cue = Vec::new();
        cue.extend((chapters.len() as u32).to_le_bytes());
        for (i, c) in chapters.iter().enumerate() {
            let frame = (c.start.max(0.0) * rate).round() as u32;
            cue.extend((i as u32 + 1).to_le_bytes());
            cue.extend(frame.to_le_bytes());
            cue.extend(b"data");
            cue.extend(0u32.to_le_bytes());
            cue.extend(0u32.to_le_bytes());
            cue.extend(frame.to_le_bytes());
        }
        body.extend(b"cue ");
        body.extend((cue.len() as u32).to_le_bytes());
        body.extend(cue);
        // LIST adtl with a labl per point.
        let mut list = Vec::new();
        list.extend(b"adtl");
        for (i, c) in chapters.iter().enumerate() {
            let mut text = c.title.as_bytes().to_vec();
            text.push(0);
            list.extend(b"labl");
            list.extend((text.len() as u32 + 4).to_le_bytes());
            list.extend((i as u32 + 1).to_le_bytes());
            list.extend(&text);
            if (text.len() + 4) % 2 == 1 {
                list.push(0);
            }
        }
        body.extend(b"LIST");
        body.extend((list.len() as u32).to_le_bytes());
        body.extend(list);
    }
    f.seek(SeekFrom::Start(end))?;
    f.write_all(&body)?;
    let total = end + body.len() as u64;
    f.seek(SeekFrom::Start(4))?;
    f.write_all(&u32::try_from(total - 8).unwrap_or(u32::MAX).to_le_bytes())?;
    f.sync_all()?;
    Ok(())
}

/// The chapters a WAV file carries (cue points with their labels).
pub fn read_wav_chapters(path: &Path) -> Result<Vec<Chapter>> {
    let mut f = File::open(path)?;
    let chunks = wav_chunks(&mut f)?;
    let rate = wav_rate(&mut f, &chunks)? as f64;
    let mut points: Vec<(u32, u32)> = Vec::new();
    let mut names: std::collections::HashMap<u32, String> = Default::default();
    for (id, at, size) in &chunks {
        f.seek(SeekFrom::Start(at + 8))?;
        let mut body = vec![0u8; *size as usize];
        f.read_exact(&mut body)?;
        if id == b"cue " && body.len() >= 4 {
            let n = u32::from_le_bytes(body[0..4].try_into().unwrap()) as usize;
            for k in 0..n {
                let p = 4 + k * 24;
                if p + 24 > body.len() {
                    break;
                }
                let cid = u32::from_le_bytes(body[p..p + 4].try_into().unwrap());
                let frame = u32::from_le_bytes(body[p + 20..p + 24].try_into().unwrap());
                points.push((cid, frame));
            }
        }
        if id == b"LIST" && body.starts_with(b"adtl") {
            let mut p = 4;
            while p + 8 <= body.len() {
                let sid = &body[p..p + 4];
                let sz = u32::from_le_bytes(body[p + 4..p + 8].try_into().unwrap()) as usize;
                let b = &body[p + 8..(p + 8 + sz).min(body.len())];
                if sid == b"labl" && b.len() >= 4 {
                    let cid = u32::from_le_bytes(b[0..4].try_into().unwrap());
                    let text = String::from_utf8_lossy(&b[4..]).trim_end_matches('\0').to_string();
                    names.insert(cid, text);
                }
                p += 8 + sz + (sz & 1);
            }
        }
    }
    Ok(points
        .into_iter()
        .map(|(id, frame)| Chapter {
            start: frame as f64 / rate,
            title: names.remove(&id).unwrap_or_default(),
        })
        .collect())
}

fn syncsafe(n: u32) -> [u8; 4] {
    [
        ((n >> 21) & 0x7F) as u8,
        ((n >> 14) & 0x7F) as u8,
        ((n >> 7) & 0x7F) as u8,
        (n & 0x7F) as u8,
    ]
}

fn unsyncsafe(b: &[u8]) -> u32 {
    b.iter().take(4).fold(0, |n, v| (n << 7) | (*v as u32 & 0x7F))
}

fn id3_frame(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut f = Vec::with_capacity(body.len() + 10);
    f.extend(id);
    f.extend(syncsafe(body.len() as u32));
    f.extend([0, 0]);
    f.extend(body);
    f
}

/// An ID3v2.4 tag with a table of contents and one chapter frame each.
fn id3_chapters(chapters: &[Chapter], duration: f64) -> Vec<u8> {
    let ms = |s: f64| (s.max(0.0) * 1000.0).round() as u32;
    let mut frames = Vec::new();
    let mut toc = b"toc\0".to_vec();
    toc.push(0x03); // top level, ordered
    toc.push(chapters.len().min(255) as u8);
    for i in 0..chapters.len().min(255) {
        toc.extend(format!("ch{i}").as_bytes());
        toc.push(0);
    }
    frames.extend(id3_frame(b"CTOC", &toc));
    for (i, c) in chapters.iter().take(255).enumerate() {
        let end = chapters.get(i + 1).map(|n| n.start).unwrap_or(duration).max(c.start);
        let mut body = format!("ch{i}").into_bytes();
        body.push(0);
        body.extend(ms(c.start).to_be_bytes());
        body.extend(ms(end).to_be_bytes());
        body.extend(u32::MAX.to_be_bytes());
        body.extend(u32::MAX.to_be_bytes());
        let mut title = vec![3u8]; // UTF-8
        title.extend(c.title.as_bytes());
        body.extend(id3_frame(b"TIT2", &title));
        frames.extend(id3_frame(b"CHAP", &body));
    }
    let mut tag = b"ID3".to_vec();
    tag.extend([4, 0, 0]);
    tag.extend(syncsafe(frames.len() as u32));
    tag.extend(frames);
    tag
}

/// Rewrites `path` through a temporary file next to it: `head` first, then
/// the rest of the original from `from`.
fn rewrite(path: &Path, head: &[u8], from: u64) -> Result<()> {
    let tmp: PathBuf = path.with_extension("chapters.tmp");
    {
        let mut src = BufReader::new(File::open(path)?);
        src.seek(SeekFrom::Start(from))?;
        let mut out = BufWriter::new(File::create(&tmp)?);
        out.write_all(head)?;
        std::io::copy(&mut src, &mut out)?;
        out.flush()?;
        out.get_ref().sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn write_mp3_chapters(path: &Path, chapters: &[Chapter]) -> Result<()> {
    let mut f = File::open(path)?;
    let mut head = [0u8; 10];
    let skip = if f.read_exact(&mut head).is_ok() && &head[0..3] == b"ID3" {
        10 + unsyncsafe(&head[6..10]) as u64
    } else {
        0
    };
    // The length for the last chapter's end: the bit rate of a 320 kbit/s
    // recording is constant.
    let audio = f.metadata()?.len().saturating_sub(skip);
    let duration = audio as f64 * 8.0 / 320_000.0;
    drop(f);
    let tag = if chapters.is_empty() {
        Vec::new()
    } else {
        id3_chapters(chapters, duration)
    };
    rewrite(path, &tag, skip)
}

/// The chapters of an MP3 file's ID3v2 tag.
pub fn read_mp3_chapters(path: &Path) -> Result<Vec<Chapter>> {
    let mut f = File::open(path)?;
    let mut head = [0u8; 10];
    f.read_exact(&mut head)?;
    if &head[0..3] != b"ID3" {
        return Ok(Vec::new());
    }
    let mut tag = vec![0u8; unsyncsafe(&head[6..10]) as usize];
    f.read_exact(&mut tag)?;
    let mut out = Vec::new();
    let mut p = 0;
    while p + 10 <= tag.len() {
        let id = &tag[p..p + 4];
        let size = unsyncsafe(&tag[p + 4..p + 8]) as usize;
        let body = &tag[p + 10..(p + 10 + size).min(tag.len())];
        if id == b"CHAP" {
            let z = body.iter().position(|b| *b == 0).unwrap_or(0);
            let start = u32::from_be_bytes(body[z + 1..z + 5].try_into().unwrap());
            let sub = &body[z + 17..];
            let title = if sub.len() > 11 && &sub[0..4] == b"TIT2" {
                let s = unsyncsafe(&sub[4..8]) as usize;
                String::from_utf8_lossy(&sub[11..(10 + s).min(sub.len())]).into_owned()
            } else {
                String::new()
            };
            out.push(Chapter { start: start as f64 / 1000.0, title });
        }
        if id == [0, 0, 0, 0] {
            break;
        }
        p += 10 + size;
    }
    Ok(out)
}

fn chapter_stamp(secs: f64) -> String {
    let ms = (secs.max(0.0) * 1000.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

fn write_flac_chapters(path: &Path, chapters: &[Chapter]) -> Result<()> {
    let mut f = File::open(path)?;
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic)?;
    if &magic != b"fLaC" {
        return Err(RecordError::Encode("not a FLAC file".into()));
    }
    // Read every metadata block; keep all but an old Vorbis comment.
    let mut blocks: Vec<(u8, Vec<u8>)> = Vec::new();
    loop {
        let mut h = [0u8; 4];
        f.read_exact(&mut h)?;
        let last = h[0] & 0x80 != 0;
        let kind = h[0] & 0x7F;
        let len = u32::from_be_bytes([0, h[1], h[2], h[3]]) as usize;
        let mut body = vec![0u8; len];
        f.read_exact(&mut body)?;
        if kind != 4 {
            blocks.push((kind, body));
        }
        if last {
            break;
        }
    }
    let audio_at = f.stream_position()?;
    drop(f);
    let mut comments: Vec<String> = Vec::new();
    for (i, c) in chapters.iter().enumerate() {
        comments.push(format!("CHAPTER{:03}={}", i + 1, chapter_stamp(c.start)));
        comments.push(format!("CHAPTER{:03}NAME={}", i + 1, c.title));
    }
    let mut vc = Vec::new();
    let vendor = b"OpenDJDecks";
    vc.extend((vendor.len() as u32).to_le_bytes());
    vc.extend(vendor);
    vc.extend((comments.len() as u32).to_le_bytes());
    for c in &comments {
        vc.extend((c.len() as u32).to_le_bytes());
        vc.extend(c.as_bytes());
    }
    // STREAMINFO stays first; the comment goes right after it.
    let at = blocks.iter().position(|b| b.0 == 0).map_or(0, |i| i + 1);
    blocks.insert(at, (4, vc));
    let mut head = b"fLaC".to_vec();
    let n = blocks.len();
    for (i, (kind, body)) in blocks.into_iter().enumerate() {
        let flag = if i + 1 == n { 0x80 } else { 0 };
        head.push(flag | kind);
        let l = (body.len() as u32).to_be_bytes();
        head.extend(&l[1..]);
        head.extend(body);
    }
    rewrite(path, &head, audio_at)
}

/// The chapters of a FLAC file's Vorbis comments.
pub fn read_flac_chapters(path: &Path) -> Result<Vec<Chapter>> {
    let mut f = File::open(path)?;
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic)?;
    let mut map: std::collections::BTreeMap<u32, Chapter> = Default::default();
    loop {
        let mut h = [0u8; 4];
        f.read_exact(&mut h)?;
        let len = u32::from_be_bytes([0, h[1], h[2], h[3]]) as usize;
        let mut body = vec![0u8; len];
        f.read_exact(&mut body)?;
        if h[0] & 0x7F == 4 {
            let mut p = 0usize;
            let take = |p: &mut usize| -> u32 {
                let v = u32::from_le_bytes(body[*p..*p + 4].try_into().unwrap());
                *p += 4;
                v
            };
            let vl = take(&mut p) as usize;
            p += vl;
            let n = take(&mut p);
            for _ in 0..n {
                let l = take(&mut p) as usize;
                let s = String::from_utf8_lossy(&body[p..p + l]).into_owned();
                p += l;
                let Some((k, v)) = s.split_once('=') else { continue };
                let Some(rest) = k.strip_prefix("CHAPTER") else { continue };
                let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                let Ok(num) = digits.parse::<u32>() else { continue };
                let e = map.entry(num).or_insert(Chapter { start: 0.0, title: String::new() });
                if rest[digits.len()..].eq_ignore_ascii_case("NAME") {
                    e.title = v.to_string();
                } else {
                    let secs = v.split(':').fold(0.0, |acc, part| acc * 60.0 + part.parse::<f64>().unwrap_or(0.0));
                    e.start = secs;
                }
            }
        }
        if h[0] & 0x80 != 0 {
            break;
        }
    }
    Ok(map.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(path: &Path, secs: f32) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(path, spec).unwrap();
        for i in 0..(48_000.0 * secs) as i32 {
            w.write_sample((i % 1000) * 100).unwrap();
            w.write_sample(-(i % 1000) * 100).unwrap();
        }
        w.finalize().unwrap();
    }

    fn chapters() -> Vec<Chapter> {
        vec![
            Chapter { start: 0.0, title: "A - One".into() },
            Chapter { start: 1.25, title: "Marker 1".into() },
            Chapter { start: 2.5, title: "Ä - Zwei".into() },
        ]
    }

    #[test]
    fn wav_chapters_round_trip_and_replace_and_audio_stays() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("mix.wav");
        wav(&p, 3.0);
        let before: Vec<i32> = hound::WavReader::open(&p).unwrap().samples::<i32>().map(|s| s.unwrap()).collect();
        write_chapters(&p, &chapters()).unwrap();
        let got = read_wav_chapters(&p).unwrap();
        assert_eq!(got.len(), 3);
        assert_eq!(got[1].title, "Marker 1");
        assert!((got[2].start - 2.5).abs() < 1e-3);
        assert_eq!(got[2].title, "Ä - Zwei");
        // Written again with fewer: the old ones are gone, the audio is whole.
        write_chapters(&p, &chapters()[..1]).unwrap();
        assert_eq!(read_wav_chapters(&p).unwrap().len(), 1);
        let after: Vec<i32> = hound::WavReader::open(&p).unwrap().samples::<i32>().map(|s| s.unwrap()).collect();
        assert_eq!(before, after);
        // A file with chapters is whole: repair leaves it as it is.
        let len = std::fs::metadata(&p).unwrap().len();
        crate::repair_wav(&p).unwrap();
        assert_eq!(std::fs::metadata(&p).unwrap().len(), len);
        assert_eq!(read_wav_chapters(&p).unwrap().len(), 1);
    }

    #[test]
    fn mp3_and_flac_chapters_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let w = dir.path().join("mix.wav");
        wav(&w, 3.0);
        let mp3 = dir.path().join("mix.mp3");
        crate::wav_to_mp3(&w, &mp3).unwrap();
        let size = std::fs::metadata(&mp3).unwrap().len();
        write_chapters(&mp3, &chapters()).unwrap();
        write_chapters(&mp3, &chapters()).unwrap();
        let got = read_mp3_chapters(&mp3).unwrap();
        assert_eq!(got.len(), 3, "{got:?}");
        assert_eq!(got[2].title, "Ä - Zwei");
        assert!((got[1].start - 1.25).abs() < 1e-3);
        // One tag, not two: the second write replaced the first.
        let grown = std::fs::metadata(&mp3).unwrap().len() - size;
        assert!(grown < 400, "{grown}");

        let flac = dir.path().join("mix.flac");
        crate::wav_to_flac(&w, &flac).unwrap();
        write_chapters(&flac, &chapters()).unwrap();
        write_chapters(&flac, &chapters()).unwrap();
        let got = read_flac_chapters(&flac).unwrap();
        assert_eq!(got, chapters());
        // Still a FLAC every decoder reads to the last sample.
        let d = od_decode::decode(&flac, od_decode::Layout::Stereo).unwrap();
        assert_eq!(d.samples.len(), 48_000 * 3 * 2);
    }
}
