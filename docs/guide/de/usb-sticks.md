---
title: USB-Sticks und Sync
description: Registriere einen Stick einmal. Danach erreicht ihn jede Änderung automatisch und schrittweise, bereit für OpenDJDecks auf einem anderen Rechner oder den Raspberry Pi.
keywords: [usb, stick, sticks, usb stick, sync, synchronisieren, sychronisieren, export, exportieren, registrieren, auswerfen, neu schreiben, stick status, gerät, sync manager]
features: [export-auto-sync, export-device-registry, export-play-from-stick, export-native-format, export-club-usb, export-club-usb-mysetting, cues-from-players, import-club-usb]
---

Der Stick ist ein Spiegel deiner Bibliothek. Registriere ihn einmal mit den Listen, die darauf gehören, und jeder geänderte Cue, jede Playlist und jedes Grid ist innerhalb von Sekunden auf dem Stick, solange er steckt. Es gibt keinen Export-Schritt, an den du denken musst.

![Das USB-Stick-Board mit einem registrierten Stick](../img/usb.png)

## Einen Stick registrieren

1. Stecke den Stick ein. Er erscheint unter **GERÄTE** im Baum (oder in der Leiste **USB**).
2. Wähle die Playlist, das Set oder den Ordner aus, der darauf soll.
3. Drücke **MIT … REGISTRIEREN**. Der erste Sync schreibt alles einmal, danach nur Änderungen.

Das Stick-Board zeigt seinen Zustand, was aussteht, und den Fortschritt.

- **JETZT SYNCHRONISIEREN** (`⌘S`) synchronisiert sofort. Von selbst startet der Sync ohnehin wenige Sekunden nach jeder Änderung.
- **STICK NEU SCHREIBEN** schreibt den ganzen Stick neu aus deiner Bibliothek.
- **AUSWERFEN** (`⌘E`) wirft ihn sicher aus. Warte darauf, bevor du ihn ziehst.
- **VERGESSEN** beendet das Synchronisieren dieses Sticks.

## Was auf dem Stick liegt

Unter **INHALT — LISTEN AUF DIESEM STICK** wählst du die Listen. Der Stick trägt deine Musik mit Cues, Beatgrids, Phrasen und, falls vorhanden, den getrennten Parts für Stems. Sticks werden von OpenDJDecks gelesen, etwa auf einem anderen Rechner oder der [Raspberry-Pi-Einheit](raspberry-pi.md). Damit ein Stick auch auf Club-Playern läuft, schalte unten das Club-Player-USB-Format ein.

## Direkt vom Stick spielen

Öffne einen Stick im Baum, um direkt von ihm zu stöbern und zu laden, schreibgeschützt. Das macht die Raspberry-Pi-Einheit den ganzen Tag.

## Auf Club-Playern spielen (Club-Player-USB)

Ab Version 0.6 kann ein Stick zusätzlich das Club-Player-USB-Format tragen, das mit Pioneer-DJ-Playern kompatibel ist. Das wählst du für jeden Stick einzeln: Stick auswählen und unter **FORMATE** **Club-Player-USB (export.pdb)** einschalten. Das eigene Format von OpenDJDecks bleibt standardmäßig an, beide können gleichzeitig aktiv sein.

Ist das Format an, schreibt jeder Sync zusätzlich die Datenbank und die Analyse, die die Player lesen, mit deinen Playlists, Cues, Beatgrids und Waveforms, und übernimmt deine Standard-Player-Einstellungen (Quantize, Auto Cue, Tempobereich, Jog-Modus, Sprache). Nur geänderte Tracks werden neu geschrieben. Ogg- und Opus-Dateien bleiben außen vor, weil Player sie nicht abspielen können.

### Cues und Historien von den Playern

Ändert ein Player Cues auf dem Stick oder schreibt eine Historie, überschreibt OpenDJDecks das nicht. Beim nächsten Sync steht es auf dem Stick-Board unter **ÄNDERUNGEN AN PLAYERN — PRÜFEN**. **STICK-CUES ÜBERNEHMEN** holt die Cues des Players in deine Bibliothek, **BIBLIOTHEK BEHALTEN** behält deine und schreibt sie wieder auf den Stick; eine Historie kannst du als Playlist importieren oder verwerfen.

### Einen Stick von anderer Software importieren

Ab Version 0.6 liest **Ablage › Club-Player-USB importieren…** einen Club-Player-USB, den andere Software geschrieben hat: Tracks, Tags, Cues, Beatgrids, Phrasen, Playlists und die Historie. Wähle den Hauptordner des Sticks. Auf dem Stick ändert sich nichts; importierte Listen erscheinen in einem Ordner **Library import**.
