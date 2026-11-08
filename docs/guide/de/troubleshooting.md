---
title: Fehlersuche und FAQ
description: Was tun, wenn ein Track fehlt, ein Stick nicht synchronisiert, kein Ton kommt oder ein Controller nicht erkannt wird.
keywords: [problem, probleme, hilfe, geht nicht, fehler, faq, fehlersuche, kein ton, audio, absturz, fehlend, track fehlt, verschoben, controller nicht erkannt, stick nicht erkannt, langsam, aussetzer, lässt sich nicht öffnen]
features: []
---

## Ein Track hat eine durchgestrichene Linie

Die Datei fehlt oder wurde verschoben. Drücke **{n} VERSCHOBEN** oben in der Liste oder `⇧⌘L` und zeige auf den Ordner, in dem die Dateien jetzt liegen. Die Tracks werden gefunden und neu verbunden. Siehe [Verschobene Dateien](library.md#verschobene-dateien).

## Kein Ton

- Prüfe **Einstellungen › Audio › Ausgabegerät**. Automatisch wählt den Controller, wenn einer angeschlossen ist, sonst die Systemausgabe.
- Audioeinstellungen gelten ab dem nächsten Audiostart. Hast du etwas geändert, starte die App neu.
- Unter Linux probiere dort ein anderes Audio-Backend.
- Hat ein Deck einen Track, die Anzeigen bleiben aber flach, prüfe den Fader und **TRIM** und ob der Crossfader auf der anderen Seite steht.

## Der Ton setzt aus

Erhöhe die **Puffergröße** unter **Einstellungen › Audio**, schließe andere schwere Programme und achte unter Linux oder macOS darauf, dass der Rechner nicht im Stromsparmodus läuft.

## Der Controller wird nicht erkannt

Schließe ihn direkt per USB an. Er erscheint in wenigen Sekunden; **Einstellungen › Controller › Erkannt** listet ihn. Schalte dort den **Monitor** an, um zu sehen, ob seine Nachrichten überhaupt ankommen.

## Der Stick synchronisiert nicht

- Der Stick muss registriert (siehe [USB-Sticks](usb-sticks.md)) und eingesteckt sein.
- Schau auf das Stick-Board und unten im Fenster nach Fehlermeldungen. Volle oder schreibgeschützte Sticks lassen sich nicht beschreiben.
- **STICK NEU SCHREIBEN** schreibt ihn komplett neu.

## macOS meldet, die App könne nicht geöffnet werden

Das ist beim ersten Start normal, weil die App nicht von Apple signiert ist. Die [Download-Seite](/download/#macos) führt durch „Dennoch öffnen“.

## Windows warnt vor der App

Drücke **Weitere Informationen** und dann **Trotzdem ausführen**. Das Programm ist noch nicht code-signiert.

## Stems bleiben bei SPLIT stehen oder scheitern

Die Trennung braucht beim ersten Mal einmalig den Download ihres Modells, prüfe dann die Internetverbindung. Danach läuft sie offline.

## Meine Cue- oder Grid-Änderung ist nicht auf dem Stick

Der Sync läuft innerhalb von Sekunden, aber nur, solange der Stick eingesteckt und registriert ist. Drücke **JETZT SYNCHRONISIEREN** (`⌘S`), um sicherzugehen.

## Wie sichere ich meine Bibliothek?

Die Bibliothek wird beim App-Start automatisch gesichert, und **JETZT SICHERN** liegt unter **Einstellungen › Erweitert**. Ein [Bibliotheksarchiv](library.md#umzug-auf-einen-neuen-rechner) bringt sie auf einen neuen Rechner.

## Wo liegen meine Dateien?

Die Bibliothek liegt in deinem Benutzerdatenordner (macOS `~/Library/Application Support/OpenDJDecks`, Linux `~/.local/share/opendjdecks`, Windows `%LOCALAPPDATA%\OpenDJDecks`). Deine Musik bleibt, wo sie immer war. Aufnahmen landen im Ordner aus **Einstellungen › Aufnahme**.

## Gibt es Video, DVS oder Streaming?

Nein. Video, DVS (Timecode-Vinyl), Streaming-Dienste und Cloud-Sync sind nicht vorgesehen: OpenDJDecks arbeitet offline und ohne Konten.
