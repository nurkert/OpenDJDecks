---
title: Bibliothek importieren
description: Eine vorhandene Bibliothek einmal als Bibliotheks-XML einlesen, mit Cues, Grids, Tonarten und Playlists.
keywords: [rekordbox, import, importieren, xml, bibliotheks-xml, itunes, apple music, mediathek, einstellungen, tastenkürzel]
features: [browser-xml-itunes-exchange, import-settings, import-encrypted-library]
---

OpenDJDecks kann eine Bibliothek aus einer **Bibliotheks-XML-Datei** einlesen, damit du mit deinen Cues, Beatgrids, Tonarten und Playlists startest.

## Die XML-Datei erstellen

Exportiere in rekordbox deine Sammlung über das Menü Datei als XML-Datei und speichere sie an einem Ort, den du wiederfindest. Hast du rekordbox auf diesem Rechner, übernimmt **Ablage › rekordbox-Einstellungen und -Tastenkürzel importieren…** (auch unter **Einstellungen › Erweitert**) zusätzlich die Einstellungen und Tastenkürzel, wo sie nicht mit vorhandenen kollidieren.

## Die Datei importieren

1. Öffne **Ablage › Bibliotheks-XML importieren…** (`⇧⌥⌘X`), oder **BIBLIOTHEK IMPORTIEREN** auf dem leeren Bibliotheksbildschirm.
2. Wähle die XML-Datei. Tracks, Playlists, Cues, Beatgrids und Tonarten kommen in deine Bibliothek.

Cover kommen aus den Dateien selbst. Deine Musikdateien bleiben, wo sie sind. Wird danach ein Track nicht gefunden, siehe [verschobene Dateien](library.md#verschobene-dateien).

## Andere Formate

- **Ablage › Bibliotheks-XML exportieren…** schreibt deine Bibliothek in eine Datei, die andere DJ-Software lesen kann (`⌥⌘X`).
- **Ablage › Musik- / iTunes-Mediathek (XML) importieren…** liest eine aus Apple Music oder iTunes exportierte `Library.xml`, mit Tags, Bewertungen und Playlist-Ordnern (`⇧⌥⌘M`).

Um deine ganze OpenDJDecks-Bibliothek auf einen neuen Rechner zu bringen, nimm das Bibliotheksarchiv; siehe [Umzug auf einen neuen Rechner](library.md#umzug-auf-einen-neuen-rechner).

## Verschlüsselte Bibliotheksdatenbank

Ab Version 0.6 kann OpenDJDecks auch eine verschlüsselte Bibliotheksdatenbank direkt lesen, mit Tracks, Cues, Beatgrids, Tonarten, Playlists, Smart-Playlists (als normale Playlists mit dem Inhalt zum Importzeitpunkt), Covern und der Wiedergabe-Historie. Das ist optional; der XML-Weg oben braucht keinen Schlüssel.

1. Wähle **Ablage › Verschlüsselte Bibliotheksdatenbank importieren…**.
2. **DATEI WÄHLEN**: Wähle die Datenbankdatei deiner Bibliothek selbst aus. OpenDJDecks durchsucht deinen Computer nie danach.
3. Gib unter **ERWEITERT** den Schlüssel für diese Bibliothek ein.
4. Drücke **IMPORTIEREN**.

Du brauchst den Schlüssel für deine eigene Bibliothek. OpenDJDecks enthält ihn nicht und beschafft ihn nicht, und kann dir auch nicht helfen, ihn zu finden. Der Schlüssel wird nur für diesen Import verwendet: Er wird nicht gespeichert und nicht protokolliert. Deine Datenbankdatei wird nur gelesen, nie verändert, und die temporäre entschlüsselte Kopie wird am Ende des Imports entfernt, auch wenn er fehlschlägt. Ist der Schlüssel falsch, erscheint die Meldung „Der Schlüssel wurde nicht akzeptiert", und es wird nichts importiert.

Listen landen in einem Ordner **Library import**. Ein erneuter Import fügt nichts doppelt hinzu.
