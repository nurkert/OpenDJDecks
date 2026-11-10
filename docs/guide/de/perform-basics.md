---
title: Die Grundlagen von Perform
description: Decks, Layouts, Sync, Keylock, Tempo, Loops und Beat Jump und jeder Pad-Modus am Deck.
keywords: [perform, deck, decks, play, cue, sync, beat sync, master, keylock, tonart halten, master tempo, tempo, tempofader, pitch, bend, nudge, jog, jogwheel, vinyl, scratch, slip, quantisieren, quantize, beat jump, pads, pad modus, hot cue pad, key shift, key sync, slicer, vier decks, 4 decks, layout, vertikal, browse, wellenform, zoom]
features: [deck-count-layers, deck-play-cue, deck-jog, deck-tempo-fader, deck-key-lock, deck-key-shift, deck-key-sync, deck-beat-sync, deck-quantize, deck-slip, deck-reverse, deck-censor, deck-beat-jump, deck-beat-jump-size, deck-needle-search, deck-search-buttons, pad-mode-hot-cue, pad-mode-beat-loop, pad-mode-beat-jump, pad-mode-keyboard, pad-mode-key-shift, pad-mode-slicer, pad-mode-memory-cue, pad-mode-timed-censor, pad-led-feedback, deck-waveform-zoom, deck-waveform-overview-detail, deck-header-readouts, deck-platter-readout, deck-waveform-zone, shell-perform-layouts, perform-view-menu, shell-panel-toggles, shell-splitters]
---

Perform ist der Live-Modus: Decks, Mixer und Browser in einer Ansicht.

![Perform mit zwei Decks, Mixer und Browser](../img/perform.png)

## Laden und spielen

Doppelklicke einen Track im Browser, ziehe ihn auf ein Deck oder nutze `⇧←` und `⇧→`. **Play** und **CUE** funktionieren wie an einem Player: CUE im Stillstand setzt den Cue-Punkt, CUE während der Wiedergabe springt zurück, gehaltenes CUE spielt, bis du loslässt.

## Deck-Anzeigen

Der Deck-Kopf zeigt Nummer, Cover, Titel und Artist, **BPM**, Rest- oder verstrichene Zeit, **TONART** und den Sync-Zustand. Der Platter darunter zeigt das Tempo, den Taktzähler und die aktuelle Phrase mit Countdown.

Die schmale Wellenform in jedem Deck ist der ganze Track mit Hot Cues und Phrasen. Ein Klick springt dorthin (Needle Search); am Controller tut das der Touch Strip.

Über den Decks zeigt der breite Bereich eine scrollende Wellenform pro Deck. Das Mausrad über einer Spur zoomt dieses Deck, Doppelklick setzt zurück, und die Zoom-Tasten links gelten für alle Decks.

## Layout und Ansichten

Der Ansichts-Knopf oben in Perform öffnet ein Menü mit einem Bild jeder Ansicht: **2 Decks** oder **4 Decks**, jeweils **horizontal** oder **vertikal**, und **Browse**, das die Decks klein hält und dem Browser den Platz gibt. Kurzbefehle: `⌥⌘V` für vertikale Wellenformen und `⌥⌘B` für Browse. Ein Vierdeck-Controller schaltet automatisch auf vier Decks.

Die obere Leiste zeigt oder verbirgt außerdem die Panels: **FX**, **VORSCHAU**, **SAMPLER**, **MIXER** und **REC**, dazu **CLUB** und **LICHTLEISTE**. Ziehe die Trennbalken zwischen den Bereichen, um die Größe zu ändern; Doppelklick setzt zurück.

## Tempo und Sync

- Der **Tempo-Fader** ändert die Geschwindigkeit; oben ist langsamer, wie am Player. Die Bereichstaste über CUE schaltet durch **±6 %**, **±10 %**, **±16 %** und **WIDE**. Der Standard kommt aus **Einstellungen › Controller › Decks**, und im Feld auf dem Platter kannst du eine BPM eintippen.
- **SYNC** koppelt ein Deck an das **MASTER**-Deck, nach Tempo und Taktposition. `⇧Q` und `⇧Y` machen Deck 1 und 2 zum Master. Unter **Einstellungen › Controller › Decks › Sync** wählst du Beat-Sync oder nur Tempo.
- **Q** (Quantisieren) lässt Cues, Loops und Sprünge auf dem Beat landen.
- **KEY SYNC** bringt das Deck in die Tonart des Master-Decks.
- **‹ ±0 ›** im Deck-Kopf verschiebt die Tonart um je einen Halbton; Klick auf den Wert setzt zurück.
- **KEYLOCK** hält die Tonart, wenn sich das Tempo ändert (auch bekannt als Master Tempo).
- Bend per Tastatur: `W` / `E` und `U` / `I`, solange gehalten.

## Jogwheels

Am Controller scratcht, bendet oder sucht das Jogwheel. Der Vinyl-Modus wird pro Deck und unter **Einstellungen › Controller › Decks › Jogwheel** geschaltet. Der Platter auf dem Bildschirm zeigt, wo du bist. Auf den Player-Screens suchen ◀◀ und ▶▶, solange gedrückt.

## Slip und Censor

**SLIP** lässt dich loopen, cuen oder scratchen, während der Track im Hintergrund weiterläuft; lässt du los, ist er dort, wo er wäre.

**Censor** gibt es am Controller und als Pad-Modus **CENSOR** (siehe unten): Das Deck spielt mit Slip rückwärts und läuft dann dort weiter, wo es wäre. Ein Controller kann ein Deck auch rückwärts spielen.

## Loops und Beat Jump

**LOOP** startet einen Loop der angezeigten Länge, **½** und **2×** ändern sie; **IN** / **OUT** / **EXIT** machen einen manuellen. Mehr in [Cues und Loops](cues-and-loops.md).

**−** und **+** (mit der Sprungweite auf der Taste) springen um Beats; die Weite stellst du im Pad-Modus **BEAT JUMP** ein, von 1/8 bis 64 Beats.

## Pads und Pad-Modi

Die sechzehn Pads links an jedem Deck ändern mit dem Modus ihre Bedeutung. Wähle einen Modus mit **‹ ›** neben seinem Namen, oder klicke den Namen für die Liste. `⇧⌘D` und `⇧⌘S` schalten bei Deck 1 durch, `⇧⌘;` und `⇧⌘K` bei Deck 2. Am Controller leuchten die Pad-Modus-Tasten für den aktuellen Modus; Pads leuchten bei Aktivität, dunkler wenn belegt.

| Modus | Die Pads |
|---|---|
| **HOT CUE** | Hot Cues A bis H anspringen oder setzen; Bank mit ‹ › |
| **BEAT LOOP** | So viele Beats loopen; erneut drücken beendet |
| **BEAT JUMP** | Um so viele Beats springen; ‹ › stellt die Weite |
| **SAMPLER** | Sampler-Slots spielen: ungerade Decks Pads 1–8, gerade Decks 9–16 |
| **HOLD FX** | Beat- und Release-Effekte, solange gehalten; zwei Seiten mit ‹ › |
| **KEYBOARD** | Den gewählten Hot Cue bei −3 bis +4 Halbtönen spielen |
| **KEY SHIFT** | Die Tonartverschiebung von −3 bis +4 setzen |
| **SLICER** | Einen von acht Slices spielen, solange gehalten; Bereich 4 bis 64 Beats |
| **STEMS** | Die getrennten Stems stummschalten und formen, siehe [Stems](stems.md) |
| **MEMORY CUE** | Die Memory Cues des Tracks anspringen |
| **CENSOR** | ¼ bis 32 Beats zensieren, dann dort weiterlaufen, wo es wäre |
| **PATTERN** | Sampler-Sequenzen speichern und abrufen |
| **LIGHTING** | A–F Looks, G Strobe, H Flash, siehe [Lichtshow](light-show.md) |

Die **GRID**-Taste neben dem Modus tauscht die Pads gegen das Grid- und Gain-Panel; siehe [Beatgrids](beatgrid.md#ein-grid-korrigieren-waehrend-der-track-laeuft).
