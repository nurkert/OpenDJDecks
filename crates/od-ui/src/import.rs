//! File menu actions on the whole library and files dropped on the window.
//! Library imports run in the background on their own library connection;
//! the UI picks up the result through the change log.

use crate::app::{App, handler};
use crate::workers::WorkerEvent;
use od_import::review::{ExportFormat, ImportPlan, LeftOut, LeftOutReason};
use od_import::{ImportOptions, Source};
use od_library::LibraryCounts;
use slint::ComponentHandle;
use std::path::Path;
use std::rc::Rc;

/// File menu actions on the whole library: XML import / export, library
/// archive, consolidate and the preferences import. Asks for
/// the file or folder, then runs the step.
pub(crate) fn library_action(app: &App, id: &str) {
    let message = match id {
        "library.export.xml" => {
            let Some(f) = rfd::FileDialog::new()
                .set_title(od_i18n::tr("Export library XML"))
                .set_file_name("opendjdecks.xml")
                .add_filter("XML", &["xml"])
                .save_file()
            else {
                return;
            };
            export_xml(app, &f)
        }
        "library.import.xml" => {
            let Some(f) = rfd::FileDialog::new()
                .set_title(od_i18n::tr("Import library XML"))
                .add_filter("XML", &["xml"])
                .pick_file()
            else {
                return;
            };
            open_review(app, &f, ExportFormat::LibraryXml);
            return;
        }
        "library.export.archive" => {
            let Some(f) = rfd::FileDialog::new()
                .set_title(od_i18n::tr("Export library archive"))
                .set_file_name("library.odlib")
                .add_filter("OpenDJDecks library", &["odlib"])
                .save_file()
            else {
                return;
            };
            let audio = rfd::MessageDialog::new()
                .set_title(od_i18n::tr("Export library archive"))
                .set_description(od_i18n::tr("Include the audio files? The archive then holds everything to move the library to another computer."))
                .set_buttons(rfd::MessageButtons::YesNo)
                .show()
                == rfd::MessageDialogResult::Yes;
            export_archive(app, &f, audio);
            return;
        }
        "library.restore.archive" => {
            let Some(f) = rfd::FileDialog::new()
                .set_title(od_i18n::tr("Restore library archive"))
                .add_filter("OpenDJDecks library", &["odlib"])
                .pick_file()
            else {
                return;
            };
            let Some(dir) = rfd::FileDialog::new()
                .set_title(od_i18n::tr("Folder for the restored library"))
                .pick_folder()
            else {
                return;
            };
            restore_archive(app, &f, &dir)
        }
        "library.consolidate" => {
            let Some(dir) = rfd::FileDialog::new()
                .set_title(od_i18n::tr("Folder to collect all tracks in"))
                .pick_folder()
            else {
                return;
            };
            consolidate(app, &dir);
            return;
        }
        "library.import.settings" => {
            pick_and_import_settings(app);
            return;
        }
        "library.import.club-usb" => {
            let Some(dir) = rfd::FileDialog::new()
                .set_title(od_i18n::tr("Choose a Club player USB"))
                .pick_folder()
            else {
                return;
            };
            if !dir.join("PIONEER/rekordbox/export.pdb").is_file() {
                app.status(od_i18n::tr("This folder is not a Club player USB"));
                return;
            }
            run_library_import(app, Source::ClubStick { root: dir }, false);
            return;
        }
        "library.import.encrypted" => {
            let ui = &app.ui;
            ui.set_encrypted_import_file("".into());
            ui.set_encrypted_import_key("".into());
            ui.set_encrypted_import_error("".into());
            ui.set_encrypted_import_busy(false);
            ui.set_encrypted_import_open(true);
            return;
        }
        "library.import.itunes" => {
            let Some(f) = rfd::FileDialog::new()
                .set_title(od_i18n::tr("Import a Music / iTunes library"))
                .add_filter("XML", &["xml"])
                .pick_file()
            else {
                return;
            };
            open_review(app, &f, ExportFormat::MusicXml);
            return;
        }
        _ => return,
    };
    app.status(message);
    crate::covers::spawn_backfill(app);
}

/// Imports `source` in the background: reports progress and the result on
/// the status line and wakes the analysis for tracks that need it.
fn run_library_import(app: &App, source: Source, dialog: bool) {
    let library = app.state.borrow().config.library_path.clone();
    let tx = app.worker_tx.clone();
    let wake = app.analysis.wake.clone();
    app.status("Reading the library…");
    std::thread::spawn(move || {
        let say = |m: String| {
            let _ = tx.send(WorkerEvent::Message(m));
        };
        let preview = match od_import::preview(&source) {
            Ok(p) => p,
            Err(e) => {
                let m = import_error_text(&e);
                if dialog {
                    let _ = tx.send(WorkerEvent::EncryptedImportFailed(m));
                    return;
                }
                return say(m);
            }
        };
        if dialog {
            let _ = tx.send(WorkerEvent::EncryptedImportStarted);
        }
        say(od_i18n::trf(
            "Importing {} tracks, {} playlists and {} cues (the source is not changed)…",
            &[&preview.tracks, &preview.playlists, &preview.cues],
        ));
        let mut lib = match od_library::Library::open(&library) {
            Ok(l) => l,
            Err(e) => return say(od_i18n::trf("Cannot open library: {}", &[&e])),
        };
        let opts = ImportOptions {
            include_history: true,
            copy_audio_to: None,
            analyse_missing: true,
        };
        let mut last = std::time::Instant::now();
        let result = od_import::import(&source, &mut lib, &opts, |p| {
            if last.elapsed().as_millis() > 200 {
                last = std::time::Instant::now();
                say(od_i18n::trf("Library import: {}/{}", &[&p.done, &p.total]));
            }
            true
        });
        match result {
            Ok(r) => {
                let mut m = od_i18n::trf(
                    "Library import done: {} tracks added, {} playlists, {} cues, {} beatgrids",
                    &[&r.tracks_added, &r.playlists, &r.cues, &r.grids],
                );
                if !r.missing_files.is_empty() {
                    m.push_str(&od_i18n::trf(
                        ", {} files not found",
                        &[&r.missing_files.len()],
                    ));
                }
                say(m);
                let _ = wake.send(());
            }
            Err(e) => say(import_error_text(&e)),
        }
    });
}

/// The message shown when a library import fails. A key that was not
/// accepted says just that.
fn import_error_text(e: &od_import::ImportError) -> String {
    use od_import::ImportError::{KeyNotAccepted, KeyRequired};
    match e {
        KeyNotAccepted => od_i18n::tr("The key was not accepted. Check it and try again.").into(),
        KeyRequired => {
            od_i18n::tr("This library database is encrypted. Enter its key under Advanced.").into()
        }
        e => od_i18n::trf("Library import failed: {}", &[e]),
    }
}

/// Closes the encrypted import dialog and clears everything it held.
pub(crate) fn close_encrypted_dialog(app: &App) {
    let ui = &app.ui;
    ui.set_encrypted_import_open(false);
    ui.set_encrypted_import_busy(false);
    ui.set_encrypted_import_key("".into());
    ui.set_encrypted_import_file("".into());
    ui.set_encrypted_import_error("".into());
}

/// Starts the import from the encrypted library dialog. The key leaves the
/// window here: the field is cleared at once, and the key lives on only in
/// the [`od_import::LibraryKey`] that is wiped when the import ends. It is
/// not stored or logged.
fn start_encrypted_import(app: &App) {
    let ui = &app.ui;
    let key = ui.get_encrypted_import_key().to_string();
    ui.set_encrypted_import_key("".into());
    let file = ui.get_encrypted_import_file().to_string();
    if file.is_empty() {
        ui.set_encrypted_import_error(od_i18n::tr("Choose the library database file first").into());
        return;
    }
    let key = od_import::LibraryKey::new(key);
    let key = (!key.is_empty()).then_some(key);
    ui.set_encrypted_import_error("".into());
    ui.set_encrypted_import_busy(true);
    run_library_import(
        app,
        Source::EncryptedLibrary {
            file: file.into(),
            key,
        },
        true,
    );
}

/// The file chooser of the encrypted library dialog. Nothing is searched for
/// on disk; the user picks the database.
fn pick_encrypted_file(app: &App) {
    let Some(f) = rfd::FileDialog::new()
        .set_title(od_i18n::tr("Choose the library database file"))
        .pick_file()
    else {
        return;
    };
    app.ui
        .set_encrypted_import_file(f.display().to_string().into());
    app.ui.set_encrypted_import_error("".into());
}

/// Documentation screenshots: `OPENDJDECKS_DEMO_PROOF=import-preview:<xml>`
/// opens the review of a library XML file; `import-done:<xml>` also runs
/// the import (on the demo library).
fn demo(app: &Rc<App>) {
    let Some(v) = od_core::env::var("DEMO_PROOF") else { return };
    let Some((run, path)) = v.split(',').find_map(|p| {
        p.strip_prefix("import-preview:")
            .map(|f| (false, f.to_string()))
            .or_else(|| p.strip_prefix("import-done:").map(|f| (true, f.to_string())))
    }) else {
        return;
    };
    let weak = Rc::downgrade(app);
    slint::Timer::single_shot(std::time::Duration::from_secs(6), move || {
        let Some(a) = weak.upgrade() else { return };
        open_review(&a, Path::new(&path), ExportFormat::LibraryXml);
        if run {
            run_review(&a);
        }
    });
}

pub(crate) fn wire(app: &Rc<App>) {
    demo(app);
    let ui = &app.ui;
    ui.on_pick_encrypted_import_file(handler!(app, |a| pick_encrypted_file(&a)));
    ui.on_run_encrypted_import(handler!(app, |a| start_encrypted_import(&a)));
    ui.on_cancel_encrypted_import(handler!(app, |a| close_encrypted_dialog(&a)));
    let g = ui.global::<crate::ImportReview>();
    g.on_run(handler!(app, |a| run_review(&a)));
    g.on_cancel(handler!(app, |a| close_review(&a)));
    g.on_undo(handler!(app, |a| undo_review(&a)));
    g.on_close(handler!(app, |a| close_review(&a)));
}

// ── Reviewed import of an export file ───────────────────────────────────

/// Items listed in the review; the rest are counted.
const LEFT_OUT_SHOWN: usize = 200;

/// A reviewed import in progress: the file, what it carries over, the
/// library before, and the snapshot that undoes it.
#[derive(Default)]
pub(crate) struct Review {
    file: Option<(std::path::PathBuf, ExportFormat)>,
    plan: Option<ImportPlan>,
    before: LibraryCounts,
    snapshot: Option<std::path::PathBuf>,
}

/// Why an item does not carry over, in words.
pub(crate) fn reason_text(r: &LeftOutReason) -> String {
    match r {
        LeftOutReason::FileMissing => od_i18n::tr("file not found").into(),
        LeftOutReason::NotALocalFile => {
            od_i18n::tr("not a file on this computer (stream or cloud)").into()
        }
        LeftOutReason::NotMusic => od_i18n::tr("video or podcast").into(),
        LeftOutReason::UnreadableCues(n) => {
            od_i18n::trf("cue points that cannot be read: {}", &[n])
        }
        LeftOutReason::BuiltInList => {
            od_i18n::tr("a list the other app makes itself").into()
        }
        LeftOutReason::ListEntries(n) => {
            od_i18n::trf("tracks that do not carry over: {}", &[n])
        }
    }
}

fn format_label(f: ExportFormat) -> String {
    match f {
        ExportFormat::LibraryXml => od_i18n::tr("LIBRARY XML").into(),
        ExportFormat::MusicXml => od_i18n::tr("MUSIC / ITUNES LIBRARY").into(),
    }
}

fn row(label: &str, before: Option<u64>, after: u64) -> crate::ImportCountRow {
    crate::ImportCountRow {
        label: od_i18n::tr(label).into(),
        before: before.map(|b| b.to_string()).unwrap_or_default().into(),
        after: after.to_string().into(),
        changed: before.is_some_and(|b| b != after),
    }
}

/// The preview's count rows (pure).
pub(crate) fn plan_rows(p: &ImportPlan, format: ExportFormat) -> Vec<crate::ImportCountRow> {
    let mut rows = vec![
        row("New tracks", None, p.tracks_new as u64),
        row("Tracks already in your library", None, p.tracks_known as u64),
        row("Playlists", None, p.playlists as u64),
        row("Folders", None, p.folders as u64),
    ];
    if format == ExportFormat::LibraryXml {
        rows.push(row("Cue points", None, p.cues as u64));
        rows.push(row("Beatgrids", None, p.grids as u64));
        rows.push(row("Keys", None, p.keys as u64));
    }
    rows
}

/// The result's rows: the library before and after (pure).
pub(crate) fn result_rows(
    before: &LibraryCounts,
    after: &LibraryCounts,
) -> Vec<crate::ImportCountRow> {
    vec![
        row("Tracks", Some(before.tracks), after.tracks),
        row("Playlists", Some(before.playlists), after.playlists),
        row("Folders", Some(before.folders), after.folders),
        row("Cue points", Some(before.cues), after.cues),
        row("Beatgrids", Some(before.grids), after.grids),
        row("Keys", Some(before.keys), after.keys),
    ]
}

fn left_out_rows(items: &[LeftOut]) -> (Vec<crate::ImportLeftOutRow>, i32) {
    let rows = items
        .iter()
        .take(LEFT_OUT_SHOWN)
        .map(|l| crate::ImportLeftOutRow {
            item: l.item.clone().into(),
            reason: reason_text(&l.reason).into(),
        })
        .collect();
    (rows, items.len().saturating_sub(LEFT_OUT_SHOWN) as i32)
}

fn plan_note(p: &ImportPlan, format: ExportFormat) -> String {
    let mut note = match format {
        ExportFormat::LibraryXml => od_i18n::tr(
            "Lists go into a new folder “XML import”. Tracks already in your library take the file's tags, cues, beatgrid and key.",
        ),
        ExportFormat::MusicXml => od_i18n::tr(
            "Lists go into a new folder “Music library import”. Tracks already in your library take the file's tags.",
        ),
    }
    .to_string();
    if p.is_empty() {
        note = od_i18n::tr("Nothing in this file can be imported.").into();
    }
    note
}

/// Reads `path` and shows what it would carry over. Nothing is written.
pub(crate) fn open_review(app: &App, path: &Path, format: ExportFormat) {
    if app.state.borrow().read_only() {
        app.state.borrow_mut().browser.stick = None;
    }
    let g = app.ui.global::<crate::ImportReview>();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let planned = {
        let st = app.state.borrow();
        od_import::review::plan(&st.lib, path, format).map(|p| (p, st.lib.counts()))
    };
    let (plan, before) = match planned {
        Ok((p, Ok(before))) => (p, before),
        Ok((_, Err(e))) => {
            app.status(od_i18n::trf("Cannot open library: {}", &[&e]));
            return;
        }
        Err(e) => {
            app.status(od_i18n::trf("This file cannot be read: {}", &[&e]));
            return;
        }
    };
    g.set_file(name.into());
    g.set_format(format_label(format).into());
    g.set_counts(crate::models::model(plan_rows(&plan, format)));
    let (rows, more) = left_out_rows(&plan.left_out);
    g.set_left_out(crate::models::model(rows));
    g.set_left_out_more(more);
    g.set_note(plan_note(&plan, format).into());
    g.set_error("".into());
    g.set_stage("preview".into());
    *app.import_review.borrow_mut() = Review {
        file: Some((path.to_path_buf(), format)),
        plan: Some(plan),
        before,
        snapshot: None,
    };
}

/// The snapshot that undoes the running import, next to the library.
fn snapshot_path(app: &App) -> std::path::PathBuf {
    let lib = app.state.borrow().config.library_path.clone();
    lib.parent()
        .unwrap_or(Path::new("."))
        .join("import-undo.snapshot")
}

/// Imports the reviewed file in the background, after a snapshot of the
/// library for undo.
fn run_review(app: &App) {
    let Some((path, format)) = app.import_review.borrow().file.clone() else {
        return;
    };
    let g = app.ui.global::<crate::ImportReview>();
    let snap = snapshot_path(app);
    if let Err(e) = app.state.borrow().lib.backup_to(&snap) {
        g.set_error(od_i18n::trf("No undo copy could be made, nothing was imported: {}", &[&e]).into());
        return;
    }
    let before = app.state.borrow().lib.counts().unwrap_or_default();
    {
        let mut r = app.import_review.borrow_mut();
        r.snapshot = Some(snap);
        r.before = before;
    }
    g.set_stage("running".into());
    let library = app.state.borrow().config.library_path.clone();
    let tx = app.worker_tx.clone();
    std::thread::spawn(move || {
        let r = od_library::Library::open(&library)
            .map_err(|e| e.to_string())
            .and_then(|mut lib| {
                od_import::review::run(&mut lib, &path, format).map_err(|e| e.to_string())
            });
        let _ = tx.send(WorkerEvent::ReviewedImportDone(r.map(|_| ())));
    });
}

/// The background import finished: the counts before and after and what
/// did not transfer.
pub(crate) fn review_done(app: &App, result: Result<(), String>) {
    let g = app.ui.global::<crate::ImportReview>();
    app.refresh_library();
    let _ = app.analysis.wake.send(());
    crate::covers::spawn_backfill(app);
    let after = app.state.borrow().lib.counts().unwrap_or_default();
    let before = app.import_review.borrow().before;
    g.set_counts(crate::models::model(result_rows(&before, &after)));
    if let Err(e) = result {
        g.set_error(od_i18n::trf("The import stopped: {}. UNDO IMPORT puts back what was written.", &[&e]).into());
    }
    g.set_note(
        od_i18n::tr("UNDO IMPORT puts the library back exactly as it was before, as long as this window is open.")
            .into(),
    );
    g.set_stage("done".into());
    app.status(od_i18n::trf(
        "Library import done: {} tracks before, {} after",
        &[&before.tracks, &after.tracks],
    ));
}

/// Puts the library back to the snapshot taken before the import.
pub(crate) fn undo_review(app: &App) {
    let g = app.ui.global::<crate::ImportReview>();
    let Some(snap) = app.import_review.borrow_mut().snapshot.take() else {
        return;
    };
    let r = app.state.borrow_mut().lib.restore_in_place(&snap);
    if let Err(e) = r {
        app.import_review.borrow_mut().snapshot = Some(snap);
        g.set_error(od_i18n::trf("Undo failed: {}", &[&e]).into());
        return;
    }
    let _ = std::fs::remove_file(&snap);
    app.refresh_library();
    let now = app.state.borrow().lib.counts().unwrap_or_default();
    let before = app.import_review.borrow().before;
    // Before = the library before the import; now = after the undo.
    g.set_counts(crate::models::model(result_rows(&before, &now)));
    g.set_note(od_i18n::tr("The library is as it was before the import.").into());
    g.set_error("".into());
    g.set_stage("undone".into());
    app.status(od_i18n::tr("Library import undone"));
}

/// Closes the review; the undo copy is removed.
pub(crate) fn close_review(app: &App) {
    let snap = std::mem::take(&mut *app.import_review.borrow_mut()).snapshot;
    if let Some(s) = snap {
        let _ = std::fs::remove_file(s);
    }
    app.ui
        .global::<crate::ImportReview>()
        .set_stage("".into());
}

/// Writes the library as XML (rekordbox-compatible).
pub(crate) fn export_xml(app: &App, path: &Path) -> String {
    match od_import::xml::export_xml(&app.state.borrow().lib, path) {
        Ok(n) => od_i18n::trf("Exported {} tracks to {}", &[&n, &path.display()]),
        Err(e) => od_i18n::trf("XML export failed: {}", &[&e]),
    }
}

/// Reads a library XML file (rekordbox-compatible) into the library at
/// once, without the review (tests).
#[cfg(test)]
pub(crate) fn import_xml(app: &App, path: &Path) -> String {
    if app.state.borrow().read_only() {
        app.state.borrow_mut().browser.stick = None;
    }
    let r = od_import::xml::import_xml(&mut app.state.borrow_mut().lib, path);
    let m = match r {
        Ok(r) => {
            let _ = app.analysis.wake.send(());
            let mut m = od_i18n::trf(
                "Imported {} tracks, {} playlists and {} cues from XML",
                &[&r.tracks, &r.playlists, &r.cues],
            );
            if !r.missing.is_empty() {
                m.push_str(&od_i18n::trf(" · {} files not found", &[&r.missing.len()]));
            }
            m
        }
        Err(e) => od_i18n::trf("XML import failed: {}", &[&e]),
    };
    app.refresh_library();
    m
}

/// Reads an iTunes / Music library export (`Library.xml`): tracks with
/// their tags and the playlists (in a folder of their own), without the
/// review (tests).
#[cfg(test)]
pub(crate) fn import_itunes(app: &App, path: &Path) -> String {
    if app.state.borrow().read_only() {
        app.state.borrow_mut().browser.stick = None;
    }
    let r = od_import::itunes::import_itunes(&mut app.state.borrow_mut().lib, path);
    let m = match r {
        Ok(r) => {
            let _ = app.analysis.wake.send(());
            let mut m = od_i18n::trf(
                "Imported {} tracks and {} playlists from the Music library",
                &[&r.tracks, &r.playlists],
            );
            if !r.missing.is_empty() {
                m.push_str(&od_i18n::trf(" · {} files not found", &[&r.missing.len()]));
            }
            m
        }
        Err(e) => od_i18n::trf("Music library import failed: {}", &[&e]),
    };
    app.refresh_library();
    m
}

/// Writes the library archive in the background (it can take minutes with
/// audio); the result arrives as a status message.
pub(crate) fn export_archive(app: &App, path: &Path, include_audio: bool) {
    let library = app.state.borrow().config.library_path.clone();
    let tx = app.worker_tx.clone();
    let path = path.to_path_buf();
    app.status("Writing the library archive…");
    std::thread::spawn(move || {
        let r = od_library::Library::open(&library)
            .and_then(|lib| lib.export_archive(&path, include_audio, |_, _| {}));
        let m = match r {
            Ok(n) => od_i18n::trf(
                "Library archive saved: {} ({} audio files)",
                &[&path.display(), &n],
            ),
            Err(e) => od_i18n::trf("Library archive failed: {}", &[&e]),
        };
        let _ = tx.send(WorkerEvent::Message(m));
    });
}

/// Restores an archive into `dir` and opens that library from the next
/// start.
pub(crate) fn restore_archive(app: &App, archive: &Path, dir: &Path) -> String {
    match od_library::Library::import_archive(archive, dir) {
        Ok(lib) => {
            drop(lib);
            let db = dir.join("library.sqlite");
            let mut c = crate::settings::GlobalConfig::load();
            c.library = Some(db.clone());
            match c.save() {
                Ok(()) => od_i18n::trf(
                    "Library restored to {} — it opens at the next start",
                    &[&db.display()],
                ),
                Err(e) => format!("{e}"),
            }
        }
        Err(e) => {
            let _ = app;
            od_i18n::trf("Restoring the archive failed: {}", &[&e])
        }
    }
}

/// Copies every track into `dir/<Artist>/` and points the library at the
/// copies, in the background (originals stay).
pub(crate) fn consolidate(app: &App, dir: &Path) {
    let library = app.state.borrow().config.library_path.clone();
    let tx = app.worker_tx.clone();
    let dir = dir.to_path_buf();
    app.status("Consolidating the library…");
    std::thread::spawn(move || {
        let r = od_library::Library::open(&library)
            .and_then(|mut lib| lib.consolidate(&dir, |_, _| {}));
        let m = match r {
            Ok(n) => od_i18n::trf(
                "Consolidated: {} tracks copied to {}",
                &[&n, &dir.display()],
            ),
            Err(e) => od_i18n::trf("Consolidating failed: {}", &[&e]),
        };
        let _ = tx.send(WorkerEvent::Message(m));
    });
}

/// Asks for the preference folder (the user picks it; nothing is looked
/// for on its own) and takes its preferences and shortcuts over.
pub(crate) fn pick_and_import_settings(app: &App) {
    let Some(dir) = rfd::FileDialog::new()
        .set_title(od_i18n::tr("Choose the rekordbox preferences folder"))
        .pick_folder()
    else {
        return;
    };
    let m = import_settings(app, &dir);
    app.status(m);
}

/// Takes over the preferences and keyboard shortcuts in `dir`, a
/// rekordbox preference folder the user picked. Shortcuts that would bind
/// one key twice are left out.
pub(crate) fn import_settings(app: &App, dir: &Path) -> String {
    let (settings, _report) = match od_import::settings::read(dir) {
        Ok(r) => r,
        Err(e) => return od_i18n::trf("No rekordbox preferences found: {}", &[&e]),
    };
    let values = settings.preference_values();
    let applied = app
        .state
        .borrow()
        .stored
        .clone()
        .apply_values(&values)
        .len();
    app.state.borrow_mut().change_settings(|s| {
        s.apply_values(&values);
    });
    let keys = apply_keymap(app, &settings.keymap_toml());
    crate::preferences::apply_all(app);
    od_i18n::trf(
        "Took over {} rekordbox preferences and {} shortcuts",
        &[&applied, &keys],
    )
}

/// Applies imported bindings one by one, skipping those that would collide
/// with a binding already in use; returns how many were taken.
fn apply_keymap(app: &App, toml_text: &str) -> usize {
    let mut st = app.state.borrow_mut();
    let mut imported = st.keymap.clone();
    if imported.import_toml(toml_text).is_err() {
        return 0;
    }
    let mut taken = 0;
    for a in crate::keymap::registry() {
        let wanted = imported.bindings(&a.id);
        if wanted == st.keymap.bindings(&a.id) {
            continue;
        }
        let mut km = st.keymap.clone();
        km.set(&a.id, wanted);
        if km.all_conflicts().is_empty() {
            st.keymap = km;
            taken += 1;
        }
    }
    if taken > 0 {
        st.save_keymap();
    }
    taken
}

// ── Files dropped from Finder / the file manager ────────────────────────

/// The audio files among dropped paths: files with a supported extension
/// and every audio file below a dropped folder, in drop order, each once.
pub(crate) fn dropped_audio(paths: &[std::path::PathBuf]) -> Vec<std::path::PathBuf> {
    let mut out: Vec<std::path::PathBuf> = Vec::new();
    for p in paths {
        let found = if p.is_dir() {
            crate::workers::scan(p)
        } else if p.is_file() && od_core::AudioFormat::is_supported_extension(p) {
            vec![p.clone()]
        } else {
            Vec::new()
        };
        for f in found {
            if !out.contains(&f) {
                out.push(f);
            }
        }
    }
    out
}

/// The list dropped files also go into: the open playlist or set of the
/// library (not a stick, the Explorer or a virtual list).
pub(crate) fn drop_list(app: &App) -> Option<od_library::NodeId> {
    drop_list_of(&app.state.borrow())
}

fn drop_list_of(st: &crate::state::AppState) -> Option<od_library::NodeId> {
    if st.read_only() || st.explorer.open.is_some() || st.browser.special.is_some() {
        return None;
    }
    st.browser.node.filter(|n| {
        st.lib.node(*n).is_ok_and(|n| {
            matches!(
                n.kind,
                od_library::NodeKind::Playlist | od_library::NodeKind::Set
            )
        })
    })
}

/// Where dropped files go, from the drop target under the pointer
/// (`DragState.target`, the ranges of `browser::drop_target`).
#[derive(Debug, PartialEq)]
pub(crate) enum FileDrop {
    /// Load the first file on this deck (0-based).
    Deck(usize),
    /// Show the first file in Prepare player 1 or 2.
    Player(u8),
    /// Add every file to the collection and to this library list.
    List(od_library::NodeId),
    /// Add every file to the collection and into the open list before row `n`.
    Row(usize),
    /// Add every file to the collection and to the open playlist or set.
    Collection,
}

/// The drop target id → what happens to dropped files (pure).
pub(crate) fn file_drop(target: i32) -> FileDrop {
    use crate::browser::DropTarget;
    match crate::browser::drop_target(target) {
        DropTarget::Row(n) => FileDrop::Row(n),
        DropTarget::Tree(id) if id >= 0 => FileDrop::List(od_library::NodeId(id as i64)),
        DropTarget::Tree(_) => FileDrop::Collection,
        DropTarget::Deck if (1..=crate::decks::DECKS as i32).contains(&target) => {
            FileDrop::Deck(target as usize - 1)
        }
        DropTarget::Deck if target == -1 || target == -2 => {
            FileDrop::Player(target.unsigned_abs() as u8)
        }
        DropTarget::Deck => FileDrop::Collection,
    }
}

/// Files dropped on the window at drop target `target` (`DragState.target`:
/// the deck, Prepare player, tree list or list position under the
/// pointer, 0 = none): onto a deck or a player the first file is added to
/// the collection and loaded there; onto a list in the tree every audio
/// file (folders included) is added to the collection and that list;
/// anywhere else to the collection and the open playlist or set (at the
/// row under the pointer when the list is in its own order), in the
/// background.
pub(crate) fn files_dropped(app: &App, paths: Vec<std::path::PathBuf>, target: i32) {
    let files = dropped_audio(&paths);
    if files.is_empty() {
        app.status("No audio files in what was dropped");
        return;
    }
    let (list, at) = match file_drop(target) {
        FileDrop::Deck(d) => {
            crate::explorer::import_and_load(app, files[0].clone(), d.min(crate::decks::DECKS - 1));
            return;
        }
        FileDrop::Player(n) => {
            crate::explorer::import_and_show(app, files[0].clone(), n);
            return;
        }
        FileDrop::List(node) => {
            let st = app.state.borrow();
            let takes = !st.read_only()
                && st.lib.node(node).is_ok_and(|n| {
                    matches!(
                        n.kind,
                        od_library::NodeKind::Playlist | od_library::NodeKind::Set
                    )
                });
            (if takes { Some(node) } else { drop_list_of(&st) }, None)
        }
        FileDrop::Row(n) => {
            let ordered = crate::browser::reorderable(&app.state.borrow());
            (drop_list(app), ordered.then_some(n))
        }
        FileDrop::Collection => (drop_list(app), None),
    };
    let library = app.state.borrow().config.library_path.clone();
    let (tx, wake) = (app.worker_tx.clone(), app.analysis.wake.clone());
    app.status(od_i18n::trf("Adding {} dropped file(s)…", &[&files.len()]));
    std::thread::spawn(move || {
        let (added, failed) = match od_library::Library::open(&library) {
            Ok(mut lib) => add_files(&mut lib, &files, list, at),
            Err(e) => {
                let _ = tx.send(WorkerEvent::Message(format!("Cannot open library: {e}")));
                return;
            }
        };
        let _ = tx.send(WorkerEvent::ImportFinished { added, failed });
        let _ = wake.send(());
    });
}

/// Adds `files` to the collection (known paths are kept) and, when `list`
/// is given, inserts them into that list in order before position `at`
/// (`None`: at the end). Returns (added, failed).
pub(crate) fn add_files(
    lib: &mut od_library::Library,
    files: &[std::path::PathBuf],
    list: Option<od_library::NodeId>,
    at: Option<usize>,
) -> (usize, usize) {
    let (mut added, mut failed) = (0, 0);
    let mut ids = Vec::new();
    for f in files {
        let known = lib.track_by_path(f).ok().flatten().map(|t| t.id);
        match known
            .map(Ok)
            .unwrap_or_else(|| crate::workers::import_file(lib, f).inspect(|_| added += 1))
        {
            Ok(id) => ids.push(id),
            Err(e) => {
                failed += 1;
                tracing::warn!(path = %f.display(), error = %e, "dropped file not added");
            }
        }
    }
    if let (Some(n), false) = (list, ids.is_empty()) {
        if let Err(e) = lib.add_to_node(n, &ids, at) {
            tracing::warn!(error = %e, "dropped files not added to the list");
        }
    }
    (added, failed)
}

/// A file drag from the file manager enters the window: the drop targets
/// (decks, players, lists) follow `DragState` as for dragged rows. The
/// pointer is unknown until [`follow_pointer`] reads it.
pub(crate) fn file_drag_begin(app: &App, files: usize) {
    let d = app.ui.global::<crate::DragState>();
    if !d.get_files() {
        d.set_x(-10_000.0);
        d.set_y(-10_000.0);
        d.set_target(0);
    }
    d.set_kind("tracks".into());
    d.set_files(true);
    d.set_active(true);
    d.set_label(od_i18n::trf("{} file(s)", &[&files.max(1)]).into());
    app.ui.set_file_drag(true);
}

/// The file drag left the window or was dropped.
pub(crate) fn file_drag_end(app: &App) {
    let d = app.ui.global::<crate::DragState>();
    d.set_active(false);
    d.set_files(false);
    d.set_target(0);
    app.ui.set_file_drag(false);
}

/// Moves the drag position to the pointer while files hover the window
/// (the OS reports no position with native file drags).
fn follow_pointer(app: &App) {
    if let Some((x, y)) = pointer_in_window(app.ui.window()) {
        let d = app.ui.global::<crate::DragState>();
        d.set_x(x);
        d.set_y(y);
    }
}

/// A point in the window's content (AppKit points, origin bottom-left of
/// the content of height `content_h`) in logical window coordinates (origin
/// top-left), for a backing scale `backing` and a UI scale factor `ui`
/// (pure).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn content_point_to_logical(
    px: f64,
    py: f64,
    content_h: f64,
    backing: f64,
    ui: f64,
) -> (f32, f32) {
    let k = if ui > 0.0 { backing / ui } else { 1.0 };
    ((px * k) as f32, ((content_h - py) * k) as f32)
}

/// The pointer over the window in logical coordinates, read from AppKit
/// while a file drag is in flight (the window gets no mouse events then).
#[cfg(target_os = "macos")]
fn pointer_in_window(window: &slint::Window) -> Option<(f32, f32)> {
    use slint::winit_030::WinitWindowAccessor;
    use slint::winit_030::winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let ui = window.scale_factor() as f64;
    window
        .with_winit_window(|w| {
            let handle = w.window_handle().ok()?;
            let RawWindowHandle::AppKit(h) = handle.as_raw() else {
                return None;
            };
            // SAFETY: winit's content view lives as long as its window, and
            // this runs on the main (UI) thread.
            let view: &objc2_app_kit::NSView = unsafe { h.ns_view.cast().as_ref() };
            let p = view.window()?.mouseLocationOutsideOfEventStream();
            let frame = view.frame();
            let (x, y) = content_point_to_logical(
                p.x - frame.origin.x,
                p.y - frame.origin.y,
                frame.size.height,
                w.scale_factor(),
                ui,
            );
            Some((x, y))
        })
        .flatten()
}

#[cfg(not(target_os = "macos"))]
fn pointer_in_window(_window: &slint::Window) -> Option<(f32, f32)> {
    None
}

/// Native file drops (winit: Finder / the file manager). The OS reports one
/// event per file; they are collected and handled together, at the drop
/// target under the pointer (macOS reads the pointer while the files
/// hover; elsewhere they go to the collection and the open list).
/// Platforms whose Slint backend delivers OS drops to `DropArea` use that
/// path instead.
pub(crate) fn wire_native_drop(app: &std::rc::Rc<App>) {
    use slint::winit_030::{WinitWindowAccessor, winit::event::WindowEvent};
    let pending: std::rc::Rc<std::cell::RefCell<Vec<std::path::PathBuf>>> = Default::default();
    let hovered = std::rc::Rc::new(std::cell::Cell::new(0usize));
    let weak = std::rc::Rc::downgrade(app);
    let timer = std::rc::Rc::new(slint::Timer::default());
    let follow = std::rc::Rc::new(slint::Timer::default());
    app.ui.window().on_winit_window_event(move |_, event| {
        match event {
            WindowEvent::HoveredFile(_) => {
                hovered.set(hovered.get() + 1);
                if let Some(a) = weak.upgrade() {
                    file_drag_begin(&a, hovered.get());
                    follow_pointer(&a);
                    let w = weak.clone();
                    follow.start(
                        slint::TimerMode::Repeated,
                        std::time::Duration::from_millis(30),
                        move || {
                            if let Some(a) = w.upgrade() {
                                follow_pointer(&a);
                            }
                        },
                    );
                }
            }
            WindowEvent::Touch(_) => {
                if let Some(a) = weak.upgrade() {
                    crate::touch::touched(&a);
                }
            }
            WindowEvent::HoveredFileCancelled => {
                hovered.set(0);
                follow.stop();
                if let Some(a) = weak.upgrade() {
                    file_drag_end(&a);
                }
            }
            WindowEvent::DroppedFile(path) => {
                hovered.set(0);
                pending.borrow_mut().push(path.clone());
                let (weak, pending, follow) = (weak.clone(), pending.clone(), follow.clone());
                timer.start(
                    slint::TimerMode::SingleShot,
                    std::time::Duration::from_millis(120),
                    move || {
                        follow.stop();
                        let paths = std::mem::take(&mut *pending.borrow_mut());
                        if let Some(a) = weak.upgrade() {
                            let target = a.ui.global::<crate::DragState>().get_target();
                            file_drag_end(&a);
                            files_dropped(&a, paths, target);
                        }
                    },
                );
            }
            _ => {}
        }
        slint::winit_030::EventResult::Propagate
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_rows_and_reasons() {
        let p = ImportPlan {
            tracks_in_file: 5,
            tracks_new: 2,
            tracks_known: 1,
            playlists: 3,
            folders: 1,
            cues: 7,
            grids: 2,
            keys: 1,
            left_out: vec![LeftOut {
                item: "Cy – Lost".into(),
                reason: LeftOutReason::FileMissing,
            }],
        };
        let rows = plan_rows(&p, ExportFormat::LibraryXml);
        let values: Vec<(String, String)> = rows
            .iter()
            .map(|r| (r.label.to_string(), r.after.to_string()))
            .collect();
        assert_eq!(values[0], ("New tracks".into(), "2".into()));
        assert_eq!(values[4], ("Cue points".into(), "7".into()));
        assert_eq!(rows.len(), 7);
        // A Music library export has no cues, grids or keys to show.
        assert_eq!(plan_rows(&p, ExportFormat::MusicXml).len(), 4);
        let before = LibraryCounts {
            tracks: 10,
            ..Default::default()
        };
        let after = LibraryCounts {
            tracks: 12,
            ..Default::default()
        };
        let r = result_rows(&before, &after);
        assert_eq!((r[0].before.as_str(), r[0].after.as_str()), ("10", "12"));
        assert!(r[0].changed && !r[1].changed);
        let many: Vec<LeftOut> = (0..LEFT_OUT_SHOWN + 5)
            .map(|i| LeftOut {
                item: format!("T{i}"),
                reason: LeftOutReason::ListEntries(i),
            })
            .collect();
        let (shown, more) = left_out_rows(&many);
        assert_eq!((shown.len(), more), (LEFT_OUT_SHOWN, 5));
        assert_eq!(
            reason_text(&LeftOutReason::UnreadableCues(3)),
            "cue points that cannot be read: 3"
        );
    }

    #[test]
    fn review_texts_have_german() {
        for s in [
            "file not found",
            "not a file on this computer (stream or cloud)",
            "video or podcast",
            "cue points that cannot be read: {}",
            "a list the other app makes itself",
            "tracks that do not carry over: {}",
            "LIBRARY XML",
            "MUSIC / ITUNES LIBRARY",
            "New tracks",
            "Tracks already in your library",
            "Playlists",
            "Folders",
            "Cue points",
            "Beatgrids",
            "Keys",
            "Tracks",
            "Nothing in this file can be imported.",
            "This file cannot be read: {}",
            "No undo copy could be made, nothing was imported: {}",
            "The import stopped: {}. UNDO IMPORT puts back what was written.",
            "UNDO IMPORT puts the library back exactly as it was before, as long as this window is open.",
            "Library import done: {} tracks before, {} after",
            "Undo failed: {}",
            "The library is as it was before the import.",
            "Library import undone",
            "Choose the rekordbox preferences folder",
            "NOW",
            "BEAT",
            "BAR",
            "KILL",
            "OFF",
        ] {
            assert!(od_i18n::has_german(s), "no German for {s:?}");
        }
    }
}
