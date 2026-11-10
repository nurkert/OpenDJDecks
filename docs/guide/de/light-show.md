---
title: Die DMX-Lichtshow
description: Eine Lichtshow, die aus Phrasen, Beat und Energie der Musik folgt, ein einfacher Modus mit Looks und ein Pro-Modus mit Programmen, Effekten und Cue-Stack.
keywords: [licht, lichter, lighting, dmx, lichtshow, light show, art-net, artnet, sacn, usb dmx, scheinwerfer, fixture, moving head, par, strobe, laser, nebel, blackout, enttec, bühne, cue stack, ofl]
features: [lighting-outputs, lighting-fixtures, lighting-auto, lighting-easy, lighting-pro, lighting-views]
---

OpenDJDecks kann das Licht selbst fahren. Der Tab **LIGHTING** (`⌥⌘3`) hat drei Ansichten: **RIG** zum Patchen deiner Lampen, **BÜHNE** mit einer live 2D-Vorschau und **SHOW** zum Fahren.

![Die Ansicht LIGHTING mit dem gepatchten Rig](../img/lighting.png)

## Lampen patchen

In **RIG**:

1. **SCHEINWERFER HINZUFÜGEN**: Gib einen **Namen**, wähle eine **Gruppe** (Moving Heads, Front und so weiter), einen eingebauten Typ wie PAR, Mover, Bar, Strobe, Blinder, Laser oder Nebel, und drücke **HINZUFÜGEN**. Die nächste freie DMX-Adresse wird benutzt, und Überschneidungen werden geprüft.
2. Oder **SCHEINWERFER-DATEI IMPORTIEREN…** für eine Definition aus der Open Fixture Library (`.json`).

**AUSGANG** wählt, wie ein Universe den Rechner verlässt: Art-Net, sACN oder ein USB-DMX-Interface (Enttec Pro). **SETZEN** übernimmt es; mehrere Universes sind möglich.

## Die Show fahren

**DMX AUS** sendet die Show an deine Ausgänge. Wähle, wie sie läuft:

- **AUTO** folgt Phrasen, Beat und Energie. Intros atmen, Build-ups jagen schneller und enden im Strobe, Choruses schwenken die Moving Heads im Takt, Breakdowns beruhigen sich, Outros blenden aus. Im Mix verschmelzen die Looks beider Tracks je nach Lautstärke, und Phrasenwechsel blenden über einen Takt.
- **EASY** ist ein **LOOK** für das ganze Rig, im Takt, mit Farbfeldern, Master-Dimmer, **BLACKOUT** und **FLASH**, **STROBE** und **NEBEL**, solange gehalten. **TAP** setzt das Tempo, wenn kein Deck läuft.
- **PRO** setzt pro Gruppe ein **PROGRAMM**, eine **FARBE** und einen Attribut-Effekt (Sinus, Säge, Rechteck, Zufall auf Dimmer, Pan, Tilt; Phasenversatz; im Takt), einen XY-Programmer für Pan und Tilt, und einen **CUE-STACK** mit **GO**, **ZURÜCK** und **SPEICHERN** mit Fades in Beats. **TRACK-CUES** speichert einen Cue an einem Takt des Tracks, den du hörst, und ruft ihn ab, wann immer er läuft.

Am Deck legt der Pad-Modus **LIGHTING** dir die Looks A bis F, Strobe (G) und Flash (H) auf die Pads. Jedes Licht-Bedienelement lässt sich auch auf MIDI legen.

## Wo du es siehst

- Der Knopf **LICHTLEISTE** (`⌥⌘K`) zeigt in Perform eine Lichtleiste.
- **FENSTER** öffnet die Lichtansicht in einem eigenen Fenster, für einen zweiten Bildschirm.
