//! CPU rendering of waveforms and the ground texture. Waveforms use the
//! three-band convention DJs read at a glance: each column draws the low band
//! (violet), then the mids (teal) and the highs (sodium) on top, each at its
//! own amplitude, so kicks, basslines, vocals and hats separate visually. The
//! three colours can be changed in Preferences > View.

use od_analysis::transients::{BeatError, Transient};
use od_core::{Beatgrid, Phrase, PhraseKind, Waveform, WaveformColumn};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

const GROUND: [u8; 3] = [0x1A, 0x1A, 0x19];
/// Band colours of the emission-line palette, as stored in the settings.
pub const LOW_HEX: &str = "#8B6CFF";
pub const MID_HEX: &str = "#2FC4B2";
pub const HIGH_HEX: &str = "#F5CE4A";
const LOW: [u8; 3] = [0x8B, 0x6C, 0xFF];
const MID: [u8; 3] = [0x2F, 0xC4, 0xB2];
const HIGH: [u8; 3] = [0xF5, 0xCE, 0x4A];
/// Colour-blind-safe bands (Okabe–Ito blue, orange, white).
const LOW_CB: [u8; 3] = [0x00, 0x72, 0xB2];
const MID_CB: [u8; 3] = [0xE6, 0x9F, 0x00];
const HIGH_CB: [u8; 3] = [0xFF, 0xFF, 0xFF];

static COLORBLIND: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The user's band colours (low, mid, high): 0 keeps the palette's colour,
/// otherwise 0x01RRGGBB.
static CUSTOM: [std::sync::atomic::AtomicU32; 3] = [
    std::sync::atomic::AtomicU32::new(0),
    std::sync::atomic::AtomicU32::new(0),
    std::sync::atomic::AtomicU32::new(0),
];

/// Waveform style: 0 = three bands, 1 = one colour, 2 = RGB.
static STYLE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Counts changes of waveform style and colours, so cached tiles can tell.
static GENERATION: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// The current generation of waveform style and colours.
pub fn waveform_generation() -> u32 {
    GENERATION.load(std::sync::atomic::Ordering::Relaxed)
}

/// Waveform style by its settings name ("3band", "mono", "rgb").
pub fn set_waveform_style(name: &str) {
    let v = match name {
        "mono" => 1,
        "rgb" => 2,
        _ => 0,
    };
    if STYLE.swap(v, std::sync::atomic::Ordering::Relaxed) != v {
        GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// `#RRGGBB` as bytes.
pub fn parse_hex(hex: &str) -> Option<[u8; 3]> {
    let digits = hex.strip_prefix('#').filter(|d| d.len() == 6)?;
    let value = u32::from_str_radix(digits, 16).ok()?;
    Some([(value >> 16) as u8, (value >> 8) as u8, value as u8])
}

/// The user's band colours; an empty or invalid entry keeps the palette's.
pub fn set_waveform_colours(colours: [&str; 3]) {
    for (slot, hex) in CUSTOM.iter().zip(colours) {
        let value = parse_hex(hex).map_or(0, |[r, g, b]| {
            0x0100_0000 | u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
        });
        if slot.swap(value, std::sync::atomic::Ordering::Relaxed) != value {
            GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

/// Switches waveform bands to the colour-blind-safe palette.
pub fn set_colorblind_safe(on: bool) {
    COLORBLIND.store(on, std::sync::atomic::Ordering::Relaxed);
}
/// Beatgrid: beats, bar starts and 16-bar starts, drawn behind the waveform.
const DOWNBEAT: [u8; 3] = [0xB7, 0xB1, 0xA4];
const DOWNBEAT_MARK: [u8; 3] = [0xE6, 0xE0, 0xD2];
const PHRASE_MARK: [u8; 3] = [0xF5, 0xCE, 0x4A];
/// 589 nm, dimmed: phrase ticks on the overview.
const TICK: [u8; 3] = [0xB8, 0x9B, 0x3A];

/// Colours of one appearance (dark, or light on paper).
struct Pal {
    ground: [u8; 3],
    downbeat: [u8; 3],
    downbeat_mark: [u8; 3],
    phrase_mark: [u8; 3],
    tick: [u8; 3],
    bands: [[u8; 3]; 3],
    bands_cb: [[u8; 3]; 3],
    /// Grid editing: beats within 5 ms / within 15 ms / further off, and
    /// the transient marks.
    fit_ok: [u8; 3],
    fit_warn: [u8; 3],
    fit_bad: [u8; 3],
    transient: [u8; 3],
}

const DARK: Pal = Pal {
    ground: GROUND,
    downbeat: DOWNBEAT,
    downbeat_mark: DOWNBEAT_MARK,
    phrase_mark: PHRASE_MARK,
    tick: TICK,
    bands: [LOW, MID, HIGH],
    bands_cb: [LOW_CB, MID_CB, HIGH_CB],
    fit_ok: [0xE6, 0xE0, 0xD2],
    fit_warn: [0xF5, 0xCE, 0x4A],
    fit_bad: [0xEF, 0x5A, 0x5A],
    transient: [0xB7, 0xB1, 0xA4],
};

/// Light appearance: paper ground, deep violet lows, dark teal mids and
/// ochre highs.
const LIGHT: Pal = Pal {
    ground: [0xE9, 0xE5, 0xDA],
    downbeat: [0x6E, 0x6A, 0x62],
    downbeat_mark: [0x1D, 0x1C, 0x1A],
    phrase_mark: [0x85, 0x63, 0x00],
    tick: [0xB0, 0x8A, 0x2A],
    bands: [[0x5B, 0x3F, 0xC8], [0x0E, 0x8A, 0x7D], [0xB0, 0x84, 0x10]],
    bands_cb: [[0x00, 0x5F, 0xA0], [0xD5, 0x7A, 0x00], [0xA0, 0xA0, 0xA0]],
    fit_ok: [0x1D, 0x1C, 0x1A],
    fit_warn: [0x85, 0x63, 0x00],
    fit_bad: [0xC2, 0x34, 0x34],
    transient: [0x4A, 0x47, 0x40],
};

fn pal() -> &'static Pal {
    if crate::appearance::is_light() {
        &LIGHT
    } else {
        &DARK
    }
}

fn bands() -> [[u8; 3]; 3] {
    let p = pal();
    if COLORBLIND.load(std::sync::atomic::Ordering::Relaxed) {
        return p.bands_cb;
    }
    let mut bands = p.bands;
    for (band, slot) in bands.iter_mut().zip(&CUSTOM) {
        let value = slot.load(std::sync::atomic::Ordering::Relaxed);
        if value != 0 {
            *band = [(value >> 16) as u8, (value >> 8) as u8, value as u8];
        }
    }
    bands
}

/// Colour of a phrase kind (shared by the phrase bar in the UI).
pub fn phrase_rgb(kind: PhraseKind) -> [u8; 3] {
    if crate::appearance::is_light() {
        return match kind {
            PhraseKind::Intro | PhraseKind::Outro => [0x4F, 0x56, 0xD6],
            PhraseKind::Up => [0x2E, 0x7A, 0x1F],
            PhraseKind::Down => [0x1A, 0x72, 0xA6],
            PhraseKind::Chorus => [0xA5, 0x4E, 0x0E],
            PhraseKind::Verse => [0x4A, 0x47, 0x40],
            PhraseKind::Bridge => [0x6A, 0x4B, 0xD8],
        };
    }
    match kind {
        PhraseKind::Intro | PhraseKind::Outro => [0x7C, 0x83, 0xFF],
        PhraseKind::Up => [0x82, 0xD1, 0x6B],
        PhraseKind::Down => [0x4F, 0xB8, 0xF0],
        PhraseKind::Chorus => [0xF0, 0x8A, 0x3C],
        PhraseKind::Verse => [0xE6, 0xE0, 0xD2],
        PhraseKind::Bridge => [0xB0, 0x7C, 0xF0],
    }
}

/// Label of a phrase kind as shown on the phrase bar.
pub fn phrase_label(kind: PhraseKind) -> &'static str {
    match kind {
        PhraseKind::Intro => "INTRO",
        PhraseKind::Verse => "VERSE",
        PhraseKind::Chorus => "CHORUS",
        PhraseKind::Bridge => "BRIDGE",
        PhraseKind::Up => "UP",
        PhraseKind::Down => "DOWN",
        PhraseKind::Outro => "OUTRO",
    }
}

fn fill(buf: &mut SharedPixelBuffer<Rgba8Pixel>, rgb: [u8; 3]) {
    for p in buf.make_mut_slice() {
        *p = Rgba8Pixel {
            r: rgb[0],
            g: rgb[1],
            b: rgb[2],
            a: 255,
        };
    }
}

/// A vertical line blended over what is drawn (`alpha` 0..1).
fn vline_over(buf: &mut SharedPixelBuffer<Rgba8Pixel>, x: u32, y0: u32, y1: u32, rgb: [u8; 3], alpha: f32) {
    let (w, h) = (buf.width(), buf.height());
    if x >= w {
        return;
    }
    let px = buf.make_mut_slice();
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * alpha).round() as u8;
    for y in y0.min(h)..y1.min(h) {
        let p = &mut px[(y * w + x) as usize];
        *p = Rgba8Pixel { r: mix(p.r, rgb[0]), g: mix(p.g, rgb[1]), b: mix(p.b, rgb[2]), a: 255 };
    }
}

fn vline(buf: &mut SharedPixelBuffer<Rgba8Pixel>, x: u32, y0: u32, y1: u32, rgb: [u8; 3]) {
    let (w, h) = (buf.width(), buf.height());
    if x >= w {
        return;
    }
    let px = buf.make_mut_slice();
    for y in y0.min(h)..y1.min(h) {
        px[(y * w + x) as usize] = Rgba8Pixel {
            r: rgb[0],
            g: rgb[1],
            b: rgb[2],
            a: 255,
        };
    }
}

/// The played part: darker on the dark ground, faded toward the paper in
/// light mode.
fn dim(rgb: [u8; 3]) -> [u8; 3] {
    if crate::appearance::is_light() {
        let g = pal().ground;
        return std::array::from_fn(|i| ((rgb[i] as u16 * 2 + g[i] as u16 * 3) / 5) as u8);
    }
    rgb.map(|c| (c as u16 * 2 / 5) as u8)
}

/// Draws one three-band column centred on `mid`; `amp` is the half height of
/// the loudest band.
fn column(
    buf: &mut SharedPixelBuffer<Rgba8Pixel>,
    x: u32,
    mid: u32,
    amp: u32,
    c: &WaveformColumn,
    played: bool,
) {
    styled_column(
        STYLE.load(std::sync::atomic::Ordering::Relaxed),
        buf,
        x,
        mid,
        amp,
        c,
        played,
    )
}

fn styled_column(
    style: u8,
    buf: &mut SharedPixelBuffer<Rgba8Pixel>,
    x: u32,
    mid: u32,
    amp: u32,
    c: &WaveformColumn,
    played: bool,
) {
    match style {
        1 => return mono_column(buf, x, mid, amp, c, played),
        2 => return rgb_column(buf, x, mid, amp, c, played),
        _ => {}
    }
    let [lo, mi, hi] = bands();
    let top = c.low.max(c.mid).max(c.high).max(1) as f32;
    let mut layers = [(c.low, lo), (c.mid, mi), (c.high, hi)];
    // On paper the bands nest, widest first, so every band stays in view;
    // on the dark ground the bright highs sit on top as the core.
    if crate::appearance::is_light() {
        layers.sort_by(|a, b| b.0.cmp(&a.0));
    }
    for (v, rgb) in layers {
        let a = (v as f32 / top * amp as f32).round() as u32;
        if a == 0 {
            continue;
        }
        let rgb = if played { dim(rgb) } else { rgb };
        vline(buf, x, mid.saturating_sub(a), mid + a, rgb);
    }
}

/// One-colour style: one envelope in the low band's colour, the highs lighter on top.
fn mono_column(
    buf: &mut SharedPixelBuffer<Rgba8Pixel>,
    x: u32,
    mid: u32,
    amp: u32,
    c: &WaveformColumn,
    played: bool,
) {
    let top = c.low.max(c.mid).max(c.high).max(1) as f32;
    let body = bands()[0];
    let light = body.map(|v| v / 2 + 120);
    for (v, rgb) in [(c.low.max(c.mid), body), (c.high, light)] {
        let a = (v as f32 / top * amp as f32).round() as u32;
        if a > 0 {
            vline(
                buf,
                x,
                mid.saturating_sub(a),
                mid + a,
                if played { dim(rgb) } else { rgb },
            );
        }
    }
}

/// RGB style: one envelope whose colour mixes the bands (lows red, mids
/// green, highs blue), so the timbre shows as hue.
fn rgb_column(
    buf: &mut SharedPixelBuffer<Rgba8Pixel>,
    x: u32,
    mid: u32,
    amp: u32,
    c: &WaveformColumn,
    played: bool,
) {
    let top = c.low.max(c.mid).max(c.high);
    if top == 0 || amp == 0 {
        return;
    }
    // The loudest band is full intensity; the others in proportion.
    let rgb = [c.low, c.mid, c.high].map(|v| ((v as u32 * 255) / top as u32).max(40) as u8);
    vline(
        buf,
        x,
        mid.saturating_sub(amp),
        mid + amp,
        if played { dim(rgb) } else { rgb },
    );
}

/// Paints phrase colours as a strip of `strip` pixels along the bottom.
fn phrase_strip(
    buf: &mut SharedPixelBuffer<Rgba8Pixel>,
    phrases: &[Phrase],
    grid: &Beatgrid,
    x_of: impl Fn(f64) -> Option<u32>,
    strip: u32,
) {
    let (w, h) = (buf.width(), buf.height());
    for p in phrases {
        let (Some(a), Some(b)) = (
            grid.beats.get(p.start_beat as usize),
            grid.beats
                .get((p.end_beat as usize).min(grid.beats.len().saturating_sub(1))),
        ) else {
            continue;
        };
        let (Some(x0), Some(x1)) = (x_of(a.time), x_of(b.time)) else {
            continue;
        };
        let rgb = phrase_rgb(p.kind);
        for x in x0..x1.min(w) {
            vline(buf, x, h.saturating_sub(strip), h, rgb);
        }
        // A dark gap marks the phrase boundary.
        vline(buf, x0, h.saturating_sub(strip), h, pal().ground);
    }
}

/// Overview of a whole track, `width` × `height` physical pixels, with
/// beatgrid ticks every 16 bars.
pub fn overview(w: &Waveform, grid: &Beatgrid, duration: f64, width: u32, height: u32) -> Image {
    let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(width.max(1), height.max(1));
    fill(&mut buf, pal().ground);
    // Ticks every 16 bars sit in a margin above and below the waveform.
    let margin = (height / 10).max(3);
    if duration > 0.0 {
        for b in grid.beats.iter().step_by(64) {
            let x = (b.time / duration * width as f64) as u32;
            vline(&mut buf, x, 0, margin - 1, pal().tick);
            vline(&mut buf, x, height - margin + 1, height, pal().tick);
        }
    }
    if w.columns.is_empty() {
        return Image::from_rgba8(buf);
    }
    let cols = w.overview(width.max(1) as usize);
    let max = cols.iter().map(|c| c.peak).max().unwrap_or(1).max(1) as f32;
    let mid = height / 2;
    for (i, c) in cols.iter().enumerate() {
        let amp = (c.peak as f32 / max * (height as f32 / 2.0 - margin as f32)).max(1.0) as u32;
        column(&mut buf, i as u32, mid, amp, c, false);
    }
    Image::from_rgba8(buf)
}

/// Small overview for list rows (transparent background).
pub fn preview(w: &Waveform, width: u32, height: u32) -> Image {
    let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(width.max(1), height.max(1));
    for p in buf.make_mut_slice() {
        *p = Rgba8Pixel {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
    }
    let cols = w.overview(width.max(1) as usize);
    let max = cols.iter().map(|c| c.peak).max().unwrap_or(1).max(1) as f32;
    let mid = height / 2;
    for (i, c) in cols.iter().enumerate() {
        let amp = (c.peak as f32 / max * (height as f32 / 2.0 - 1.0)).max(1.0) as u32;
        column(&mut buf, i as u32, mid, amp, c, false);
    }
    Image::from_rgba8(buf)
}

/// Scrolling detail waveform centred on `position` seconds, showing
/// `span` seconds in total, with beat ticks from `grid` and a phrase strip
/// along the bottom. Audio already played is drawn dimmed.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub fn detail(
    w: &Waveform,
    grid: &Beatgrid,
    phrases: &[Phrase],
    position: f64,
    span: f64,
    width: u32,
    height: u32,
) -> Image {
    draw_detail(
        w,
        grid,
        phrases,
        position - span / 2.0,
        span,
        Some(position),
        width,
        height,
    )
}

/// What the grid editor draws over the enlarged waveform: the attacks
/// along the bottom edge (kicks long), the error of each beat (its tick's
/// colour) and the pins (◆).
pub struct GridEvidence<'a> {
    pub transients: &'a [Transient],
    /// Per beat of the grid drawn (index = beat index).
    pub errors: &'a [BeatError],
    pub pins: &'a [f64],
}

/// [`detail`] with grid editing evidence (`None`: the plain detail).
#[allow(clippy::too_many_arguments)]
pub fn detail_with(
    w: &Waveform,
    grid: &Beatgrid,
    phrases: &[Phrase],
    position: f64,
    span: f64,
    width: u32,
    height: u32,
    evidence: Option<&GridEvidence>,
) -> Image {
    let mut buf = draw_detail_buf(w, grid, phrases, position - span / 2.0, span, Some(position), width, height, evidence);
    if let Some(ev) = evidence {
        draw_evidence(&mut buf, ev, position - span / 2.0, span);
    }
    Image::from_rgba8(buf)
}

/// Colour of a beat by its error (grid editing).
fn fit_rgb(err: &BeatError) -> Option<[u8; 3]> {
    if !err.matched {
        return None;
    }
    let p = pal();
    let e = err.err.abs();
    Some(if e <= 0.005 {
        p.fit_ok
    } else if e <= 0.015 {
        p.fit_warn
    } else {
        p.fit_bad
    })
}

fn draw_evidence(buf: &mut SharedPixelBuffer<Rgba8Pixel>, ev: &GridEvidence, start: f64, span: f64) {
    let (width, height) = (buf.width(), buf.height());
    let secs_per_px = span / width.max(1) as f64;
    let strip = (height / 18).max(3);
    let wave_h = height - strip - 1;
    let mark_h = (wave_h / 7).max(5);
    let x_of = |t: f64| -> Option<u32> {
        let x = (t - start) / secs_per_px;
        (x >= 0.0 && x < width as f64).then_some(x as u32)
    };
    // Transients: marks rising from the bottom edge, kicks long and solid.
    let a = ev.transients.partition_point(|t| t.time < start);
    let peak = ev.transients.iter().map(|t| t.strength).fold(0f32, f32::max).max(1e-9);
    for t in ev.transients[a..].iter().take_while(|t| t.time <= start + span) {
        let Some(x) = x_of(t.time) else { continue };
        let len = if t.low { mark_h + mark_h / 2 } else { mark_h / 2 + 1 };
        let alpha = (0.45 + 0.55 * (t.strength / peak).sqrt()).min(1.0);
        vline_over(buf, x, wave_h.saturating_sub(len), wave_h, pal().transient, alpha);
        if t.low {
            vline_over(buf, x + 1, wave_h.saturating_sub(len), wave_h, pal().transient, alpha * 0.6);
        }
    }
    // Pins: a full-height hairline and a diamond on the top edge.
    for &p in ev.pins {
        let Some(x) = x_of(p) else { continue };
        let tone = pal().fit_warn;
        vline_over(buf, x, 0, wave_h, tone, 0.55);
        let r = (mark_h / 2 + 2) as i64;
        for dy in 0..=2 * r {
            let half = r - (dy - r).abs();
            for dx in -half..=half {
                let xx = x as i64 + dx;
                if xx >= 0 && (xx as u32) < width {
                    vline(buf, xx as u32, dy as u32, dy as u32 + 1, tone);
                }
            }
        }
    }
}

/// The DRIFT strip: the whole track left to right, each beat's error as a
/// mark up (late grid, the beat early) or down from the centre line, in
/// the colour of its class; beats without a transient leave a gap.
pub fn drift_strip(errors: &[BeatError], grid: &Beatgrid, duration: f64, width: u32, height: u32) -> Image {
    let (width, height) = (width.max(1), height.max(3));
    let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(width, height);
    let p = pal();
    fill(&mut buf, p.ground);
    let mid = height / 2;
    for x in 0..width {
        vline(&mut buf, x, mid, mid + 1, dim(p.downbeat));
    }
    if duration <= 0.0 {
        return Image::from_rgba8(buf);
    }
    // Worst error per column.
    let mut worst: Vec<Option<BeatError>> = vec![None; width as usize];
    for e in errors.iter().filter(|e| e.matched) {
        let Some(b) = grid.beats.get(e.beat) else { continue };
        let x = ((b.time / duration) * width as f64) as usize;
        if let Some(slot) = worst.get_mut(x.min(width as usize - 1))
            && slot.is_none_or(|w| e.err.abs() > w.err.abs())
        {
            *slot = Some(*e);
        }
    }
    let half = (height / 2) as f64;
    for (x, e) in worst.iter().enumerate() {
        let Some(e) = e else { continue };
        let Some(rgb) = fit_rgb(e) else { continue };
        let len = ((e.err.abs() / 0.030).min(1.0) * (half - 1.0)).max(1.0) as u32;
        if e.err >= 0.0 {
            vline(&mut buf, x as u32, mid.saturating_sub(len), mid + 1, rgb);
        } else {
            vline(&mut buf, x as u32, mid, (mid + len + 1).min(height), rgb);
        }
    }
    Image::from_rgba8(buf)
}

/// One tile of the scrolling waveform: `span` seconds from `start`, nothing
/// dimmed. The UI scrolls tiles by translating them, so the CPU renders a
/// tile only when the playhead enters a new one.
pub fn detail_tile(
    w: &Waveform,
    grid: &Beatgrid,
    phrases: &[Phrase],
    start: f64,
    span: f64,
    width: u32,
    height: u32,
) -> Image {
    draw_detail(w, grid, phrases, start, span, None, width, height)
}

/// A tile turned for the vertical lanes: time runs top to bottom, the
/// waveform's amplitude spreads left and right.
pub fn transpose(img: &Image) -> Image {
    let Some(src) = img.to_rgba8() else {
        return img.clone();
    };
    let (w, h) = (src.width(), src.height());
    let mut out = SharedPixelBuffer::<Rgba8Pixel>::new(h, w);
    let from = src.as_slice();
    let to = out.make_mut_slice();
    for y in 0..h {
        for x in 0..w {
            to[(x * h + (h - 1 - y)) as usize] = from[(y * w + x) as usize];
        }
    }
    Image::from_rgba8(out)
}

#[allow(clippy::too_many_arguments)]
fn draw_detail(
    w: &Waveform,
    grid: &Beatgrid,
    phrases: &[Phrase],
    start: f64,
    span: f64,
    dim_before: Option<f64>,
    width: u32,
    height: u32,
) -> Image {
    Image::from_rgba8(draw_detail_buf(w, grid, phrases, start, span, dim_before, width, height, None))
}

#[allow(clippy::too_many_arguments)]
fn draw_detail_buf(
    w: &Waveform,
    grid: &Beatgrid,
    phrases: &[Phrase],
    start: f64,
    span: f64,
    dim_before: Option<f64>,
    width: u32,
    height: u32,
    evidence: Option<&GridEvidence>,
) -> SharedPixelBuffer<Rgba8Pixel> {
    let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(width.max(1), height.max(1));
    fill(&mut buf, pal().ground);
    let secs_per_px = span / width.max(1) as f64;
    let strip = (height / 18).max(3);
    let wave_h = height - strip - 1;
    // Waveform amplitude leaves a band free for the grid markers.
    let mark_h = (wave_h / 7).max(5);
    let x_of = |t: f64| {
        let x = (t - start) / secs_per_px;
        Some(x.clamp(0.0, width as f64) as u32)
    };
    phrase_strip(&mut buf, phrases, grid, x_of, strip);
    let mid = wave_h / 2;
    let rate = w.columns_per_second;
    for x in 0..width {
        let t = start + x as f64 * secs_per_px;
        if t < 0.0 {
            continue;
        }
        let a = (t * rate) as usize;
        let b = (((t + secs_per_px) * rate) as usize).max(a + 1);
        if a >= w.columns.len() {
            continue;
        }
        let c = w.columns[a..b.min(w.columns.len())].iter().fold(
            WaveformColumn::default(),
            |acc, c| WaveformColumn {
                low: acc.low.max(c.low),
                mid: acc.mid.max(c.mid),
                high: acc.high.max(c.high),
                peak: acc.peak.max(c.peak),
            },
        );
        let amp =
            (c.peak as f32 / 255.0 * (wave_h as f32 / 2.0 - mark_h as f32 - 2.0)).max(1.0) as u32;
        column(&mut buf, x, mid, amp, &c, dim_before.is_some_and(|p| t < p));
    }
    // Beatgrid over the waveform: clear ticks along the top and bottom edge
    // (every beat; bars longer and brighter; 16-bar starts amber and
    // widest) and a translucent line through the wave, so the grid reads at
    // a glance without hiding the waveform's colours.
    if !grid.beats.is_empty() {
        let first = grid.beat_index_at(start.max(0.0)).unwrap_or(0);
        for (i, b) in grid.beats.iter().enumerate().skip(first) {
            if b.time > start + span {
                break;
            }
            if b.time < start {
                continue;
            }
            let x = ((b.time - start) / secs_per_px) as u32;
            let downbeat = b.bar_position == 1;
            let phrase = downbeat && (i / 4) % 16 == 0;
            let (mut color, mut through, tick, wide): ([u8; 3], f32, u32, u32) = if phrase {
                (pal().phrase_mark, 0.55, mark_h, 3u32)
            } else if downbeat {
                (pal().downbeat_mark, 0.38, mark_h, 2)
            } else {
                (pal().downbeat, 0.16, mark_h / 2 + 1, 1)
            };
            // Grid editing: the tick tells how far the beat is off; the
            // bottom edge belongs to the transients.
            if let Some(ev) = evidence {
                match ev.errors.get(i).and_then(fit_rgb) {
                    Some(rgb) => {
                        color = rgb;
                        through = through.max(0.3);
                    }
                    None => color = dim(color),
                }
                vline_over(&mut buf, x, tick, wave_h, color, through);
                for dx in 0..wide {
                    let xx = (x + dx).saturating_sub(wide / 2);
                    vline(&mut buf, xx, 0, tick + tick / 2, color);
                }
                continue;
            }
            vline_over(&mut buf, x, tick, wave_h - tick, color, through);
            for dx in 0..wide {
                let xx = (x + dx).saturating_sub(wide / 2);
                vline(&mut buf, xx, 0, tick, color);
                vline(&mut buf, xx, wave_h - tick, wave_h, color);
            }
        }
    }
    buf
}

/// The charcoal continuum: a faint, deterministic vertical banding drawn
/// once at start-up and stretched over the window.
pub fn ground_texture() -> Image {
    let (w, h) = (640u32, 4u32);
    let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
    let px = buf.make_mut_slice();
    let mut seed: u32 = 0x9E37_79B9;
    let mut level = 0i32;
    for x in 0..w {
        // Slow random walk in ±3 levels around the ground value.
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        if seed % 9 == 0 {
            level = (level + if seed & 0x100 == 0 { 1 } else { -1 }).clamp(-3, 3);
        }
        let v = |base: i32| (base + level).clamp(0, 255) as u8;
        for y in 0..h {
            px[(y * w + x) as usize] = Rgba8Pixel {
                r: v(0x15),
                g: v(0x15),
                b: v(0x14),
                a: 255,
            };
        }
    }
    Image::from_rgba8(buf)
}

/// Cover art scaled to a square thumbnail of `size` pixels (PNG / JPEG).
pub fn artwork_thumbnail(data: &[u8], size: u32) -> Option<Image> {
    artwork_pixels(data, size).map(Image::from_rgba8)
}

/// [`artwork_thumbnail`] as a pixel buffer (can be made on any thread).
pub fn artwork_pixels(data: &[u8], size: u32) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let img = image::load_from_memory(data).ok()?;
    let thumb = img
        .resize_to_fill(
            size.max(1),
            size.max(1),
            image::imageops::FilterType::Triangle,
        )
        .to_rgba8();
    Some(SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        thumb.as_raw(),
        thumb.width(),
        thumb.height(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_sizes() {
        let w = Waveform {
            columns_per_second: 150.0,
            columns: vec![
                WaveformColumn {
                    low: 200,
                    mid: 20,
                    high: 10,
                    peak: 200
                };
                1500
            ],
        };
        let g = Beatgrid::constant(120.0, 0.0, 1, 10.0);
        let img = overview(&w, &g, 10.0, 400, 80);
        assert_eq!(img.size().width, 400);
        let d = detail(&w, &g, &[], 5.0, 8.0, 600, 100);
        assert_eq!(d.size().height, 100);
        assert_eq!(
            overview(&Waveform::default(), &g, 0.0, 10, 10).size().width,
            10
        );
        assert_eq!(ground_texture().size().width, 640);
    }

    #[test]
    fn three_bands_stack_by_amplitude() {
        let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(1, 41);
        fill(&mut buf, GROUND);
        column(
            &mut buf,
            0,
            20,
            20,
            &WaveformColumn {
                low: 200,
                mid: 100,
                high: 50,
                peak: 200,
            },
            false,
        );
        let at = |y: usize| {
            let p = buf.as_slice()[y];
            [p.r, p.g, p.b]
        };
        // Outermost: only the low band reaches; then mids; the centre is highs.
        assert_eq!(at(1), LOW);
        assert_eq!(at(14), MID);
        assert_eq!(at(20), HIGH);
    }

    #[test]
    fn beatgrid_lines_sit_on_the_beats() {
        let w = Waveform {
            columns_per_second: 150.0,
            columns: vec![WaveformColumn::default(); 1500],
        };
        // 120 BPM: a beat every 0.5 s; 8 s across 400 px = 25 px per beat.
        let g = Beatgrid::constant(120.0, 0.0, 1, 10.0);
        let img = detail(&w, &g, &[], 4.0, 8.0, 400, 90);
        let buf = img.to_rgba8().unwrap();
        let px = |x: u32, y: u32| {
            let p = buf.as_slice()[(y * 400 + x) as usize];
            [p.r, p.g, p.b]
        };
        // Ticks along the top and bottom edges: beat 0 (bar 1, a 16-bar
        // start) amber at x = 0, beat 8 (bar 3) at x = 200, a plain beat 25 px
        // later, the next bar at x = 300.
        assert_eq!(px(0, 2), PHRASE_MARK);
        assert_eq!(px(200, 2), DOWNBEAT_MARK);
        assert_eq!(px(200, 82), DOWNBEAT_MARK);
        assert_eq!(px(225, 2), DOWNBEAT);
        assert_eq!(px(300, 2), DOWNBEAT_MARK);
        // Through the wave: translucent lines, bars stronger than beats,
        // the ground between beats untouched.
        let lum = |p: [u8; 3]| p[0] as u32 + p[1] as u32 + p[2] as u32;
        let ground = lum(px(212, 40));
        assert!(lum(px(225, 40)) > ground, "beat line over the ground");
        assert!(lum(px(200, 40)) > lum(px(225, 40)), "bar line brighter than a beat line");
        assert_ne!(px(0, 40), PHRASE_MARK, "the 16-bar line is translucent");
        assert!(lum(px(0, 40)) > ground);
    }

    #[test]
    fn tiles_join_seamlessly() {
        let w = Waveform {
            columns_per_second: 150.0,
            columns: (0..1500)
                .map(|i| WaveformColumn {
                    low: (i % 200) as u8,
                    mid: 40,
                    high: 20,
                    peak: (i % 200) as u8,
                })
                .collect(),
        };
        let g = Beatgrid::constant(120.0, 0.0, 1, 10.0);
        // Two 4 s tiles side by side equal one 8 s detail without dimming.
        let a = detail_tile(&w, &g, &[], 0.0, 4.0, 200, 60)
            .to_rgba8()
            .unwrap();
        let b = detail_tile(&w, &g, &[], 4.0, 4.0, 200, 60)
            .to_rgba8()
            .unwrap();
        let whole = detail_tile(&w, &g, &[], 0.0, 8.0, 400, 60)
            .to_rgba8()
            .unwrap();
        for y in [10u32, 30, 50] {
            for x in [0u32, 57, 199] {
                assert_eq!(
                    a.as_slice()[(y * 200 + x) as usize],
                    whole.as_slice()[(y * 400 + x) as usize]
                );
                assert_eq!(
                    b.as_slice()[(y * 200 + x) as usize],
                    whole.as_slice()[(y * 400 + 200 + x) as usize]
                );
            }
        }
    }

    #[test]
    fn phrase_strip_is_painted() {
        let w = Waveform {
            columns_per_second: 150.0,
            columns: vec![WaveformColumn::default(); 1500],
        };
        let g = Beatgrid::constant(120.0, 0.0, 1, 10.0);
        let phrases = [Phrase {
            start_beat: 0,
            end_beat: 16,
            kind: PhraseKind::Chorus,
        }];
        let img = detail(&w, &g, &phrases, 4.0, 8.0, 200, 90);
        let buf = img.to_rgba8().unwrap();
        let p = buf.as_slice()[(89 * 200 + 150) as usize];
        assert_eq!([p.r, p.g, p.b], phrase_rgb(PhraseKind::Chorus));
    }

    #[test]
    fn artwork_thumbnails_are_square() {
        let mut png = Vec::new();
        let img = image::RgbaImage::from_pixel(40, 20, image::Rgba([200, 10, 10, 255]));
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let t = artwork_thumbnail(&png, 16).unwrap();
        assert_eq!((t.size().width, t.size().height), (16, 16));
        assert!(artwork_thumbnail(b"not an image", 16).is_none());
    }

    #[test]
    fn waveform_styles() {
        let c = WaveformColumn {
            low: 200,
            mid: 50,
            high: 10,
            peak: 200,
        };
        // Styles by number (the global switch is shared by parallel tests).
        let px = |style: u8| {
            let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(1, 21);
            styled_column(style, &mut buf, 0, 10, 8, &c, false);
            let p = buf.as_slice()[10];
            [p.r, p.g, p.b]
        };
        let rgb = px(2);
        assert!(
            rgb[0] == 255 && rgb[1] < 100 && rgb[2] < 100,
            "bass-heavy column is red in RGB: {rgb:?}"
        );
        let mono = px(1);
        assert_ne!(mono, px(0), "one-colour style differs");
        assert_ne!(px(0), rgb);
    }

    #[test]
    fn transposed_tiles_run_top_to_bottom() {
        let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(3, 2);
        // Mark the first column (time 0) red.
        for y in 0..2 {
            buf.make_mut_slice()[(y * 3) as usize] = Rgba8Pixel {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            };
        }
        let t = transpose(&Image::from_rgba8(buf)).to_rgba8().unwrap();
        assert_eq!((t.width(), t.height()), (2, 3));
        assert!(
            t.as_slice()[..2].iter().all(|p| p.r == 255),
            "time 0 is the top row"
        );
        assert!(t.as_slice()[2..].iter().all(|p| p.r == 0));
    }

    #[test]
    fn evidence_colours_beats_by_error_and_marks_transients() {
        use od_analysis::transients::{BeatError, Transient};
        let w = Waveform { columns_per_second: 150.0, columns: vec![WaveformColumn::default(); 150 * 8] };
        let g = Beatgrid::constant(120.0, 0.0, 1, 8.0);
        let errors: Vec<BeatError> = (0..g.beats.len())
            .map(|i| BeatError { beat: i, err: if i == 8 { 0.03 } else { 0.0 }, matched: true })
            .collect();
        let tr = [Transient { time: 4.25, strength: 1.0, low: true }];
        let ev = GridEvidence { transients: &tr, errors: &errors, pins: &[] };
        let img = detail_with(&w, &g, &[], 4.0, 8.0, 400, 90, Some(&ev)).to_rgba8().unwrap();
        let px = |x: u32, y: u32| img.as_slice()[(y * 400 + x) as usize];
        // Beat 8 (4.0 s, the centre) is far off: fault red on its top tick.
        let p = px(200, 1);
        assert_eq!([p.r, p.g, p.b], DARK.fit_bad);
        // Beat 6 (3.0 s) fits: bone.
        let p = px(150, 1);
        assert_eq!([p.r, p.g, p.b], DARK.fit_ok);
        // The kick at 4.25 s rises from the bottom edge of the wave.
        let strip = (90 / 18).max(3);
        let p = px(212, 90 - strip - 3);
        assert_ne!([p.r, p.g, p.b], DARK.ground);
        // The drift strip marks beat 8 in red, up from the centre.
        let d = drift_strip(&errors, &g, 8.0, 160, 14).to_rgba8().unwrap();
        let p = d.as_slice()[(2 * 160 + 80) as usize];
        assert_eq!([p.r, p.g, p.b], DARK.fit_bad);
    }
}
