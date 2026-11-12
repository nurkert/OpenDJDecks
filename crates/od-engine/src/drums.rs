//! Built-in drum sounds (CREATE, roadmap [79]): eight voices synthesised
//! here, so a kit carries no third-party samples and sounds the same on
//! every platform. Runs off the real-time thread (it allocates); a sound is
//! rendered once as a one-shot and played as a sample.
//!
//! Each pad has three settings: PITCH in semitones, DECAY (how long it
//! rings, 50 = the voice's own length) and TONE (dark to bright). A kit
//! ([`KITS`]) is one set of settings for the eight pads.
//!
//! **The same on every platform.** The sounds use only the basic float
//! operations (add, subtract, multiply, divide, compare), which IEEE 754
//! rounds the same everywhere, in a fixed order. Sine and powers of two come
//! from the polynomials below instead of the platform's maths library, whose
//! results may differ in the last bit between systems; noise comes from a
//! seeded integer generator. The settings are whole numbers, so a stored
//! kit renders bit for bit alike. The tests pin a hash of every kit at
//! 48 kHz; CI checks it on x86_64 and aarch64.

/// Pads of a kit.
pub const PADS: usize = 8;

/// The longest a sound may ring, seconds.
pub const MAX_SECONDS: f32 = 5.0;

/// The fade at the end of every sound, seconds: it ends on silence.
const END_FADE: f32 = 0.005;

/// One of the eight sounds of a kit, in pad order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Voice {
    Kick,
    Snare,
    Hat,
    OpenHat,
    Clap,
    Tom1,
    Tom2,
    Ride,
}

impl Voice {
    /// The voices in pad order.
    pub const ALL: [Voice; PADS] =
        [Voice::Kick, Voice::Snare, Voice::Hat, Voice::OpenHat, Voice::Clap, Voice::Tom1, Voice::Tom2, Voice::Ride];

    /// Stable id ("kick", "open_hat" …).
    pub fn id(self) -> &'static str {
        match self {
            Voice::Kick => "kick",
            Voice::Snare => "snare",
            Voice::Hat => "hat",
            Voice::OpenHat => "open_hat",
            Voice::Clap => "clap",
            Voice::Tom1 => "tom1",
            Voice::Tom2 => "tom2",
            Voice::Ride => "ride",
        }
    }

    /// The voice's own length at DECAY 50, seconds (to about -60 dB).
    fn seconds(self) -> f32 {
        match self {
            Voice::Kick => 0.5,
            Voice::Snare => 0.28,
            Voice::Hat => 0.08,
            Voice::OpenHat => 0.55,
            Voice::Clap => 0.3,
            Voice::Tom1 => 0.4,
            Voice::Tom2 => 0.5,
            Voice::Ride => 1.4,
        }
    }

    /// The peak a sound is set to: the hats and the ride sit under the
    /// kick and the snare, as in a mixed kit.
    fn level(self) -> f32 {
        match self {
            Voice::Kick => 0.9,
            Voice::Snare => 0.8,
            Voice::Hat | Voice::OpenHat => 0.5,
            Voice::Clap => 0.7,
            Voice::Tom1 | Voice::Tom2 => 0.8,
            Voice::Ride => 0.45,
        }
    }

    /// The noise generator's seed, one per voice.
    fn seed(self) -> u32 {
        0x9E37_79B9 ^ (self as u32 + 1).wrapping_mul(0x85EB_CA6B)
    }
}

/// A pad's settings. Whole numbers, so a stored kit renders alike.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Params {
    /// Semitones, -12..=12.
    pub pitch: i8,
    /// 0..=100; 50 is the voice's own length, each 25 doubles or halves it.
    pub decay: u8,
    /// 0..=100, dark to bright.
    pub tone: u8,
}

impl Default for Params {
    fn default() -> Self {
        Self { pitch: 0, decay: 50, tone: 50 }
    }
}

impl Params {
    pub const PITCH: std::ops::RangeInclusive<i8> = -12..=12;

    /// Within the ranges.
    pub fn clamped(self) -> Self {
        Self { pitch: self.pitch.clamp(-12, 12), decay: self.decay.min(100), tone: self.tone.min(100) }
    }

    fn ratio(self) -> f32 {
        exp2(f32::from(self.pitch) / 12.0)
    }

    fn stretch(self) -> f32 {
        exp2((f32::from(self.decay) - 50.0) / 25.0)
    }

    fn bright(self) -> f32 {
        f32::from(self.tone) / 100.0
    }
}

/// A built-in kit: an id (its name in capitals is shown) and the eight
/// pads' settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kit {
    pub id: &'static str,
    pub pads: [Params; PADS],
}

const fn p(pitch: i8, decay: u8, tone: u8) -> Params {
    Params { pitch, decay, tone }
}

/// The built-in kits. TIGHT: short and bright. WARM: lower, rounder and
/// longer. DUSTY: dark and low, with a driven kick and toms.
pub const KITS: [Kit; 3] = [
    Kit {
        id: "tight",
        pads: [p(0, 35, 60), p(2, 35, 70), p(0, 30, 75), p(0, 40, 70), p(0, 40, 60), p(0, 40, 55), p(0, 40, 55), p(0, 45, 70)],
    },
    Kit {
        id: "warm",
        pads: [p(-2, 65, 25), p(-2, 55, 35), p(-1, 45, 40), p(-1, 60, 40), p(-1, 60, 35), p(-2, 60, 30), p(-2, 60, 30), p(-1, 60, 40)],
    },
    Kit {
        id: "dusty",
        pads: [p(-1, 45, 85), p(-3, 45, 20), p(-3, 35, 25), p(-3, 45, 25), p(-2, 50, 25), p(-3, 45, 75), p(-3, 45, 75), p(-3, 50, 25)],
    },
];

/// A built-in kit by id.
pub fn kit(id: &str) -> Option<&'static Kit> {
    KITS.iter().find(|k| k.id == id)
}

/// The frames a sound takes at `rate`.
pub fn frames(voice: Voice, p: Params, rate: u32) -> usize {
    let p = p.clamped();
    let seconds = (voice.seconds() * p.stretch()).min(MAX_SECONDS);
    // Rounded to whole frames; at least the end fade.
    ((seconds * rate as f32 + 0.5) as usize).max((END_FADE * rate as f32) as usize + 1)
}

/// Renders one sound at `rate` as mono samples in -1..1: it starts at
/// once, rings out and ends on silence.
pub fn render(voice: Voice, p: Params, rate: u32) -> Vec<f32> {
    let p = p.clamped();
    let len = frames(voice, p, rate);
    let mut s = Synth::new(voice, p, rate);
    let mut out: Vec<f32> = (0..len).map(|_| s.next()).collect();
    // Set to the voice's level (whatever TONE and PITCH did to the peak).
    let peak = out.iter().fold(0.0f32, |m, x| if x.abs() > m { x.abs() } else { m });
    if peak > 0.0 {
        let g = voice.level() / peak;
        for x in &mut out {
            *x *= g;
        }
    }
    // The end fade: the last sample is silent.
    let fade = ((END_FADE * rate as f32) as usize).max(1).min(len);
    for (i, x) in out[len - fade..].iter_mut().enumerate() {
        *x *= (fade - 1 - i) as f32 / fade as f32;
    }
    out
}

/// Samples as 16-bit integers (rounded, clamped).
pub fn to_i16(samples: &[f32]) -> Vec<i16> {
    samples.iter().map(|x| (x.clamp(-1.0, 1.0) * 32767.0).round() as i16).collect()
}

// ── The voices ─────────────────────────────────────────────────────────

/// Six square waves at inharmonic ratios: the metal of hats and ride.
const METAL: [f32; 6] = [1.0, 1.4826, 1.8, 2.546, 2.63, 3.897];

struct Synth {
    voice: Voice,
    rate: f32,
    n: usize,
    noise: Noise,
    /// The main envelope and its per-sample factor.
    env: f32,
    k_env: f32,
    /// A second, faster envelope (pitch sweep, click, attack) and its factor.
    fast: f32,
    k_fast: f32,
    /// Oscillator phases in turns.
    phase: [f32; 6],
    /// The base frequency, Hz.
    freq: f32,
    bright: f32,
    /// Filter states and coefficients.
    lp: f32,
    lp2: f32,
    a: f32,
    svf: (f32, f32),
    f_svf: f32,
    /// The clap's burst and tail.
    tail: f32,
    k_tail: f32,
}

impl Synth {
    fn new(voice: Voice, p: Params, rate: u32) -> Self {
        let rate = rate as f32;
        let ring = voice.seconds() * p.stretch();
        let bright = p.bright();
        let r = p.ratio();
        let (freq, fast, cutoff) = match voice {
            Voice::Kick => (50.0 * r, 0.07, 0.0),
            Voice::Snare => (180.0 * r, 0.02, 900.0 + 5000.0 * bright),
            Voice::Hat | Voice::OpenHat => (300.0 * r, 0.02, 4000.0 + 5000.0 * bright),
            Voice::Clap => (1000.0 * r, 0.009, 0.0),
            Voice::Tom1 => (160.0 * r, 0.09, 1500.0 + 4000.0 * bright),
            Voice::Tom2 => (105.0 * r, 0.1, 1500.0 + 4000.0 * bright),
            Voice::Ride => (330.0 * r, 0.25, 2500.0 + 4000.0 * bright),
        };
        let nyquist = rate * 0.45;
        Self {
            voice,
            rate,
            n: 0,
            noise: Noise(voice.seed()),
            env: 1.0,
            k_env: decay_factor(ring, rate),
            fast: 1.0,
            k_fast: decay_factor(fast, rate),
            phase: [0.0; 6],
            freq,
            bright,
            lp: 0.0,
            lp2: 0.0,
            // PITCH moves the filters with the oscillators.
            a: one_pole((cutoff * r).min(nyquist), rate),
            svf: (0.0, 0.0),
            // The clap's band: TONE moves it up.
            f_svf: 2.0 * sin_turns((freq * (0.6 + 0.9 * bright)).min(nyquist) / (2.0 * rate)),
            tail: 0.0,
            k_tail: decay_factor(ring, rate),
        }
    }

    fn next(&mut self) -> f32 {
        let s = match self.voice {
            Voice::Kick => self.kick(),
            Voice::Snare => self.snare(),
            Voice::Hat | Voice::OpenHat | Voice::Ride => self.metal(),
            Voice::Clap => self.clap(),
            Voice::Tom1 | Voice::Tom2 => self.tom(),
        };
        self.n += 1;
        self.env *= self.k_env;
        self.fast *= self.k_fast;
        s
    }

    /// A sine swept down from about 3.5 times its pitch, a click of noise
    /// at the start; TONE adds click and drive.
    fn kick(&mut self) -> f32 {
        let f = self.freq * (1.0 + 2.5 * self.fast);
        let body = sin_turns(advance(&mut self.phase[0], f / self.rate));
        let drive = 1.0 + 4.0 * self.bright;
        let x = body * drive;
        let shaped = x / (1.0 + x.abs()) * (1.0 + 1.0 / drive);
        let click = self.noise.next() * self.fast * self.fast * (0.05 + 0.5 * self.bright);
        shaped * self.env + click
    }

    /// Two sines for the body under high-passed noise for the wires; TONE
    /// opens the noise and lets it ring a little longer than the body.
    fn snare(&mut self) -> f32 {
        let b1 = sin_turns(advance(&mut self.phase[0], self.freq / self.rate));
        let b2 = sin_turns(advance(&mut self.phase[1], self.freq * 1.78 / self.rate));
        let body = (b1 * 0.65 + b2 * 0.35) * self.env * self.env;
        let n = self.noise.next();
        let wires = self.high(n) * self.env;
        body * (0.9 - 0.4 * self.bright) + wires * (0.6 + 0.4 * self.bright) + body * self.fast * 0.3
    }

    /// Square waves at the metal ratios over noise, high-passed twice: the
    /// hats (short and long) and the ride (lower, with a ping at the start).
    fn metal(&mut self) -> f32 {
        let mut sq = 0.0;
        for (i, m) in METAL.iter().enumerate() {
            let ph = advance(&mut self.phase[i], self.freq * m / self.rate);
            sq += if ph < 0.5 { 1.0 } else { -1.0 };
        }
        let noise_share = if self.voice == Voice::Ride { 0.25 } else { 0.5 };
        let x = sq / 6.0 * (1.0 - noise_share) + self.noise.next() * noise_share;
        let hp = self.high(x);
        // The second pole: a steeper slope keeps the metal off the low end.
        self.lp2 += self.a * (hp - self.lp2);
        let y = hp - self.lp2;
        let ping = if self.voice == Voice::Ride { 1.0 + 1.5 * self.fast } else { 1.0 };
        y * self.env * ping
    }

    /// Band-passed noise in three quick bursts, then a tail.
    fn clap(&mut self) -> f32 {
        // A new burst every 11 ms for three, the tail from the third.
        if self.n == 0 || self.n == (0.011 * self.rate) as usize || self.n == (0.022 * self.rate) as usize {
            self.fast = 1.0;
        }
        if self.n == (0.022 * self.rate) as usize {
            self.tail = 1.0;
        }
        self.tail *= self.k_tail;
        let x = self.noise.next();
        // State-variable band-pass at the clap's centre.
        let (low, band) = self.svf;
        let low = low + self.f_svf * band;
        let high = x - low - 0.9 * band;
        let band = band + self.f_svf * high;
        self.svf = (low, band);
        band * (self.fast + 0.6 * self.tail)
    }

    /// A sine swept down a little, with a breath of noise at the start;
    /// TONE brings out the attack.
    fn tom(&mut self) -> f32 {
        let f = self.freq * (1.0 + 0.6 * self.fast);
        let body = sin_turns(advance(&mut self.phase[0], f / self.rate));
        let n = self.noise.next();
        let attack = self.high(n) * self.fast * self.fast * (0.1 + 0.6 * self.bright);
        body * self.env + attack
    }

    /// One-pole high-pass at the voice's cutoff.
    fn high(&mut self, x: f32) -> f32 {
        self.lp += self.a * (x - self.lp);
        x - self.lp
    }
}

/// Moves a phase on by `inc` turns; returns the phase before (0..1).
fn advance(phase: &mut f32, inc: f32) -> f32 {
    let now = *phase;
    *phase += inc;
    while *phase >= 1.0 {
        *phase -= 1.0;
    }
    now
}

/// A seeded integer noise generator (xorshift): the same numbers on every
/// platform.
struct Noise(u32);

impl Noise {
    /// The next value in -1..1.
    fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        // 24 bits convert to f32 exactly.
        (x >> 8) as f32 / 8_388_608.0 - 1.0
    }
}

// ── Maths from the basic operations ─────────────────────────────────────

/// Sine of `turns` whole turns (1.0 = 2π), from a polynomial: within about
/// 1e-6 of the true sine, and the same on every platform.
pub(crate) fn sin_turns(turns: f32) -> f32 {
    // Into -0.5..0.5, then folded into -0.25..0.25 (a quarter turn each way).
    let mut x = turns - (turns + 0.5).floor();
    if x > 0.25 {
        x = 0.5 - x;
    } else if x < -0.25 {
        x = -0.5 - x;
    }
    let t = x * std::f32::consts::TAU;
    let t2 = t * t;
    // Taylor series to t^11 (the rest is under 1e-8 at a quarter turn).
    t * (1.0 + t2 * (-1.0 / 6.0 + t2 * (1.0 / 120.0 + t2 * (-1.0 / 5040.0 + t2 * (1.0 / 362_880.0 + t2 * (-1.0 / 39_916_800.0))))))
}

/// 2 to the power `x`, from a polynomial on the fraction and the exponent
/// bits: the same on every platform.
pub(crate) fn exp2(x: f32) -> f32 {
    let x = x.clamp(-126.0, 127.0);
    let whole = x.floor();
    let f = (x - whole) * std::f32::consts::LN_2;
    // e^f for f in 0..ln 2, Taylor series to f^8.
    let poly = 1.0
        + f * (1.0 + f * (0.5 + f * (1.0 / 6.0 + f * (1.0 / 24.0 + f * (1.0 / 120.0 + f * (1.0 / 720.0 + f * (1.0 / 5040.0 + f * (1.0 / 40_320.0))))))));
    let scale = f32::from_bits(((whole as i32 + 127) as u32) << 23);
    poly * scale
}

/// The per-sample factor of an envelope that falls by 60 dB in `seconds`.
fn decay_factor(seconds: f32, rate: f32) -> f32 {
    // log2(1000) = 9.9658.
    exp2(-9.965_784 / (seconds.max(0.0005) * rate))
}

/// The coefficient of a one-pole low-pass at `cutoff` Hz.
fn one_pole(cutoff: f32, rate: f32) -> f32 {
    if cutoff <= 0.0 {
        return 0.0;
    }
    // 1 - e^(-2π fc / rate), with e^y = 2^(y log2 e).
    1.0 - exp2(-std::f32::consts::TAU * cutoff / rate * std::f32::consts::LOG2_E)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_polynomials_follow_the_maths_library() {
        for i in -400..=400 {
            let t = i as f32 / 100.0;
            assert!((sin_turns(t) - (t * std::f32::consts::TAU).sin()).abs() < 2e-5, "sin at {t}");
            let x = i as f32 / 37.0;
            let want = x.exp2();
            assert!(((exp2(x) - want) / want).abs() < 1e-5, "exp2 at {x}");
        }
    }
}
