---
title: Die DMX-Lichtshow
description: Eine Lichtshow, die aus Phrasen, Beat und Energie der Musik folgt, ein einfacher Modus mit Looks, ein Pro-Modus mit Programmen, Effekten und Cue-Stack, eine Patch-Tabelle, der Zustand der Ausgänge mit Failsafe und Sicherheitsgrenzen.
keywords: [licht, lichter, lighting, dmx, lichtshow, light show, art-net, artnet, sacn, usb dmx, scheinwerfer, fixture, moving head, par, strobe, laser, nebel, blackout, bühne, cue stack, ofl, patch, universum, adresse, überschneidung, identifizieren, failsafe, sicherheit, epilepsie, fotosensitiv, pin]
features: [lighting-outputs, lighting-fixtures, lighting-patch-table, lighting-failsafe, lighting-safety, lighting-auto, lighting-easy, lighting-pro, lighting-views]
---

OpenDJDecks kann das Licht selbst fahren. Der Tab **LIGHTING** (`⌥⌘3`) hat drei Spalten: links das Setup (**RIG**, **AUSGANG**, **SICHERHEIT**), in der Mitte **BÜHNE** (Vorschau), **PATCH** (Tabelle) und **KANÄLE**, rechts **SHOW** zum Fahren.

![Die Ansicht LIGHTING mit dem gepatchten Rig](../img/lighting.png)

## Lampen patchen

In **RIG** wählst du einen eingebauten Typ (PAR, Mover, Bar, Strobe, Blinder, Laser, Nebel), gibst einen **Namen** und eine **Gruppe** an und drückst **HINZUFÜGEN**:

- **ANZAHL** fügt mehrere Scheinwerfer des Typs nacheinander hinzu, benannt "Name 1", "Name 2" und so weiter.
- **UNIVERSUM** und **START** legen fest, wo der erste hinkommt. Leer werden die ersten freien Adressen benutzt. Passen die Scheinwerfer nicht hinein oder würden sie einen schon gepatchten überschneiden, wird keiner hinzugefügt; die Statuszeile nennt den Scheinwerfer, der im Weg ist.
- **SCHEINWERFER-DATEI IMPORTIEREN…** patcht eine Definition aus der Open Fixture Library (`.json`) mit allen ihren Modi.

**PATCH** in der Mitte listet jeden Scheinwerfer. **UNIV**, **ADRESSE** und **KAN** (Kanalzahl) änderst du, indem du einen Wert eintippst und Enter drückst; **MODUS** schaltet durch die Modi des Typs. Eine Kanalzahl, zu der es einen Modus gibt, wechselt in diesen Modus; jede andere Zahl behält die Belegung und füllt die zusätzlichen Kanäle mit null ("Custom"). In Dateien musst du nichts ändern.

Teilen sich nach einer Änderung zwei Scheinwerfer Kanäle, bleibt die Änderung stehen, damit du ein Rig Schritt für Schritt umadressieren kannst. Beide Zeilen bekommen eine durchgestrichene Linie und nennen den Scheinwerfer, den sie überschneiden; ein Scheinwerfer, dessen Kanäle über 512 hinausgehen, sagt das ebenfalls.

**ID** lässt einen Scheinwerfer 1,5-mal pro Sekunde blinken, damit du ihn an der Traverse findest. Noch einmal drücken beendet es.

![Die Patch-Tabelle mit einer Überschneidung und einem blinkenden Scheinwerfer](../img/lighting-patch.png)

## Ausgänge und eine verlorene Verbindung

**AUSGANG** wählt, wie jedes Universum den Rechner verlässt: Art-Net, sACN oder ein USB-DMX-Interface (serielles Gerät). Tippe das Universum (leer ist Universum 1) und das Ziel ein und drücke **SETZEN**; mehrere Universen sind möglich.

Jeder Ausgang hat eine Statuslinie: durchgehend grün, solange er sendet, durchgestrichen mit **VERLOREN** in Rot, wenn die Verbindung weg ist, gestrichelt für 20 Sekunden, nachdem sie zurückkam. Eine Verbindung, die ausfällt, wird jede Sekunde neu versucht. Ein Interface, das wieder eingesteckt wird oder erst nach dem Start der App, wird so von selbst gefunden. Perform zeigt denselben Zustand neben DMX in der Lichtleiste, und in der oberen Leiste, solange eine Verbindung fehlt und die Leiste ausgeblendet ist.

**WENN EINE VERBINDUNG FEHLT** entscheidet, was das ganze Rig zeigt, solange irgendeine Verbindung fehlt:

- **HALTEN** behält den letzten Look.
- **SICHERE SZENE** blendet in zwei Sekunden auf ein ruhiges warmes Licht mit 30 %: kein Strobe, kein Nebel, keine Laser.
- **BLACKOUT** wird sofort dunkel.

Sind alle Verbindungen zurück, blendet das Rig in einer Sekunde zurück zur Show.

![AUSGANG mit einem Universum, das sendet, und einer verlorenen USB-Verbindung](../img/lighting-output.png)

**KANÄLE** zeigt die 512 Werte eines Universums so, wie sie gesendet werden, nach den Sicherheitsgrenzen.

### Was passiert, wenn die App abstürzt

- Fällt die Show selbst innerhalb der App aus, fängt der Licht-Thread das ab und sendet ab dann die Failsafe-Szene.
- Wird die App normal beendet, senden **SICHERE SZENE** und **BLACKOUT** diese Szene als letztes Bild. **HALTEN** sendet nichts mehr.
- Wird die App hart beendet oder bleibt der Rechner stehen, kann die App nichts tun. Was das Licht dann macht, entscheidet die Hardware: Die meisten USB-DMX-Interfaces mit eigenem Prozessor und die meisten Art-Net-Nodes wiederholen das letzte Bild, das sie bekommen haben; sACN-Empfänger halten meist einige Sekunden und folgen dann ihrer eigenen Einstellung. Stelle das Verhalten bei DMX-Verlust an Interface und Scheinwerfern ein, wenn es darauf ankommt.

Verlust und Wiederkehr sind mit einer simulierten Verbindung getestet. Mit einem echten Interface, das abgezogen wird, sind sie noch nicht ausprobiert.

## Sicherheitsgrenzen

**SICHERHEIT** setzt Grenzen, die jeder Look einhalten muss. Sie wirken auf das fertige DMX-Bild direkt vor dem Senden, also gehen automatische Looks, Tasten, Cues, Track-Cues, die Fernsteuerung und das Identifizieren alle hindurch.

- **GRENZE FÜR STROBE UND BLITZE**: höchstens so viele Blitze pro Sekunde, voreingestellt 3, 0 für gar keinen Strobe. Schnell blitzendes Licht kann bei Menschen mit fotosensitiver Epilepsie Anfälle auslösen; Richtlinien für Fernsehen und Veranstaltungen sehen das Risiko ab etwa 3 Blitzen pro Sekunde, daher ist 3 oder weniger die sichere Voreinstellung. Die Grenze wirkt auf zwei Wegen. Strobe-Kanäle werden auf die erlaubte Rate gesenkt; dabei wird angenommen, dass der schnellste Strobe-Wert eines Scheinwerfers 25 Blitze pro Sekunde sind, der übliche Bereich. Ein Scheinwerfer, der schneller strobt, kann also etwas schneller als die Grenze blitzen. Und ein Scheinwerfer, dessen Pegel von dunkel auf hell springt, bleibt dunkel, bis seit seinem letzten Blitz eine Periode der Grenze vergangen ist. Das deckt schnelle Lauflichter und Effekte auf Scheinwerfern ohne Strobe-Kanal ab.
- **SPERRE**: Nebelmaschinen und Laser bleiben auf null, bis du **NEBEL SCHARF SCHALTEN** oder **LASER SCHARF SCHALTEN** drückst. Das wird nicht gespeichert: Jeder Start der App beginnt gesperrt. **NEBEL** in SHOW tut nichts, solange Nebel gesperrt ist.
- **PEGELGRENZE PRO GRUPPE**: der höchste Pegel, den eine Gruppe erreichen darf, in Schritten von 10 %.
- **SETUP-PIN**: 4 bis 8 Ziffern. Mit einer PIN schließt **SPERREN** das Setup: Patch, Ausgänge und Grenzen brauchen die PIN, die Show-Bedienung rechts bleibt für Gäste frei. Die PIN steht in `lighting.json` neben der Bibliothek; sie hält Gäste aus dem Setup heraus und ist kein Passwort gegen jemanden mit Zugriff auf den Rechner.

![SICHERHEIT mit Strobe-Grenze, Sperre, Pegelgrenzen und Setup-PIN](../img/lighting-safety.png)

## Die Show fahren

**DMX AUS** sendet die Show an deine Ausgänge. Wähle, wie sie läuft:

- **AUTO** folgt Phrasen, Beat und Energie. Intros atmen, Build-ups jagen schneller und enden im Strobe, Choruses schwenken die Moving Heads im Takt, Breakdowns beruhigen sich, Outros blenden aus. Im Mix verschmelzen die Looks beider Tracks je nach Lautstärke, und Phrasenwechsel blenden über einen Takt.
- **EASY** ist ein **LOOK** für das ganze Rig, im Takt, mit Farbfeldern, Master-Dimmer, **BLACKOUT** und **FLASH**, **STROBE** und **NEBEL**, solange gehalten. **TAP** setzt das Tempo, wenn kein Deck läuft.
- **PRO** setzt pro Gruppe ein **PROGRAMM**, eine **FARBE** und einen Attribut-Effekt (Sinus, Säge, Rechteck, Zufall auf Dimmer, Pan, Tilt; Phasenversatz; im Takt), einen XY-Programmer für Pan und Tilt, und einen **CUE-STACK** mit **GO**, **ZURÜCK** und **SPEICHERN** mit Fades in Beats. **TRACK-CUES** speichert einen Cue an einem Takt des Tracks, den du hörst, und ruft ihn ab, wann immer er läuft.

Am Deck legt der Pad-Modus **LIGHTING** dir die Looks A bis F, Strobe (G) und Flash (H) auf die Pads. Jedes Licht-Bedienelement lässt sich auch auf MIDI legen.

## Wo du es siehst

- Der Knopf **LICHTLEISTE** (`⌥⌘K`) zeigt in Perform eine Lichtleiste mit dem Zustand der Ausgänge.
- **FENSTER** öffnet die Lichtansicht in einem eigenen Fenster, für einen zweiten Bildschirm.

![Die Lichtleiste in Perform mit einer verlorenen Verbindung](../img/lighting-perform.png)
