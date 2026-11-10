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

Welche Controller OpenDJDecks kennt und wie weit jeder bestätigt ist. Ein Modell gilt erst dann als **bestätigt**, wenn jedes seiner Bedienelemente mit OpenDJDecks auf einem echten Gerät bestätigt wurde. Das ist noch bei keinem der Fall, darum steht in jeder Zeile **nicht bestätigt**. Hersteller- und Modellnamen sagen nur, mit welcher Hardware ein Mapping kompatibel sein soll.

<!-- controller-list:start (generated: cargo run -p od-controller --example hardware_status -- --write) -->

| Controller | Decks | Mapping | Erkannt an | Status |
|---|---|---|---|---|
| kompatibel mit DDJ-FLX10 | 4 | eingebaut | MIDI-Portname `DDJ-FLX10` | nicht bestätigt: 0 bestätigt, 125 teilweise, 135 ungetestet |
| kompatibel mit DDJ-FLX4 | 2 | eingebaut | MIDI-Portname `DDJ-FLX4` | nicht bestätigt: 0 bestätigt, 126 teilweise, 47 ungetestet |
| Jeder Einsteiger-Controller mit zwei Decks | 2 | geführte Einrichtung, Vorlage `two-deck-entry` (39 Bedienelemente) | der Port des eingerichteten Geräts | nicht bestätigt: deine eigene Aufnahme, kein anderes Gerät |
| Jeder Battle-Controller mit zwei Decks | 2 | geführte Einrichtung, Vorlage `two-deck-battle` (34 Bedienelemente) | der Port des eingerichteten Geräts | nicht bestätigt: deine eigene Aufnahme, kein anderes Gerät |
| Jeder Controller mit vier Decks | 4 | geführte Einrichtung, Vorlage `four-deck` (73 Bedienelemente) | der Port des eingerichteten Geräts | nicht bestätigt: deine eigene Aufnahme, kein anderes Gerät |
<!-- controller-list:end -->

Jeder andere Controller lässt sich mit der geführten Einrichtung unten einrichten, ausgehend von einer von drei allgemeinen Vorlagen. Weitere Modell-Mappings, auch für Einsteiger- und Battle-Controller, kommen von Besitzern, die ihr eigenes Gerät mit der geführten Einrichtung aufnehmen und die Datei weitergeben. Keines davon ist schon für andere Geräte bestätigt. OpenDJDecks leitet keine Mappings aus Herstellerhandbüchern, MIDI-Tabellen, Herstellersoftware oder Mapping-Dateien anderer Projekte ab.

## Geführte Einrichtung für andere Controller

![Die Controller-Einrichtung unter der oberen Leiste fragt nach dem Tempo-Fader von Deck 1](../img/controller-setup.png)

Für einen Controller ohne Mapping drückst du **CONTROLLER EINRICHTEN** unter **Einstellungen › Controller** (oder **Ansicht › Controller einrichten**). Unter der oberen Leiste öffnet sich ein Band:

1. Schließe den Controller per USB an und wähle, was ihm am nächsten kommt: **2 DECKS · EINSTEIGER**, **2 DECKS · BATTLE** oder **4 DECKS**. Drücke **START**.
2. Das Band fragt nach einem Bedienelement nach dem anderen: PLAY, CUE und SYNC, das Jogwheel (berühren, dann drehen), Tempo-Fader und LOAD jedes Decks, dann die Hot-Cue-Pads, die Kanalfader, TRIM und die EQ-Regler, Kopfhörer-CUE, den Crossfader, den Browse-Regler sowie Master- und Kopfhörerpegel. Das passende Element auf dem Bildschirm bekommt den Rahmen der Anleitung.
3. Bewege das Element. **Empfangen** zeigt, was ankam; sobald das Element einen Moment ruht, ist der Schritt übernommen und der nächste kommt. Ein Element, das ein früherer Schritt schon benutzt, wird mit der Nummer dieses Schritts abgelehnt. **ÜBERSPRINGEN** lässt weg, was dein Controller nicht hat, **ZURÜCK** geht einen Schritt zurück, um ihn neu zu lernen, **NOCHMAL** vergisst, was ankam.
4. Am Ende gibst du dem Controller einen Namen und drückst **MAPPING SPEICHERN**. Die Datei landet neben deiner Bibliothek in `mappings/`, wird deine eigene Mapping-Datei und gilt ab dem nächsten Start. Die Battle-Vorlage stellt außerdem die Crossfader-Kurve auf **CUT**.

Die Datei ist eine gewöhnliche Mapping-Datei, die du weitergeben kannst. Jeder Eintrag darin ist als nicht bestätigt markiert (`verify = true`), denn gehört wurde nur dein Gerät. Solange die Einrichtung, MIDI-Learn oder der Monitor läuft, öffnet OpenDJDecks auch MIDI-Eingänge, die kein Mapping kennt; auf manchen Systemen kann ein anderes Programm einen solchen Port dann nicht gleichzeitig benutzen. Die geführte Einrichtung wurde mit simulierten MIDI-Nachrichten getestet, noch nicht mit einem echten Controller.

## Am Controller stöbern

Drehe den Browse-Encoder, um durch die Liste zu gehen. **BACK** verschiebt den Fokus in den Baum, wo Drehen durch die Listen geht (mit Shift um acht) und ein Druck einen Ordner öffnet oder in die Trackliste wechselt. In der Liste lädt ein Druck auf ein freies Deck, Shift-Druck fügt der Tag-Liste hinzu. **VIEW** schaltet in Prepare den ganzen Browser und in Perform das Browse-Layout.

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

Noch kein Element ist bestätigt: Die Prüfungen an echten Geräten stehen noch aus. Beim Vier-Deck-Mapping deckt die öffentliche Quelle verwandte Modelle derselben Familie ab. Die Liste unten wird aus den Mapping-Dateien erzeugt und ändert sich mit ihnen. Mit dem Selbsttest oben findest du es für dein eigenes Gerät heraus.

<!-- hardware-status:start (generated: cargo run -p od-controller --example hardware_status -- --write) -->

| Modell | Bedienelemente | Bestätigt | Teilweise | Ungetestet |
|---|---|---|---|---|
| DDJ-FLX10 | 260 | 0 | 125 | 135 |
| DDJ-FLX4 | 173 | 0 | 126 | 47 |

### DDJ-FLX10

| Bedienelement | Art | Status |
|---|---|---|
| Browse rotate | Eingabe | ungetestet |
| Browse rotate (shift) | Eingabe | ungetestet |
| Browse press | Eingabe | ungetestet |
| Back | Eingabe | teilweise |
| Back (shift) | Eingabe | ungetestet |
| Browse view | Eingabe | teilweise |
| Browse view (long press) | Eingabe | ungetestet |
| Browse view (shift) | Eingabe | ungetestet |
| Load deck 1 | Eingabe | teilweise |
| Load deck 2 | Eingabe | teilweise |
| Load deck 3 | Eingabe | teilweise |
| Load deck 4 | Eingabe | teilweise |
| Deck select | Eingabe | ungetestet |
| Play/Pause | Eingabe | teilweise |
| Play/Pause (shift) | Eingabe | ungetestet |
| Cue | Eingabe | teilweise |
| Cue (shift) | Eingabe | ungetestet |
| Sync | Eingabe | teilweise |
| Sync (long press) | Eingabe | ungetestet |
| Sync (shift) | Eingabe | ungetestet |
| Key sync | Eingabe | teilweise |
| Key sync (long press) | Eingabe | teilweise |
| Key sync (shift) | Eingabe | teilweise |
| Tempo reset | Eingabe | teilweise |
| Tempo reset (shift) | Eingabe | teilweise |
| Shift | Eingabe | teilweise |
| Quantize | Eingabe | teilweise |
| Quantize (shift) | Eingabe | ungetestet |
| Slip | Eingabe | teilweise |
| Slip (shift) | Eingabe | ungetestet |
| Slip reverse | Eingabe | teilweise |
| Slip reverse (shift) | Eingabe | ungetestet |
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
| Mix point / part buttons | Eingabe | teilweise |
| Mix point / part buttons 1 | Eingabe | teilweise |
| Mix point / part buttons 2 | Eingabe | teilweise |
| Mix point / part buttons 3 | Eingabe | teilweise |
| Tempo fader | Eingabe | ungetestet |
| Tempo fader (shift) | Eingabe | ungetestet |
| Jog touch | Eingabe | ungetestet |
| Jog touch (shift) | Eingabe | ungetestet |
| Jog scratch | Eingabe | ungetestet |
| Jog top bend | Eingabe | ungetestet |
| Jog side bend | Eingabe | ungetestet |
| Jog search (shift) | Eingabe | ungetestet |
| Loop in | Eingabe | teilweise |
| Loop in (loop active) | Eingabe | ungetestet |
| Loop out | Eingabe | teilweise |
| Loop out (loop active) | Eingabe | ungetestet |
| Loop out (shift) | Eingabe | teilweise |
| 4 beat loop | Eingabe | teilweise |
| 4 beat loop (shift) | Eingabe | ungetestet |
| Trim | Eingabe | teilweise |
| EQ high | Eingabe | teilweise |
| EQ mid | Eingabe | teilweise |
| EQ low | Eingabe | teilweise |
| Part isolator vocals | Eingabe | ungetestet |
| Part isolator inst | Eingabe | ungetestet |
| Part isolator drums | Eingabe | ungetestet |
| Channel fader | Eingabe | ungetestet |
| CH FX knob | Eingabe | teilweise |
| Crossfader | Eingabe | ungetestet |
| Master level | Eingabe | ungetestet |
| Booth level | Eingabe | ungetestet |
| Headphone mix | Eingabe | ungetestet |
| Headphone level | Eingabe | ungetestet |
| Headphone cue | Eingabe | teilweise |
| Headphone cue (shift) | Eingabe | ungetestet |
| Master cue | Eingabe | ungetestet |
| Master cue (shift) | Eingabe | ungetestet |
| Crossfader assign A | Eingabe | ungetestet |
| Crossfader assign THRU | Eingabe | ungetestet |
| Crossfader assign B | Eingabe | ungetestet |
| Input select / fader start 1 | Eingabe | ungetestet |
| Input select / fader start 2 | Eingabe | ungetestet |
| Input select / fader start 3 | Eingabe | ungetestet |
| Fader start (play) | Eingabe | teilweise |
| Fader start (cue) | Eingabe | ungetestet |
| CH FX select 1 | Eingabe | ungetestet |
| CH FX select 2 | Eingabe | ungetestet |
| CH FX select 3 | Eingabe | ungetestet |
| CH FX select 4 | Eingabe | ungetestet |
| CH FX select 5 | Eingabe | ungetestet |
| CH FX select 6 | Eingabe | ungetestet |
| CH FX select (shift) 1 | Eingabe | ungetestet |
| CH FX select (shift) 2 | Eingabe | ungetestet |
| CH FX select (shift) 3 | Eingabe | ungetestet |
| CH FX select (shift) 4 | Eingabe | ungetestet |
| CH FX select (shift) 5 | Eingabe | ungetestet |
| CH FX select (shift) 6 | Eingabe | ungetestet |
| FX part select 1 | Eingabe | teilweise |
| FX part select 2 | Eingabe | teilweise |
| FX part select 3 | Eingabe | teilweise |
| FX part select 4 | Eingabe | teilweise |
| FX part select 5 | Eingabe | teilweise |
| FX part select 6 | Eingabe | teilweise |
| Sampler volume | Eingabe | teilweise |
| Sampler cue | Eingabe | ungetestet |
| Tempo FX beat down | Eingabe | ungetestet |
| Tempo FX beat up | Eingabe | ungetestet |
| Tempo FX auto beat (shift) | Eingabe | ungetestet |
| Tempo FX tap (shift) | Eingabe | ungetestet |
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
| Tempo FX on | Eingabe | ungetestet |
| Tempo FX release (shift) | Eingabe | ungetestet |
| Pad mode hot cue | Eingabe | ungetestet |
| Pad mode keyboard (shift) | Eingabe | teilweise |
| Pad mode hold FX 1 | Eingabe | ungetestet |
| Pad mode hold FX 2 (shift) | Eingabe | ungetestet |
| Pad mode beat jump | Eingabe | ungetestet |
| Pad mode beat loop (shift) | Eingabe | ungetestet |
| Pad mode sampler | Eingabe | teilweise |
| Pad mode key shift (shift) | Eingabe | ungetestet |
| Pad page left 1 | Eingabe | ungetestet |
| Pad page left 2 | Eingabe | ungetestet |
| Pad page left 3 | Eingabe | ungetestet |
| Pad page left 4 | Eingabe | ungetestet |
| Pad page right 1 | Eingabe | ungetestet |
| Pad page right 2 | Eingabe | ungetestet |
| Pad page right 3 | Eingabe | ungetestet |
| Pad page right 4 | Eingabe | ungetestet |
| Sampler bank down | Eingabe | ungetestet |
| Sampler bank up | Eingabe | ungetestet |
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
| Beat jump pad 1 | Eingabe | ungetestet |
| Beat jump pad 2 | Eingabe | ungetestet |
| Beat jump pad 3 | Eingabe | ungetestet |
| Beat jump pad 4 | Eingabe | ungetestet |
| Beat jump pad 5 | Eingabe | ungetestet |
| Beat jump pad 6 | Eingabe | ungetestet |
| Beat jump pad 7 | Eingabe | ungetestet |
| Beat jump pad 8 | Eingabe | ungetestet |
| Sampler pad 1 | Eingabe | ungetestet |
| Sampler pad 2 | Eingabe | ungetestet |
| Sampler pad 3 | Eingabe | ungetestet |
| Sampler pad 4 | Eingabe | ungetestet |
| Sampler pad 5 | Eingabe | ungetestet |
| Sampler pad 6 | Eingabe | ungetestet |
| Sampler pad 7 | Eingabe | ungetestet |
| Sampler pad 8 | Eingabe | ungetestet |
| Sampler pad (shift) 1 | Eingabe | ungetestet |
| Sampler pad (shift) 2 | Eingabe | ungetestet |
| Sampler pad (shift) 3 | Eingabe | ungetestet |
| Sampler pad (shift) 4 | Eingabe | ungetestet |
| Sampler pad (shift) 5 | Eingabe | ungetestet |
| Sampler pad (shift) 6 | Eingabe | ungetestet |
| Sampler pad (shift) 7 | Eingabe | ungetestet |
| Sampler pad (shift) 8 | Eingabe | ungetestet |
| Keyboard pad 1 | Eingabe | ungetestet |
| Keyboard pad 2 | Eingabe | ungetestet |
| Keyboard pad 3 | Eingabe | ungetestet |
| Keyboard pad 4 | Eingabe | ungetestet |
| Keyboard pad 5 | Eingabe | ungetestet |
| Keyboard pad 6 | Eingabe | ungetestet |
| Keyboard pad 7 | Eingabe | ungetestet |
| Keyboard pad 8 | Eingabe | ungetestet |
| Hold FX 2 pad 1 | Eingabe | teilweise |
| Hold FX 2 pad 2 | Eingabe | teilweise |
| Hold FX 2 pad 3 | Eingabe | teilweise |
| Hold FX 2 pad 4 | Eingabe | teilweise |
| Hold FX 2 pad 5 | Eingabe | teilweise |
| Hold FX 2 pad 6 | Eingabe | teilweise |
| Hold FX 2 pad 7 | Eingabe | teilweise |
| Hold FX 2 pad 8 | Eingabe | teilweise |
| Beat loop pad 1 | Eingabe | ungetestet |
| Beat loop pad 2 | Eingabe | ungetestet |
| Beat loop pad 3 | Eingabe | ungetestet |
| Beat loop pad 4 | Eingabe | ungetestet |
| Beat loop pad 5 | Eingabe | ungetestet |
| Beat loop pad 6 | Eingabe | ungetestet |
| Beat loop pad 7 | Eingabe | ungetestet |
| Beat loop pad 8 | Eingabe | ungetestet |
| Key shift pad 1 | Eingabe | ungetestet |
| Key shift pad 2 | Eingabe | ungetestet |
| Key shift pad 3 | Eingabe | ungetestet |
| Key shift pad 4 | Eingabe | ungetestet |
| Key shift pad 5 | Eingabe | ungetestet |
| Key shift pad 6 | Eingabe | ungetestet |
| Key shift pad 7 | Eingabe | ungetestet |
| Key shift pad 8 | Eingabe | ungetestet |
| deck play | Anzeige | teilweise |
| deck cue | Anzeige | teilweise |
| deck sync | Anzeige | teilweise |
| deck master | Anzeige | ungetestet |
| deck key sync | Anzeige | teilweise |
| deck keylock | Anzeige | teilweise |
| deck tempo reset | Anzeige | teilweise |
| deck tempo range | Anzeige | teilweise |
| deck quantize | Anzeige | teilweise |
| deck slip | Anzeige | teilweise |
| deck vinyl | Anzeige | ungetestet |
| deck censor | Anzeige | teilweise |
| deck reverse | Anzeige | ungetestet |
| deck loop in | Anzeige | teilweise |
| deck loop out | Anzeige | teilweise |
| deck reloop | Anzeige | teilweise |
| deck loop active | Anzeige | teilweise |
| mixer cue | Anzeige | teilweise |
| mixer vu | Anzeige | ungetestet |
| deck loaded | Anzeige | ungetestet |
| deck pad mode | Anzeige | teilweise |
| deck pad | Anzeige | teilweise |
| master cue | Anzeige | ungetestet |
| channel fx | Anzeige | teilweise |
| tempo fx on | Anzeige | teilweise |
| tempo fx release | Anzeige | ungetestet |
| sampler cue | Anzeige | ungetestet |

### DDJ-FLX4

| Bedienelement | Art | Status |
|---|---|---|
| Browse rotate | Eingabe | ungetestet |
| Browse rotate (shift) | Eingabe | ungetestet |
| Browse press | Eingabe | teilweise |
| Browse press (shift) | Eingabe | teilweise |
| Load deck 1 | Eingabe | teilweise |
| Load deck 2 | Eingabe | teilweise |
| Load deck 1 (shift) | Eingabe | ungetestet |
| Load deck 2 (shift) | Eingabe | ungetestet |
| Play/Pause | Eingabe | teilweise |
| Play/Pause (shift) | Eingabe | ungetestet |
| Cue | Eingabe | teilweise |
| Cue (shift) | Eingabe | teilweise |
| Sync | Eingabe | teilweise |
| Sync (long press) | Eingabe | teilweise |
| Sync (shift) | Eingabe | teilweise |
| Shift | Eingabe | teilweise |
| Tempo fader | Eingabe | ungetestet |
| Tempo fader (shift) | Eingabe | ungetestet |
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
| Call left (loop) | Eingabe | ungetestet |
| Call left | Eingabe | ungetestet |
| Call right (loop) | Eingabe | ungetestet |
| Call right | Eingabe | ungetestet |
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
| Master level | Eingabe | ungetestet |
| Headphone mix | Eingabe | ungetestet |
| Headphone level | Eingabe | ungetestet |
| Headphone cue | Eingabe | teilweise |
| Headphone cue (shift) | Eingabe | teilweise |
| Auto Channel FX | Eingabe | ungetestet |
| Auto Channel FX select | Eingabe | ungetestet |
| Auto fader | Eingabe | ungetestet |
| Master cue (shift) | Eingabe | ungetestet |
| Fader start (play) | Eingabe | ungetestet |
| Fader start (cue) | Eingabe | ungetestet |
| Tempo FX lever CH1 | Eingabe | ungetestet |
| Tempo FX lever CH2 | Eingabe | ungetestet |
| Tempo FX beat down | Eingabe | teilweise |
| Tempo FX beat down (lever CH2) | Eingabe | ungetestet |
| Tempo FX beat up | Eingabe | teilweise |
| Tempo FX beat up (lever CH2) | Eingabe | ungetestet |
| Tempo FX auto beat (shift) | Eingabe | ungetestet |
| Tempo FX tap (shift) | Eingabe | ungetestet |
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
| Hold FX 1 pad 1 | Eingabe | ungetestet |
| Hold FX 1 pad 2 | Eingabe | ungetestet |
| Hold FX 1 pad 3 | Eingabe | ungetestet |
| Hold FX 1 pad 4 | Eingabe | ungetestet |
| Hold FX 1 pad 5 | Eingabe | ungetestet |
| Hold FX 1 pad 6 | Eingabe | ungetestet |
| Hold FX 1 pad 7 | Eingabe | ungetestet |
| Hold FX 1 pad 8 | Eingabe | ungetestet |
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
| Hold FX 2 pad 1 | Eingabe | ungetestet |
| Hold FX 2 pad 2 | Eingabe | ungetestet |
| Hold FX 2 pad 3 | Eingabe | ungetestet |
| Hold FX 2 pad 4 | Eingabe | ungetestet |
| Hold FX 2 pad 5 | Eingabe | ungetestet |
| Hold FX 2 pad 6 | Eingabe | ungetestet |
| Hold FX 2 pad 7 | Eingabe | ungetestet |
| Hold FX 2 pad 8 | Eingabe | ungetestet |
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
| deck play | Anzeige | teilweise |
| deck cue | Anzeige | teilweise |
| deck sync | Anzeige | teilweise |
| deck tempo range | Anzeige | teilweise |
| deck loop in | Anzeige | teilweise |
| deck loop out | Anzeige | teilweise |
| deck loop active | Anzeige | teilweise |
| deck vinyl | Anzeige | ungetestet |
| mixer cue | Anzeige | teilweise |
| mixer vu | Anzeige | ungetestet |
| deck loaded | Anzeige | ungetestet |
| deck pad mode | Anzeige | teilweise |
| deck pad | Anzeige | teilweise |
| tempo fx on | Anzeige | teilweise |
| tempo fx release | Anzeige | teilweise |
| channel fx | Anzeige | ungetestet |
<!-- hardware-status:end -->
