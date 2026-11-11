//! Controller layer of OpenDJDecks.
//!
//! - [`action`]: vendor-neutral [`ControlAction`] model (what the user did).
//! - [`feedback`]: [`ControlFeedback`] model (what the controller should show).
//! - [`mapping`]: TOML mapping files (`mappings/controllers/<model>.toml`),
//!   schema documented in `docs/controller-mappings.md`.
//! - [`session`]: I/O-free state machine (shift, deck layers, 14-bit pairs,
//!   soft takeover, device module hooks, feedback rendering).
//! - [`output`]: diffed, rate-limited LED/meter output.
//! - [`monitor`] and [`learn`]: MIDI monitor and MIDI learn.
//! - [`guided`]: guided setup for controllers without a mapping, from the
//!   class templates in `mappings/templates/`.
//! - `io` (feature `midi-io`): midir port discovery, hot-plug, threads, and
//!   controllers assigned to the lights.
//! - [`clock`]: MIDI clock out from the master deck (24 ppqn, Start, Stop).
//! - [`osc`]: OSC for the lights: the address space, a UDP receiver and a
//!   sender for the music.

pub mod action;
pub mod clock;
pub mod device_module;
pub mod feedback;
pub mod guided;
pub mod hardware;
#[cfg(feature = "midi-io")]
pub mod io;
pub mod learn;
pub mod mapping;
pub mod midi;
pub mod monitor;
pub mod osc;
pub mod output;
pub mod session;
pub mod takeover;
pub mod templates;
pub mod usb;

pub use action::*;
pub use feedback::*;
pub use mapping::{Mapping, MappingError};
pub use midi::{MidiAddr, MidiMessage};
pub use session::{ControllerSession, MappingHit, Processed};

/// Directory containing the mapping files shipped in this repository.
pub fn bundled_mappings_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mappings")
}
