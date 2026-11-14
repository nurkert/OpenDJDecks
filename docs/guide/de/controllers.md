---
title: Controller und MIDI-Learn
description: Kompatiblen Controller einstecken und spielen. Jeden anderen Controller mit der geführten Einrichtung einrichten, einzelne Regler mit MIDI-Learn zuordnen und im MIDI-Monitor sehen, was dein Controller sendet.
keywords: [controller, midi, midi learn, mapping, ddj, ddj-flx10, ddj-flx4, flx10, flx4, pioneer, led, soft takeover, browse, encoder, midi monitor, hardware, belegen, controller-liste, geführte einrichtung, einrichten, vorlage, battle, einsteiger, unbekannter controller, neuer controller]
features: [ctrl-controller-list, ctrl-guided-setup, ctrl-class-templates, ctrl-self-test, ctrl-hardware-status, ctrl-led-feedback, ctrl-soft-takeover, ctrl-midi-learn-monitor, ctrl-mapping-four-deck, ctrl-mapping-two-deck, browser-controller-browse]
---

OpenDJDecks bringt Mappings mit, kompatibel mit Pioneer DJ DDJ-FLX10 und DDJ-FLX4, mit LED-Rückmeldung für Play, Cue, Sync, Master, Keylock, Slip, Loop, die Pads und die Pegelanzeigen. Die Mappings sind noch nicht vollständig auf echten Geräten bestätigt, einzelne Regler oder LEDs funktionieren vielleicht nicht: Probiere sie vor einem Gig aus. Stecke einen Controller per USB ein; er wird in wenigen Sekunden erkannt und sein Audio-Interface automatisch benutzt (**Einstellungen › Audio › Ausgabegerät**).

Bewegst du nach einem Sprung im Preset einen Regler, sorgt Soft Takeover dafür, dass sich der Wert erst ändert, wenn der echte Regler dort angekommen ist.

![Die Seite Controller in den Einstellungen](../img/controllers.png)

## Controller-Liste

Welche Controller OpenDJDecks kennt und wie weit jeder bestätigt ist. Ein Modell gilt erst dann als **bestätigt**, wenn jedes seiner Bedienelemente mit OpenDJDecks auf einem echten Gerät bestätigt wurde. Das ist noch bei keinem der Fall: In der Zeile des DDJ-FLX10 steht **teilweise bestätigt** für die an einem Gerät geprüften Elemente, in den anderen **nicht bestätigt**. Hersteller- und Modellnamen sagen nur, mit welcher Hardware ein Mapping kompatibel sein soll.

<!-- controller-list:start (generated: cargo run -p od-controller --example hardware_status -- --write) -->

| Controller | Decks | Mapping | Erkannt an | Status |
|---|---|---|---|---|
| kompatibel mit DDJ-FLX10 | 4 | eingebaut | MIDI-Portname `DDJ-FLX10` | teilweise bestätigt: 10 bestätigt, 451 teilweise, 12 ungetestet |
| kompatibel mit DDJ-FLX4 | 2 | eingebaut | MIDI-Portname `DDJ-FLX4` | nicht bestätigt: 0 bestätigt, 219 teilweise, 2 ungetestet |
| Jeder Einsteiger-Controller mit zwei Decks | 2 | geführte Einrichtung, Vorlage `two-deck-entry` (76 Bedienelemente) | der Port des eingerichteten Geräts | nicht bestätigt: deine eigene Aufnahme, kein anderes Gerät |
| Jeder Battle-Controller mit zwei Decks | 2 | geführte Einrichtung, Vorlage `two-deck-battle` (74 Bedienelemente) | der Port des eingerichteten Geräts | nicht bestätigt: deine eigene Aufnahme, kein anderes Gerät |
| Jeder Controller mit vier Decks | 4 | geführte Einrichtung, Vorlage `four-deck` (134 Bedienelemente) | der Port des eingerichteten Geräts | nicht bestätigt: deine eigene Aufnahme, kein anderes Gerät |
<!-- controller-list:end -->

Jeder andere Controller lässt sich mit der geführten Einrichtung unten einrichten, ausgehend von einer von drei allgemeinen Vorlagen. Weitere Modell-Mappings, auch für Einsteiger- und Battle-Controller, kommen von Besitzern, die ihr eigenes Gerät mit der geführten Einrichtung aufnehmen und die Datei weitergeben. Keines davon ist schon für andere Geräte bestätigt. OpenDJDecks leitet keine Mappings aus Herstellerhandbüchern, MIDI-Tabellen, Herstellersoftware oder Mapping-Dateien anderer Projekte ab.

## Geführte Einrichtung für andere Controller

![Die Controller-Einrichtung unter der oberen Leiste fragt nach dem Tempo-Fader von Deck 1](../img/controller-setup.png)

Für einen Controller ohne Mapping drückst du **CONTROLLER EINRICHTEN** unter **Einstellungen › Controller** (oder **Ansicht › Controller einrichten**). Steckst du ein MIDI-Gerät ein, für das OpenDJDecks kein Mapping hat, nennt die Statuszeile es und verweist hierher (nicht in CREATE, wo ein MIDI-Keyboard so spielt, wie es ist). Unter der oberen Leiste öffnet sich ein Band:

1. Schließe den Controller per USB an und wähle, was ihm am nächsten kommt: **2 DECKS · EINSTEIGER**, **2 DECKS · BATTLE** oder **4 DECKS**. Drücke **START**.
2. Das Band fragt nach einem Bedienelement nach dem anderen: PLAY, CUE und SYNC, das Jogwheel (berühren, dann drehen), Tempo-Fader und LOAD jedes Decks, dann die Hot-Cue-Pads, die Kanalfader, TRIM und die EQ-Regler, Kopfhörer-CUE, den Crossfader, den Browse-Regler sowie Master- und Kopfhörerpegel. Das passende Element auf dem Bildschirm bekommt den Rahmen der Anleitung.
3. Bewege das Element. **Empfangen** zeigt, was ankam; sobald das Element einen Moment ruht, ist der Schritt übernommen und der nächste kommt. Ein Element, das ein früherer Schritt schon benutzt, wird mit der Nummer dieses Schritts abgelehnt. **ÜBERSPRINGEN** lässt weg, was dein Controller nicht hat, **ZURÜCK** geht einen Schritt zurück, um ihn neu zu lernen, **NOCHMAL** vergisst, was ankam.
4. Nach den Grund-Bedienelementen fragt das Band, ob es weitergehen soll: **WEITERE ELEMENTE** lernt SHIFT, die Hot-Cue-Pads E bis H, LOOP IN und OUT, die 4-Beat-Loop, Beat Jump zurück und vor, KEY LOCK und SLIP jedes Decks, die CH-FX-Regler, den Kopfhörer-Mix, Browse zurück sowie eine Effekt-an/aus-Taste und einen Effekt-Pegel, jedes davon überspringbar. **JETZT SPEICHERN** geht direkt zum Speichern mit den Grund-Bedienelementen.
5. Am Ende gibst du dem Controller einen Namen und drückst **MAPPING SPEICHERN**. Die Datei landet neben deiner Bibliothek in `mappings/`, wird deine eigene Mapping-Datei und gilt ab dem nächsten Start. Die Battle-Vorlage stellt außerdem die Crossfader-Kurve auf **CUT**.

Die Datei ist eine gewöhnliche Mapping-Datei, die du weitergeben kannst. Jeder Eintrag darin ist als nicht bestätigt markiert (`verify = true`), denn gehört wurde nur dein Gerät. Solange die Einrichtung, MIDI-Learn oder der Monitor läuft, öffnet OpenDJDecks auch MIDI-Eingänge, die kein Mapping kennt; auf manchen Systemen kann ein anderes Programm einen solchen Port dann nicht gleichzeitig benutzen. Die geführte Einrichtung wurde mit simulierten MIDI-Nachrichten getestet, noch nicht mit einem echten Controller.

## Am Controller stöbern

Drehe den Browse-Encoder, um durch die Liste zu gehen. **BACK** verschiebt den Fokus in die Listenspalte, wo Drehen von oben nach unten durch alles darin geht: die Sammlung, Playlists, die Ordner des Explorers, ähnliche Tracks, Sticks, Verläufe, Aufnahmen, die Tag-Liste, erstellte Tracks und AUTO SORT (mit Shift um acht). Die Spalte scrollt mit. Ein Druck oder **LOAD** an einem Deck geht eine Ebene hinein: Ein Ordner öffnet sich und sein erster Eintrag wird gewählt, eine Liste übergibt an ihre Tracks. **BACK** geht jedes Mal eine Ebene hinaus: von den Tracks zu ihrer Liste, von einem Eintrag zum Ordner, der ihn enthält, und ein offener Ordner schließt sich. In der Trackliste lädt ein Druck auf ein freies Deck, **LOAD** auf sein Deck, und Shift-Druck fügt der Tag-Liste hinzu. Ein Track, den du mit der Maus anklickst, ist der, den **LOAD** lädt. **VIEW** schaltet in Prepare den ganzen Browser und in Perform das Browse-Layout.

Verbindet sich ein Controller mit Kanalfadern, starten seine Kanäle geschlossen, weil die App nicht wissen kann, wo die Fader stehen, bis sie sich bewegen: Bewege jeden Fader einmal. **PLAY** auf einem Deck, dessen Fader sich noch nicht bewegt hat, sagt das in der Statuszeile.

## MIDI-Learn

Unter **Einstellungen › Controller › MIDI-Learn**:

1. Wähle eine **Aktion** und, falls sinnvoll, ein **Deck / Kanal**.
2. Drücke **LERNEN** und bewege dann das Bedienelement.
3. Es wird beschrieben und als Mapping-Zeile gezeigt. **ZU MEINEM MAPPING** schreibt es in deine eigene Mapping-Datei, die ab dem nächsten Start gilt.

**Eigene Mapping-Datei** lädt ein Mapping zusätzlich zu den eingebauten und hat für sein Gerät Vorrang. Das Format beschreibt `docs/controller-mappings.md`; Details in den [Entwickler-Docs](../../controller-mappings.md).

## MIDI-Monitor

Schalte **Monitor** an, um jede eingehende MIDI-Nachricht und ihre Wirkung zu sehen, zum Beispiel wenn du herausfinden willst, was ein Regler sendet.

## Selbsttest vor dem Gig

Unter **Einstellungen › Controller › Selbsttest** prüfst du einen Controller, bevor du dich auf ihn verlässt:

1. Wähle den Controller unter **Mapping**. Ein angeschlossener Controller ist schon gewählt.
2. Drücke **START** und bewege jedes Bedienelement einmal: jede Taste und jedes Pad drücken, jeden Regler drehen, jeden Fader und die Jogwheels bewegen. Jedes Element wird abgehakt, sobald seine Nachricht ankommt. Die Zahl zeigt, wie viele sich gemeldet haben.
3. Drücke **LEDS BLINKEN**. Jede LED aus dem Mapping leuchtet zwei Sekunden und zeigt danach wieder, was die Decks anzeigen. Bleibt eine LED dunkel, ist sie für dein Gerät nicht richtig zugeordnet.

Der Selbsttest führt die Elemente jedes Decks einzeln auf, darum ist seine Zahl größer als in der Liste unten.

![Der Controller-Selbsttest in den Einstellungen](../img/controller-self-test.png)

Bleibt ein Element nach dem Bewegen offen, kommt es nicht so bei OpenDJDecks an, wie das Mapping es erwartet. Schalte den **Monitor** an, um zu sehen, was es stattdessen sendet, und melde es mit den Monitor-Zeilen.

## Hardware-Status

Jedes zugeordnete Bedienelement hat einen von drei Zuständen:

- **bestätigt**: Jemand hat es mit OpenDJDecks auf einem echten Gerät bestätigt.
- **teilweise**: Die MIDI-Nachricht stammt aus einer öffentlichen Quelle, dem funktionierenden Mapping eines anderen Projekts, aber noch niemand hat sie mit OpenDJDecks auf einem Gerät ausprobiert.
- **ungetestet**: Es gibt noch keine öffentliche Quelle und keine Bestätigung (`verify = true` in der Mapping-Datei).

Einige Elemente des DDJ-FLX10 sind an einem Gerät bestätigt: Browsen und Laden, PLAY, Kopfhörer-CUE, der Kanalfader, der Jog-Teller und die Deck-Umschaltung. Die übrigen Prüfungen an echten Geräten stehen noch aus. Beim Vier-Deck-Mapping deckt die öffentliche Quelle verwandte Modelle derselben Familie ab. Die Liste unten wird aus den Mapping-Dateien erzeugt und ändert sich mit ihnen. Mit dem Selbsttest oben findest du es für dein eigenes Gerät heraus.

<!-- hardware-status:start (generated: cargo run -p od-controller --example hardware_status -- --write) -->

| Modell | Bedienelemente | Bestätigt | Teilweise | Ungetestet |
|---|---|---|---|---|
| DDJ-FLX10 | 473 | 10 | 451 | 12 |
| DDJ-FLX4 | 221 | 0 | 219 | 2 |

### DDJ-FLX10

| Bedienelement | Art | Status |
|---|---|---|
| Browse rotate | Eingabe | bestätigt |
| Browse rotate (shift) | Eingabe | bestätigt |
| Back | Eingabe | bestätigt |
| Back (shift) | Eingabe | teilweise |
| Browse view | Eingabe | teilweise |
| Browse view (long press) | Eingabe | teilweise |
| Browse view (shift) | Eingabe | teilweise |
| Browse press | Eingabe | teilweise |
| Browse press (shift, deck 1 active) | Eingabe | teilweise |
| Browse press (shift, deck 2 active) | Eingabe | teilweise |
| Browse press (shift, deck 3 active) | Eingabe | teilweise |
| Browse press (shift, deck 4 active) | Eingabe | teilweise |
| Deck select | Eingabe | bestätigt |
| Deck select button | Eingabe | teilweise |
| Deck select 1/3 (shift) | Eingabe | teilweise |
| Deck select 2/4 (shift) | Eingabe | teilweise |
| Play/Pause | Eingabe | bestätigt |
| Play/Pause (shift) | Eingabe | teilweise |
| Cue | Eingabe | teilweise |
| Cue (shift) | Eingabe | teilweise |
| Sync | Eingabe | teilweise |
| Sync (long press) | Eingabe | teilweise |
| Sync (shift) | Eingabe | teilweise |
| Key sync | Eingabe | teilweise |
| Key sync (long press) | Eingabe | teilweise |
| Key sync (shift) | Eingabe | teilweise |
| Tempo reset | Eingabe | teilweise |
| Tempo reset (shift) | Eingabe | teilweise |
| Shift | Eingabe | teilweise |
| Quantize | Eingabe | teilweise |
| Quantize (shift) | Eingabe | teilweise |
| Slip | Eingabe | teilweise |
| Slip (shift) | Eingabe | ungetestet |
| Slip reverse | Eingabe | teilweise |
| Slip reverse (shift) | Eingabe | teilweise |
| Beat jump back | Eingabe | teilweise |
| Beat jump forward | Eingabe | teilweise |
| Beat jump back (shift) | Eingabe | teilweise |
| Beat jump forward (shift) | Eingabe | teilweise |
| Search back (long press) | Eingabe | teilweise |
| Search forward (long press) | Eingabe | teilweise |
| Memory cue call back | Eingabe | teilweise |
| Memory cue call forward | Eingabe | teilweise |
| Memory cue call back (shift) | Eingabe | teilweise |
| Memory cue call forward (shift) | Eingabe | teilweise |
| Mix point select back | Eingabe | teilweise |
| Mix point select forward | Eingabe | teilweise |
| Mix point select back (shift) | Eingabe | teilweise |
| Mix point select forward (shift) | Eingabe | teilweise |
| Mix point link | Eingabe | teilweise |
| Mix point link (shift) | Eingabe | teilweise |
| Stem button DRUMS | Eingabe | teilweise |
| Stem button VOCAL | Eingabe | teilweise |
| Stem button INST | Eingabe | teilweise |
| Stem button DRUMS (shift, twice) | Eingabe | teilweise |
| Stem button VOCAL (shift, twice) | Eingabe | teilweise |
| Stem button INST (shift, twice) | Eingabe | teilweise |
| Tempo fader | Eingabe | ungetestet |
| Tempo fader (shift) | Eingabe | ungetestet |
| Jog touch | Eingabe | bestätigt |
| Jog touch (shift) | Eingabe | teilweise |
| Jog platter (vinyl on) | Eingabe | teilweise |
| Jog platter (vinyl off) | Eingabe | bestätigt |
| Jog side bend | Eingabe | bestätigt |
| Jog platter search (shift) | Eingabe | teilweise |
| Jog side search (shift) | Eingabe | ungetestet |
| Jog (4 beat jump held) | Eingabe | teilweise |
| Loop in | Eingabe | teilweise |
| Loop out | Eingabe | teilweise |
| Loop in (shift) | Eingabe | ungetestet |
| Loop out (shift) | Eingabe | ungetestet |
| 4 beat loop | Eingabe | teilweise |
| 4 beat / exit (shift) | Eingabe | teilweise |
| Trim | Eingabe | teilweise |
| EQ high | Eingabe | teilweise |
| EQ mid | Eingabe | teilweise |
| EQ low | Eingabe | teilweise |
| Part isolator vocals | Eingabe | ungetestet |
| Part isolator inst | Eingabe | ungetestet |
| Part isolator drums | Eingabe | ungetestet |
| Channel fader | Eingabe | bestätigt |
| CH FX knob | Eingabe | teilweise |
| Crossfader | Eingabe | teilweise |
| Master level | Eingabe | teilweise |
| Booth level | Eingabe | teilweise |
| Headphone mix | Eingabe | teilweise |
| Headphone level | Eingabe | teilweise |
| Headphone cue | Eingabe | bestätigt |
| Headphone cue (shift) | Eingabe | teilweise |
| Master cue | Eingabe | teilweise |
| Master cue (shift) | Eingabe | teilweise |
| Crossfader assign A | Eingabe | teilweise |
| Crossfader assign THRU | Eingabe | teilweise |
| Crossfader assign B | Eingabe | teilweise |
| Input select (PC-A / line-phono / PC-B) 1 | Eingabe | teilweise |
| Input select (PC-A / line-phono / PC-B) 2 | Eingabe | teilweise |
| Input select (PC-A / line-phono / PC-B) 3 | Eingabe | teilweise |
| Fader start (play) | Eingabe | teilweise |
| Fader start (sync) | Eingabe | teilweise |
| Fader start (cue) | Eingabe | teilweise |
| Line/phono (channel 3) | Eingabe | teilweise |
| Line/phono (channel 4) | Eingabe | teilweise |
| Mic EQ high | Eingabe | teilweise |
| Mic EQ low | Eingabe | teilweise |
| Mic switch (off / on / talkover) 1 | Eingabe | teilweise |
| Mic switch (off / on / talkover) 2 | Eingabe | teilweise |
| Mic switch (off / on / talkover) 3 | Eingabe | teilweise |
| CH FX select 1 | Eingabe | teilweise |
| CH FX select 2 | Eingabe | teilweise |
| CH FX select 3 | Eingabe | teilweise |
| CH FX select 4 | Eingabe | teilweise |
| CH FX select 5 | Eingabe | teilweise |
| CH FX select 6 | Eingabe | teilweise |
| CH FX select (shift) 1 | Eingabe | teilweise |
| CH FX select (shift) 2 | Eingabe | teilweise |
| CH FX select (shift) 3 | Eingabe | teilweise |
| CH FX select (shift) 4 | Eingabe | teilweise |
| CH FX select (shift) 5 | Eingabe | teilweise |
| CH FX select (shift) 6 | Eingabe | teilweise |
| FX part select 1 | Eingabe | teilweise |
| FX part select 2 | Eingabe | teilweise |
| FX part select 3 | Eingabe | teilweise |
| FX part select (shift) 1 | Eingabe | teilweise |
| FX part select (shift) 2 | Eingabe | teilweise |
| FX part select (shift) 3 | Eingabe | teilweise |
| Sampler volume | Eingabe | teilweise |
| Sampler cue | Eingabe | teilweise |
| Sampler cue (shift) | Eingabe | teilweise |
| Tempo FX beat down | Eingabe | teilweise |
| Tempo FX beat up | Eingabe | teilweise |
| Tempo FX auto beat (shift) | Eingabe | teilweise |
| Tempo FX tap (shift) | Eingabe | teilweise |
| Tempo FX select 1 | Eingabe | teilweise |
| Tempo FX select 2 | Eingabe | teilweise |
| Tempo FX select 3 | Eingabe | teilweise |
| Tempo FX select 4 | Eingabe | teilweise |
| Tempo FX select 5 | Eingabe | teilweise |
| Tempo FX select 6 | Eingabe | teilweise |
| Tempo FX select 7 | Eingabe | teilweise |
| Tempo FX select 8 | Eingabe | teilweise |
| Tempo FX select 9 | Eingabe | teilweise |
| Tempo FX select 10 | Eingabe | teilweise |
| Tempo FX select 11 | Eingabe | teilweise |
| Tempo FX select 12 | Eingabe | teilweise |
| Tempo FX select 13 | Eingabe | teilweise |
| Tempo FX select 14 | Eingabe | teilweise |
| Tempo FX channel 1 | Eingabe | teilweise |
| Tempo FX channel 2 | Eingabe | teilweise |
| Tempo FX channel 3 | Eingabe | teilweise |
| Tempo FX channel 4 | Eingabe | teilweise |
| Tempo FX channel master | Eingabe | teilweise |
| Tempo FX channel mic/sampler 1 | Eingabe | teilweise |
| Tempo FX channel mic/sampler 2 | Eingabe | teilweise |
| Tempo FX level | Eingabe | teilweise |
| Tempo FX on | Eingabe | teilweise |
| Tempo FX release (shift) | Eingabe | teilweise |
| Pad mode hot cue | Eingabe | teilweise |
| Pad mode hot cue (long press) | Eingabe | teilweise |
| Pad mode keyboard (shift) | Eingabe | teilweise |
| Pad mode hold FX 1 | Eingabe | teilweise |
| Pad mode hold FX 2 (shift) | Eingabe | teilweise |
| Pad mode beat jump | Eingabe | teilweise |
| Pad mode beat loop (shift) | Eingabe | teilweise |
| Pad mode sampler | Eingabe | teilweise |
| Pad mode key shift (shift) | Eingabe | teilweise |
| Pad page left 1 | Eingabe | teilweise |
| Pad page left 2 | Eingabe | teilweise |
| Pad page left 3 | Eingabe | teilweise |
| Pad page left 4 | Eingabe | teilweise |
| Pad page right 1 | Eingabe | teilweise |
| Pad page right 2 | Eingabe | teilweise |
| Pad page right 3 | Eingabe | teilweise |
| Pad page right 4 | Eingabe | teilweise |
| Sampler bank down | Eingabe | teilweise |
| Sampler bank up | Eingabe | teilweise |
| Pad page left (shift) 1 | Eingabe | teilweise |
| Pad page left (shift) 2 | Eingabe | teilweise |
| Pad page left (shift) 3 | Eingabe | teilweise |
| Pad page left (shift) 4 | Eingabe | teilweise |
| Pad page left (shift) 5 | Eingabe | teilweise |
| Pad page left (shift) 6 | Eingabe | teilweise |
| Pad page left (shift) 7 | Eingabe | teilweise |
| Pad page left (shift) 8 | Eingabe | teilweise |
| Pad page right (shift) | Eingabe | teilweise |
| Pad page right (shift) 1 | Eingabe | teilweise |
| Pad page right (shift) 2 | Eingabe | teilweise |
| Pad page right (shift) 3 | Eingabe | teilweise |
| Pad page right (shift) 4 | Eingabe | teilweise |
| Pad page right (shift) 5 | Eingabe | teilweise |
| Pad page right (shift) 6 | Eingabe | teilweise |
| Pad page right (shift, key shift mode) | Eingabe | teilweise |
| Hot cue pad 1 | Eingabe | teilweise |
| Hot cue pad 2 | Eingabe | teilweise |
| Hot cue pad 3 | Eingabe | teilweise |
| Hot cue pad 4 | Eingabe | teilweise |
| Hot cue pad 5 | Eingabe | teilweise |
| Hot cue pad 6 | Eingabe | teilweise |
| Hot cue pad 7 | Eingabe | teilweise |
| Hot cue pad 8 | Eingabe | teilweise |
| Hot cue pad (shift) 1 | Eingabe | teilweise |
| Hot cue pad (shift) 2 | Eingabe | teilweise |
| Hot cue pad (shift) 3 | Eingabe | teilweise |
| Hot cue pad (shift) 4 | Eingabe | teilweise |
| Hot cue pad (shift) 5 | Eingabe | teilweise |
| Hot cue pad (shift) 6 | Eingabe | teilweise |
| Hot cue pad (shift) 7 | Eingabe | teilweise |
| Hot cue pad (shift) 8 | Eingabe | teilweise |
| Hold FX 1 pad 1 | Eingabe | teilweise |
| Hold FX 1 pad 2 | Eingabe | teilweise |
| Hold FX 1 pad 3 | Eingabe | teilweise |
| Hold FX 1 pad 4 | Eingabe | teilweise |
| Hold FX 1 pad 5 | Eingabe | teilweise |
| Hold FX 1 pad 6 | Eingabe | teilweise |
| Hold FX 1 pad 7 | Eingabe | teilweise |
| Hold FX 1 pad 8 | Eingabe | teilweise |
| Beat jump pad 1 | Eingabe | teilweise |
| Beat jump pad 2 | Eingabe | teilweise |
| Beat jump pad 3 | Eingabe | teilweise |
| Beat jump pad 4 | Eingabe | teilweise |
| Beat jump pad 5 | Eingabe | teilweise |
| Beat jump pad 6 | Eingabe | teilweise |
| Beat jump pad 7 | Eingabe | teilweise |
| Beat jump pad 8 | Eingabe | teilweise |
| Sampler pad 1 | Eingabe | teilweise |
| Sampler pad 2 | Eingabe | teilweise |
| Sampler pad 3 | Eingabe | teilweise |
| Sampler pad 4 | Eingabe | teilweise |
| Sampler pad 5 | Eingabe | teilweise |
| Sampler pad 6 | Eingabe | teilweise |
| Sampler pad 7 | Eingabe | teilweise |
| Sampler pad 8 | Eingabe | teilweise |
| Sampler pad 9 | Eingabe | teilweise |
| Sampler pad 10 | Eingabe | teilweise |
| Sampler pad 11 | Eingabe | teilweise |
| Sampler pad 12 | Eingabe | teilweise |
| Sampler pad 13 | Eingabe | teilweise |
| Sampler pad 14 | Eingabe | teilweise |
| Sampler pad 15 | Eingabe | teilweise |
| Sampler pad 16 | Eingabe | teilweise |
| Sampler pad (shift) 1 | Eingabe | teilweise |
| Sampler pad (shift) 2 | Eingabe | teilweise |
| Sampler pad (shift) 3 | Eingabe | teilweise |
| Sampler pad (shift) 4 | Eingabe | teilweise |
| Sampler pad (shift) 5 | Eingabe | teilweise |
| Sampler pad (shift) 6 | Eingabe | teilweise |
| Sampler pad (shift) 7 | Eingabe | teilweise |
| Sampler pad (shift) 8 | Eingabe | teilweise |
| Sampler pad (shift) 9 | Eingabe | teilweise |
| Sampler pad (shift) 10 | Eingabe | teilweise |
| Sampler pad (shift) 11 | Eingabe | teilweise |
| Sampler pad (shift) 12 | Eingabe | teilweise |
| Sampler pad (shift) 13 | Eingabe | teilweise |
| Sampler pad (shift) 14 | Eingabe | teilweise |
| Sampler pad (shift) 15 | Eingabe | teilweise |
| Sampler pad (shift) 16 | Eingabe | teilweise |
| Keyboard pad 1 | Eingabe | teilweise |
| Keyboard pad 2 | Eingabe | teilweise |
| Keyboard pad 3 | Eingabe | teilweise |
| Keyboard pad 4 | Eingabe | teilweise |
| Keyboard pad 5 | Eingabe | teilweise |
| Keyboard pad 6 | Eingabe | teilweise |
| Keyboard pad 7 | Eingabe | teilweise |
| Keyboard pad 8 | Eingabe | teilweise |
| Hold FX 2 pad 1 | Eingabe | teilweise |
| Hold FX 2 pad 2 | Eingabe | teilweise |
| Hold FX 2 pad 3 | Eingabe | teilweise |
| Hold FX 2 pad 4 | Eingabe | teilweise |
| Hold FX 2 pad 5 | Eingabe | teilweise |
| Hold FX 2 pad 6 | Eingabe | teilweise |
| Hold FX 2 pad 7 | Eingabe | teilweise |
| Hold FX 2 pad 8 | Eingabe | teilweise |
| Beat loop pad 1 | Eingabe | teilweise |
| Beat loop pad 2 | Eingabe | teilweise |
| Beat loop pad 3 | Eingabe | teilweise |
| Beat loop pad 4 | Eingabe | teilweise |
| Beat loop pad 5 | Eingabe | teilweise |
| Beat loop pad 6 | Eingabe | teilweise |
| Beat loop pad 7 | Eingabe | teilweise |
| Beat loop pad 8 | Eingabe | teilweise |
| Key shift pad 1 | Eingabe | teilweise |
| Key shift pad 2 | Eingabe | teilweise |
| Key shift pad 3 | Eingabe | teilweise |
| Key shift pad 4 | Eingabe | teilweise |
| Key shift pad 5 | Eingabe | teilweise |
| Key shift pad 6 | Eingabe | teilweise |
| Key shift pad 7 | Eingabe | teilweise |
| Key shift pad 8 | Eingabe | teilweise |
| Hot cue pad (page 2) 1 | Eingabe | teilweise |
| Hot cue pad (page 2) 2 | Eingabe | teilweise |
| Hot cue pad (page 2) 3 | Eingabe | teilweise |
| Hot cue pad (page 2) 4 | Eingabe | teilweise |
| Hot cue pad (page 2) 5 | Eingabe | teilweise |
| Hot cue pad (page 2) 6 | Eingabe | teilweise |
| Hot cue pad (page 2) 7 | Eingabe | teilweise |
| Hot cue pad (page 2) 8 | Eingabe | teilweise |
| Hot cue pad (page 2, shift) 1 | Eingabe | teilweise |
| Hot cue pad (page 2, shift) 2 | Eingabe | teilweise |
| Hot cue pad (page 2, shift) 3 | Eingabe | teilweise |
| Hot cue pad (page 2, shift) 4 | Eingabe | teilweise |
| Hot cue pad (page 2, shift) 5 | Eingabe | teilweise |
| Hot cue pad (page 2, shift) 6 | Eingabe | teilweise |
| Hot cue pad (page 2, shift) 7 | Eingabe | teilweise |
| Hot cue pad (page 2, shift) 8 | Eingabe | teilweise |
| Hold FX 1 pad (page 2) 1 | Eingabe | teilweise |
| Hold FX 1 pad (page 2) 2 | Eingabe | teilweise |
| Hold FX 1 pad (page 2) 3 | Eingabe | teilweise |
| Hold FX 1 pad (page 2) 4 | Eingabe | teilweise |
| Hold FX 1 pad (page 2) 5 | Eingabe | teilweise |
| Hold FX 1 pad (page 2) 6 | Eingabe | teilweise |
| Hold FX 1 pad (page 2) 7 | Eingabe | teilweise |
| Hold FX 1 pad (page 2) 8 | Eingabe | teilweise |
| Beat jump pad (page 2) 1 | Eingabe | teilweise |
| Beat jump pad (page 2) 2 | Eingabe | teilweise |
| Beat jump pad (page 2) 3 | Eingabe | teilweise |
| Beat jump pad (page 2) 4 | Eingabe | teilweise |
| Beat jump pad (page 2) 5 | Eingabe | teilweise |
| Beat jump pad (page 2) 6 | Eingabe | teilweise |
| Beat jump pad (page 2) 7 | Eingabe | teilweise |
| Beat jump pad (page 2) 8 | Eingabe | teilweise |
| Keyboard pad (page 2) 1 | Eingabe | teilweise |
| Keyboard pad (page 2) 2 | Eingabe | teilweise |
| Keyboard pad (page 2) 3 | Eingabe | teilweise |
| Keyboard pad (page 2) 4 | Eingabe | teilweise |
| Keyboard pad (page 2) 5 | Eingabe | teilweise |
| Keyboard pad (page 2) 6 | Eingabe | teilweise |
| Keyboard pad (page 2) 7 | Eingabe | teilweise |
| Keyboard pad (page 2) 8 | Eingabe | teilweise |
| Hold FX 2 pad (page 2) 1 | Eingabe | teilweise |
| Hold FX 2 pad (page 2) 2 | Eingabe | teilweise |
| Hold FX 2 pad (page 2) 3 | Eingabe | teilweise |
| Hold FX 2 pad (page 2) 4 | Eingabe | teilweise |
| Hold FX 2 pad (page 2) 5 | Eingabe | teilweise |
| Hold FX 2 pad (page 2) 6 | Eingabe | teilweise |
| Hold FX 2 pad (page 2) 7 | Eingabe | teilweise |
| Hold FX 2 pad (page 2) 8 | Eingabe | teilweise |
| Beat loop pad (page 2) 1 | Eingabe | teilweise |
| Beat loop pad (page 2) 2 | Eingabe | teilweise |
| Beat loop pad (page 2) 3 | Eingabe | teilweise |
| Beat loop pad (page 2) 4 | Eingabe | teilweise |
| Beat loop pad (page 2) 5 | Eingabe | teilweise |
| Beat loop pad (page 2) 6 | Eingabe | teilweise |
| Beat loop pad (page 2) 7 | Eingabe | teilweise |
| Beat loop pad (page 2) 8 | Eingabe | teilweise |
| Key shift pad (page 2) 1 | Eingabe | teilweise |
| Key shift pad (page 2) 2 | Eingabe | teilweise |
| Key shift pad (page 2) 3 | Eingabe | teilweise |
| Key shift pad (page 2) 4 | Eingabe | teilweise |
| Key shift pad (page 2) 5 | Eingabe | teilweise |
| Key shift pad (page 2) 6 | Eingabe | teilweise |
| Key shift pad (page 2) 7 | Eingabe | teilweise |
| Key shift pad (page 2) 8 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 1 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 2 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 3 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 4 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 5 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 6 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 7 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 8 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 9 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 10 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 11 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 12 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 13 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 14 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 15 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 16 | Eingabe | teilweise |
| Beat jump pad (shift) 1 | Eingabe | teilweise |
| Beat jump pad (shift) 2 | Eingabe | teilweise |
| Beat jump pad (shift) 3 | Eingabe | teilweise |
| Beat jump pad (shift) 4 | Eingabe | teilweise |
| Beat jump pad (shift) 5 | Eingabe | teilweise |
| Beat jump pad (shift) 6 | Eingabe | teilweise |
| Beat jump pad (shift) 7 | Eingabe | teilweise |
| Beat jump pad (shift) 8 | Eingabe | teilweise |
| Beat jump pad (shift) 9 | Eingabe | teilweise |
| Beat jump pad (shift) 10 | Eingabe | teilweise |
| Beat jump pad (shift) 11 | Eingabe | teilweise |
| Beat jump pad (shift) 12 | Eingabe | teilweise |
| Beat jump pad (shift) 13 | Eingabe | teilweise |
| Beat jump pad (shift) 14 | Eingabe | teilweise |
| Beat jump pad (shift) 15 | Eingabe | teilweise |
| Beat jump pad (shift) 16 | Eingabe | teilweise |
| Keyboard pad (shift) 1 | Eingabe | teilweise |
| Keyboard pad (shift) 2 | Eingabe | teilweise |
| Keyboard pad (shift) 3 | Eingabe | teilweise |
| Keyboard pad (shift) 4 | Eingabe | teilweise |
| Keyboard pad (shift) 5 | Eingabe | teilweise |
| Keyboard pad (shift) 6 | Eingabe | teilweise |
| Keyboard pad (shift) 7 | Eingabe | teilweise |
| Keyboard pad (shift) 8 | Eingabe | teilweise |
| Keyboard pad (shift) 9 | Eingabe | teilweise |
| Keyboard pad (shift) 10 | Eingabe | teilweise |
| Keyboard pad (shift) 11 | Eingabe | teilweise |
| Keyboard pad (shift) 12 | Eingabe | teilweise |
| Keyboard pad (shift) 13 | Eingabe | teilweise |
| Keyboard pad (shift) 14 | Eingabe | teilweise |
| Keyboard pad (shift) 15 | Eingabe | teilweise |
| Keyboard pad (shift) 16 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 1 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 2 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 3 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 4 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 5 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 6 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 7 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 8 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 9 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 10 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 11 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 12 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 13 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 14 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 15 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 16 | Eingabe | teilweise |
| Beat loop pad (shift) 1 | Eingabe | teilweise |
| Beat loop pad (shift) 2 | Eingabe | teilweise |
| Beat loop pad (shift) 3 | Eingabe | teilweise |
| Beat loop pad (shift) 4 | Eingabe | teilweise |
| Beat loop pad (shift) 5 | Eingabe | teilweise |
| Beat loop pad (shift) 6 | Eingabe | teilweise |
| Beat loop pad (shift) 7 | Eingabe | teilweise |
| Beat loop pad (shift) 8 | Eingabe | teilweise |
| Beat loop pad (shift) 9 | Eingabe | teilweise |
| Beat loop pad (shift) 10 | Eingabe | teilweise |
| Beat loop pad (shift) 11 | Eingabe | teilweise |
| Beat loop pad (shift) 12 | Eingabe | teilweise |
| Beat loop pad (shift) 13 | Eingabe | teilweise |
| Beat loop pad (shift) 14 | Eingabe | teilweise |
| Beat loop pad (shift) 15 | Eingabe | teilweise |
| Beat loop pad (shift) 16 | Eingabe | teilweise |
| Key shift pad (shift) 1 | Eingabe | teilweise |
| Key shift pad (shift) 2 | Eingabe | teilweise |
| Key shift pad (shift) 3 | Eingabe | teilweise |
| Key shift pad (shift) 4 | Eingabe | teilweise |
| Key shift pad (shift) 5 | Eingabe | teilweise |
| Key shift pad (shift) 6 | Eingabe | teilweise |
| Key shift pad (shift) 7 | Eingabe | teilweise |
| Key shift pad (shift) 8 | Eingabe | teilweise |
| Key shift pad (shift) 9 | Eingabe | teilweise |
| Key shift pad (shift) 10 | Eingabe | teilweise |
| Key shift pad (shift) 11 | Eingabe | teilweise |
| Key shift pad (shift) 12 | Eingabe | teilweise |
| Key shift pad (shift) 13 | Eingabe | teilweise |
| Key shift pad (shift) 14 | Eingabe | teilweise |
| Key shift pad (shift) 15 | Eingabe | teilweise |
| Key shift pad (shift) 16 | Eingabe | teilweise |
| deck play | Anzeige | teilweise |
| deck cue | Anzeige | teilweise |
| deck sync | Anzeige | teilweise |
| deck master | Anzeige | teilweise |
| deck key sync | Anzeige | teilweise |
| deck keylock | Anzeige | teilweise |
| deck tempo reset | Anzeige | teilweise |
| deck tempo range | Anzeige | teilweise |
| deck quantize | Anzeige | teilweise |
| deck slip | Anzeige | teilweise |
| deck vinyl | Anzeige | ungetestet |
| deck censor | Anzeige | teilweise |
| deck reverse | Anzeige | teilweise |
| deck loop in | Anzeige | teilweise |
| deck loop out | Anzeige | teilweise |
| deck loop active | Anzeige | teilweise |
| mixer cue | Anzeige | teilweise |
| mixer vu | Anzeige | teilweise |
| deck loaded | Anzeige | ungetestet |
| deck pad mode | Anzeige | teilweise |
| deck pad | Anzeige | teilweise |
| master cue | Anzeige | ungetestet |
| channel fx | Anzeige | teilweise |
| tempo fx on | Anzeige | teilweise |
| tempo fx release | Anzeige | teilweise |
| sampler cue | Anzeige | teilweise |

### DDJ-FLX4

| Bedienelement | Art | Status |
|---|---|---|
| Browse rotate | Eingabe | teilweise |
| Browse rotate (shift) | Eingabe | teilweise |
| Browse press | Eingabe | teilweise |
| Browse press (shift) | Eingabe | teilweise |
| Load deck 1 | Eingabe | teilweise |
| Load deck 2 | Eingabe | teilweise |
| Load deck 1 (shift) | Eingabe | teilweise |
| Load deck 2 (shift) | Eingabe | teilweise |
| Play/Pause | Eingabe | teilweise |
| Play/Pause (shift) | Eingabe | teilweise |
| Cue | Eingabe | teilweise |
| Cue (shift) | Eingabe | teilweise |
| Sync | Eingabe | teilweise |
| Sync (long press) | Eingabe | teilweise |
| Sync (shift) | Eingabe | teilweise |
| Shift | Eingabe | teilweise |
| Tempo fader | Eingabe | ungetestet |
| Jog touch | Eingabe | teilweise |
| Jog touch (shift) | Eingabe | teilweise |
| Jog scratch | Eingabe | teilweise |
| Jog top bend | Eingabe | teilweise |
| Jog side bend | Eingabe | teilweise |
| Jog search (shift) | Eingabe | teilweise |
| Loop in | Eingabe | teilweise |
| Loop out | Eingabe | teilweise |
| 4 beat loop | Eingabe | teilweise |
| 4 beat loop (shift) | Eingabe | teilweise |
| Loop in (shift) | Eingabe | teilweise |
| Loop out (shift) | Eingabe | teilweise |
| Call left | Eingabe | teilweise |
| Call right | Eingabe | teilweise |
| Call left (shift) | Eingabe | teilweise |
| Call right (shift) | Eingabe | teilweise |
| Trim | Eingabe | teilweise |
| EQ high | Eingabe | teilweise |
| EQ mid | Eingabe | teilweise |
| EQ low | Eingabe | teilweise |
| Channel fader | Eingabe | teilweise |
| CH FX knob ch1 | Eingabe | teilweise |
| CH FX knob ch2 | Eingabe | teilweise |
| Crossfader | Eingabe | teilweise |
| Master level | Eingabe | teilweise |
| Headphone mix | Eingabe | teilweise |
| Headphone level | Eingabe | teilweise |
| Headphone cue | Eingabe | teilweise |
| Headphone cue (shift) | Eingabe | teilweise |
| Auto Channel FX | Eingabe | teilweise |
| Auto Channel FX select | Eingabe | teilweise |
| Auto fader | Eingabe | teilweise |
| Master cue | Eingabe | teilweise |
| Master cue (shift) | Eingabe | teilweise |
| Auto fader (shift) | Eingabe | teilweise |
| Mic level | Eingabe | teilweise |
| Android mono/stereo | Eingabe | teilweise |
| Fader start (play) | Eingabe | teilweise |
| Fader start (cue) | Eingabe | teilweise |
| Tempo FX lever CH1 | Eingabe | teilweise |
| Tempo FX lever CH2 | Eingabe | teilweise |
| Tempo FX beat down | Eingabe | teilweise |
| Tempo FX beat up | Eingabe | teilweise |
| Tempo FX auto beat (shift) | Eingabe | teilweise |
| Tempo FX tap (shift) | Eingabe | teilweise |
| Tempo FX select | Eingabe | teilweise |
| Tempo FX select (shift) | Eingabe | teilweise |
| Tempo FX level | Eingabe | teilweise |
| Tempo FX level (lever CH2) | Eingabe | ungetestet |
| Tempo FX on | Eingabe | teilweise |
| Tempo FX on (lever CH2) | Eingabe | teilweise |
| Tempo FX release (shift) | Eingabe | teilweise |
| Tempo FX release (shift, lever CH2) | Eingabe | teilweise |
| Pad mode hot cue | Eingabe | teilweise |
| Pad mode keyboard (shift) | Eingabe | teilweise |
| Pad mode hold FX 1 | Eingabe | teilweise |
| Pad mode hold FX 2 (shift) | Eingabe | teilweise |
| Pad mode beat jump | Eingabe | teilweise |
| Pad mode beat loop (shift) | Eingabe | teilweise |
| Pad mode sampler | Eingabe | teilweise |
| Pad mode key shift (shift) | Eingabe | teilweise |
| Hot cue pad 1 | Eingabe | teilweise |
| Hot cue pad 2 | Eingabe | teilweise |
| Hot cue pad 3 | Eingabe | teilweise |
| Hot cue pad 4 | Eingabe | teilweise |
| Hot cue pad 5 | Eingabe | teilweise |
| Hot cue pad 6 | Eingabe | teilweise |
| Hot cue pad 7 | Eingabe | teilweise |
| Hot cue pad 8 | Eingabe | teilweise |
| Hot cue pad (shift) 1 | Eingabe | teilweise |
| Hot cue pad (shift) 2 | Eingabe | teilweise |
| Hot cue pad (shift) 3 | Eingabe | teilweise |
| Hot cue pad (shift) 4 | Eingabe | teilweise |
| Hot cue pad (shift) 5 | Eingabe | teilweise |
| Hot cue pad (shift) 6 | Eingabe | teilweise |
| Hot cue pad (shift) 7 | Eingabe | teilweise |
| Hot cue pad (shift) 8 | Eingabe | teilweise |
| Hold FX 1 pad 1 | Eingabe | teilweise |
| Hold FX 1 pad 2 | Eingabe | teilweise |
| Hold FX 1 pad 3 | Eingabe | teilweise |
| Hold FX 1 pad 4 | Eingabe | teilweise |
| Hold FX 1 pad 5 | Eingabe | teilweise |
| Hold FX 1 pad 6 | Eingabe | teilweise |
| Hold FX 1 pad 7 | Eingabe | teilweise |
| Hold FX 1 pad 8 | Eingabe | teilweise |
| Beat jump pad 1 | Eingabe | teilweise |
| Beat jump pad 2 | Eingabe | teilweise |
| Beat jump pad 3 | Eingabe | teilweise |
| Beat jump pad 4 | Eingabe | teilweise |
| Beat jump pad 5 | Eingabe | teilweise |
| Beat jump pad 6 | Eingabe | teilweise |
| Beat jump pad 7 | Eingabe | teilweise |
| Beat jump pad 8 | Eingabe | teilweise |
| Sampler pad 1 | Eingabe | teilweise |
| Sampler pad 2 | Eingabe | teilweise |
| Sampler pad 3 | Eingabe | teilweise |
| Sampler pad 4 | Eingabe | teilweise |
| Sampler pad 5 | Eingabe | teilweise |
| Sampler pad 6 | Eingabe | teilweise |
| Sampler pad 7 | Eingabe | teilweise |
| Sampler pad 8 | Eingabe | teilweise |
| Sampler pad (shift) 1 | Eingabe | teilweise |
| Sampler pad (shift) 2 | Eingabe | teilweise |
| Sampler pad (shift) 3 | Eingabe | teilweise |
| Sampler pad (shift) 4 | Eingabe | teilweise |
| Sampler pad (shift) 5 | Eingabe | teilweise |
| Sampler pad (shift) 6 | Eingabe | teilweise |
| Sampler pad (shift) 7 | Eingabe | teilweise |
| Sampler pad (shift) 8 | Eingabe | teilweise |
| Keyboard pad 1 | Eingabe | teilweise |
| Keyboard pad 2 | Eingabe | teilweise |
| Keyboard pad 3 | Eingabe | teilweise |
| Keyboard pad 4 | Eingabe | teilweise |
| Keyboard pad 5 | Eingabe | teilweise |
| Keyboard pad 6 | Eingabe | teilweise |
| Keyboard pad 7 | Eingabe | teilweise |
| Keyboard pad 8 | Eingabe | teilweise |
| Hold FX 2 pad 1 | Eingabe | teilweise |
| Hold FX 2 pad 2 | Eingabe | teilweise |
| Hold FX 2 pad 3 | Eingabe | teilweise |
| Hold FX 2 pad 4 | Eingabe | teilweise |
| Hold FX 2 pad 5 | Eingabe | teilweise |
| Hold FX 2 pad 6 | Eingabe | teilweise |
| Hold FX 2 pad 7 | Eingabe | teilweise |
| Hold FX 2 pad 8 | Eingabe | teilweise |
| Beat loop pad 1 | Eingabe | teilweise |
| Beat loop pad 2 | Eingabe | teilweise |
| Beat loop pad 3 | Eingabe | teilweise |
| Beat loop pad 4 | Eingabe | teilweise |
| Beat loop pad 5 | Eingabe | teilweise |
| Beat loop pad 6 | Eingabe | teilweise |
| Beat loop pad 7 | Eingabe | teilweise |
| Beat loop pad 8 | Eingabe | teilweise |
| Key shift pad 1 | Eingabe | teilweise |
| Key shift pad 2 | Eingabe | teilweise |
| Key shift pad 3 | Eingabe | teilweise |
| Key shift pad 4 | Eingabe | teilweise |
| Key shift pad 5 | Eingabe | teilweise |
| Key shift pad 6 | Eingabe | teilweise |
| Key shift pad 7 | Eingabe | teilweise |
| Key shift pad 8 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 1 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 2 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 3 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 4 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 5 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 6 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 7 | Eingabe | teilweise |
| Hold FX 1 pad (shift) 8 | Eingabe | teilweise |
| Beat jump pad (shift) 1 | Eingabe | teilweise |
| Beat jump pad (shift) 2 | Eingabe | teilweise |
| Beat jump pad (shift) 3 | Eingabe | teilweise |
| Beat jump pad (shift) 4 | Eingabe | teilweise |
| Beat jump pad (shift) 5 | Eingabe | teilweise |
| Beat jump pad (shift) 6 | Eingabe | teilweise |
| Beat jump pad (shift) 7 | Eingabe | teilweise |
| Beat jump pad (shift) 8 | Eingabe | teilweise |
| Keyboard pad (shift) 1 | Eingabe | teilweise |
| Keyboard pad (shift) 2 | Eingabe | teilweise |
| Keyboard pad (shift) 3 | Eingabe | teilweise |
| Keyboard pad (shift) 4 | Eingabe | teilweise |
| Keyboard pad (shift) 5 | Eingabe | teilweise |
| Keyboard pad (shift) 6 | Eingabe | teilweise |
| Keyboard pad (shift) 7 | Eingabe | teilweise |
| Keyboard pad (shift) 8 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 1 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 2 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 3 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 4 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 5 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 6 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 7 | Eingabe | teilweise |
| Hold FX 2 pad (shift) 8 | Eingabe | teilweise |
| Beat loop pad (shift) 1 | Eingabe | teilweise |
| Beat loop pad (shift) 2 | Eingabe | teilweise |
| Beat loop pad (shift) 3 | Eingabe | teilweise |
| Beat loop pad (shift) 4 | Eingabe | teilweise |
| Beat loop pad (shift) 5 | Eingabe | teilweise |
| Beat loop pad (shift) 6 | Eingabe | teilweise |
| Beat loop pad (shift) 7 | Eingabe | teilweise |
| Beat loop pad (shift) 8 | Eingabe | teilweise |
| Key shift pad (shift) 1 | Eingabe | teilweise |
| Key shift pad (shift) 2 | Eingabe | teilweise |
| Key shift pad (shift) 3 | Eingabe | teilweise |
| Key shift pad (shift) 4 | Eingabe | teilweise |
| Key shift pad (shift) 5 | Eingabe | teilweise |
| Key shift pad (shift) 6 | Eingabe | teilweise |
| Key shift pad (shift) 7 | Eingabe | teilweise |
| Key shift pad (shift) 8 | Eingabe | teilweise |
| deck play | Anzeige | teilweise |
| deck cue | Anzeige | teilweise |
| deck loop in | Anzeige | teilweise |
| deck loop out | Anzeige | teilweise |
| mixer cue | Anzeige | teilweise |
| deck sync | Anzeige | teilweise |
| deck tempo range | Anzeige | teilweise |
| deck loop active | Anzeige | teilweise |
| deck vinyl | Anzeige | teilweise |
| mixer vu | Anzeige | teilweise |
| deck loaded | Anzeige | teilweise |
| deck pad mode | Anzeige | teilweise |
| deck pad | Anzeige | teilweise |
| tempo fx on | Anzeige | teilweise |
| channel fx | Anzeige | teilweise |
<!-- hardware-status:end -->
