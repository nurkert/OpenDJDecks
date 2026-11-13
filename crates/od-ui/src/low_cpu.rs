//! Low-CPU mode (roadmap [68]): Preferences › View › Low-CPU mode, stored
//! in the settings, always on in the Lite profile (the Raspberry Pi kiosk).
//!
//! What costs the processor while a set runs, and what the mode does about
//! it without taking a function away:
//!
//! - **The frame tick** projects the engine into the decks, meters and
//!   mixer 60 times a second; the mode runs it 30 times ([`frame_period`]).
//! - **The playing waveforms** glide on Slint's animation clock, so the
//!   window is redrawn at the display's rate (60 or 120 Hz) as long as a
//!   deck plays. The mode steps them to the engine position on each frame
//!   tick instead ([`stepped`]), so the window is redrawn at most 30 times a
//!   second. Beat lines, cues and the playhead stay where they are.
//! - **Decorative motion** (key fades, panel slides, the armed pulse, the
//!   loading sweep) is off, as with Reduce motion.
//! - **The Prepare player's enlarged waveform** is drawn in software on
//!   every pixel step while a preview plays; the mode draws it at most
//!   [`DETAIL_EVERY`] apart.
//! - **Stem separation of a loaded track** keeps half the cores free
//!   ([`separation_threads`]); the parts still arrive, only later.
//! - **The automatic audio buffer** is 256 frames instead of 128 (from the
//!   next audio start, never mid-set): fewer wake-ups of the audio thread.
//!
//! Untouched: the audio engine's timing, controller input and LED feedback,
//! the lighting output, analysis results. The automatic analysis and the
//! stems-ahead batch already wait while decks play (`perform_lock`).

use crate::app::App;
use crate::settings::Settings;
use crate::state::AppConfig;
use std::time::{Duration, Instant};

/// Frame tick at full rate (≈ 60 Hz).
pub(crate) const FRAME_FULL: Duration = Duration::from_millis(16);
/// Frame tick in low-CPU mode (≈ 30 Hz).
pub(crate) const FRAME_LOW: Duration = Duration::from_millis(33);
/// Shortest time between two drawings of the Prepare player's enlarged
/// waveform while a preview plays in low-CPU mode (≈ 15 a second).
pub(crate) const DETAIL_EVERY: Duration = Duration::from_millis(66);
/// Automatic audio buffer (frames) at full rate and in low-CPU mode.
pub(crate) const AUTO_BUFFER: u32 = 128;
pub(crate) const AUTO_BUFFER_LOW: u32 = 256;

/// The mode is on: the setting, or the Lite profile.
pub(crate) fn active(settings: &Settings, config: &AppConfig) -> bool {
    settings.view.low_cpu || config.lite
}

/// The mode is on in this app.
pub(crate) fn on(app: &App) -> bool {
    let st = app.state.borrow();
    active(&st.settings, &st.config)
}

/// The frame tick's period.
pub(crate) fn frame_period(on: bool) -> Duration {
    if on { FRAME_LOW } else { FRAME_FULL }
}

/// Playing waveforms are stepped by the frame tick instead of gliding on
/// the display clock.
pub(crate) fn stepped(on: bool) -> bool {
    on
}

/// Automatic audio buffer size in frames.
pub(crate) fn auto_buffer(on: bool) -> u32 {
    if on { AUTO_BUFFER_LOW } else { AUTO_BUFFER }
}

/// Threads of the loaded track's stem separation (0 = all cores).
pub(crate) fn separation_threads(on: bool, cores: usize) -> usize {
    if on { (cores / 2).max(1) } else { 0 }
}

/// The Prepare player's enlarged waveform may be drawn again now.
pub(crate) fn detail_due(on: bool, last: Option<Instant>, now: Instant) -> bool {
    !on || last.is_none_or(|t| now.duration_since(t) >= DETAIL_EVERY)
}

/// Applies a change of the mode: the frame tick's rate (motion follows
/// the theme, the rest is read where it is used).
pub(crate) fn apply(app: &App) {
    crate::engine::retime(app, frame_period(on(app)));
}

/// Documentation screenshots: `OPENDJDECKS_DEMO_PROOF=low-cpu[:<y>]`
/// opens Preferences › View scrolled to `y`, where the mode is.
pub(crate) fn demo(app: &std::rc::Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else { return };
    let Some(y) = v.split(',').find_map(|p| p.strip_prefix("low-cpu")) else { return };
    let y: f32 = y.trim_start_matches(':').parse().unwrap_or(0.0);
    let weak = std::rc::Rc::downgrade(app);
    slint::Timer::single_shot(Duration::from_secs(4), move || {
        let Some(a) = weak.upgrade() else { return };
        crate::preferences::open(&a);
        crate::preferences::demo_page(&a, "view", y);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(lite: bool) -> AppConfig {
        let mut c = AppConfig::new(Default::default());
        c.lite = lite;
        c
    }

    #[test]
    fn the_setting_or_the_lite_profile_turns_it_on() {
        let mut s = Settings::default();
        assert!(!active(&s, &config(false)), "off by default");
        assert!(active(&s, &config(true)), "the Lite profile (Raspberry Pi) always has it");
        s.view.low_cpu = true;
        assert!(active(&s, &config(false)));
    }

    #[test]
    fn it_halves_the_frame_rate_and_doubles_the_automatic_buffer() {
        assert_eq!(frame_period(false), Duration::from_millis(16));
        assert_eq!(frame_period(true), Duration::from_millis(33));
        assert_eq!(auto_buffer(false), 128);
        assert_eq!(auto_buffer(true), 256);
        assert!(stepped(true) && !stepped(false));
    }

    #[test]
    fn separation_keeps_half_the_cores_free() {
        assert_eq!(separation_threads(false, 10), 0, "all cores when off");
        assert_eq!(separation_threads(true, 10), 5);
        assert_eq!(separation_threads(true, 1), 1, "never none");
    }

    #[test]
    fn the_player_detail_is_drawn_less_often() {
        let t = Instant::now();
        assert!(detail_due(true, None, t), "the first drawing is due");
        assert!(!detail_due(true, Some(t), t + Duration::from_millis(20)));
        assert!(detail_due(true, Some(t), t + DETAIL_EVERY));
        assert!(detail_due(false, Some(t), t + Duration::from_millis(1)), "every step when off");
    }

    #[test]
    fn the_setting_is_stored_and_read_back() {
        let mut s = Settings::default();
        s.view.low_cpu = true;
        assert!(Settings::from_json(&s.to_json()).view.low_cpu);
        // Settings stored before the mode existed read as off.
        assert!(!Settings::from_json(r#"{"view":{"scale":1.0}}"#).view.low_cpu);
    }
}
