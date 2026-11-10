---
title: Die DMX-Lichtshow
description: Eine Lichtshow, die aus Phrasen, Beat und Energie der Musik folgt, eigene Looks und Lauflichter auf Pads, ein AUTO, das du lenken kannst, ein einfacher und ein Pro-Modus, ein Lichtpanel in Perform und Club, eine Patch-Tabelle, das Verbinden von Interfaces mit geführter Einrichtung, der Zustand der Ausgänge mit Failsafe und Sicherheitsgrenzen.
keywords: [pads, look, looks, lauflicht, chase, gruppen-master, palette, halten, freigeben, testmuster, node-suche, artpoll, anschluss, licht, lichter, lighting, dmx, lichtshow, light show, art-net, artnet, sacn, usb dmx, scheinwerfer, fixture, moving head, par, strobe, laser, nebel, blackout, bühne, cue stack, ofl, patch, universum, adresse, überschneidung, identifizieren, failsafe, sicherheit, epilepsie, fotosensitiv, pin]
features: [lighting-pads, lighting-panel, lighting-auto-steer, lighting-connect, lighting-outputs, lighting-fixtures, lighting-patch-table, lighting-failsafe, lighting-safety, lighting-auto, lighting-easy, lighting-pro, lighting-views]
---

OpenDJDecks kann das Licht selbst fahren. Der Tab **LIGHTING** (`⌥⌘3`) hat drei Spalten: links das Setup (**RIG**, **AUSGANG**, **SICHERHEIT**), in der Mitte **BÜHNE** (Vorschau), **PADS** für deine eigenen Looks, **PATCH** (Tabelle) und **KANÄLE**, rechts **SHOW** zum Fahren.

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

## Ein Interface verbinden

**GEFÜHRTE EINRICHTUNG** in **AUSGANG** führt dich in vier kurzen Schritten durch eine Verbindung:

1. Wie das Licht verbunden ist: **ART-NET-NODE**, **SACN-EMPFÄNGER** oder **USB-DMX-INTERFACE**.
2. Welches: Für Art-Net fragt die App das Netzwerk, welche Nodes da sind, und listet sie mit Name, Adresse und Universen, oder du tippst eine Adresse ein; für USB-DMX wählst du den seriellen Anschluss; sACN braucht keine Wahl.
3. Welches Universum dorthin geht (1 bei einem kleinen Rig; die Patch-Tabelle zeigt das Universum deiner Scheinwerfer).
4. **SETZEN UND TESTEN** setzt den Ausgang, schaltet DMX AUS an und startet das Testmuster. **JA, ES FOLGT** beendet den Test und schließt die Einrichtung; **NICHTS PASSIERT** zählt der Reihe nach auf, was du prüfen kannst.

![Die geführte Einrichtung mit einem gefundenen Art-Net-Node](../img/lighting-guide.png)

Dieselben Teile gibt es einzeln in **AUSGANG**:

- **NODES SUCHEN** (mit ART-NET gewählt) schickt ein ArtPoll, die "Wer ist da?"-Nachricht des Art-Net-Protokolls, an die Broadcast-Adresse des lokalen Netzwerks und listet jeden Node, der in etwa 1,5 Sekunden antwortet. **NUTZEN** schickt das Universum, das daneben eingetippt ist, an diesen Node. Ein Node mit mehreren Ausgängen bekommt das Universum auf dem Ausgang mit derselben Nummer, sonst auf seinem ersten.
- Mit **USB-DMX** gewählt listet **ANSCHLÜSSE** die seriellen Geräte, die nach USB-Adaptern aussehen (unter macOS `/dev/cu.usbserial…` und `/dev/cu.usbmodem…`, unter Linux `/dev/ttyUSB…`, `/dev/ttyACM…` und `/dev/serial/by-id`, unter Windows die vorhandenen COM-Anschlüsse). **AKTUALISIEREN** sucht nach dem Einstecken erneut; **NUTZEN** schickt das Universum an diesen Anschluss.
- **Mehrere Universen**: Gib jedem Universum eine eigene Zeile. An denselben Art-Net-Node geht Universum 1 an seine Port-Adresse 0, Universum 2 an 1 und so weiter; sACN ohne Nummer sendet Universum 1 als sACN-Universum 1, Universum 2 als 2 und so weiter.
- **TESTMUSTER**: **EINZELN** lässt einen Scheinwerfer nach dem anderen leuchten, je eine Sekunde; **FARBEN** schaltet das ganze Rig durch Rot, Grün, Blau und Weiß; **VOLL** lässt alles leuchten. Es ersetzt die Show bis **AUS**, mit 80 %, und geht wie alles andere durch die Sicherheitsgrenzen: Es kann nicht schneller blitzen als die Strobe-Grenze, die Pegelgrenze einer Gruppe nicht überschreiten und weder Nebel noch Laser öffnen.

![AUSGANG mit einem gefundenen Node, das Testmuster läuft](../img/lighting-connect.png)

Node-Suche, Anschlusswahl und Einrichtung sind mit einem Stellvertreter-Node auf der eigenen Loopback-Adresse des Rechners und vorgegebenen Testgeräten getestet. **Mit einem echten Art-Net-Node, sACN-Empfänger oder USB-DMX-Interface sind sie noch nicht ausprobiert.** Nodes, die nur an Port 6454 antworten, während ein anderes Lichtprogramm auf demselben Rechner diesen Port belegt, erscheinen eventuell nicht; tippe dann ihre Adresse ein.

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

Am Deck legt der Pad-Modus **LIGHTING** dir deine eigenen Pads der aktuellen Seite auf die Pads, oder die Looks A bis F, Strobe (G) und Flash (H), wo ein Pad leer ist. Jedes Licht-Bedienelement lässt sich auch auf MIDI legen.

## Eigene Looks und Lauflichter

**PADS** in der Mitte hält deine eigenen Looks auf vier Seiten mit je acht Pads (**SEITE 1** bis **4**). Ein Pad zeigt seine Nummer (Seite und Pad, etwa 1.3), seinen Namen, ob es ein Lauflicht oder ein Flash-Pad ist, und eine Linie aus den Farben, die es setzt, eine pro Gruppe. Das Pad, das gerade steuert, ist grün gefüllt.

- Drücke ein Pad, um es zu spielen. Ein **RASTEN**-Pad bleibt an, bis du es erneut drückst oder **FREIGEBEN** drückst; ein **FLASH**-Pad ist an, solange du es hältst.
- **EDIT** macht die Pads zum Editor: Klicke ein Pad an, um es zu öffnen. Ein leeres Pad bietet **NEUER LOOK** (jede Gruppe weiß und ruhig als Anfang), **NEUES LAUFLICHT** und **AUS PRO** (die Gruppeneinstellungen aus dem PRO-Modus).
- Ein Look setzt Gruppen: pro Gruppe ein Programm (**MUSIK** folgt der Phrase), eine Farbe (**TRACK** nimmt sie aus der Palette von AUTO oder vom Track) und einen Pegel (**—** überlässt ihn dem Programm). **×** lässt eine Gruppe aus; Gruppen, die ein Look auslässt, behalten, was darunter läuft. Die Farblinie des Pads folgt jeder Änderung sofort.
- Ein Lauflicht schaltet im Takt durch Looks. **SCHRITTE HINZU**, dann die Looks in ihrer Reihenfolge anklicken; der Stepper legt fest, wie lange jeder Schritt dauert (eine Viertel-Zählzeit bis zwei Takte). Ein Lauflicht folgt dem Deck, das du hörst, oder **TAP** ohne Musik.
- **GRUPPEN-MASTER** neben den Pads setzen den Pegel jeder Gruppe über allem, auch über dem Master-Dimmer.

Looks, Lauflichter, Seiten und Gruppen-Master werden mit dem Rig in `lighting.json` gespeichert. Welches Pad eingerastet oder gehalten ist, wird nicht gespeichert.

![PADS mit einem Look im Editor, rechts AUTO gehalten](../img/lighting-pads.png)

Am Deck spielt der Pad-Modus **LIGHTING** die acht Pads der aktuellen Seite mit den Pads A bis H; ein Pad der Seite ohne Look behält die eingebauten Looks A bis F, Strobe (G) und Flash (H). In einem Mapping erreichen `lighting.pad`, `lighting.page`, `lighting.page_next`, `lighting.page_prev` und `lighting.group_master` sie von einem Controller, und MIDI-Learn kann sie lernen (wähle Pad, Seite oder Gruppe unter **Pad, Seite oder Gruppe**).

## AUTO lenken

Mit **AUTO** gewählt lenkt die rechte Spalte es:

- **PALETTE** wählt die Farben von AUTO: **TONART** (aus der Tonart des Tracks, wie bisher), **WARM**, **KÜHL**, **ABENDROT**, **MEER**, **WEISS** oder **EIGENE** mit den Farbfeldern, die du ankreuzt. Die Farbe wechselt alle acht Takte, in Phrasen mit hoher Energie jedes zweite Taktpaar zur nächsten Farbe.
- **PHRASE → LOOK** legt fest, was AUTO für Intro, Strophe, Aufbau, Drop (Chorus), Breakdown und Outro spielt: ein eingebautes Programm oder eines deiner Pads. Die Phrase, die gerade läuft, ist markiert.
- **HALTEN** behält den Look, den AUTO gerade spielt, über Phrasenwechsel hinweg, bis du es erneut drückst oder **FREIGEBEN** drückst.

**Wer gewinnt.** Von oben:

1. **BLACKOUT**.
2. Die Tasten **FLASH** und **STROBE**, solange gehalten.
3. Ein Flash-Pad, solange gehalten (das zuletzt gedrückte).
4. Das eingerastete Pad.
5. **HALTEN**.
6. AUTO (oder EASY oder PRO).

Ein Pad übernimmt nur die Gruppen, die sein Look setzt; die anderen folgen weiter der Ebene darunter. Gruppen-Master und Master-Dimmer skalieren danach das Ergebnis, und die Sicherheitsgrenzen wirken zuletzt auf jedes Bild. **FREIGEBEN** gibt an AUTO zurück: Das eingerastete Pad und HALTEN enden, mit einer Überblendung über eine Zählzeit; ein Flash-Pad endet sofort beim Loslassen. Die Reihenfolge steht unter **WER GEWINNT, OBEN ZUERST**, die steuernde Ebene leuchtet, und **STEUERT** nennt sie, etwa "PAD 1.1 · RED WASH · EINGERASTET".

## Wo du es siehst

- Der Knopf **LICHTLEISTE** (`⌥⌘K`, in schmaleren Fenstern **LEISTE**) öffnet das Lichtpanel in Perform und Club. Es nimmt den Platz des Browsers ein, so behalten Decks, Wellenformen und Mixer ihre Größe; **LICHTLEISTE** bringt den Browser zurück. Es enthält die Bühnenvorschau (ab 1360 px Breite), Seiten und Pads, Master- und Gruppenfader, AUTO / EASY / PRO, DMX AUS, HALTEN und FREIGEBEN, wer gerade steuert (oder eine verlorene Verbindung) und BLACKOUT, FLASH, STROBE und NEBEL. Jede Taste, jedes Pad und jeder Fader ist mindestens 44 px groß für Touch, mit dunklen Flächen und heller Schrift für eine dunkle Kabine.
- **FENSTER** öffnet die Lichtansicht in einem eigenen Fenster, für einen zweiten Bildschirm.

![Das Lichtpanel in Perform, ein Look eingerastet](../img/lighting-perform.png)

![Das Lichtpanel im Club-Modus](../img/lighting-club.png)

Das Panel ist mit der Maus und in UI-Tests bei 1440 × 900 und 1280 × 800 ausprobiert; auf einem echten Touchscreen noch nicht.
