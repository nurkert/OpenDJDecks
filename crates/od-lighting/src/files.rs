//! Named rig and show files and venue profiles (roadmap [35]).
//!
//! Next to the working rig (`lighting.json`, which the app and the MCP
//! server keep using) a `lighting/` folder holds:
//!
//! * `rigs/`: the patch and the outputs, by name;
//! * `shows/`: looks, pads, chases, cue lists, the AUTO table and palettes
//!   (the whole [`ShowSettings`]), by name;
//! * `venues/`: a venue profile binds a rig to a show plus the safety
//!   limits and the failsafe, picked on arrival at a venue.
//!
//! [`Bundle`] carries a venue with its rig and show in one file for import
//! and export. A rig file from before this step is migrated into a
//! "Default" rig, show and venue the first time the folder is opened.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::show::ShowSettings;
use crate::{Error, Failsafe, OutputKind, Patch, RigConfig, SafetyLimits};

/// Format marker of a [`Bundle`] file.
pub const BUNDLE_FORMAT: &str = "opendjdecks-lighting";
/// Name given to what a rig file from before named files becomes.
pub const DEFAULT_NAME: &str = "Default";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileKind {
    Venue,
    Rig,
    Show,
}

impl FileKind {
    pub const ALL: [FileKind; 3] = [FileKind::Venue, FileKind::Rig, FileKind::Show];

    fn folder(self) -> &'static str {
        match self {
            FileKind::Venue => "venues",
            FileKind::Rig => "rigs",
            FileKind::Show => "shows",
        }
    }
}

/// The names in use: which venue, rig and show the working rig came from
/// (or was last saved as).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InUse {
    #[serde(default)]
    pub venue: String,
    #[serde(default)]
    pub rig: String,
    #[serde(default)]
    pub show: String,
}

impl InUse {
    pub fn get(&self, kind: FileKind) -> &str {
        match kind {
            FileKind::Venue => &self.venue,
            FileKind::Rig => &self.rig,
            FileKind::Show => &self.show,
        }
    }

    fn set(&mut self, kind: FileKind, name: &str) {
        let slot = match kind {
            FileKind::Venue => &mut self.venue,
            FileKind::Rig => &mut self.rig,
            FileKind::Show => &mut self.show,
        };
        *slot = name.to_string();
    }
}

/// A named rig: the patch and the outputs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RigFile {
    pub name: String,
    pub patch: Patch,
    #[serde(default)]
    pub outputs: Vec<(u16, OutputKind)>,
}

/// A named show: everything the show plays.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ShowFile {
    pub name: String,
    pub settings: ShowSettings,
}

/// A venue: a rig, a show and the limits that hold there.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VenueProfile {
    pub name: String,
    pub rig: String,
    pub show: String,
    #[serde(default)]
    pub safety: SafetyLimits,
    #[serde(default)]
    pub failsafe: Failsafe,
}

/// One file with a venue, its rig and its show (import and export).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bundle {
    pub format: String,
    pub version: u32,
    pub venue: VenueProfile,
    pub rig: RigFile,
    pub show: ShowFile,
}

/// The `lighting/` folder of a library.
#[derive(Clone, Debug)]
pub struct Files {
    dir: PathBuf,
}

/// A name as a file stem: letters and digits kept, the rest a dash.
fn slug(name: &str) -> String {
    let s: String = name
        .trim()
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "unnamed".into() } else { s }
}

fn err(e: impl std::fmt::Display) -> Error {
    Error::Patch(e.to_string())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), Error> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(value).map_err(err)?;
    let tmp = path.with_extension("json.part");
    std::fs::write(&tmp, text)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, Error> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(err)
}

impl Files {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The folder beside a working rig file (`…/lighting.json` →
    /// `…/lighting/`).
    pub fn beside(rig: &Path) -> Self {
        Self::new(rig.with_file_name("lighting"))
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path(&self, kind: FileKind, name: &str) -> PathBuf {
        self.dir
            .join(kind.folder())
            .join(format!("{}.json", slug(name)))
    }

    pub fn exists(&self, kind: FileKind, name: &str) -> bool {
        self.path(kind, name).exists()
    }

    /// Names of `kind`, sorted.
    pub fn list(&self, kind: FileKind) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.dir.join(kind.folder()))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .filter_map(|p| {
                let v: serde_json::Value = read_json(&p).ok()?;
                v["name"].as_str().map(str::to_string)
            })
            .collect();
        names.sort_by_key(|n| n.to_lowercase());
        names
    }

    pub fn rig(&self, name: &str) -> Result<RigFile, Error> {
        read_json(&self.path(FileKind::Rig, name))
    }

    pub fn show(&self, name: &str) -> Result<ShowFile, Error> {
        read_json(&self.path(FileKind::Show, name))
    }

    pub fn venue(&self, name: &str) -> Result<VenueProfile, Error> {
        read_json(&self.path(FileKind::Venue, name))
    }

    /// A free name: `base`, else "base 2", "base 3", …
    pub fn unique(&self, kind: FileKind, base: &str) -> String {
        let base = if base.trim().is_empty() { DEFAULT_NAME } else { base.trim() };
        if !self.exists(kind, base) {
            return base.to_string();
        }
        (2..)
            .map(|n| format!("{base} {n}"))
            .find(|n| !self.exists(kind, n))
            .expect("a free name")
    }

    /// Saves what `config` holds of `kind` under `name` and marks it in
    /// use. A venue also saves its rig and show (under the names in use,
    /// or its own name).
    pub fn save(&self, kind: FileKind, name: &str, config: &mut RigConfig) -> Result<(), Error> {
        let name = name.trim();
        if name.is_empty() {
            return Err(err("a name is needed"));
        }
        match kind {
            FileKind::Rig => write_json(
                &self.path(kind, name),
                &RigFile {
                    name: name.into(),
                    patch: config.patch.clone(),
                    outputs: config.outputs.clone(),
                },
            )?,
            FileKind::Show => write_json(
                &self.path(kind, name),
                &ShowFile {
                    name: name.into(),
                    settings: config.settings.clone(),
                },
            )?,
            FileKind::Venue => {
                // An existing venue keeps its own rig and show files; a new
                // one reuses the rig and show in use when they are saved as
                // they are, and otherwise gets its own (never overwriting
                // what another venue uses).
                let (rig, show) = match self.venue(name) {
                    Ok(v) => (v.rig, v.show),
                    Err(_) => {
                        let rig = match self.rig(&config.in_use.rig) {
                            Ok(r) if r.patch == config.patch && r.outputs == config.outputs => r.name,
                            _ => self.unique(FileKind::Rig, name),
                        };
                        let show = match self.show(&config.in_use.show) {
                            Ok(s) if s.settings == config.settings => s.name,
                            _ => self.unique(FileKind::Show, name),
                        };
                        (rig, show)
                    }
                };
                self.save(FileKind::Rig, &rig, config)?;
                self.save(FileKind::Show, &show, config)?;
                write_json(
                    &self.path(kind, name),
                    &VenueProfile {
                        name: name.into(),
                        rig,
                        show,
                        safety: config.safety.clone(),
                        failsafe: config.failsafe,
                    },
                )?;
            }
        }
        config.in_use.set(kind, name);
        Ok(())
    }

    /// Loads `name` into `config`: a rig replaces patch and outputs, a show
    /// the show settings, a venue its rig, its show, the safety limits and
    /// the failsafe.
    pub fn load(&self, kind: FileKind, name: &str, config: &mut RigConfig) -> Result<(), Error> {
        match kind {
            FileKind::Rig => {
                let r = self.rig(name)?;
                config.patch = r.patch;
                config.outputs = r.outputs;
                config.in_use.rig = r.name;
            }
            FileKind::Show => {
                let s = self.show(name)?;
                config.settings = s.settings;
                config.in_use.show = s.name;
            }
            FileKind::Venue => {
                let v = self.venue(name)?;
                // Both first, so a missing one changes nothing.
                let (r, s) = (self.rig(&v.rig)?, self.show(&v.show)?);
                config.patch = r.patch;
                config.outputs = r.outputs;
                config.settings = s.settings;
                config.safety = v.safety;
                config.failsafe = v.failsafe;
                config.in_use = InUse {
                    venue: v.name,
                    rig: r.name,
                    show: s.name,
                };
            }
        }
        Ok(())
    }

    /// Renames `old` to `new`; venues that use a renamed rig or show follow,
    /// and so do the names in use.
    pub fn rename(
        &self,
        kind: FileKind,
        old: &str,
        new: &str,
        config: &mut RigConfig,
    ) -> Result<(), Error> {
        let new = new.trim();
        if new.is_empty() {
            return Err(err("a name is needed"));
        }
        if slug(new) != slug(old) && self.exists(kind, new) {
            return Err(err(format!("{new} exists already")));
        }
        let mut v: serde_json::Value = read_json(&self.path(kind, old))?;
        v["name"] = serde_json::Value::String(new.into());
        std::fs::remove_file(self.path(kind, old))?;
        write_json(&self.path(kind, new), &v)?;
        if kind != FileKind::Venue {
            for venue in self.list(FileKind::Venue) {
                let mut p = self.venue(&venue)?;
                let field = if kind == FileKind::Rig { &mut p.rig } else { &mut p.show };
                if field == old {
                    *field = new.into();
                    write_json(&self.path(FileKind::Venue, &venue), &p)?;
                }
            }
        }
        if config.in_use.get(kind) == old {
            config.in_use.set(kind, new);
        }
        Ok(())
    }

    /// Copies `name` to a free name ("name 2"); returns it.
    pub fn duplicate(&self, kind: FileKind, name: &str) -> Result<String, Error> {
        let mut v: serde_json::Value = read_json(&self.path(kind, name))?;
        let copy = self.unique(kind, name);
        v["name"] = serde_json::Value::String(copy.clone());
        write_json(&self.path(kind, &copy), &v)?;
        Ok(copy)
    }

    pub fn delete(&self, kind: FileKind, name: &str, config: &mut RigConfig) -> Result<(), Error> {
        std::fs::remove_file(self.path(kind, name))?;
        if config.in_use.get(kind) == name {
            config.in_use.set(kind, "");
        }
        Ok(())
    }

    /// The first time: the working rig becomes the "Default" rig, show and
    /// venue. True when something was written.
    pub fn migrate(&self, config: &mut RigConfig) -> Result<bool, Error> {
        if !self.list(FileKind::Venue).is_empty() || !config.in_use.venue.is_empty() {
            return Ok(false);
        }
        self.save(FileKind::Venue, DEFAULT_NAME, config)?;
        Ok(true)
    }

    /// The working rig as one file: venue, rig and show under the names in
    /// use.
    pub fn bundle(config: &RigConfig) -> Bundle {
        let venue = non_empty(&config.in_use.venue, DEFAULT_NAME);
        let rig = non_empty(&config.in_use.rig, &venue);
        let show = non_empty(&config.in_use.show, &venue);
        Bundle {
            format: BUNDLE_FORMAT.into(),
            version: 1,
            venue: VenueProfile {
                name: venue,
                rig: rig.clone(),
                show: show.clone(),
                safety: config.safety.clone(),
                failsafe: config.failsafe,
            },
            rig: RigFile {
                name: rig,
                patch: config.patch.clone(),
                outputs: config.outputs.clone(),
            },
            show: ShowFile {
                name: show,
                settings: config.settings.clone(),
            },
        }
    }

    pub fn export(config: &RigConfig, path: &Path) -> Result<(), Error> {
        write_json(path, &Self::bundle(config))
    }

    /// Reads a bundle, stores its venue, rig and show (a name taken by
    /// something else gets a number) and loads the venue into `config`.
    /// Returns the venue's name.
    pub fn import(&self, path: &Path, config: &mut RigConfig) -> Result<String, Error> {
        let b: Bundle = read_json(path)?;
        if b.format != BUNDLE_FORMAT {
            return Err(err("not an OpenDJDecks lighting file"));
        }
        let place = |kind: FileKind, name: &str, same: bool| -> String {
            if same { name.to_string() } else { self.unique(kind, name) }
        };
        let rig_name = place(
            FileKind::Rig,
            &b.rig.name,
            self.rig(&b.rig.name).is_ok_and(|r| r.patch == b.rig.patch && r.outputs == b.rig.outputs),
        );
        let show_name = place(
            FileKind::Show,
            &b.show.name,
            self.show(&b.show.name).is_ok_and(|s| s.settings == b.show.settings),
        );
        let venue_name = place(FileKind::Venue, &b.venue.name, false);
        write_json(
            &self.path(FileKind::Rig, &rig_name),
            &RigFile { name: rig_name.clone(), ..b.rig },
        )?;
        write_json(
            &self.path(FileKind::Show, &show_name),
            &ShowFile { name: show_name.clone(), ..b.show },
        )?;
        write_json(
            &self.path(FileKind::Venue, &venue_name),
            &VenueProfile {
                name: venue_name.clone(),
                rig: rig_name,
                show: show_name,
                ..b.venue
            },
        )?;
        self.load(FileKind::Venue, &venue_name, config)?;
        Ok(venue_name)
    }
}

fn non_empty(name: &str, fallback: &str) -> String {
    if name.trim().is_empty() { fallback.to_string() } else { name.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FixtureDef, Mode};

    fn config() -> RigConfig {
        let mut c = RigConfig::default();
        let par = FixtureDef::builtin("rgbw-par").unwrap();
        c.patch.add_many(&par, 2, Some(0), Some(1), "Par", "Wash").unwrap();
        c.outputs.push((0, OutputKind::Sacn { universe: 1 }));
        c.settings.mode = Mode::Pro;
        c.safety.strobe_hz = 2.0;
        c
    }

    #[test]
    fn old_rig_file_migrates_into_a_default_profile() {
        let dir = tempfile::tempdir().unwrap();
        let rig = dir.path().join("lighting.json");
        // A rig file written before named files existed.
        let mut old = serde_json::to_value(config()).unwrap();
        old.as_object_mut().unwrap().remove("in_use");
        std::fs::write(&rig, old.to_string()).unwrap();
        let mut c = RigConfig::load(&rig).unwrap();
        assert_eq!(c.in_use, InUse::default());
        let files = Files::beside(&rig);
        assert!(files.migrate(&mut c).unwrap());
        assert!(!files.migrate(&mut c).unwrap(), "only once");
        assert_eq!(files.list(FileKind::Venue), vec!["Default"]);
        assert_eq!(files.list(FileKind::Rig), vec!["Default"]);
        assert_eq!(files.list(FileKind::Show), vec!["Default"]);
        assert_eq!(c.in_use.venue, "Default");
        let v = files.venue("Default").unwrap();
        assert_eq!((v.rig.as_str(), v.show.as_str(), v.safety.strobe_hz), ("Default", "Default", 2.0));
        assert_eq!(files.rig("Default").unwrap().patch, c.patch);
    }

    #[test]
    fn save_load_rename_duplicate_delete() {
        let dir = tempfile::tempdir().unwrap();
        let files = Files::new(dir.path());
        let mut c = config();
        files.save(FileKind::Rig, "Club rig", &mut c).unwrap();
        files.save(FileKind::Show, "Friday", &mut c).unwrap();
        files.save(FileKind::Venue, "Club", &mut c).unwrap();
        assert_eq!(c.in_use, InUse { venue: "Club".into(), rig: "Club rig".into(), show: "Friday".into() });
        // A new venue with a changed rig gets its own rig file; the club's
        // stays as it was.
        let mut changed = c.clone();
        changed.patch.fixtures.pop();
        files.save(FileKind::Venue, "Bar", &mut changed).unwrap();
        assert_eq!(changed.in_use.rig, "Bar");
        assert_eq!(changed.in_use.show, "Friday", "the show is unchanged: reused");
        assert_eq!(files.rig("Club rig").unwrap().patch.fixtures.len(), 2);
        files.delete(FileKind::Venue, "Bar", &mut changed).unwrap();
        // A different show and rig, then back to the venue.
        let mut other = RigConfig::default();
        files.save(FileKind::Rig, "Bar rig", &mut other).unwrap();
        files.load(FileKind::Rig, "Bar rig", &mut c).unwrap();
        assert!(c.patch.fixtures.is_empty());
        c.safety.strobe_hz = 5.0;
        files.load(FileKind::Venue, "Club", &mut c).unwrap();
        assert_eq!(c.patch.fixtures.len(), 2);
        assert_eq!(c.safety.strobe_hz, 2.0, "the venue's limits");
        assert_eq!(c.settings.mode, Mode::Pro);
        // Rename a show: the venue follows.
        files.rename(FileKind::Show, "Friday", "Saturday", &mut c).unwrap();
        assert_eq!(files.venue("Club").unwrap().show, "Saturday");
        assert_eq!(c.in_use.show, "Saturday");
        assert!(files.rename(FileKind::Rig, "Bar rig", "Club rig", &mut c).is_err(), "taken");
        // Duplicate and delete.
        assert_eq!(files.duplicate(FileKind::Venue, "Club").unwrap(), "Club 2");
        assert_eq!(files.list(FileKind::Venue), vec!["Club", "Club 2"]);
        files.delete(FileKind::Venue, "Club", &mut c).unwrap();
        assert_eq!(files.list(FileKind::Venue), vec!["Club 2"]);
        assert_eq!(c.in_use.venue, "");
        files.load(FileKind::Venue, "Club 2", &mut c).unwrap();
        assert_eq!(c.in_use.venue, "Club 2");
    }

    #[test]
    fn one_file_export_and_import() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = config();
        c.in_use.venue = "Harbour".into();
        let path = dir.path().join("harbour.json");
        Files::export(&c, &path).unwrap();
        let files = Files::new(dir.path().join("other"));
        let mut fresh = RigConfig::default();
        assert_eq!(files.import(&path, &mut fresh).unwrap(), "Harbour");
        assert_eq!(fresh.patch, c.patch);
        assert_eq!(fresh.settings, c.settings);
        assert_eq!(fresh.safety, c.safety);
        assert_eq!(fresh.in_use.rig, "Harbour");
        // Again: the same rig and show are reused, the venue gets a number.
        assert_eq!(files.import(&path, &mut fresh).unwrap(), "Harbour 2");
        assert_eq!(files.list(FileKind::Rig), vec!["Harbour"]);
        // Not a lighting file.
        std::fs::write(dir.path().join("x.json"), r#"{"format":"x","version":1}"#).unwrap();
        assert!(files.import(&dir.path().join("x.json"), &mut fresh).is_err());
    }
}
