//! Translations for text produced in Rust code (status messages, states).
//! Text in the Slint UI is translated with Slint's own gettext bundles.
//!
//! Messages are looked up by their English text; `{}` placeholders are
//! filled in order by [`trf`].

use std::sync::atomic::{AtomicU8, Ordering};

static LANG: AtomicU8 = AtomicU8::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    English,
    German,
}

pub fn set_language(lang: Language) {
    LANG.store(lang as u8, Ordering::Relaxed);
}

pub fn language() -> Language {
    if LANG.load(Ordering::Relaxed) == 1 {
        Language::German
    } else {
        Language::English
    }
}

const DE: &[(&str, &str)] = &[
    (
        "Export the files to a folder",
        "Dateien in einen Ordner exportieren",
    ),
    (
        "Analyse the track first: there is no beatgrid to lock",
        "Zuerst den Track analysieren: es gibt kein Beatgrid zum Sperren",
    ),
    ("OVERLAPS {}", "ÜBERLAPPT {}"),
    ("LINEAR", "LINEAR"),
    ("SQUARE", "QUADRAT"),
    ("ROOT", "WURZEL"),
    ("SOFT", "WEICH"),
    ("CUE {} {}", "CUE {} {}"),
    ("CUE {} MISSING", "CUE {} FEHLT"),
    ("CHASE {} {}", "CHASE {} {}"),
    ("PAD {} {}", "PAD {} {}"),
    ("PAD {} EMPTY", "PAD {} LEER"),
    ("AT ONCE", "SOFORT"),
    ("½ BEAT", "½ BEAT"),
    ("{} BEATS", "{} BEATS"),
    ("BAR {}", "TAKT {}"),
    ("RIG {} · SHOW {} · STROBE CAP {}", "RIG {} · SHOW {} · STROBE-GRENZE {}"),
    ("{} FIXTURES · {} OUTPUTS", "{} SCHEINWERFER · {} AUSGÄNGE"),
    ("{} LOOKS · {} CUES · {} LIGHT CUES", "{} LOOKS · {} CUES · {} LICHT-CUES"),
    ("Loaded {}", "{} geladen"),
    ("Saved {}", "{} gesichert"),
    ("Copied to {}", "Nach {} kopiert"),
    ("Deleted {}", "{} gelöscht"),
    ("Imported and loaded {}", "{} importiert und geladen"),
    ("{} exists already: choose it and press SAVE", "{} gibt es schon: wähle es und drück SICHERN"),
    ("Renamed to {}", "Umbenannt in {}"),
    ("Lighting file (venue, rig and show)", "Lichtdatei (Location, Rig und Show)"),
    ("Exported to {}", "Nach {} exportiert"),
    ("PAST CHANNEL 512", "ÜBER KANAL 512"),
    ("DMX OUT IS OFF", "DMX-AUSGABE IST AUS"),
    ("LOST {} S AGO · RETRYING · {}", "VOR {} S VERLOREN · NEUER VERSUCH · {}"),
    ("BACK {} S AGO · SENDING", "VOR {} S ZURÜCK · SENDET"),
    ("SENDING", "SENDET"),
    ("OFF", "AUS"),
    ("SHOW FAILED · FAILSAFE", "SHOW AUSGEFALLEN · FAILSAFE"),
    ("{} LOST · RETRYING", "{} VERLOREN · NEUER VERSUCH"),
    ("BACK · SENDING", "ZURÜCK · SENDET"),
    ("{} SENDING", "{} SENDEN"),
    ("Lighting: universes are 1 to {}", "Licht: Universen gehen von 1 bis {}"),
    ("Lighting: addresses are 1 to 512", "Licht: Adressen gehen von 1 bis 512"),
    ("Lighting: the count is 1 to 512", "Licht: die Anzahl geht von 1 bis 512"),
    ("Lighting: {} fixture(s) patched", "Licht: {} Scheinwerfer gepatcht"),
    ("Lighting: a fixture has 1 to 512 channels", "Licht: ein Scheinwerfer hat 1 bis 512 Kanäle"),
    ("Wrong PIN", "Falsche PIN"),
    ("The PIN is 4 to 8 digits", "Die PIN hat 4 bis 8 Ziffern"),
    ("Lighting: setup PIN removed", "Licht: Setup-PIN entfernt"),
    ("Lighting: setup PIN set. LOCK closes the setup.", "Licht: Setup-PIN gesetzt. SPERREN schließt das Setup."),
    ("Lighting: {}", "Licht: {}"),
    ("Analysis locked", "Analyse gesperrt"),
    ("Analysis unlocked", "Analyse entsperrt"),
    (
        "Tags reloaded from the files",
        "Tags neu aus den Dateien gelesen",
    ),
    (
        "Exported {} file(s) to {}",
        "{} Datei(en) nach {} exportiert",
    ),
    ("Play count reset", "Wiedergabezähler zurückgesetzt"),
    (
        "Memory cues made hot cues",
        "Memory Cues zu Hot Cues gemacht",
    ),
    ("Hot cues sorted by time", "Hot Cues nach Zeit sortiert"),
    ("Nothing to change", "Nichts zu ändern"),
    ("Already in the collection", "Schon in der Sammlung"),
    ("Syncing sticks…", "Sticks werden synchronisiert…"),
    (
        "Sequence stored on Pattern Recall pad {}",
        "Sequenz auf Pattern-Recall-Pad {} gespeichert",
    ),
    (
        "Every Pattern Recall pad holds a sequence: clear one first",
        "Jedes Pattern-Recall-Pad hält eine Sequenz: zuerst eines leeren",
    ),
    ("All tracks", "Alle Tracks"),
    ("{} TRACKS  ·  {}", "{} TRACKS  ·  {}"),
    ("Not connected", "Nicht verbunden"),
    ("In sync", "Synchron"),
    (
        "No moved files found in that folder",
        "In diesem Ordner wurden keine verschobenen Dateien gefunden",
    ),
    (
        "Found {} moved file(s)",
        "{} verschobene Datei(en) gefunden",
    ),
    (
        "Split into {} track files next to the recording",
        "In {} Track-Dateien neben der Aufnahme geteilt",
    ),
    ("Split failed: {}", "Teilen fehlgeschlagen: {}"),
    (
        "Tracks on a stick are read-only here — edit them in your library and the stick follows",
        "Tracks auf einem Stick sind hier schreibgeschützt — bearbeite sie in deiner Bibliothek, der Stick folgt",
    ),
    ("In sync · {}", "Synchron · {}"),
    ("Add tracks", "Tracks hinzufügen"),
    ("Remove tracks", "Tracks entfernen"),
    ("Reorder", "Neu ordnen"),
    ("Edit cues", "Cues bearbeiten"),
    ("Edit beatgrid", "Beatgrid bearbeiten"),
    ("Edit key", "Tonart bearbeiten"),
    ("Edit tags", "Tags bearbeiten"),
    ("Rename", "Umbenennen"),
    ("Move", "Verschieben"),
    ("Delete", "Löschen"),
    ("Edit transition note", "Übergangsnotiz bearbeiten"),
    ("Edit backup tracks", "Ersatztracks bearbeiten"),
    ("Edit smart rules", "Smart-Regeln bearbeiten"),
    ("Edit set", "Set bearbeiten"),
    ("Create playlist", "Playlist erstellen"),
    ("Create set", "Set erstellen"),
    ("Create folder", "Ordner erstellen"),
    ("Create smart", "Smart-Liste erstellen"),
    ("ANALYSIS", "ANALYSE"),
    ("IMPORTED", "IMPORTIERT"),
    ("MANUAL", "MANUELL"),
    ("NONE", "KEINS"),
    ("{} change waiting", "{} Änderung offen"),
    ("{} changes waiting", "{} Änderungen offen"),
    ("Copying {}", "Kopiere {}"),
    ("Converting {}", "Konvertiere {}"),
    (", {} converted to WAV", ", {} in WAV konvertiert"),
    (", {} could not be converted", ", {} nicht konvertierbar"),
    ("Writing {} data", "Schreibe {}-Daten"),
    ("Preparing", "Vorbereitung"),
    ("{} FREE OF {}", "{} FREI VON {}"),
    (
        "Adding music: {} of {} files",
        "Musik wird hinzugefügt: {} von {} Dateien",
    ),
    ("Added {} tracks", "{} Tracks hinzugefügt"),
    (
        "Added {} tracks, {} files could not be read",
        "{} Tracks hinzugefügt, {} Dateien nicht lesbar",
    ),
    ("Analysis complete", "Analyse abgeschlossen"),
    ("Writing sample tracks", "Beispieltracks werden geschrieben"),
    ("Sample tracks could not be written", "Beispieltracks konnten nicht geschrieben werden"),
    ("{} closed: not enough room", "{} geschlossen: zu wenig Platz"),
    ("ANALYSING {}/{}", "ANALYSE {}/{}"),
    ("ANALYSING {}/{} · {} BPM", "ANALYSE {}/{} · {} BPM"),
    ("DROP {}", "DROP {}"),
    ("BREAK {}", "BREAK {}"),
    ("Synced to the phrase: {} beats", "Auf die Phrase gesynct: {} Beats"),
    ("Phrase start cancelled", "Phrasenstart abgebrochen"),
    (
        "Phrase start: cue a track on a paused deck first",
        "Phrasenstart: zuerst einen Track auf einem pausierten Deck cuen",
    ),
    (
        "Phrase start: no other deck is playing to wait for",
        "Phrasenstart: kein anderes Deck läuft, auf das gewartet werden kann",
    ),
    (
        "Deck {} starts on deck {}'s next phrase ({} bars)",
        "Deck {} startet auf der nächsten Phrase von Deck {} ({} Takte)",
    ),
    (
        "Nothing to capture: roll or play a loop first",
        "Nichts zu sichern: zuerst einen Roll oder Loop spielen",
    ),
    ("Loop captured on hot cue {}", "Loop auf Hot Cue {} gesichert"),
    ("This loop is already stored", "Dieser Loop ist schon gespeichert"),
    (
        "Loop captured as a memory loop (the hot cues are full)",
        "Loop als Memory Loop gesichert (die Hot Cues sind voll)",
    ),
    (
        "{} synced: {} copied, {} updated, {} removed{}",
        "{} synchronisiert: {} kopiert, {} aktualisiert, {} entfernt{}",
    ),
    ("Sync failed: {}", "Sync fehlgeschlagen: {}"),
    ("{} can be removed safely", "{} kann sicher entfernt werden"),
    ("Eject failed: {}", "Auswerfen fehlgeschlagen: {}"),
    ("Added {} track(s) to {}", "{} Track(s) zu {} hinzugefügt"),
    (
        "Created — click the title to rename it",
        "Erstellt — Titel anklicken zum Umbenennen",
    ),
    ("Renamed", "Umbenannt"),
    (
        "Deleted — ⌘Z brings it back",
        "Gelöscht — ⌘Z holt es zurück",
    ),
    ("Rules updated", "Regeln aktualisiert"),
    ("Hot cue {} set", "Hot Cue {} gesetzt"),
    ("Hot cue removed", "Hot Cue entfernt"),
    ("Memory cue added", "Memory Cue hinzugefügt"),
    ("Queued for analysis", "Zur Analyse eingereiht"),
    ("Transition note saved", "Übergangsnotiz gespeichert"),
    ("Rebuilding stick…", "Stick wird neu geschrieben…"),
    ("Ejecting…", "Wird ausgeworfen…"),
    (
        "Stick forgotten. Its files were left untouched.",
        "Stick vergessen. Seine Dateien bleiben unverändert.",
    ),
    (
        "Select the playlist, set or folder that should go on the stick first",
        "Wähle zuerst die Playlist, das Set oder den Ordner für den Stick",
    ),
    (
        "Registering {} with {} — the first sync starts now",
        "{} wird mit {} registriert — der erste Sync startet jetzt",
    ),
    (
        "Removed {} track(s) — ⌘Z to undo",
        "{} Track(s) entfernt — ⌘Z macht es rückgängig",
    ),
    ("Undid: {}", "Rückgängig: {}"),
    ("Redid: {}", "Wiederholt: {}"),
    ("Nothing to undo", "Nichts rückgängig zu machen"),
    ("Nothing to redo", "Nichts zu wiederholen"),
    ("Change taken over", "Änderung übernommen"),
    (
        "Kept the library version",
        "Version der Bibliothek behalten",
    ),
    (
        "This track has no beatgrid yet — analyse it first",
        "Dieser Track hat noch kein Beatgrid — zuerst analysieren",
    ),
    ("Adding music…", "Musik wird hinzugefügt…"),
    ("Controller connected: {}", "Controller verbunden: {}"),
    ("Controller disconnected: {}", "Controller getrennt: {}"),
    ("Loaded {} on deck {}", "{} auf Deck {} geladen"),
    ("Deck {}: {} BPM", "Deck {}: {} BPM"),
    ("Type a tempo the track can reach (half to double its BPM)", "Ein Tempo eingeben, das der Track erreichen kann (halbe bis doppelte BPM)"),
    ("Could not load {}", "{} konnte nicht geladen werden"),
    ("Stems ready on deck {}", "Stems auf Deck {} bereit"),
    (
        "Downloading the stem model — {} % ({} of {} MB)",
        "Stem-Modell wird geladen — {} % ({} von {} MB)",
    ),
    (
        "No internet connection. Check your network and try again.",
        "Keine Internetverbindung. Prüfe dein Netzwerk und versuche es noch einmal.",
    ),
    (
        "The download server did not answer properly. Try again in a few minutes.",
        "Der Download-Server hat nicht richtig geantwortet. Versuche es in ein paar Minuten noch einmal.",
    ),
    (
        "There is not enough free disk space. The stem model needs {} MB free.",
        "Auf der Festplatte ist nicht genug Platz. Das Stem-Modell braucht {} MB freien Speicher.",
    ),
    (
        "The download was damaged on the way and was thrown away. Please try again.",
        "Der Download wurde unterwegs beschädigt und verworfen. Bitte versuche es noch einmal.",
    ),
    (
        "The download stopped before it was finished. It continues where it left off.",
        "Der Download wurde unterbrochen. Er macht dort weiter, wo er aufgehört hat.",
    ),
    (
        "The stem model could not be saved on this computer. Check that the disk is writable and try again.",
        "Das Stem-Modell konnte nicht auf diesem Computer gespeichert werden. Prüfe, ob die Festplatte beschreibbar ist, und versuche es noch einmal.",
    ),
    (
        "The stem model is damaged. RETRY deletes it and downloads it again.",
        "Das Stem-Modell ist beschädigt. NOCHMAL löscht es und lädt es neu herunter.",
    ),
    (
        "Stems need a one-time download of {} MB. Your music stays on this computer.",
        "Stems brauchen einmalig einen Download von {} MB. Deine Musik bleibt auf diesem Computer.",
    ),
    ("The stem model is incomplete", "Das Stem-Modell ist unvollständig"),
    ("The stem model is damaged", "Das Stem-Modell ist beschädigt"),
    ("Stems need a one-time download", "Stems brauchen einmalig einen Download"),
    (
        "Stems cannot be separated with this build. Stems separated on another computer still play.",
        "Mit dieser Version lassen sich keine Stems trennen. Auf einem anderen Computer getrennte Stems spielen trotzdem.",
    ),
    ("Damaged, {} MB on disk", "Beschädigt, {} MB auf der Festplatte"),
    ("Not installed. The download is {} MB.", "Nicht installiert. Der Download hat {} MB."),
    ("Incomplete, {} of {} MB", "Unvollständig, {} von {} MB"),
    ("Installed, {} MB on disk", "Installiert, {} MB auf der Festplatte"),
    ("The stem model is still downloading", "Das Stem-Modell wird noch geladen"),
    (
        "Stem model removed. Stems you already separated keep working.",
        "Stem-Modell entfernt. Bereits getrennte Stems funktionieren weiter.",
    ),
    (
        "The stem model could not be removed. Is it in use by another program?",
        "Das Stem-Modell konnte nicht entfernt werden. Wird es von einem anderen Programm benutzt?",
    ),
    (
        "Stems stay off for now. The download is in Preferences › Analysis › Stems.",
        "Stems bleiben erst einmal aus. Der Download liegt unter Einstellungen › Analyse › Stems.",
    ),
    ("Stem model installed", "Stem-Modell installiert"),
    (
        "Download paused. DOWNLOAD continues it where it stopped.",
        "Download angehalten. HERUNTERLADEN setzt ihn an dieser Stelle fort.",
    ),
    (
        "OpenDJDecks {} is available",
        "OpenDJDecks {} ist verfügbar",
    ),
    (" (pre-release)", " (Vorabversion)"),
    (
        "Installing OpenDJDecks {} …",
        "OpenDJDecks {} wird installiert …",
    ),
    (
        "Downloading OpenDJDecks {} — {} %",
        "OpenDJDecks {} wird geladen — {} %",
    ),
    (
        "Downloading OpenDJDecks {} …",
        "OpenDJDecks {} wird geladen …",
    ),
    (
        "Version {} · the last check failed: {}",
        "Version {} · die letzte Prüfung ist fehlgeschlagen: {}",
    ),
    (
        "OpenDJDecks {} is ready — restart to finish",
        "OpenDJDecks {} ist bereit — zum Abschließen neu starten",
    ),
    (
        "Update failed: {} — this version keeps working",
        "Update fehlgeschlagen: {} — diese Version läuft weiter",
    ),
    (
        "OpenDJDecks {} is up to date",
        "OpenDJDecks {} ist aktuell",
    ),
    (
        "Could not check for updates: {}",
        "Suche nach Updates fehlgeschlagen: {}",
    ),
    ("Checking for updates …", "Suche nach Updates …"),
    (
        "An update is already in progress",
        "Ein Update läuft bereits",
    ),
    (
        "Version {} skipped — newer versions are offered again",
        "Version {} übersprungen — neuere Versionen werden wieder angeboten",
    ),
    (
        "The update is installed when you restart",
        "Das Update wird beim Neustart installiert",
    ),
    (
        "OpenDJDecks {} is verified and open — drag it to Applications, then restart",
        "OpenDJDecks {} ist geprüft und geöffnet — in Programme ziehen, dann neu starten",
    ),
    (
        "This copy cannot update itself — download OpenDJDecks {} from the release page",
        "Diese Installation kann sich nicht selbst aktualisieren — OpenDJDecks {} von der Release-Seite laden",
    ),
    (
        "Version {} · up to date, checked {}",
        "Version {} · aktuell, geprüft {}",
    ),
    ("Version {} · {} is available", "Version {} · {} ist verfügbar"),
    ("Version {} · not checked yet", "Version {} · noch nicht geprüft"),
    ("Checking …", "Prüfe …"),
    ("never", "nie"),
    ("just now", "gerade eben"),
    ("{} min ago", "vor {} Min."),
    ("{} h ago", "vor {} Std."),
    ("{} days ago", "vor {} Tagen"),
    ("Check for updates", "Nach Updates suchen"),
    ("Rating set", "Bewertung gesetzt"),
    ("Tags saved", "Tags gespeichert"),
    (
        "Recorded as WAV, then encoded when the recording stops.",
        "Als WAV aufgenommen und nach dem Stopp kodiert.",
    ),
    ("Colour set", "Farbe gesetzt"),
    (
        "Stem separation failed: {}",
        "Stem-Trennung fehlgeschlagen: {}",
    ),
    (
        "Recording saved: {} ({} tracks)",
        "Aufnahme gespeichert: {} ({} Tracks)",
    ),
    (
        "Audio starts when you load a track",
        "Audio startet, sobald du einen Track lädst",
    ),
    (
        "Adding {} to the collection…",
        "{} wird zur Sammlung hinzugefügt…",
    ),
    (
        "Added {} to the collection — analysing",
        "{} zur Sammlung hinzugefügt — wird analysiert",
    ),
    (
        "Could not add {}: {}",
        "{} konnte nicht hinzugefügt werden: {}",
    ),
    (
        "Add a folder to the Explorer",
        "Ordner zum Explorer hinzufügen",
    ),
    // Preferences and keyboard (SPEC §12.2).
    ("Memory cue removed", "Memory Cue entfernt"),
    ("Recording failed: {}", "Aufnahme fehlgeschlagen: {}"),
    (
        "Recording could not start: {}",
        "Aufnahme konnte nicht starten: {}",
    ),
    ("Recording saved as {}", "Aufnahme gespeichert als {}"),
    (
        "Converting the recording failed — the WAV file is kept",
        "Umwandeln der Aufnahme fehlgeschlagen — die WAV-Datei bleibt",
    ),
    ("Backup saved: {}", "Backup gespeichert: {}"),
    (
        "Library location saved — applies after restart",
        "Speicherort der Bibliothek gespeichert — gilt nach Neustart",
    ),
    (
        "All keys are back to their defaults",
        "Alle Tasten sind wieder auf Standard",
    ),
    ("AUTO", "AUTO"),
    ("Automatic", "Automatisch"),
    ("DEVICE", "GERÄT"),
    ("Controller mapping file", "Controller-Mapping-Datei"),
    ("Export keymap", "Tastenbelegung exportieren"),
    ("Import keymap", "Tastenbelegung importieren"),
    ("Folder for recordings", "Ordner für Aufnahmen"),
    (
        "Library file (library.sqlite)",
        "Bibliotheksdatei (library.sqlite)",
    ),
    (
        "Needs ffmpeg on the PATH; without it the recording stays a WAV file.",
        "Braucht ffmpeg im PATH; ohne bleibt die Aufnahme eine WAV-Datei.",
    ),
    (
        "Recorded as WAV, then converted with ffmpeg when the recording stops.",
        "Wird als WAV aufgenommen und beim Stoppen mit ffmpeg umgewandelt.",
    ),
    ("No backups yet", "Noch keine Backups"),
    ("{} backups · newest {}", "{} Backups · neuestes {}"),
    (
        "OPEN WITH ⌘,  ·  CLOSE WITH ESC OR ⌘W",
        "ÖFFNEN MIT ⌘,  ·  SCHLIESSEN MIT ESC ODER ⌘W",
    ),
    (
        "OPEN WITH CTRL+,  ·  CLOSE WITH ESC OR CTRL+W",
        "ÖFFNEN MIT STRG+,  ·  SCHLIESSEN MIT ESC ODER STRG+W",
    ),
    ("PREPARE", "PREPARE"),
    ("PERFORM", "PERFORM"),
    ("{} (not connected)", "{} (nicht verbunden)"),
    ("{} — connected", "{} — verbunden"),
    ("MIDI input: {}", "MIDI-Eingang: {}"),
    ("{} now on {}", "{} jetzt auf {}"),
    (
        "{} is already used by {}. Replace it there?",
        "{} ist schon belegt mit {}. Dort ersetzen?",
    ),
    ("{} has no key now", "{} hat jetzt keine Taste"),
    ("{} is back to its default", "{} ist wieder auf Standard"),
    (
        "Press the new keys for {}",
        "Drücke die neuen Tasten für {}",
    ),
    ("Keymap exported to {}", "Tastenbelegung exportiert nach {}"),
    ("Imported {} bindings", "{} Belegungen importiert"),
    (
        " · skipped {} unknown, {} invalid",
        " · {} unbekannte, {} ungültige übersprungen",
    ),
    ("Not a keymap file: {}", "Keine Tastenbelegungs-Datei: {}"),
    (
        "Not imported: {} is used by both {} and {}",
        "Nicht importiert: {} ist doppelt belegt mit {} und {}",
    ),
    ("Deck {}: {}", "Deck {}: {}"),
    ("Deck {}", "Deck {}"),
    ("General", "Allgemein"),
    ("Browser", "Browser"),
    ("Prepare player", "Prepare-Player"),
    ("Sampler", "Sampler"),
    // Keyboard actions.
    ("Open preferences", "Einstellungen öffnen"),
    ("Undo", "Rückgängig"),
    ("Redo", "Wiederholen"),
    ("Switch to Prepare", "Zu Prepare wechseln"),
    ("Switch to Perform", "Zu Perform wechseln"),
    ("Switch to Lighting", "Zu Lighting wechseln"),
    ("Club mode on / off", "Club-Modus an / aus"),
    ("Load tracks on two decks", "Tracks auf zwei Decks laden"),
    (
        "Set a cue point while paused",
        "Im Pausenzustand einen Cue-Punkt setzen",
    ),
    ("Start a deck with PLAY", "Ein Deck mit PLAY starten"),
    (
        "Match the other deck's tempo by ear",
        "Das Tempo des anderen Decks nach Gehör angleichen",
    ),
    (
        "Start it on the one and keep the beats together",
        "Auf der Eins starten und die Beats zusammenhalten",
    ),
    (
        "Club mode: sync is off, beatmatch by ear",
        "Club-Modus: Sync ist aus, Beatmatching nach Gehör",
    ),
    ("Club mode off", "Club-Modus aus"),
    ("Audio is not running", "Audio läuft nicht"),
    (
        "Show / hide the lighting strip",
        "Lichtleiste ein- / ausblenden",
    ),
    ("Browse layout on / off", "Browse-Layout an / aus"),
    (
        "Vertical waveforms on / off",
        "Senkrechte Wellenformen an / aus",
    ),
    (
        "Fixture file (Open Fixture Library)",
        "Scheinwerfer-Datei (Open Fixture Library)",
    ),
    ("TRACKS", "TRACKS"),
    ("PLAYLISTS", "PLAYLISTS"),
    ("Playlists", "Playlists"),
    ("SOURCE", "QUELLE"),
    ("Show the library", "Bibliothek zeigen"),
    ("Show USB sticks", "USB-Sticks zeigen"),
    ("Show recordings", "Aufnahmen zeigen"),
    ("Start / stop recording", "Aufnahme starten / stoppen"),
    ("Sync USB sticks now", "USB-Sticks jetzt synchronisieren"),
    ("Eject the selected stick", "Gewählten Stick auswerfen"),
    ("Quantize on / off", "Quantisierung an / aus"),
    ("Search", "Suchen"),
    ("Previous track", "Vorheriger Track"),
    ("Next track", "Nächster Track"),
    ("Previous list", "Vorherige Liste"),
    ("Next list", "Nächste Liste"),
    ("Select all tracks", "Alle Tracks auswählen"),
    (
        "Load the selected track (free deck)",
        "Gewählten Track laden (freies Deck)",
    ),
    (
        "Load the selected track to deck {}",
        "Gewählten Track auf Deck {} laden",
    ),
    (
        "Remove the selected tracks from the list",
        "Gewählte Tracks aus der Liste entfernen",
    ),
    ("New playlist", "Neue Playlist"),
    ("New set", "Neues Set"),
    ("New folder", "Neuer Ordner"),
    ("New smart list", "Neue Smart-Liste"),
    ("Delete the open list", "Offene Liste löschen"),
    ("Add a music folder", "Musikordner hinzufügen"),
    ("Find moved files", "Verschobene Dateien finden"),
    ("Preview play / pause", "Vorhören Play / Pause"),
    ("Back to the cue point", "Zurück zum Cue-Punkt"),
    ("Hot cue {}", "Hot Cue {}"),
    ("Clear hot cue {}", "Hot Cue {} löschen"),
    ("Add a memory cue", "Memory Cue setzen"),
    (
        "Delete the memory cue at the cursor",
        "Memory Cue am Cursor löschen",
    ),
    ("Previous cue", "Vorheriger Cue"),
    ("Next cue", "Nächster Cue"),
    ("Cursor back (fine)", "Cursor zurück (fein)"),
    ("Cursor forward (fine)", "Cursor vor (fein)"),
    ("Cursor back 2 s", "Cursor 2 s zurück"),
    ("Cursor forward 2 s", "Cursor 2 s vor"),
    ("Beatgrid 10 ms earlier", "Beatgrid 10 ms früher"),
    ("Beatgrid 10 ms later", "Beatgrid 10 ms später"),
    ("Beatgrid double tempo", "Beatgrid doppeltes Tempo"),
    ("Beatgrid half tempo", "Beatgrid halbes Tempo"),
    (
        "Set the downbeat at the cursor",
        "Downbeat am Cursor setzen",
    ),
    (
        "Lock / unlock the beatgrid",
        "Beatgrid sperren / entsperren",
    ),
    ("Analyse the track", "Track analysieren"),
    ("Play / pause", "Play / Pause"),
    ("Cue", "Cue"),
    ("Jump to the track start", "Zum Trackanfang springen"),
    ("Beat sync", "Beat Sync"),
    ("Sync master", "Sync-Master"),
    ("Master tempo (key lock)", "Master Tempo (Key Lock)"),
    ("Pitch bend + (hold)", "Pitch Bend + (halten)"),
    ("Pitch bend − (hold)", "Pitch Bend − (halten)"),
    ("Tempo up", "Tempo hoch"),
    ("Tempo down", "Tempo runter"),
    ("Beat jump forward", "Beat Jump vor"),
    ("Beat jump back", "Beat Jump zurück"),
    ("Loop in", "Loop In"),
    ("Loop out", "Loop Out"),
    ("Exit / reloop", "Exit / Reloop"),
    ("Auto beat loop", "Auto Beat Loop"),
    ("Halve the loop", "Loop halbieren"),
    ("Double the loop", "Loop verdoppeln"),
    ("{}-beat loop", "{}-Beat-Loop"),
    ("Next memory cue", "Nächster Memory Cue"),
    ("Previous memory cue", "Vorheriger Memory Cue"),
    ("Sampler slot {}", "Sampler-Slot {}"),
    // Keymap: pad modes, key, effects, sampler (Perform panels).
    ("Next pad mode", "Nächster Pad-Modus"),
    ("Previous pad mode", "Vorheriger Pad-Modus"),
    ("Next hot cue bank", "Nächste Hot-Cue-Bank"),
    ("Key sync on / off", "Key Sync an / aus"),
    ("Key shift up a semitone", "Tonart einen Halbton höher"),
    ("Key shift down a semitone", "Tonart einen Halbton tiefer"),
    ("Effects", "Effekte"),
    (
        "Show / hide the Tempo FX panel",
        "Tempo-FX-Panel ein- / ausblenden",
    ),
    ("FX {} on / off", "FX {} an / aus"),
    ("FX {}: next effect", "FX {}: nächster Effekt"),
    ("FX {}: shorter beat", "FX {}: kürzerer Beat"),
    ("FX {}: longer beat", "FX {}: längerer Beat"),
    ("FX {}: tap the tempo", "FX {}: Tempo tappen"),
    ("FX {}: vinyl brake (hold)", "FX {}: Vinyl Brake (halten)"),
    ("FX {}: backspin (hold)", "FX {}: Backspin (halten)"),
    ("FX {}: echo out (hold)", "FX {}: Echo Out (halten)"),
    ("Channel FX: next", "Channel FX: nächster"),
    ("Channel FX: previous", "Channel FX: vorheriger"),
    (
        "Show / hide the sampler panel",
        "Sampler-Panel ein- / ausblenden",
    ),
    ("Previous sampler bank", "Vorherige Sampler-Bank"),
    ("Next sampler bank", "Nächste Sampler-Bank"),
    ("Start / stop the sequencer", "Sequencer starten / stoppen"),
    ("Stop all samples", "Alle Samples stoppen"),
    // Status messages of the Perform panels.
    (
        "Sample could not be loaded: {}",
        "Sample konnte nicht geladen werden: {}",
    ),
    (
        "Loaded {} into sampler slot {}",
        "{} in Sampler-Slot {} geladen",
    ),
    (
        "Select a track to load into the sampler",
        "Wähle einen Track, der in den Sampler geladen werden soll",
    ),
    ("Loading {} into the sampler …", "Lade {} in den Sampler …"),
    ("Memory cue deleted", "Memory Cue gelöscht"),
    (
        "The beatgrid is locked — unlock it in Prepare to tap the tempo",
        "Das Beatgrid ist gesperrt — entsperre es in Prepare, um das Tempo zu tappen",
    ),
    ("Tapped tempo: {} BPM", "Getapptes Tempo: {} BPM"),
    // Browser: columns, filter, Tags, tag list, similar tracks, histories.
    ("PREVIEW", "VORSCHAU"),
    ("ART", "COVER"),
    ("TITLE", "TITEL"),
    ("ARTIST", "INTERPRET"),
    ("ALBUM", "ALBUM"),
    ("GENRE", "GENRE"),
    ("LABEL", "LABEL"),
    ("COMMENT", "KOMMENTAR"),
    ("TAGS", "TAGS"),
    ("COLOUR", "FARBE"),
    ("BPM", "BPM"),
    ("KEY", "TONART"),
    ("TIME", "ZEIT"),
    ("YEAR", "JAHR"),
    ("ADDED", "HINZUGEFÜGT"),
    ("PLAYS", "GESPIELT"),
    ("BITRATE", "BITRATE"),
    ("RATING", "BEWERTUNG"),
    ("Tag list", "Tag-Liste"),
    ("Similar tracks", "Passende Tracks"),
    ("Fits after {}", "Passt nach {}"),
    ("MASTER", "MASTER"),
    ("SELECTED", "AUSGEWÄHLT"),
    ("Edit Tags", "Tags bearbeiten"),
    (
        "Select tracks to add them to the tag list",
        "Wähle Tracks, um sie zur Tag-Liste hinzuzufügen",
    ),
    (
        "Added {} track(s) to the tag list",
        "{} Track(s) zur Tag-Liste hinzugefügt",
    ),
    (
        "Saved the tag list as a playlist ({} tracks)",
        "Tag-Liste als Playlist gespeichert ({} Tracks)",
    ),
    ("Tag list cleared", "Tag-Liste geleert"),
    (
        "Select tracks to tag them",
        "Wähle Tracks, um sie zu taggen",
    ),
    ("Tags {} updated", "Tags {} aktualisiert"),
    (
        "Type a Tags as Category: Value, e.g. Mood: Peak",
        "Gib einen Tags als Kategorie: Wert ein, z. B. Stimmung: Peak",
    ),
    (
        "Sort the list by # to reorder it",
        "Sortiere die Liste nach #, um sie neu zu ordnen",
    ),
    (
        "Reordered — ⌘Z to undo",
        "Neu geordnet — ⌘Z macht es rückgängig",
    ),
    ("Moved — ⌘Z to undo", "Verschoben — ⌘Z macht es rückgängig"),
    (
        "Add the selected tracks to the tag list",
        "Ausgewählte Tracks zur Tag-Liste hinzufügen",
    ),
    (
        "Show / hide the filter bar",
        "Filterleiste ein- / ausblenden",
    ),
    (
        "Prepare layout: player and browser",
        "Prepare-Layout: Player und Browser",
    ),
    (
        "Prepare layout: full browser",
        "Prepare-Layout: ganzer Browser",
    ),
    ("Prepare layout: 2 players", "Prepare-Layout: 2 Player"),
    ("Prepare layout: dual player", "Prepare-Layout: Dual-Player"),
    (
        "Prepare layout: simple player",
        "Prepare-Layout: einfacher Player",
    ),
    (
        "Show / hide the info panel",
        "Info-Bereich ein- / ausblenden",
    ),
    (
        "Show / hide the recording section",
        "Aufnahmebereich ein- / ausblenden",
    ),
    ("Show / hide the mixer", "Mixer ein- / ausblenden"),
    (
        "Show / hide Mix Preview",
        "Mix-Vorschau ein- / ausblenden",
    ),
    (
        "Import a Music / iTunes library XML",
        "Musik- / iTunes-Mediathek (XML) importieren",
    ),
    (
        "Adding {} dropped file(s)…",
        "{} abgelegte Datei(en) werden hinzugefügt …",
    ),
    (
        "Import a Music / iTunes library",
        "Musik- / iTunes-Mediathek importieren",
    ),
    (
        "Imported {} tracks and {} playlists from the Music library",
        "{} Tracks und {} Playlists aus der Musik-Mediathek importiert",
    ),
    (
        "Music library import failed: {}",
        "Import der Musik-Mediathek fehlgeschlagen: {}",
    ),
    (
        "No audio files in what was dropped",
        "Unter dem Abgelegten sind keine Audiodateien",
    ),
    ("{} · channel {} · {}", "{} · Kanal {} · {}"),
    ("14-bit fader", "14-Bit-Fader"),
    ("button", "Taste"),
    ("encoder / jog", "Drehregler / Jog"),
    ("knob / fader", "Regler / Fader"),
    (
        "Add the file to the collection first",
        "Füge die Datei zuerst der Sammlung hinzu",
    ),
    (
        "Load a track on the deck first",
        "Lade zuerst einen Track auf das Deck",
    ),
    (
        "Mix Preview: deck {} comes in",
        "Mix-Vorschau: Deck {} steigt ein",
    ),
    ("Mix Preview on", "Mix-Vorschau an"),
    ("Mix Preview off", "Mix-Vorschau aus"),
    ("Reserved {} — {} waiting", "{} vorgemerkt — {} warten"),
    ("Reserved track loads on deck {}", "Vorgemerkter Track lädt auf Deck {}"),
    ("Reservations cleared", "Vormerkungen gelöscht"),
    (
        "Select tracks in your library to reserve them",
        "Wähle Tracks in deiner Bibliothek, um sie vorzumerken",
    ),
    ("No free hot cue pad", "Kein freies Hot-Cue-Pad"),
    (
        "Select a track in the list first",
        "Wähle zuerst einen Track in der Liste",
    ),
    (
        "Tempo marker: {} BPM from the cursor",
        "Tempomarker: {} BPM ab dem Cursor",
    ),
    (
        "Control added to {} — used from the next start",
        "Bedienelement zu {} hinzugefügt — gilt ab dem nächsten Start",
    ),
    ("Learn a control first", "Lerne zuerst ein Bedienelement"),
    (
        "No controller layer running — connect a controller and restart",
        "Keine Controller-Schicht aktiv — Controller anschließen und neu starten",
    ),
    (
        "The control could not be added: {}",
        "Das Bedienelement konnte nicht hinzugefügt werden: {}",
    ),
    (
        "Stems are separated on a desktop computer; the stick brings them along",
        "Stems werden am Desktop-Rechner getrennt; der Stick bringt sie mit",
    ),
    ("Stems ready for {} tracks", "Stems für {} Tracks bereit"),
    (
        "Stems ready for {} tracks, {} failed: {}",
        "Stems für {} Tracks bereit, {} fehlgeschlagen: {}",
    ),
    (
        "Select the tracks to separate first",
        "Wähle zuerst die zu trennenden Tracks",
    ),
    (
        "Separating stems: {} of {} tracks …",
        "Stems werden getrennt: {} von {} Tracks …",
    ),
    (
        "This build separates no stems (feature separation)",
        "Diese Version trennt keine Stems (Feature separation)",
    ),
    ("Active loop changed", "Aktiver Loop geändert"),
    (
        "Separate the stems of the selected tracks",
        "Stems der ausgewählten Tracks trennen",
    ),
    (
        "Show / hide similar tracks",
        "Passende Tracks ein- / ausblenden",
    ),
    ("Show / hide Tags", "Tags ein- / ausblenden"),
    (
        "Import a library XML (rekordbox-compatible)",
        "Bibliotheks-XML importieren (rekordbox-kompatibel)",
    ),
    (
        "Export the library as XML (rekordbox-compatible)",
        "Bibliothek als XML exportieren (rekordbox-kompatibel)",
    ),
    ("Export a library archive", "Bibliotheksarchiv exportieren"),
    (
        "Restore a library archive",
        "Bibliotheksarchiv wiederherstellen",
    ),
    ("Consolidate the library", "Bibliothek zusammenführen"),
    (
        "Import rekordbox preferences and shortcuts",
        "rekordbox-Einstellungen und -Tastenkürzel importieren",
    ),
    ("Show duplicate tracks", "Doppelte Tracks anzeigen"),
    ("Tap the tempo", "Tempo tappen"),
    ("Tap {} of {}", "Tap {} von {}"),
    ("Beatgrid set to {} BPM", "Beatgrid auf {} BPM gesetzt"),
    (
        "Type a tempo between 30 and 300 BPM",
        "Gib ein Tempo zwischen 30 und 300 BPM ein",
    ),
    (
        "The beatgrid is locked — unlock it to edit",
        "Das Beatgrid ist gesperrt — entsperre es zum Bearbeiten",
    ),
    (
        "Recording started automatically",
        "Aufnahme automatisch gestartet",
    ),
    ("{} TRACKS  ·  {}  ·  {}", "{} TRACKS  ·  {}  ·  {}"),
    ("{} SELECTED", "{} AUSGEWÄHLT"),
    ("Duplicate tracks", "Doppelte Tracks"),
    (
        "Removed {} track(s) from the collection — the files stay on disk",
        "{} Track(s) aus der Sammlung entfernt — die Dateien bleiben erhalten",
    ),
    ("Planned start saved", "Geplanter Start gespeichert"),
    (
        "Select a position of the set first",
        "Wähle zuerst eine Position im Set",
    ),
    (
        "The tag list is empty — collect the backup tracks there first",
        "Die Tag-Liste ist leer — sammle die Ersatz-Tracks zuerst dort",
    ),
    ("Backup tracks removed", "Ersatz-Tracks entfernt"),
    ("{} backup track(s) set", "{} Ersatz-Track(s) gesetzt"),
    ("Export library XML", "Bibliotheks-XML exportieren"),
    ("Import library XML", "Bibliotheks-XML importieren"),
    ("Export library archive", "Bibliotheksarchiv exportieren"),
    (
        "Include the audio files? The archive then holds everything to move the library to another computer.",
        "Audiodateien einschließen? Dann enthält das Archiv alles, um die Bibliothek auf einen anderen Computer umzuziehen.",
    ),
    (
        "Restore library archive",
        "Bibliotheksarchiv wiederherstellen",
    ),
    (
        "Folder for the restored library",
        "Ordner für die wiederhergestellte Bibliothek",
    ),
    (
        "Folder to collect all tracks in",
        "Ordner, in dem alle Tracks gesammelt werden",
    ),
    ("Exported {} tracks to {}", "{} Tracks nach {} exportiert"),
    ("XML export failed: {}", "XML-Export fehlgeschlagen: {}"),
    (
        "Imported {} tracks, {} playlists and {} cues from XML",
        "{} Tracks, {} Playlists und {} Cues aus XML importiert",
    ),
    (" · {} files not found", " · {} Dateien nicht gefunden"),
    ("XML import failed: {}", "XML-Import fehlgeschlagen: {}"),
    (
        "Writing the library archive…",
        "Bibliotheksarchiv wird geschrieben …",
    ),
    (
        "Library archive saved: {} ({} audio files)",
        "Bibliotheksarchiv gespeichert: {} ({} Audiodateien)",
    ),
    (
        "Library archive failed: {}",
        "Bibliotheksarchiv fehlgeschlagen: {}",
    ),
    (
        "Library restored to {} — it opens at the next start",
        "Bibliothek nach {} wiederhergestellt — sie öffnet sich beim nächsten Start",
    ),
    (
        "Restoring the archive failed: {}",
        "Wiederherstellen des Archivs fehlgeschlagen: {}",
    ),
    (
        "Consolidating the library…",
        "Bibliothek wird zusammengeführt …",
    ),
    (
        "Consolidated: {} tracks copied to {}",
        "Zusammengeführt: {} Tracks nach {} kopiert",
    ),
    (
        "Consolidating failed: {}",
        "Zusammenführen fehlgeschlagen: {}",
    ),
    (
        "No rekordbox preferences found: {}",
        "Keine rekordbox-Einstellungen gefunden: {}",
    ),
    (
        "Took over {} rekordbox preferences and {} shortcuts",
        "{} rekordbox-Einstellungen und {} Tastenkürzel übernommen",
    ),
    // Phrase editing, edit mode, sub-browser, file drops.
    ("Edit phrases", "Phrasen bearbeiten"),
    (
        "Show / hide the sub-browser",
        "Sub-Browser ein- / ausblenden",
    ),
    ("Cut the phrase at the cursor", "Phrase am Cursor teilen"),
    ("Merge the phrase at the cursor with the next", "Phrase am Cursor mit der nächsten verbinden"),
    ("Phrase type picker for the phrase at the cursor", "Phrasentyp der Phrase am Cursor wählen"),
    ("Phrases merged", "Phrasen verbunden"),
    ("Beatgrid 1 ms earlier", "Beatgrid 1 ms früher"),
    ("Beatgrid 1 ms later", "Beatgrid 1 ms später"),
    ("Edit the beatgrid on the waveform (again: done)", "Beatgrid an der Wellenform bearbeiten (nochmal: fertig)"),
    ("Select a track to edit its beatgrid", "Einen Track wählen, um sein Beatgrid zu bearbeiten"),
    ("The beatgrid is locked — unlock it (LOCK GRID) to edit", "Das Beatgrid ist gesperrt — zum Bearbeiten entsperren (LOCK GRID)"),
    ("Edit grid: Shift-drag the waveform until the lines sit on the kicks, click the 1 of a bar", "Grid bearbeiten: Wellenform mit Shift ziehen, bis die Linien auf den Kicks liegen, die 1 eines Takts anklicken"),
    ("Beatgrid saved", "Beatgrid gespeichert"),
    ("Beatgrid edit cancelled", "Beatgrid-Bearbeitung abgebrochen"),
    ("No phrase follows to merge with", "Keine folgende Phrase zum Verbinden"),
    ("Phrase boundary moved", "Phrasengrenze verschoben"),
    ("No phrase at the cursor", "Keine Phrase am Cursor"),
    (
        "Move the nearest phrase boundary to the cursor",
        "Nächste Phrasengrenze an den Cursor verschieben",
    ),
    ("Clear the phrases", "Phrasen löschen"),
    (
        "Edit region start at the cursor",
        "Beginn des Bearbeitungsbereichs am Cursor",
    ),
    (
        "Edit region end at the cursor",
        "Ende des Bearbeitungsbereichs am Cursor",
    ),
    (
        "New track without the edit region",
        "Neuer Track ohne den Bearbeitungsbereich",
    ),
    (
        "New track with only the edit region",
        "Neuer Track nur mit dem Bearbeitungsbereich",
    ),
    (
        "New track with the edit region repeated",
        "Neuer Track mit wiederholtem Bearbeitungsbereich",
    ),
    (
        "This track has no phrases",
        "Dieser Track hat keine Phrasen",
    ),
    ("Phrases cleared", "Phrasen gelöscht"),
    ("Phrase changed", "Phrase geändert"),
    ("Phrase cut at the cursor", "Phrase am Cursor geteilt"),
    (
        "No phrase to cut at the cursor",
        "Am Cursor ist keine Phrase zum Teilen",
    ),
    (
        "Cut at least 4 bars from the edges of the phrase",
        "Teile mindestens 4 Takte vom Rand der Phrase entfernt",
    ),
    (
        "Phrase boundary moved to the cursor",
        "Phrasengrenze an den Cursor verschoben",
    ),
    (
        "The boundary cannot move past the next one",
        "Die Grenze kann nicht über die nächste hinaus verschoben werden",
    ),
    (
        "Phrases shorter than 4 bars join their neighbour — nothing changed",
        "Phrasen unter 4 Takten gehen in ihrer Nachbarphrase auf — nichts geändert",
    ),
    (
        "An edit is being rendered — one moment",
        "Eine Bearbeitung wird gerade erstellt — einen Moment",
    ),
    (
        "Set IN and OUT on the waveform first",
        "Setze zuerst IN und OUT auf der Wellenform",
    ),
    (
        "Rendering the edit without the region…",
        "Bearbeitung ohne den Bereich wird erstellt …",
    ),
    (
        "Rendering the edit with only the region…",
        "Bearbeitung nur mit dem Bereich wird erstellt …",
    ),
    (
        "Rendering the edit with the region repeated…",
        "Bearbeitung mit wiederholtem Bereich wird erstellt …",
    ),
    (
        "Edit saved as a new track: {}",
        "Bearbeitung als neuer Track gespeichert: {}",
    ),
    ("The edit failed: {}", "Bearbeitung fehlgeschlagen: {}"),
    ("{} file(s)", "{} Datei(en)"),
    (
        "Sort the list by # to drop tracks into it",
        "Sortiere die Liste nach #, um Tracks hineinzuziehen",
    ),
    (
        "Added to the list — ⌘Z to undo",
        "Zur Liste hinzugefügt — ⌘Z macht es rückgängig",
    ),
    // MATCH column (fit with the master deck).
    ("MATCH", "PASSUNG"),
    ("DECK {}", "DECK {}"),
    ("key unknown", "Tonart unbekannt"),
    ("same key", "gleiche Tonart"),
    ("adjacent key", "benachbarte Tonart"),
    ("key two steps away", "Tonart zwei Schritte entfernt"),
    ("key clash", "Tonart passt nicht"),
    ("tempo unknown", "Tempo unbekannt"),
    ("tempo {} %", "Tempo {} %"),
    ("vs {}: {} · {}", "zu {}: {} · {}"),
    // AUTO SORT groups and lists.
    ("Tempo", "Tempo"),
    ("Energy", "Energie"),
    ("Mood", "Stimmung"),
    ("Character", "Charakter"),
    ("Genre", "Genre"),
    ("Low energy", "Wenig Energie"),
    ("Mid energy", "Mittlere Energie"),
    ("High energy", "Viel Energie"),
    ("Euphoric", "Euphorisch"),
    ("Warm", "Warm"),
    ("Driving", "Treibend"),
    ("Dark", "Dunkel"),
    ("Bass-heavy", "Bassbetont"),
    ("Balanced", "Ausgewogen"),
    ("Bright", "Hell"),
    ("Other genres", "Andere Genres"),
    // Cues like my others (learned cue pattern).
    (
        "Set cues on at least {} analysed tracks first: their places become the pattern",
        "Setze zuerst Cues auf mindestens {} analysierten Tracks: ihre Positionen werden zum Muster",
    ),
    (
        "Nothing to change: analyse the tracks first, or every pad is taken",
        "Nichts zu ändern: analysiere die Tracks zuerst, oder alle Pads sind belegt",
    ),
    (
        "Cues set on {} track(s) like your others — ⌘Z to undo",
        "Cues auf {} Track(s) wie bei deinen anderen gesetzt — ⌘Z macht es rückgängig",
    ),
    ("Set cues like my others", "Cues wie bei meinen anderen setzen"),    // Beatgrid editing.
    ("EDIT GRID", "GRID BEARBEITEN"),
    ("Grid moved one beat earlier", "Grid einen Beat früher"),
    ("Grid moved one beat later", "Grid einen Beat später"),
    ("Grid moved {}", "Grid verschoben: {}"),
    ("No beat to set", "Kein Beat zum Setzen"),
    ("Beat 1 at {}", "Beat 1 bei {}"),
    ("Pin at {}: the beat sits there", "Pin bei {}: der Beat sitzt dort"),
    ("These pins do not fit one grid", "Diese Pins passen nicht in ein Grid"),
    ("Tempo {} → {} ({} pins, {} bars)", "Tempo {} → {} ({} Pins, {} Takte)"),
    ("Pins cleared", "Pins gelöscht"),
    ("The transients are still being computed — try again in a moment", "Die Transienten werden noch berechnet — gleich noch einmal versuchen"),
    ("Too few kicks to measure the grid", "Zu wenige Kicks, um das Grid zu messen"),
    ("Fixed: {} BPM, moved {} — the beat jumps at {}: TEMPO CHANGE HERE there", "Korrigiert: {} BPM, verschoben {} — der Beat springt bei {}: dort TEMPOWECHSEL HIER"),
    ("Fixed: {} BPM, moved {}", "Korrigiert: {} BPM, verschoben {}"),
    ("A tempo change needs beats before it — move the cursor into the track", "Ein Tempowechsel braucht Beats davor — den Cursor in den Track setzen"),
    ("Too few kicks after the cursor to measure a tempo", "Zu wenige Kicks nach dem Cursor, um ein Tempo zu messen"),
    ("The beats after the cursor already fit — no tempo change here", "Die Beats nach dem Cursor passen schon — hier kein Tempowechsel"),
    ("Tempo change at {}: {} BPM", "Tempowechsel bei {}: {} BPM"),
    ("Tempo {} BPM", "Tempo {} BPM"),
    ("Beatgrid locked", "Beatgrid gesperrt"),
    ("Beatgrid unlocked", "Beatgrid entsperrt"),
    ("Edit grid: drag the grid onto the kicks, double-click the 1 · PIN two kicks far apart to solve the tempo", "Grid bearbeiten: das Grid auf die Kicks ziehen, die 1 doppelklicken · zwei weit auseinanderliegende Kicks pinnen löst das Tempo"),
    ("Beatgrid unchanged", "Beatgrid unverändert"),
    ("Undone", "Widerrufen"),
    ("Redone", "Wiederholt"),
    ("Snap to transients on", "Einrasten an Transienten an"),
    ("Snap to transients off", "Einrasten an Transienten aus"),
    ("No more spots where the grid is off", "Keine weiteren Stellen, an denen das Grid daneben liegt"),
    ("Click on (headphones)", "Klick an (Kopfhörer)"),
    ("Click off", "Klick aus"),
    ("Load a track on this deck first", "Zuerst einen Track auf dieses Deck laden"),
    ("Edit grid: grid 1 ms earlier", "Grid bearbeiten: Grid 1 ms früher"),
    ("Edit grid: grid 1 ms later", "Grid bearbeiten: Grid 1 ms später"),
    ("Edit grid: grid 10 ms earlier", "Grid bearbeiten: Grid 10 ms früher"),
    ("Edit grid: grid 10 ms later", "Grid bearbeiten: Grid 10 ms später"),
    ("Edit grid: grid one beat earlier", "Grid bearbeiten: Grid einen Beat früher"),
    ("Edit grid: grid one beat later", "Grid bearbeiten: Grid einen Beat später"),
    ("Edit grid: tempo 0.01 BPM faster about the cursor", "Grid bearbeiten: Tempo 0,01 BPM schneller um den Cursor"),
    ("Edit grid: tempo 0.01 BPM slower about the cursor", "Grid bearbeiten: Tempo 0,01 BPM langsamer um den Cursor"),
    ("Edit grid: tempo 0.1 BPM faster about the cursor", "Grid bearbeiten: Tempo 0,1 BPM schneller um den Cursor"),
    ("Edit grid: tempo 0.1 BPM slower about the cursor", "Grid bearbeiten: Tempo 0,1 BPM langsamer um den Cursor"),
    ("Edit grid: the beat at the cursor becomes the 1", "Grid bearbeiten: der Beat am Cursor wird die 1"),
    ("Edit grid: pin the beat at the cursor", "Grid bearbeiten: Beat am Cursor pinnen"),
    ("Edit grid: auto-fix slip and tempo", "Grid bearbeiten: Versatz und Tempo automatisch korrigieren"),
    ("Edit grid: metronome click on / off", "Grid bearbeiten: Metronom-Klick an / aus"),
    ("Edit grid: snap to transients on / off", "Grid bearbeiten: Einrasten an Transienten an / aus"),
    ("Edit grid: tap the tempo", "Grid bearbeiten: Tempo tippen"),
    ("Edit grid: previous spot where the grid is off", "Grid bearbeiten: vorige Stelle, an der das Grid daneben liegt"),
    ("Edit grid: next spot where the grid is off", "Grid bearbeiten: nächste Stelle, an der das Grid daneben liegt"),
    ("Edit grid: done (keep the grid)", "Grid bearbeiten: fertig (Grid behalten)"),
    ("Edit grid: cancel", "Grid bearbeiten: abbrechen"),
    ("Edit grid: undo", "Grid bearbeiten: widerrufen"),
    ("Edit grid: redo", "Grid bearbeiten: wiederholen"),
    ("Import a Club player USB", "Club-Player-USB importieren"),
    ("Choose a Club player USB", "Club-Player-USB auswählen"),
    ("This folder is not a Club player USB", "Dieser Ordner ist kein Club-Player-USB"),
    ("Reading the library…", "Bibliothek wird gelesen…"),
    ("Importing {} tracks, {} playlists and {} cues (the source is not changed)…", "{} Tracks, {} Playlists und {} Cues werden importiert (die Quelle bleibt unverändert)…"),
    ("Library import failed: {}", "Bibliotheksimport fehlgeschlagen: {}"),
    ("Library import: {}/{}", "Bibliotheksimport: {}/{}"),
    ("Library import done: {} tracks added, {} playlists, {} cues, {} beatgrids", "Bibliotheksimport fertig: {} Tracks hinzugefügt, {} Playlists, {} Cues, {} Beatgrids"),
    (", {} files not found", ", {} Dateien nicht gefunden"),
    ("Cannot open library: {}", "Bibliothek kann nicht geöffnet werden: {}"),
    ("Player set {} cue(s); library has {}", "Player hat {} Cue(s) gesetzt; Bibliothek hat {}"),
    ("History: {}", "Historie: {}"),
    ("{} tracks played", "{} Tracks gespielt"),
    ("A player changed cues or wrote history on a stick — review it under Devices", "Ein Player hat auf einem Stick Cues geändert oder eine Historie geschrieben — unter Geräte prüfen"),
    ("Import an encrypted library database", "Verschlüsselte Bibliotheksdatenbank importieren"),
    ("The key was not accepted. Check it and try again.", "Der Schlüssel wurde nicht akzeptiert. Prüfe ihn und versuche es erneut."),
    ("This library database is encrypted. Enter its key under Advanced.", "Diese Bibliotheksdatenbank ist verschlüsselt. Gib ihren Schlüssel unter Erweitert ein."),
    ("Choose the library database file first", "Wähle zuerst die Datei der Bibliotheksdatenbank"),
    ("Choose the library database file", "Datei der Bibliotheksdatenbank wählen"),
    // Audio health and the gig check (roadmap [10]).
    ("Audio output lost: {}", "Audioausgang verloren: {}"),
    ("Audio back on {}", "Ton wieder auf {}"),
    ("Audio output: {}", "Audioausgang: {}"),
    ("Audio output could not start: {}", "Audioausgang konnte nicht starten: {}"),
    ("{} dropouts so far", "bisher {} Aussetzer"),
    ("no headphone output", "kein Kopfhörerausgang"),
    ("master and headphones", "Master und Kopfhörer"),
    ("{} is lost", "{} ist verloren"),
    ("No controller connected", "Kein Controller verbunden"),
    ("{} GB free, about {} hours of recording", "{} GB frei, etwa {} Stunden Aufnahme"),
    ("Free space unknown", "Freier Platz unbekannt"),
    ("{} of {} tracks in order", "{} von {} Tracks in Ordnung"),
    ("{} of {} playlists as in the library", "{} von {} Playlists wie in der Bibliothek"),
    ("A sync did not finish. Sync the stick again.", "Ein Sync wurde nicht beendet. Synchronisiere den Stick noch einmal."),
    ("Not found on the stick: {}", "Nicht auf dem Stick gefunden: {}"),
    ("Playlists not as in the library: {}. Sync the stick.", "Playlists nicht wie in der Bibliothek: {}. Synchronisiere den Stick."),
    ("Some tracks changed since the last sync. Sync the stick.", "Einige Tracks haben sich seit dem letzten Sync geändert. Synchronisiere den Stick."),
    ("Plug the stick in to check it.", "Stecke den Stick ein, um ihn zu prüfen."),
    ("The stick could not be read: {}", "Der Stick konnte nicht gelesen werden: {}"),
    ("Connect this controller to flash its LEDs", "Schließe diesen Controller an, um seine LEDs blinken zu lassen"),
    ("Every track file is in place", "Alle Trackdateien sind da"),
    ("{} missing: {}", "{} fehlen: {}"),
    (" and {} more", " und {} weitere"),
    ("No audio output", "Kein Audioausgang"),
    (
        "Check before the gig: outputs, controller, disk, files",
        "Vor dem Gig prüfen: Ausgänge, Controller, Speicher, Dateien",
    ),
    // Restore after a crash (roadmap [9]).
    ("Deck {}: {} at {}", "Deck {}: {} bei {}"),
    ("The recording {} was kept: {}", "Die Aufnahme {} ist erhalten: {}"),
    ("Restored deck {}: press PLAY to go on", "Deck {} wiederhergestellt: PLAY drücken, um weiterzuspielen"),
    // Performance lock (roadmap [11]).
    (
        "Locked: deck {} is playing. Load onto a stopped deck, or unlock",
        "Gesperrt: Deck {} spielt. Lade auf ein gestopptes Deck oder entsperre",
    ),
    ("Locked: nothing is deleted until you unlock", "Gesperrt: Bis zum Entsperren wird nichts gelöscht"),
    (
        "Locked: the beatgrid stays as it is until you unlock",
        "Gesperrt: Das Beatgrid bleibt, wie es ist, bis du entsperrst",
    ),
    (
        "Locked for the set: no loading onto a playing deck, no deleting, no grid edits",
        "Für das Set gesperrt: kein Laden auf ein spielendes Deck, kein Löschen, keine Grid-Änderungen",
    ),
    ("Unlocked", "Entsperrt"),
    // Phase and tempo between decks (roadmap [12]).
    (
        "Practice: SYNC is off. Match tempo and phase by ear and with the phase meter",
        "Üben: SYNC ist aus. Gleiche Tempo und Phase nach Gehör und mit der Phasenanzeige an",
    ),
    ("SYNC can be used again", "SYNC ist wieder verfügbar"),
    // Booth view (roadmap [58]).
    (
        "Booth view: the decks large in their own window",
        "Booth-Ansicht: die Decks groß in einem eigenen Fenster",
    ),
    (
        "Practice: SYNC stays off until you turn practice off",
        "Üben: SYNC bleibt aus, bis du das Üben beendest",
    ),
    (
        "Lock for the set: no loading onto a playing deck, no deleting, no grid edits",
        "Für das Set sperren: kein Laden auf ein spielendes Deck, kein Löschen, keine Grid-Änderungen",
    ),
    // Mixer for bass swaps and the reviewed library import (roadmap [54], [55]).
    ("NOW", "SOFORT"),
    ("BEAT", "BEAT"),
    ("BAR", "TAKT"),
    ("KILL", "KILL"),
    ("file not found", "Datei nicht gefunden"),
    ("not a file on this computer (stream or cloud)", "keine Datei auf diesem Computer (Stream oder Cloud)"),
    ("video or podcast", "Video oder Podcast"),
    ("cue points that cannot be read: {}", "nicht lesbare Cue-Punkte: {}"),
    ("a list the other app makes itself", "eine Liste, die die andere App selbst anlegt"),
    ("tracks that do not carry over: {}", "Tracks, die nicht übernommen werden: {}"),
    ("LIBRARY XML", "BIBLIOTHEKS-XML"),
    ("MUSIC / ITUNES LIBRARY", "MUSIK- / ITUNES-MEDIATHEK"),
    ("New tracks", "Neue Tracks"),
    ("Tracks already in your library", "Tracks schon in deiner Bibliothek"),
    ("Folders", "Ordner"),
    ("Cue points", "Cue-Punkte"),
    ("Beatgrids", "Beatgrids"),
    ("Keys", "Tonarten"),
    ("Tracks", "Tracks"),
    ("Lists go into a new folder “XML import”. Tracks already in your library take the file's tags, cues, beatgrid and key.", "Listen kommen in einen neuen Ordner „XML import“. Tracks, die schon in deiner Bibliothek sind, übernehmen Tags, Cues, Beatgrid und Tonart aus der Datei."),
    ("Lists go into a new folder “Music library import”. Tracks already in your library take the file's tags.", "Listen kommen in einen neuen Ordner „Music library import“. Tracks, die schon in deiner Bibliothek sind, übernehmen die Tags aus der Datei."),
    ("Nothing in this file can be imported.", "Aus dieser Datei lässt sich nichts importieren."),
    ("This file cannot be read: {}", "Diese Datei kann nicht gelesen werden: {}"),
    ("No undo copy could be made, nothing was imported: {}", "Es ließ sich keine Kopie zum Rückgängigmachen anlegen, nichts wurde importiert: {}"),
    ("The import stopped: {}. UNDO IMPORT puts back what was written.", "Der Import wurde abgebrochen: {}. IMPORT RÜCKGÄNGIG nimmt zurück, was geschrieben wurde."),
    ("UNDO IMPORT puts the library back exactly as it was before, as long as this window is open.", "IMPORT RÜCKGÄNGIG stellt die Bibliothek genau so wieder her, wie sie vorher war, solange dieses Fenster offen ist."),
    ("Library import done: {} tracks before, {} after", "Bibliotheksimport fertig: {} Tracks vorher, {} nachher"),
    ("Undo failed: {}", "Rückgängig fehlgeschlagen: {}"),
    ("The library is as it was before the import.", "Die Bibliothek ist wieder so wie vor dem Import."),
    ("Library import undone", "Bibliotheksimport rückgängig gemacht"),
    ("Choose the rekordbox preferences folder", "rekordbox-Einstellungsordner wählen"),    ("Fits deck {} ({})", "Passt zu Deck {} ({})"),
    ("Energy change against deck {} ({})", "Energiewechsel gegenüber Deck {} ({})"),
    ("Clashes with deck {} ({})", "Beißt sich mit Deck {} ({})"),
    ("Key set to {}", "Tonart auf {} gesetzt"),
    // EDIT IN PREPARE.
    ("Deck {} in Prepare — the decks play on; BACK TO PERFORM or Esc returns", "Deck {} in Prepare — die Decks spielen weiter; ZURÜCK ZU PERFORM oder Esc führt zurück"),
    ("Back in Perform", "Zurück in Perform"),
    ("Phrase boundary one bar earlier", "Phrasengrenze einen Takt früher"),
    ("Phrase boundary one bar later", "Phrasengrenze einen Takt später"),
    ("The analysis has no other likely key for this track", "Die Analyse kennt für diesen Track keine andere wahrscheinliche Tonart"),
    ("Edit in Prepare: this deck's track in the player (again: back to Perform)", "In Prepare bearbeiten: der Track dieses Decks im Player (nochmal: zurück zu Perform)"),
    ("Back to Perform (after EDIT on a deck)", "Zurück zu Perform (nach EDIT an einem Deck)"),
    ("Could not set the key: {}", "Tonart ließ sich nicht setzen: {}"),
    ("Deck {}: only {} plays", "Deck {}: nur {} spielt"),
    ("No stems yet for {} on deck {}: separating now, the stem keys work when it is done", "Noch keine Stems für {} auf Deck {}: wird jetzt getrennt, die Stem-Tasten gehen, sobald es fertig ist"),
    ("No stems for {} on deck {}: press a stem key to separate it, or prepare the set ahead", "Keine Stems für {} auf Deck {}: eine Stem-Taste drücken, um ihn zu trennen, oder das Set vorher vorbereiten"),
    ("QUEUED", "WARTET"),
    ("READY", "BEREIT"),
    ("FAILED", "FEHLER"),
    ("Every track here has its stems already", "Jeder Track hier hat seine Stems schon"),
    ("All effects off", "Alle Effekte aus"),
    ("All effects off (panic)", "Alle Effekte aus (Panik)"),
    ("Separate the stems of every track in the list", "Die Stems aller Tracks der Liste trennen"),
    ("STEMS", "STEMS"),
    // Deck layouts and the guided controller setup (roadmap [6], [13]).
    ("Layout saved: {}", "Layout gespeichert: {}"),
    ("2 DECKS · ENTRY LEVEL", "2 DECKS · EINSTEIGER"),
    ("2 DECKS · BATTLE", "2 DECKS · BATTLE"),
    ("4 DECKS", "4 DECKS"),
    ("OTHER", "ANDERE"),
    (
        "Mapping saved to {} — used from the next start",
        "Mapping gespeichert unter {} – wird ab dem nächsten Start benutzt",
    ),
    (
        "The mapping could not be saved: {}",
        "Das Mapping konnte nicht gespeichert werden: {}",
    ),
    ("{} EMPTY", "{} LEER"),
    ("INTRO", "INTRO"),
    ("VERSE", "STROPHE"),
    ("BUILD-UP", "AUFBAU"),
    ("DROP", "DROP"),
    ("BREAKDOWN", "BREAKDOWN"),
    ("OUTRO", "OUTRO"),
    ("TRACK KEY", "TONART"),
    ("WARM", "WARM"),
    ("COOL", "KÜHL"),
    ("SUNSET", "ABENDROT"),
    ("OCEAN", "MEER"),
    ("WHITE", "WEISS"),
    ("OWN", "EIGENE"),
    ("1/4 BEAT", "1/4 SCHLAG"),
    ("1/2 BEAT", "1/2 SCHLAG"),
    ("1 BEAT", "1 SCHLAG"),
    ("2 BEATS", "2 SCHLÄGE"),
    ("1 BAR", "1 TAKT"),
    ("2 BARS", "2 TAKTE"),
    ("BLACKOUT", "BLACKOUT"),
    ("FLASH KEY HELD", "FLASH-TASTE GEHALTEN"),
    ("STROBE KEY HELD", "STROBE-TASTE GEHALTEN"),
    ("PAD {} · {} · HELD", "PAD {} · {} · GEHALTEN"),
    ("PAD {} · {} · LATCHED", "PAD {} · {} · EINGERASTET"),
    ("AUTO HELD · {}", "AUTO GEHALTEN · {}"),
    ("MUSIC", "MUSIK"),
    ("Lighting: a chase steps through looks; that pad holds none", "Licht: ein Lauflicht schaltet durch Looks; auf diesem Pad liegt keiner"),
    ("Chase {}", "Lauflicht {}"),
    ("Look {}", "Look {}"),
    ("CHASE", "LAUFLICHT"),
    ("FLASH", "FLASH"),
    ("LATCH", "RASTEN"),
    ("ASKING THE NETWORK…", "FRAGE DAS NETZWERK…"),
    ("NO NODE ANSWERED", "KEIN NODE HAT GEANTWORTET"),
    ("{} FOUND", "{} GEFUNDEN"),
    ("NETWORK: {}", "NETZWERK: {}"),
    ("sACN multicast", "sACN-Multicast"),
    ("Not an address: {}", "Keine Adresse: {}"),
    ("Check, in this order: the fixtures are on and set to DMX; their universe and address match the patch table; the cable runs from the interface to the first fixture and on from there, with a terminator at the end if you have one; for a node, its universe matches and it is on this computer’s network. The OUTPUT line above the guide says if the link itself is lost.", "Prüfe der Reihe nach: Die Scheinwerfer sind an und auf DMX gestellt; Universum und Adresse stimmen mit der Patch-Tabelle überein; das Kabel führt vom Interface zum ersten Scheinwerfer und von dort weiter, am Ende mit einem Abschlusswiderstand, falls du einen hast; bei einem Node stimmt sein Universum und er ist im Netzwerk dieses Rechners. Ob die Verbindung selbst verloren ist, zeigt die Zeile unter AUSGANG."),
    ("TEMPO {} %", "TEMPO {} %"),
    ("KEY CLASH {} → {}", "TONART BEISST SICH {} → {}"),
    ("ENERGY DROP", "ENERGIEABFALL"),
    ("ENERGY JUMP", "ENERGIESPRUNG"),
    ("key {} → {} fits", "Tonart {} → {} passt"),
    ("key {} → {} changes the energy", "Tonart {} → {} ändert die Energie"),
    ("key {} → {} clashes", "Tonart {} → {} beißt sich"),
    ("energy drops", "Energie fällt ab"),
    ("energy jumps", "Energie springt"),
    ("Warning", "Warnung"),
    ("Worth rehearsing", "Lohnt eine Probe"),
    ("Not analysed", "Nicht analysiert"),
    ("Fits", "Passt"),
    ("TEMPO ±{} %", "TEMPO ±{} %"),
    ("KEY FITS", "TONART PASST"),
    ("RATING ≥ {}", "BEWERTUNG ≥ {}"),
    // Spoken names of controls (od-ui a11y) and the help overlay.
    ("Player {}", "Player {}"),
    ("Channel {}", "Kanal {}"),
    ("FX {}", "FX {}"),
    ("{}: {}", "{}: {}"),
    (
        "Help: name every control on screen",
        "Hilfe: jedes Bedienelement auf dem Bildschirm benennen",
    ),
    ("TRACK LOADED", "TRACK GELADEN"),
    ("PLAY STARTS", "PLAY STARTET"),
    ("PHRASE CHANGES", "PHRASE WECHSELT"),
    ("CROSSFADER LEFT", "CROSSFADER LINKS"),
    ("CROSSFADER RIGHT", "CROSSFADER RECHTS"),
    ("TRACK END NEAR", "TRACKENDE NAH"),
    ("NOTHING", "NICHTS"),
    ("RELEASE", "FREIGEBEN"),
    ("UNTIL NEXT", "BIS ZUM NÄCHSTEN"),
    ("THE DECKS OF THIS APP", "DIE DECKS DIESER APP"),
    ("{} · {} BPM", "{} · {} BPM"),
    ("{} · SILENCE", "{} · STILLE"),
    ("FROM THE NETWORK · {} BPM", "AUS DEM NETZWERK · {} BPM"),
    ("NOTHING ARRIVES · OSC IN IS ON", "NICHTS KOMMT AN · OSC-EINGANG IST AN"),
    ("TURN OSC IN ON UNDER SETUP › REMOTE", "OSC-EINGANG UNTER SETUP › EXTERN EINSCHALTEN"),
    ("LISTENING ON PORT {} · {} MESSAGES", "HÖRT AUF PORT {} · {} NACHRICHTEN"),
    ("CANNOT LISTEN · {}", "KANN NICHT HÖREN · {}"),
    ("SENDING · {} MESSAGES", "SENDET · {} NACHRICHTEN"),
    ("{} MIN", "{} MIN"),
    ("{} S", "{} S"),
    ("SILENCE · THE CALM LOOK", "STILLE · DER RUHIGE LOOK"),
    ("SILENCE FOR {} S · CALM AFTER {}", "SEIT {} S STILLE · RUHIG NACH {}"),
    ("FOLLOWING THE MUSIC", "FOLGT DER MUSIK"),
    ("CONNECTED · PLAYS THE LIGHTS WITH THE LIGHTING LAYOUT", "VERBUNDEN · SPIELT DAS LICHT MIT DEM LICHT-LAYOUT"),
    ("NOT CONNECTED · PLAYS THE LIGHTS ONCE IT IS", "NICHT VERBUNDEN · SPIELT DAS LICHT, SOBALD ER ES IST"),
    ("CANNOT SEND · {}", "KANN NICHT SENDEN · {}"),
    ("NO AUDIO INPUT (AUDIO IS OFF)", "KEIN AUDIOEINGANG (AUDIO IST AUS)"),
    ("NO AUDIO INPUT · {}", "KEIN AUDIOEINGANG · {}"),
    ("Lighting: the show runs unattended", "Licht: die Show läuft unbeaufsichtigt"),
    ("Lighting: the OSC port is 1024 to 65535", "Licht: der OSC-Port geht von 1024 bis 65535"),
    ("Lighting: the show runs unattended. LOCK closes it.", "Licht: die Show läuft unbeaufsichtigt. SPERREN schließt sie ab."),
    ("Lighting: unattended mode off", "Licht: unbeaufsichtigter Betrieb aus"),
    ("Lighting controller connected: {}", "Licht-Controller verbunden: {}"),
    ("Lighting controller disconnected: {}", "Licht-Controller getrennt: {}"),
    ("TRIGGER · {} · {}", "TRIGGER · {} · {}"),
    ("Recording saved: {} · true peak {} · {} clips", "Aufnahme gesichert: {} · True Peak {} · {} Clips"),
    ("{}  ·  TRUE PEAK {}  ·  {} CLIPS", "{}  ·  TRUE PEAK {}  ·  {} CLIPS"),
    ("Markers are set while recording", "Marken werden während der Aufnahme gesetzt"),
    ("Marker {}", "Marke {}"),
    ("Marker {} set at {}", "Marke {} bei {} gesetzt"),
    ("{} ENTRIES  ·  {}", "{} EINTRÄGE  ·  {}"),
    ("Export failed: {}", "Export fehlgeschlagen: {}"),
    ("Tracklist written to {}", "Trackliste geschrieben nach {}"),
    ("Chapters could not be written into {}: {}", "Kapitel konnten nicht in {} geschrieben werden: {}"),
    ("\"{}\" is not a time in the recording: type minutes and seconds such as 12:34 (up to {})", "„{}“ ist keine Zeit in der Aufnahme: tippe Minuten und Sekunden wie 12:34 (bis {})"),
    ("New entry", "Neuer Eintrag"),
    ("Set a chapter marker in the recording", "Eine Kapitelmarke in der Aufnahme setzen"),
    ("ON STICK", "AUF DEM STICK"),
    ("STEMS ON THE STICK {} OF {}", "STEMS AUF DEM STICK {} VON {}"),
    ("Nothing to copy: open a set or playlist, or load a track to a deck", "Nichts zu kopieren: öffne ein Set oder eine Playlist oder lade einen Track auf ein Deck"),
    ("Plug in a USB stick for the emergency copy", "Für die Notfallkopie einen USB-Stick einstecken"),
    ("Emergency stick {}", "Notfall-Stick {}"),
    ("{} tracks into {}", "{} Tracks in {}"),
    ("Emergency stick: copying {} tracks to {}", "Notfall-Stick: kopiert {} Tracks auf {}"),
    ("{} tracks, ready to play", "{} Tracks, spielbereit"),
    (" · {} missing files left out", " · {} fehlende Dateien ausgelassen"),
    (" · {} not playable on older club players", " · {} auf älteren Club-Playern nicht spielbar"),
    ("Emergency stick ready: {} tracks on {}", "Notfall-Stick bereit: {} Tracks auf {}"),
    ("The stick was pulled out before the copy was done", "Der Stick wurde abgezogen, bevor die Kopie fertig war"),
    ("The stick could not be registered: is it writable?", "Der Stick konnte nicht registriert werden: ist er beschreibbar?"),
    ("Emergency stick: copy the open list and the decks to a stick", "Notfall-Stick: die offene Liste und die Decks auf einen Stick kopieren"),
    ("Select tracks in the browser to add them to the queue", "Wähle Tracks im Browser, um sie zur Warteschlange hinzuzufügen"),
    ("Added {} to the queue; {} tagged do not play (auto-play leaves them out)", "{} zur Warteschlange hinzugefügt; {} als nicht spielen markiert (Auto-Play lässt sie aus)"),
    ("Added {} to the queue", "{} zur Warteschlange hinzugefügt"),
    ("Request queued: {}", "Wunsch eingereiht: {}"),
    ("Request noted: no track in the library matches it yet", "Wunsch notiert: noch passt kein Track der Bibliothek dazu"),
    ("Select the requested track in the browser first", "Wähle zuerst den gewünschten Track im Browser"),
    ("No free deck: every deck is playing or holds a track", "Kein freies Deck: jedes Deck spielt oder hält einen Track"),
    ("The queue holds no track to load", "Die Warteschlange hält keinen Track zum Laden"),
    ("Deck {}: this track is tagged do not play", "Deck {}: dieser Track ist als nicht spielen markiert"),
    ("Dead air in {}", "Stille in {}"),
    ("Deck {} runs out and no deck is cued. LOAD NEXT takes the queue's next track.", "Deck {} läuft aus und kein Deck steht bereit. NÄCHSTEN LADEN nimmt den nächsten Track der Warteschlange."),
    ("Deck {} runs out and no deck is cued, and the queue is empty: load a track.", "Deck {} läuft aus, kein Deck steht bereit und die Warteschlange ist leer: lade einen Track."),
    ("Do not play: {}", "Nicht spielen: {}"),
    ("Deck {} holds a track tagged do not play.", "Deck {} hält einen als nicht spielen markierten Track."),
    ("MIXING DECK {} › DECK {} · {} BEATS", "MISCHT DECK {} › DECK {} · {} BEATS"),
    ("CUT", "SCHNITT"),
    ("Do-not-play tag removed", "Nicht-spielen-Markierung entfernt"),
    ("Tagged do not play: auto-play leaves these out", "Als nicht spielen markiert: Auto-Play lässt sie aus"),
    ("Auto-play on: the queue plays on by itself", "Auto-Play an: die Warteschlange spielt von selbst weiter"),
    ("Auto-play off", "Auto-Play aus"),
    ("Add the selected tracks to the queue", "Die ausgewählten Tracks zur Warteschlange hinzufügen"),
    ("Load the next track of the queue", "Den nächsten Track der Warteschlange laden"),
    ("TONIGHT", "HEUTE"),
    ("NOT PLAYED TONIGHT", "HEUTE NICHT GESPIELT"),
    ("Edit tags of {} tracks", "Tags von {} Tracks bearbeiten"),
    ("CONSTANT POWER", "KONSTANTE LEISTUNG"),
    ("Crossfader curve:", "Crossfader-Kurve:"),
    ("No beatgrid: phrase jumps need the track's grid", "Kein Beatgrid: Phrasensprünge brauchen das Grid des Tracks"),
    ("No {} to jump to", "Kein Sprungziel: {}"),
    ("phrase", "Phrase"),
    ("marked drop", "markierter Drop"),
    ("build-up", "Build-up"),
    ("drop", "Drop"),
    ("breakdown", "Breakdown"),
    ("outro", "Outro"),
    ("Drop marked at {}", "Drop markiert bei {}"),
    ("No drop marked", "Kein Drop markiert"),
    ("ON TIME", "IM PLAN"),
    ("BEHIND {}", "{} ZURÜCK"),
    ("AHEAD {}", "{} VORAUS"),
    ("1 TRACK BEHIND", "1 TRACK ZURÜCK"),
    ("1 TRACK AHEAD", "1 TRACK VORAUS"),
    ("{} TRACKS BEHIND", "{} TRACKS ZURÜCK"),
    ("{} TRACKS AHEAD", "{} TRACKS VORAUS"),
    ("NO PLANNED START", "KEIN GEPLANTER START"),
    ("OFF PLAN: THE TRACK ON AIR IS NOT IN THE SET", "AUSSER PLAN: DER LAUFENDE TRACK IST NICHT IM SET"),
    ("WAITING FOR THE FIRST TRACK", "WARTET AUF DEN ERSTEN TRACK"),
    ("This pad has no hot cue yet: press it to store one", "Dieses Pad hat noch keinen Hot Cue: drücken speichert einen"),
    ("Drop marked", "Drop markiert"),
    ("Drop cleared", "Drop entfernt"),
    ("Mark the drop", "Drop markieren"),
    ("Clear the drop", "Drop entfernen"),
    ("Rename hot cue", "Hot Cue umbenennen"),
    ("Recolour hot cue", "Hot-Cue-Farbe ändern"),
    ("Move hot cue", "Hot Cue versetzen"),
    ("Delete hot cue", "Hot Cue löschen"),
    ("Hot cue pad behaviour", "Spielweise des Hot-Cue-Pads"),
    ("Hot cue stored", "Hot Cue gespeichert"),
    ("master", "Master"),
    ("headphones", "Kopfhörer"),
    ("deck {}", "Deck {}"),
    ("split cue on 1/2: headphones left, master right", "Split Cue auf 1/2: Kopfhörer links, Master rechts"),
    ("master {}", "Master {}"),
    ("headphones {}", "Kopfhörer {}"),
    ("deck {} {}", "Deck {} {}"),
    ("headphones on {}", "Kopfhörer auf {}"),
    ("left out (the device has {} outputs): {}", "weggelassen (das Gerät hat {} Ausgänge): {}"),
    ("headphone device did not start: {}", "Kopfhörer-Gerät startete nicht: {}"),
    ("OUT {}", "AUSG. {}"),
    ("MIDI clock: {} did not open ({})", "MIDI-Clock: {} ließ sich nicht öffnen ({})"),
    ("MIDI clock on {}: {} pulses sent", "MIDI-Clock auf {}: {} Pulse gesendet"),
    ("MIDI clock did not open: {}", "MIDI-Clock ließ sich nicht öffnen: {}"),
    ("MIDI clock off", "MIDI-Clock aus"),
    ("The output device", "Das Ausgabegerät"),
    ("Off", "Aus"),
    ("Deck {}: pick the parts to hear on the headphones", "Deck {}: Wähle die Parts für den Kopfhörer"),
    ("No headphone output: choose one in Preferences › Audio to hear the stem cue", "Kein Kopfhörerausgang: Wähle einen unter Einstellungen › Audio, um den Stem-Cue zu hören"),
    ("OWN FILES · {}", "EIGENE DATEIEN · {}"),
    ("SEPARATED", "GETRENNT"),
    ("SEPARATING…", "TRENNT …"),
    ("NOT SEPARATED", "NICHT GETRENNT"),
    ("PLAY THE TRACK TO PICK ITS PARTS", "TRACK ABSPIELEN, UM SEINE PARTS ZU WÄHLEN"),
    ("the player", "dem Player"),
    ("Eight-output interface", "Interface mit acht Ausgängen"),
    ("Built-in output", "Eingebauter Ausgang"),
    ("Own stem files on {}: {}", "Eigene Stem-Dateien auf {}: {}"),
    ("Own stem files for {} not used: {}", "Eigene Stem-Dateien für {} nicht verwendet: {}"),
    ("parts missing: {}", "Parts fehlen: {}"),
    ("{} is {} Hz, the track {} Hz", "{} hat {} Hz, der Track {} Hz"),
    ("{} is {} s long, the track {} s", "{} ist {} s lang, der Track {} s"),
    ("{} could not be read: {}", "{} ließ sich nicht lesen: {}"),
];

/// Whether `english` has a German translation (independent of the
/// current language).
pub fn has_german(english: &str) -> bool {
    DE.iter().any(|(e, _)| *e == english)
}

/// Translates `english` into the current language.
pub fn tr(english: &str) -> &str {
    match language() {
        Language::English => english,
        Language::German => DE
            .iter()
            .find(|(e, _)| *e == english)
            .map(|(_, d)| *d)
            .unwrap_or(english),
    }
}

/// Translates `english` and fills its `{}` placeholders with `args`.
pub fn trf(english: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::new();
    let mut parts = tr(english).split("{}");
    if let Some(first) = parts.next() {
        out.push_str(first);
    }
    for (i, p) in parts.enumerate() {
        if let Some(a) = args.get(i) {
            out.push_str(&a.to_string());
        }
        out.push_str(p);
    }
    out
}

/// Matches `text` against `template` (with `{}` placeholders) and returns
/// the captured values.
fn capture<'a>(template: &str, text: &'a str) -> Option<Vec<&'a str>> {
    let parts: Vec<&str> = template.split("{}").collect();
    let mut rest = text.strip_prefix(parts[0])?;
    let mut caps = Vec::new();
    for (i, lit) in parts.iter().enumerate().skip(1) {
        let last = i == parts.len() - 1;
        let end = if last {
            if !rest.ends_with(lit) {
                return None;
            }
            rest.len() - lit.len()
        } else if lit.is_empty() {
            return None;
        } else {
            rest.find(lit)?
        };
        caps.push(&rest[..end]);
        rest = &rest[end + lit.len()..];
    }
    (rest.is_empty() || parts.len() == 1 && rest.is_empty()).then_some(caps)
}

/// Translates a finished message by finding the template it was formatted
/// from. Unknown messages are returned unchanged.
pub fn translate_message(text: &str) -> String {
    if language() == Language::English {
        return text.to_string();
    }
    for (e, d) in DE {
        if !e.contains("{}") {
            if *e == text {
                return d.to_string();
            }
            continue;
        }
        if let Some(caps) = capture(e, text) {
            let mut out = String::new();
            let mut parts = d.split("{}");
            out.push_str(parts.next().unwrap_or_default());
            for (i, p) in parts.enumerate() {
                out.push_str(caps.get(i).copied().unwrap_or_default());
                out.push_str(p);
            }
            return out;
        }
    }
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_and_formats() {
        set_language(Language::English);
        assert_eq!(trf("Hot cue {} set", &[&'A']), "Hot cue A set");
        set_language(Language::German);
        assert_eq!(trf("Hot cue {} set", &[&'A']), "Hot Cue A gesetzt");
        assert_eq!(tr("unknown text"), "unknown text");
        assert_eq!(
            translate_message("Added 3 track(s) to Warmup"),
            "3 Track(s) zu Warmup hinzugefügt"
        );
        assert_eq!(
            translate_message("Red synced: 1 copied, 2 updated, 0 removed"),
            "Red synced: 1 copied, 2 updated, 0 removed"
                .replace("synced", "synchronisiert")
                .replace("copied", "kopiert")
                .replace("updated", "aktualisiert")
                .replace("removed", "entfernt")
        );
        assert_eq!(
            translate_message("Nothing to undo"),
            "Nichts rückgängig zu machen"
        );
        assert_eq!(
            translate_message("Added Deeper to the collection — analysing"),
            "Deeper zur Sammlung hinzugefügt — wird analysiert"
        );
        set_language(Language::English);
    }

    #[test]
    fn no_duplicate_entries() {
        let mut seen = std::collections::HashSet::new();
        for (e, _) in DE {
            assert!(seen.insert(*e), "duplicate entry {e}");
        }
        assert!(has_german("Undo") && !has_german("no such text"));
    }

    #[test]
    fn placeholder_counts_match() {
        for (e, d) in DE {
            assert_eq!(e.matches("{}").count(), d.matches("{}").count(), "{e}");
        }
    }
}
