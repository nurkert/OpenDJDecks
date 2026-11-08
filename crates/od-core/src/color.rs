use serde::{Deserialize, Serialize};

/// Colour of a cue or track as RGB.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CueColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl CueColor {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.strip_prefix('#').unwrap_or(s);
        if s.len() != 6 {
            return None;
        }
        let v = u32::from_str_radix(s, 16).ok()?;
        Some(Self::rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
    }

    /// Index of the palette entry closest to this colour.
    pub fn nearest(self, palette: &[CueColor]) -> usize {
        let dist = |c: &CueColor| {
            let dr = self.r as i32 - c.r as i32;
            let dg = self.g as i32 - c.g as i32;
            let db = self.b as i32 - c.b as i32;
            dr * dr + dg * dg + db * db
        };
        palette
            .iter()
            .enumerate()
            .min_by_key(|(_, c)| dist(c))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let c = CueColor::rgb(0x12, 0xab, 0xff);
        assert_eq!(CueColor::from_hex(&c.to_hex()), Some(c));
        assert_eq!(CueColor::from_hex("zzz"), None);
    }

    #[test]
    fn nearest_palette_entry() {
        let p = [CueColor::rgb(255, 0, 0), CueColor::rgb(0, 255, 0)];
        assert_eq!(CueColor::rgb(10, 200, 30).nearest(&p), 1);
    }
}

/// The eight track colours offered in the browser, with their names.
pub const TRACK_COLORS: [(&str, CueColor); 8] = [
    ("Pink", CueColor::rgb(0xf8, 0x70, 0xf8)),
    ("Red", CueColor::rgb(0xf8, 0x2c, 0x30)),
    ("Orange", CueColor::rgb(0xf8, 0x80, 0x00)),
    ("Yellow", CueColor::rgb(0xf8, 0xe3, 0x31)),
    ("Green", CueColor::rgb(0x10, 0xe0, 0x20)),
    ("Aqua", CueColor::rgb(0x10, 0xb0, 0xf8)),
    ("Blue", CueColor::rgb(0x30, 0x5a, 0xff)),
    ("Purple", CueColor::rgb(0x91, 0x48, 0xfc)),
];
