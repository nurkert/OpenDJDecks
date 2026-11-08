//! One-time import into the OpenDJDecks library (SPEC §7).
//!
//! * [`xml`]: the library XML format (rekordbox-compatible), import and export.
//! * [`itunes`]: an iTunes / Music library export.
//! * [`settings`]: preferences and keyboard shortcuts.

pub mod itunes;
pub mod settings;
pub mod xml;
