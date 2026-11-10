//! The application: owns the window, the domain state, the per-module
//! runtime parts and the workers, and wires the feature modules together.
//!
//! Each feature module exposes `wire(app)` for its callbacks and refresh
//! functions that project [`AppState`] into its view model.

use crate::state::{AppConfig, AppState};
use crate::workers::{AnalysisWorker, WorkerEvent};
use crate::{
    AppWindow, browser, controller, decks, devices, engine, explorer, fx, mixer, player,
    deck_grid, preferences, recorder, sampler, shell, stems_model, update,
};
use crossbeam_channel::{Receiver, Sender, unbounded};
use od_devices::{DeviceManager, ManagerConfig};
use od_library::Library;
use slint::ComponentHandle;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// Builds a Slint callback that runs `body` with a strong reference to the
/// app while it is alive: `handler!(app, |a, x: i32| a.do_it(x))`.
macro_rules! handler {
    ($app:expr, |$a:ident $(, $arg:ident : $t:ty)*| $body:expr) => {{
        let weak = std::rc::Rc::downgrade($app);
        move |$($arg: $t),*| {
            if let Some($a) = weak.upgrade() {
                #[allow(clippy::unused_unit)]
                {
                    $body;
                }
            }
        }
    }};
}
pub(crate) use handler;

pub struct App {
    pub ui: AppWindow,
    /// Domain state (library, browser, player, devices, explorer).
    pub(crate) state: RefCell<AppState>,
    /// Audio engine and host.
    pub(crate) engine: RefCell<engine::EngineHost>,
    /// Loaded tracks, stems and the deck view models.
    pub(crate) decks: RefCell<decks::Decks>,
    pub(crate) mixer: mixer::MixerView,
    /// Tempo FX panel rows and tap tempo.
    pub(crate) fx: fx::FxRuntime,
    /// Sampler slots, bank and panel rows.
    pub(crate) sampler: sampler::SamplerRuntime,
    pub(crate) recorder: RefCell<recorder::RecorderState>,
    pub(crate) controller: RefCell<controller::Controllers>,
    pub(crate) stick_check: RefCell<crate::stick_check::Running>,
    /// The library import under review (preview, result, undo).
    pub(crate) import_review: RefCell<crate::import::Review>,
    pub(crate) browser: browser::BrowserViews,
    pub(crate) explorer: explorer::ExplorerRuntime,
    pub(crate) device_manager: DeviceManager,
    pub(crate) analysis: AnalysisWorker,
    pub(crate) worker_tx: Sender<WorkerEvent>,
    /// The Preferences window, created when first opened.
    pub(crate) prefs: RefCell<preferences::Prefs>,
    /// Held keyboard actions (pitch bend): (action, deck).
    pub(crate) held: RefCell<Vec<(String, usize)>>,
    /// Update check and install (SPEC §14.1).
    pub(crate) update: RefCell<update::UpdateRuntime>,
    /// The stem separation model: asked for, downloaded, repaired.
    pub(crate) stems_model: RefCell<stems_model::StemsModel>,
    /// DMX rig and show (SPEC §10.5).
    pub(crate) lighting: RefCell<crate::lighting::LightingRuntime>,
    /// The LIGHTING view detached into its own window.
    pub(crate) lighting_window: RefCell<Option<crate::LightingWindow>>,
    /// The booth view's window, once opened.
    pub(crate) booth_window: RefCell<Option<crate::BoothWindow>>,
    /// Club mode checklist (SPEC §10.3c).
    pub(crate) club: RefCell<crate::club::ClubRuntime>,
    /// Microphone / aux input.
    pub(crate) mic: RefCell<crate::mic::MicRuntime>,
    /// The first-start guide.
    pub(crate) first_start: RefCell<crate::first_start::Guide>,
    worker_events: Receiver<WorkerEvent>,
    tick_timer: slint::Timer,
}

impl App {
    pub fn new(config: AppConfig) -> anyhow::Result<Rc<Self>> {
        let lib = Library::open(&config.library_path)?;
        let state = AppState::new(lib, config);
        if state.settings.advanced.backup_on_start
            && (state.lib.backups().map(|b| b.is_empty()).unwrap_or(true)
                || state.lib.track_count()? > 0)
        {
            if let Err(e) = state.lib.backup_now("start") {
                tracing::warn!(error = %e, "start-up backup failed");
            }
        }
        let ui = AppWindow::new()?;
        shell::setup_window(&ui, &state.config, &state.settings);
        let (wtx, wrx) = unbounded();
        let analysis = AnalysisWorker::start(
            state.config.library_path.clone(),
            preferences::analysis_control(&state.settings, state.config.lite),
            wtx.clone(),
        );
        let device_manager =
            DeviceManager::start(ManagerConfig::new(state.config.library_path.clone()))?;
        let lighting = crate::lighting::LightingRuntime::load(crate::lighting::rig_path(
            &state.config.library_path,
        ));
        let app = Rc::new(App {
            ui,
            state: RefCell::new(state),
            prefs: RefCell::new(preferences::Prefs::default()),
            held: RefCell::new(Vec::new()),
            update: RefCell::new(update::UpdateRuntime::default()),
            stems_model: RefCell::new(stems_model::StemsModel::default()),
            lighting: RefCell::new(lighting),
            lighting_window: RefCell::new(None),
            booth_window: RefCell::new(None),
            club: RefCell::new(crate::club::ClubRuntime::default()),
            mic: RefCell::new(crate::mic::MicRuntime::default()),
            first_start: RefCell::new(crate::first_start::Guide::default()),
            engine: RefCell::new(engine::EngineHost::default()),
            decks: RefCell::new(decks::Decks::default()),
            mixer: mixer::MixerView::default(),
            fx: fx::FxRuntime::default(),
            sampler: sampler::SamplerRuntime::default(),
            recorder: RefCell::new(recorder::RecorderState::default()),
            controller: RefCell::new(controller::Controllers::default()),
            stick_check: RefCell::new(Default::default()),
            import_review: RefCell::new(Default::default()),
            browser: browser::BrowserViews::default(),
            explorer: explorer::ExplorerRuntime::default(),
            device_manager,
            analysis,
            worker_tx: wtx,
            worker_events: wrx,
            tick_timer: slint::Timer::default(),
        });
        app.bind_models();
        shell::wire(&app);
        browser::wire(&app);
        explorer::wire(&app);
        player::wire(&app);
        devices::wire(&app);
        recorder::wire(&app);
        if !app.state.borrow().config.headless {
            controller::start(&app);
        }
        decks::wire(&app);
        deck_grid::wire(&app);
        shell::wire_window_chrome(&app);
        mixer::wire(&app);
        fx::wire(&app);
        sampler::wire(&app);
        preferences::wire(&app);
        crate::appearance::wire(&app);
        crate::gig::wire(&app);
        update::wire(&app);
        stems_model::wire(&app);
        crate::lighting::wire(&app);
        crate::club::wire(&app);
        crate::mic::wire(&app);
        crate::first_start::wire(&app);
        crate::audio_health::wire(&app);
        crate::gig_check::wire(&app);
        crate::stick_check::wire(&app);
        engine::start(&app);
        crate::restore::wire(&app);
        crate::perform_lock::wire(&app);
        crate::phase::wire(&app);
        crate::booth::wire(&app);
        decks::autoload(&app);

        let _ = app.state.borrow_mut().lib.check_missing();
        shell::restore_view(&app);
        app.refresh_all();
        crate::covers::spawn_backfill(&app);
        let weak = Rc::downgrade(&app);
        app.tick_timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(250),
            move || {
                if let Some(app) = weak.upgrade() {
                    app.tick();
                }
            },
        );
        Ok(app)
    }

    /// Binds the persistent models to their properties once.
    fn bind_models(&self) {
        self.ui.set_tracks(self.browser.tracks.handle());
        self.ui.set_columns(self.browser.columns.handle());
        self.ui.set_channels(self.mixer.channels.handle());
        let d = self.decks.borrow();
        self.ui.set_decks(d.views.all.handle());
        self.ui.set_decks_left(d.views.left.handle());
        self.ui.set_decks_right(d.views.right.handle());
    }

    pub fn run(self: &Rc<Self>) -> anyhow::Result<()> {
        self.ui.run()?;
        crate::restore::clean_exit(self);
        crate::perform_lock::release();
        shell::remember_view(self);
        update::apply_on_quit(self);
        Ok(())
    }

    pub(crate) fn refresh_all(&self) {
        self.state.borrow_mut().load_rows();
        browser::refresh_tree(self);
        browser::refresh_tracks(self);
        player::refresh_detail(self);
        devices::refresh(self);
        shell::refresh_status(self);
        recorder::refresh_list(self);
        explorer::refresh_tree(self);
    }

    /// Periodic work: worker progress, device events, outside changes.
    fn tick(&self) {
        let mut need_refresh = false;
        for e in self.worker_events.try_iter() {
            match e {
                WorkerEvent::ImportProgress { done, total } => {
                    self.status(format!("Adding music: {done} of {total} files"))
                }
                WorkerEvent::ImportFinished { added, failed } => {
                    self.status(if failed > 0 {
                        format!("Added {added} tracks, {failed} files could not be read")
                    } else {
                        format!("Added {added} tracks")
                    });
                    need_refresh = true;
                }
                WorkerEvent::AnalysisProgress { done, total } => {
                    self.state.borrow_mut().analysis = (done, total);
                    need_refresh = true;
                }
                WorkerEvent::AnalysisIdle => {
                    let was = self.state.borrow().analysis;
                    if was.1 > 0 {
                        self.state.borrow_mut().analysis = (0, 0);
                        self.status("Analysis complete");
                        need_refresh = true;
                    }
                }
                WorkerEvent::Message(m) => self.status(m),
                WorkerEvent::SamplesDone => crate::first_start::samples_done(self),
                WorkerEvent::EncryptedImportStarted => crate::import::close_encrypted_dialog(self),
                WorkerEvent::ReviewedImportDone(r) => crate::import::review_done(self, r),
                WorkerEvent::EncryptedImportFailed(m) => {
                    self.ui.set_encrypted_import_busy(false);
                    self.ui.set_encrypted_import_error(m.into());
                }
            }
        }
        crate::appearance::tick(self);
        crate::gig::tick(self);
        crate::first_start::tick(self);
        shell::tick_chrome(self);
        browser::tick(self);
        if !self.state.borrow().config.headless {
            update::poll(self);
        }
        stems_model::poll(self);
        crate::stick_check::poll(self);
        if devices::poll(self) {
            need_refresh = true;
        }
        // Changes made through other connections (MCP server, od-cli).
        let changed = self.state.borrow().lib.external_change().unwrap_or(false);
        if changed || need_refresh {
            self.refresh_library();
        }
    }
}
