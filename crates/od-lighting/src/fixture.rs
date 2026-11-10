//! Fixtures: what each DMX channel of a light does, the built-in generic
//! types, a subset of the Open Fixture Library (OFL) JSON format, and the
//! patch that places fixtures on universes.

use serde::{Deserialize, Serialize};

/// What one DMX channel controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelKind {
    Dimmer,
    Red,
    Green,
    Blue,
    White,
    Amber,
    Uv,
    Pan,
    PanFine,
    Tilt,
    TiltFine,
    /// Shutter / strobe: written "open" (255) at rest, the strobe rate
    /// scaled into the range above 64 (the common layout).
    Strobe,
    /// Pan/tilt speed (0 = fastest on most movers).
    Speed,
    ColorWheel,
    Gobo,
    /// Pixel `n` of a multi-cell fixture (light bars): red, green, blue.
    PixelRed(u8),
    PixelGreen(u8),
    PixelBlue(u8),
    /// Fog output (fog machines, hazers): full while fog is on.
    Fog,
    /// Anything else; held at the given value.
    Fixed(u8),
}

/// A fixture type in one mode.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FixtureDef {
    pub manufacturer: String,
    pub model: String,
    pub mode: String,
    pub channels: Vec<ChannelKind>,
    /// Every mode of the type (including the current one), so the patch
    /// table can switch modes without the source file.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modes: Vec<FixtureMode>,
    /// A laser: its channels stay at zero until lasers are armed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub laser: bool,
}

/// One DMX mode of a fixture type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FixtureMode {
    pub name: String,
    pub channels: Vec<ChannelKind>,
}

/// Rough role used by the automatic show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FixtureRole {
    /// Static wash / par.
    Wash,
    /// Moving head (has pan/tilt).
    Mover,
    /// Multi-cell bar.
    Bar,
    /// Plain dimmer / strobe.
    Effect,
}

impl FixtureDef {
    pub fn has(&self, kind: ChannelKind) -> bool {
        self.channels.contains(&kind)
    }

    /// Fog machines and hazers (a fog channel).
    pub fn is_fog(&self) -> bool {
        self.has(ChannelKind::Fog)
    }

    /// Lasers: marked as such, or named so.
    pub fn is_laser(&self) -> bool {
        self.laser || self.model.to_lowercase().contains("laser")
    }

    /// Mode names to choose from (just the current one when none are known).
    pub fn mode_names(&self) -> Vec<String> {
        if self.modes.is_empty() {
            vec![self.mode.clone()]
        } else {
            self.modes.iter().map(|m| m.name.clone()).collect()
        }
    }

    /// Switches to the mode called `name`. False when there is no such mode.
    pub fn set_mode(&mut self, name: &str) -> bool {
        match self.modes.iter().find(|m| m.name == name) {
            Some(m) => {
                self.mode = m.name.clone();
                self.channels = m.channels.clone();
                true
            }
            None => name == self.mode,
        }
    }

    /// Sets the footprint to `count` channels (1..=512): a mode with that
    /// many channels when the type has one, otherwise the current layout cut
    /// or padded with channels held at zero ("Custom").
    pub fn set_channel_count(&mut self, count: u16) {
        let count = count.clamp(1, 512) as usize;
        if count == self.channels.len() {
            return;
        }
        if let Some(m) = self.modes.iter().find(|m| m.channels.len() == count) {
            self.mode = m.name.clone();
            self.channels = m.channels.clone();
            return;
        }
        self.channels.resize(count, ChannelKind::Fixed(0));
        self.mode = "Custom".into();
    }

    pub fn pixels(&self) -> u8 {
        self.channels
            .iter()
            .filter_map(|c| match c {
                ChannelKind::PixelRed(n) => Some(n + 1),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }

    pub fn role(&self) -> FixtureRole {
        if self.has(ChannelKind::Pan) || self.has(ChannelKind::Tilt) {
            FixtureRole::Mover
        } else if self.pixels() > 1 {
            FixtureRole::Bar
        } else if self.has(ChannelKind::Red) || self.pixels() == 1 {
            FixtureRole::Wash
        } else {
            FixtureRole::Effect
        }
    }

    /// Generic built-in types (no library file needed). The channel
    /// layouts are our own generic ones; real fixtures differ, so the patch
    /// table can switch modes and set the channel count.
    pub fn builtin(name: &str) -> Option<Self> {
        use ChannelKind::*;
        let (model, modes): (&str, Vec<(&str, Vec<ChannelKind>)>) = match name {
            "dimmer" => ("Dimmer", vec![("1ch", vec![Dimmer])]),
            "rgb-par" => (
                "RGB par",
                vec![
                    ("3ch", vec![Red, Green, Blue]),
                    ("4ch", vec![Dimmer, Red, Green, Blue]),
                    (
                        "7ch",
                        vec![Dimmer, Red, Green, Blue, Strobe, Fixed(0), Fixed(0)],
                    ),
                ],
            ),
            "rgbw-par" => (
                "RGBW par",
                vec![
                    ("6ch", vec![Dimmer, Red, Green, Blue, White, Strobe]),
                    ("4ch", vec![Red, Green, Blue, White]),
                    (
                        "8ch",
                        vec![Dimmer, Red, Green, Blue, White, Strobe, Fixed(0), Fixed(0)],
                    ),
                ],
            ),
            "moving-head" => (
                "Moving head",
                vec![
                    (
                        "14ch",
                        vec![
                            Pan,
                            PanFine,
                            Tilt,
                            TiltFine,
                            Speed,
                            Dimmer,
                            Strobe,
                            Red,
                            Green,
                            Blue,
                            White,
                            ColorWheel,
                            Gobo,
                            Fixed(0),
                        ],
                    ),
                    (
                        "9ch",
                        vec![Pan, Tilt, Speed, Dimmer, Strobe, Red, Green, Blue, White],
                    ),
                ],
            ),
            "light-bar-8" => {
                let mut ch = vec![Dimmer, Strobe];
                for n in 0..8 {
                    ch.extend([PixelRed(n), PixelGreen(n), PixelBlue(n)]);
                }
                (
                    "Light bar 8 × RGB",
                    vec![("26ch", ch), ("3ch", vec![Red, Green, Blue])],
                )
            }
            "strobe" => (
                "Strobe",
                vec![("2ch", vec![Dimmer, Strobe]), ("1ch", vec![Dimmer])],
            ),
            "blinder" => (
                "Blinder 2 × warm white",
                vec![("2ch", vec![Dimmer, Dimmer])],
            ),
            "laser" => ("Laser", vec![("3ch", vec![Dimmer, Fixed(0), Speed])]),
            "fog" => (
                "Fog machine",
                vec![("1ch", vec![Fog]), ("2ch", vec![Fog, Fixed(0)])],
            ),
            _ => return None,
        };
        let modes: Vec<FixtureMode> = modes
            .into_iter()
            .map(|(n, channels)| FixtureMode {
                name: n.into(),
                channels,
            })
            .collect();
        Some(Self {
            manufacturer: "Generic".into(),
            model: model.into(),
            mode: modes[0].name.clone(),
            channels: modes[0].channels.clone(),
            modes,
            laser: name == "laser",
        })
    }

    pub const BUILTIN: [&'static str; 9] = [
        "dimmer",
        "rgb-par",
        "rgbw-par",
        "moving-head",
        "light-bar-8",
        "strobe",
        "blinder",
        "laser",
        "fog",
    ];

    /// Reads one mode of an Open Fixture Library fixture (`.json`). Channels
    /// whose capability is not understood become `Fixed(0)`, so the address
    /// footprint stays right.
    pub fn from_ofl(
        json: &str,
        manufacturer: &str,
        mode: Option<&str>,
    ) -> Result<Self, crate::Error> {
        let v: serde_json::Value =
            serde_json::from_str(json).map_err(|e| crate::Error::Fixture(e.to_string()))?;
        let model = v["name"].as_str().unwrap_or("Fixture").to_string();
        let modes = v["modes"]
            .as_array()
            .ok_or_else(|| crate::Error::Fixture("no modes".into()))?;
        let m = match mode {
            Some(name) => modes
                .iter()
                .find(|m| m["name"].as_str() == Some(name) || m["shortName"].as_str() == Some(name))
                .ok_or_else(|| crate::Error::Fixture(format!("no mode {name}")))?,
            None => modes
                .first()
                .ok_or_else(|| crate::Error::Fixture("no modes".into()))?,
        };
        let available = &v["availableChannels"];
        let channels_of = |m: &serde_json::Value| -> Vec<ChannelKind> {
            m["channels"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|key| match key.as_str() {
                    Some(key) => ofl_channel(key, available),
                    None => ChannelKind::Fixed(0), // null = unused slot
                })
                .collect()
        };
        let laser = v["categories"]
            .as_array()
            .is_some_and(|c| c.iter().any(|x| x.as_str() == Some("Laser")));
        Ok(Self {
            manufacturer: manufacturer.into(),
            model,
            mode: m["name"].as_str().unwrap_or("").into(),
            channels: channels_of(m),
            modes: modes
                .iter()
                .map(|m| FixtureMode {
                    name: m["name"].as_str().unwrap_or("").into(),
                    channels: channels_of(m),
                })
                .collect(),
            laser,
        })
    }
}

fn ofl_channel(key: &str, available: &serde_json::Value) -> ChannelKind {
    use ChannelKind::*;
    // Fine channels are listed as aliases ("Pan fine") of the coarse one.
    for (name, ch) in available.as_object().into_iter().flatten() {
        let aliases = ch["fineChannelAliases"].as_array();
        if aliases.is_some_and(|a| a.iter().any(|x| x.as_str() == Some(key))) {
            return match ofl_channel(name, available) {
                Pan => PanFine,
                Tilt => TiltFine,
                _ => Fixed(0),
            };
        }
    }
    let ch = &available[key];
    let cap = if ch["capability"].is_object() {
        &ch["capability"]
    } else {
        &ch["capabilities"][0]
    };
    let default = ch["defaultValue"]
        .as_u64()
        .map(|v| v.min(255) as u8)
        .unwrap_or(0);
    match cap["type"].as_str().unwrap_or("") {
        "Intensity" => Dimmer,
        "ColorIntensity" => match cap["color"].as_str().unwrap_or("") {
            "Red" => Red,
            "Green" => Green,
            "Blue" => Blue,
            "White" | "Warm White" | "Cold White" => White,
            "Amber" => Amber,
            "UV" => Uv,
            _ => Fixed(0),
        },
        "Pan" => Pan,
        "Tilt" => Tilt,
        "ShutterStrobe" => Strobe,
        "PanTiltSpeed" => Speed,
        "WheelSlot" | "ColorPreset" if key.to_lowercase().contains("color") => ColorWheel,
        "WheelSlot" if key.to_lowercase().contains("gobo") => Gobo,
        _ => Fixed(default),
    }
}

/// One fixture placed on the rig.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Patched {
    pub name: String,
    pub def: FixtureDef,
    /// Universe index (0-based).
    pub universe: u16,
    /// First channel, 1..=512.
    pub address: u16,
    /// Group the show addresses ("Front wash", "Movers", …).
    pub group: String,
    /// Mirror pan (for movers hung on the opposite side).
    #[serde(default)]
    pub invert_pan: bool,
}

/// Universes the patch can use.
pub const MAX_UNIVERSES: u16 = 64;

/// Why a fixture's place is wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Collision {
    /// Shares channels with the fixture at this index.
    Overlaps(usize),
    /// Its channels run past 512 (or it starts at 0).
    OutOfRange,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    pub fixtures: Vec<Patched>,
}

impl Patch {
    /// Adds a fixture after checking its footprint fits and does not overlap.
    pub fn add(&mut self, fixture: Patched) -> Result<usize, crate::Error> {
        let len = fixture.def.channels.len() as u16;
        if fixture.address == 0 || fixture.address + len - 1 > 512 {
            return Err(crate::Error::Patch(format!(
                "{} does not fit at {}",
                fixture.name, fixture.address
            )));
        }
        let range = fixture.address..fixture.address + len;
        if let Some(other) = self.fixtures.iter().find(|f| {
            f.universe == fixture.universe
                && f.address < range.end
                && range.start < f.address + f.def.channels.len() as u16
        }) {
            return Err(crate::Error::Patch(format!(
                "{} overlaps {}",
                fixture.name, other.name
            )));
        }
        self.fixtures.push(fixture);
        Ok(self.fixtures.len() - 1)
    }

    /// Next free address on a universe for a footprint of `len` channels.
    pub fn next_free(&self, universe: u16, len: u16) -> Option<u16> {
        let mut start = 1;
        loop {
            if start + len - 1 > 512 {
                return None;
            }
            match self.fixtures.iter().find(|f| {
                f.universe == universe
                    && f.address < start + len
                    && start < f.address + f.def.channels.len() as u16
            }) {
                Some(f) => start = f.address + f.def.channels.len() as u16,
                None => return Some(start),
            }
        }
    }

    /// What is wrong with each fixture's place (same order as `fixtures`).
    pub fn collisions(&self) -> Vec<Option<Collision>> {
        self.fixtures
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let len = f.def.channels.len() as u16;
                if f.address == 0 || f.address as u32 + len as u32 - 1 > 512 {
                    return Some(Collision::OutOfRange);
                }
                self.fixtures
                    .iter()
                    .enumerate()
                    .find(|(j, o)| {
                        *j != i
                            && o.universe == f.universe
                            && o.address < f.address + len
                            && f.address < o.address + o.def.channels.len() as u16
                    })
                    .map(|(j, _)| Collision::Overlaps(j))
            })
            .collect()
    }

    /// Moves fixture `index` (no overlap check: the patch table shows
    /// collisions instead of refusing the edit). Universe 0..64, address
    /// 1..=512.
    pub fn set_place(&mut self, index: usize, universe: u16, address: u16) -> bool {
        match self.fixtures.get_mut(index) {
            Some(f) if universe < MAX_UNIVERSES && (1..=512).contains(&address) => {
                f.universe = universe;
                f.address = address;
                true
            }
            _ => false,
        }
    }

    /// Patches `count` fixtures of `def` one after another, starting at
    /// `start` on `universe` (or the first free run of addresses). All or
    /// nothing: nothing is added when one would not fit or would overlap.
    pub fn add_many(
        &mut self,
        def: &FixtureDef,
        count: u16,
        universe: Option<u16>,
        start: Option<u16>,
        name: &str,
        group: &str,
    ) -> Result<std::ops::Range<usize>, crate::Error> {
        let count = count.max(1);
        let len = def.channels.len() as u16;
        let total = len as u32 * count as u32;
        if total > 512 {
            return Err(crate::Error::Patch(format!(
                "{count} × {len} channels do not fit in one universe"
            )));
        }
        let fits = |p: &Patch, u: u16, a: u16| {
            a >= 1
                && a as u32 + total - 1 <= 512
                && !p.fixtures.iter().any(|f| {
                    f.universe == u
                        && (f.address as u32) < a as u32 + total
                        && (a as u32) < f.address as u32 + f.def.channels.len() as u32
                })
        };
        let (universe, start) = match (universe, start) {
            (Some(u), Some(a)) => {
                if !fits(self, u, a) {
                    let clash = self.fixtures.iter().find(|f| {
                        f.universe == u
                            && (f.address as u32) < a as u32 + total
                            && (a as u32) < f.address as u32 + f.def.channels.len() as u32
                    });
                    return Err(crate::Error::Patch(match clash {
                        Some(f) => format!(
                            "addresses {a} to {} on universe {} overlap {}",
                            a as u32 + total - 1,
                            u + 1,
                            f.name
                        ),
                        None => format!("addresses {a} to {} do not fit", a as u32 + total - 1),
                    }));
                }
                (u, a)
            }
            (u, a) => {
                let universes: Vec<u16> = match u {
                    Some(u) => vec![u],
                    None => (0..MAX_UNIVERSES).collect(),
                };
                universes
                    .into_iter()
                    .find_map(|u| {
                        let mut s = a.unwrap_or(1);
                        loop {
                            if s as u32 + total - 1 > 512 {
                                return None;
                            }
                            if fits(self, u, s) {
                                return Some((u, s));
                            }
                            s += 1;
                        }
                    })
                    .ok_or_else(|| crate::Error::Patch("no free run of DMX addresses".into()))?
            }
        };
        let first = self.fixtures.len();
        let same = self
            .fixtures
            .iter()
            .filter(|f| f.def.model == def.model)
            .count();
        for n in 0..count {
            let name = if name.trim().is_empty() {
                format!("{} {}", def.model, same + n as usize + 1)
            } else if count == 1 {
                name.trim().to_string()
            } else {
                format!("{} {}", name.trim(), n + 1)
            };
            self.fixtures.push(Patched {
                name,
                def: def.clone(),
                universe,
                address: start + n * len,
                group: group.to_string(),
                invert_pan: false,
            });
        }
        Ok(first..self.fixtures.len())
    }

    pub fn universes(&self) -> u16 {
        self.fixtures
            .iter()
            .map(|f| f.universe + 1)
            .max()
            .unwrap_or(0)
    }

    pub fn groups(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for f in &self.fixtures {
            if !out.contains(&f.group) {
                out.push(f.group.clone());
            }
        }
        out
    }

    pub fn load(path: &std::path::Path) -> Result<Self, crate::Error> {
        let text = std::fs::read_to_string(path).map_err(|e| crate::Error::Patch(e.to_string()))?;
        serde_json::from_str(&text).map_err(|e| crate::Error::Patch(e.to_string()))
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), crate::Error> {
        let text =
            serde_json::to_string_pretty(self).map_err(|e| crate::Error::Patch(e.to_string()))?;
        std::fs::write(path, text).map_err(|e| crate::Error::Patch(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OFL: &str = r#"{
      "name": "Spot 60",
      "availableChannels": {
        "Pan": { "fineChannelAliases": ["Pan fine"], "capability": { "type": "Pan", "angleStart": "0deg", "angleEnd": "540deg" } },
        "Tilt": { "capability": { "type": "Tilt" } },
        "Dimmer": { "capability": { "type": "Intensity" } },
        "Shutter": { "capabilities": [ { "dmxRange": [0, 7], "type": "ShutterStrobe", "shutterEffect": "Closed" } ] },
        "Color Wheel": { "capabilities": [ { "dmxRange": [0, 9], "type": "WheelSlot" } ] },
        "Red": { "capability": { "type": "ColorIntensity", "color": "Red" } },
        "Macro": { "defaultValue": 12, "capability": { "type": "Effect" } }
      },
      "modes": [
        { "name": "Basic", "channels": ["Pan", "Tilt", "Dimmer"] },
        { "name": "Extended", "channels": ["Pan", "Pan fine", "Tilt", "Dimmer", "Shutter", "Color Wheel", "Red", null, "Macro"] }
      ]
    }"#;

    #[test]
    fn reads_an_ofl_mode() {
        use ChannelKind::*;
        let def = FixtureDef::from_ofl(OFL, "Acme", Some("Extended")).unwrap();
        assert_eq!(
            def.channels,
            vec![
                Pan,
                PanFine,
                Tilt,
                Dimmer,
                Strobe,
                ColorWheel,
                Red,
                Fixed(0),
                Fixed(12)
            ]
        );
        assert_eq!(def.role(), FixtureRole::Mover);
        assert_eq!(
            FixtureDef::from_ofl(OFL, "Acme", None)
                .unwrap()
                .channels
                .len(),
            3
        );
        assert!(FixtureDef::from_ofl(OFL, "Acme", Some("Nope")).is_err());
    }

    #[test]
    fn builtins_have_roles() {
        let role = |n| FixtureDef::builtin(n).unwrap().role();
        assert_eq!(role("moving-head"), FixtureRole::Mover);
        assert_eq!(role("light-bar-8"), FixtureRole::Bar);
        assert_eq!(role("rgbw-par"), FixtureRole::Wash);
        assert_eq!(role("strobe"), FixtureRole::Effect);
        assert_eq!(
            FixtureDef::builtin("light-bar-8").unwrap().channels.len(),
            26
        );
    }

    #[test]
    fn patch_rejects_overlaps_and_finds_space() {
        let mut patch = Patch::default();
        let par = FixtureDef::builtin("rgbw-par").unwrap();
        let fixture = |name: &str, address| Patched {
            name: name.into(),
            def: par.clone(),
            universe: 0,
            address,
            group: "Wash".into(),
            invert_pan: false,
        };
        patch.add(fixture("A", 1)).unwrap();
        assert!(patch.add(fixture("B", 6)).is_err());
        assert!(patch.add(fixture("C", 510)).is_err());
        assert_eq!(patch.next_free(0, 6), Some(7));
        patch.add(fixture("D", 7)).unwrap();
        assert_eq!(patch.groups(), vec!["Wash".to_string()]);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rig.json");
        patch.save(&path).unwrap();
        assert_eq!(Patch::load(&path).unwrap(), patch);
    }

    #[test]
    fn modes_and_channel_count() {
        let mut def = FixtureDef::builtin("rgb-par").unwrap();
        assert_eq!(def.mode_names(), vec!["3ch", "4ch", "7ch"]);
        assert!(def.set_mode("7ch"));
        assert_eq!(def.channels.len(), 7);
        assert!(def.has(ChannelKind::Strobe));
        def.set_channel_count(4);
        assert_eq!(
            (def.mode.as_str(), def.channels[0]),
            ("4ch", ChannelKind::Dimmer)
        );
        def.set_channel_count(10);
        assert_eq!(def.mode, "Custom");
        assert_eq!(def.channels.len(), 10);
        assert_eq!(def.channels[9], ChannelKind::Fixed(0));
        assert!(!def.set_mode("99ch"));
        assert!(FixtureDef::builtin("laser").unwrap().is_laser());
        assert!(FixtureDef::builtin("fog").unwrap().is_fog());
        let ofl = FixtureDef::from_ofl(OFL, "Acme", Some("Basic")).unwrap();
        assert_eq!(ofl.mode_names(), vec!["Basic", "Extended"]);
        // Old rig files without modes still read.
        let old: FixtureDef = serde_json::from_str(
            r#"{"manufacturer":"G","model":"Par","mode":"3ch","channels":["Red","Green","Blue"]}"#,
        )
        .unwrap();
        assert_eq!(old.mode_names(), vec!["3ch"]);
    }

    #[test]
    fn bulk_add_and_collisions() {
        let par = FixtureDef::builtin("rgbw-par").unwrap();
        let mut patch = Patch::default();
        let r = patch
            .add_many(&par, 4, Some(0), Some(1), "Front", "Wash")
            .unwrap();
        assert_eq!(r, 0..4);
        let addresses: Vec<u16> = patch.fixtures.iter().map(|f| f.address).collect();
        assert_eq!(addresses, vec![1, 7, 13, 19]);
        assert_eq!(patch.fixtures[3].name, "Front 4");
        // Refused as a whole when it would overlap.
        let err = patch
            .add_many(&par, 2, Some(0), Some(20), "", "Wash")
            .unwrap_err()
            .to_string();
        assert!(err.contains("Front 4"), "{err}");
        assert_eq!(patch.fixtures.len(), 4);
        // Without a start: the next free run.
        let r = patch.add_many(&par, 2, Some(0), None, "", "Wash").unwrap();
        assert_eq!(patch.fixtures[r.start].address, 25);
        assert!(patch.collisions().iter().all(Option::is_none));
        // An edit that overlaps is kept and shown.
        assert!(patch.set_place(1, 0, 3));
        let c = patch.collisions();
        assert_eq!(c[1], Some(Collision::Overlaps(0)));
        assert_eq!(c[0], Some(Collision::Overlaps(1)));
        assert!(patch.set_place(2, 0, 510));
        assert_eq!(patch.collisions()[2], Some(Collision::OutOfRange));
        assert!(!patch.set_place(2, 0, 0));
        assert!(!patch.set_place(2, MAX_UNIVERSES, 1));
        assert!(patch.add_many(&par, 90, None, None, "", "W").is_err());
    }
}
