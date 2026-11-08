---
title: USB-Sticks und Sync
description: Registriere einen Stick einmal. Danach erreicht ihn jede Änderung automatisch und schrittweise, bereit für OpenDJDecks auf einem anderen Rechner oder den Raspberry Pi.
keywords: [usb, stick, sticks, usb stick, sync, synchronisieren, sychronisieren, export, exportieren, registrieren, auswerfen, neu schreiben, stick status, gerät, sync manager]
features: [export-auto-sync, export-device-registry, export-play-from-stick, export-native-format]
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

Unter **INHALT — LISTEN AUF DIESEM STICK** wählst du die Listen. Der Stick trägt deine Musik mit Cues, Beatgrids, Phrasen und, falls vorhanden, den getrennten Parts für Stems. Sticks werden von OpenDJDecks gelesen, etwa auf einem anderen Rechner oder der [Raspberry-Pi-Einheit](raspberry-pi.md); auf Club-Playern lassen sie sich nicht abspielen.

## Direkt vom Stick spielen

Öffne einen Stick im Baum, um direkt von ihm zu stöbern und zu laden, schreibgeschützt. Das macht die Raspberry-Pi-Einheit den ganzen Tag.
