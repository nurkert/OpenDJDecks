//! Slint user interface of OpenDJDecks.
//!
//! Organised by feature (SPEC §4.1a): each module owns its part of the
//! state, its view-model projection and its callbacks (`wire`).
//!
//! - core: [`app`] (window, wiring, periodic tick), [`state`] (`AppState`),
//!   [`commands`] (status, errors, refresh after edits, undo), [`models`],
//!   [`format`]
//! - features: [`shell`], [`browser`], [`explorer`], [`player`], [`decks`],
//!   [`mixer`], [`fx`], [`sampler`], [`recorder`], [`devices`],
//!   [`preferences`] (with [`settings`] and the [`keymap`]), [`update`], plus [`engine`] (audio lifecycle, frame tick) and
//!   [`controller`] (MIDI)
//! - [`render`]: waveform images; [`appearance`]: dark / light / system

slint::include_modules!();

mod app;
mod appearance;
mod browser;
mod club;
mod commands;
mod controller;
mod covers;
mod deck_grid;
mod decks;
mod devices;
mod engine;
mod explorer;
mod format;
mod fx;
mod gig;
mod grid_edit;
mod import;
mod keymap;
mod lighting;
mod mic;
mod midi_tools;
mod mixer;
mod models;
mod mix_preview;
mod pads;
mod phrase_edit;
mod player;
mod preferences;
mod recorder;
mod render;
mod sampler;
mod settings;
mod shell;
mod smart_text;
mod state;
mod stems_batch;
mod stems_model;
mod track_edit;
mod transients;
mod update;
mod workers;

#[cfg(test)]
mod ui_tests;

pub use app::App;
pub use settings::{Overrides, library_path};
pub use state::AppConfig;

/// Runs the application until the window closes.
pub fn run(config: AppConfig) -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    select_backend();
    let app = App::new(config)?;
    app.run()
}

/// The desktop id of the app (`de.opendjdecks.OpenDJDecks.desktop`, the
/// icon name and the Flatpak id).
pub const APP_ID: &str = "de.opendjdecks.OpenDJDecks";

/// Wayland compositors match a window's app_id, X11 desktops its WM_CLASS
/// (`StartupWMClass`), to the `.desktop` file: both are set to [`APP_ID`] so
/// docks and task switchers show the app's name and icon. Only applies to
/// the winit backend (`SLINT_BACKEND=linuxkms…` on the Pi ignores it).
#[cfg(target_os = "linux")]
fn select_backend() {
    use slint::winit_030::winit::platform::{
        wayland::WindowAttributesExtWayland, x11::WindowAttributesExtX11,
    };
    let selected = slint::BackendSelector::new()
        .with_winit_window_attributes_hook(|attrs| {
            let attrs = WindowAttributesExtWayland::with_name(attrs, APP_ID, APP_ID);
            WindowAttributesExtX11::with_name(attrs, APP_ID, APP_ID)
        })
        .select();
    if let Err(e) = selected {
        tracing::warn!(error = %e, "window backend selection failed; using the default");
    }
}
