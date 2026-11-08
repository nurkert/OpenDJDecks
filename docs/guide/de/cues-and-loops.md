---
title: Hot Cues, Memory Cues und Loops
description: Markiere die Stellen, die du spielst. Acht Hot Cues pro Bank, Memory Cues, gespeicherte und aktive Loops, Namen und Farben, und Cues wie bei deinen anderen Tracks.
keywords: [hot cue, hotcue, hot cues, memory cue, memory, cue punkt, cue punkte, cue setzen, loop, loops, gespeicherter loop, aktiver loop, springen, marker, banken, name, farbe, auto cue]
features: [cues-hot-cues-prepare, cues-hot-cues-perform, cues-hot-cue-banks, cues-memory-add, cues-memory-edit, cues-memory-call, cues-memory-controller, cues-saved-loops, cues-auto-personalised, cues-active-loop, cues-colours, cues-labels, cues-auto-cue, deck-manual-loop, deck-auto-loop, deck-loop-roll]
---

Öffne einen Track im Player oben in Prepare, um Cues zu setzen. Alles, was du hier setzt, wird mit dem Track gespeichert und erreicht deine [USB-Sticks](usb-sticks.md) von selbst.

![Der Prepare-Player mit Hot Cues, Cue-Liste und Memory Cues](../img/cues.png)

## Hot Cues

Acht Pads, **A** bis **H**, springen sofort an eine Stelle im Track.

1. Bewege den Cursor an die Stelle: Klick auf die Wellenform oder Pfeiltasten (`←` `→` fein, `⌥←` `⌥→` um zwei Sekunden).
2. Klicke ein leeres Pad oder drücke `1` bis `8`. Mit **Q** (Quantisieren) rastet der Cue auf dem Beat ein.
3. Ein gesetztes Pad springt dorthin. In Prepare löschen `⇧⌘1` bis `⇧⌘8` die Pads.

In Perform funktionieren die Pads genauso und werden sofort gespeichert: am Deck oder am Controller ist Setzen und Löschen je ein Rückgängig-Schritt. Halte einen gesetzten Hot Cue im Pausenzustand gedrückt, um ihn vorzuhören; ein Rechtsklick löscht.

Bei mehr als acht Stellen schaltest du am Deck die Bank um: **A–H**, **I–P**, **Q–X** und **Y–AF** mit **‹ ›** im **HOT CUE**-Pad-Modus oder per Tastatur.

## Memory Cues

Memory Cues sind Marker, die du ohne Pad durchschalten kannst.

1. Drücke **MEMORY +** (oder `M`) am Cursor.
2. **‹** und **›** schalten durch, oder `R` und `T`.
3. **ENTF** (oder `⇧M`) entfernt den Memory Cue am Cursor.

In Perform rufen die Tasten für vorherigen / nächsten Memory Cue und dieselben Controller-Knöpfe sie auf; der Pad-Modus Memory Cue zeigt sie auf den Pads. Setzen und Löschen vom Controller ändert den Track mit.

## Die Cue-Liste

Die Liste rechts im Player hat die Tabs **ALLE**, **MEMORY** und **HOT CUE**. Wähle einen Cue aus, um ihn zu bearbeiten:

- Doppelklick, **Name** eintippen, Return. Namen stehen auf den Pads.
- Wähle eine der acht **Farben**.
- **ZUM CURSOR** verschiebt ihn zum Cursor, auf den Beat; Loops behalten ihre Länge.
- **→ HOT** macht aus einem Memory Cue einen Hot Cue auf dem ersten freien Pad.
- Die Löschtaste entfernt ihn.

## Loops

Im Player-Tab **CUE / LOOP** und am Perform-Deck:

- **LOOP** startet einen Auto-Loop der angezeigten Länge; ein zweiter Druck verlässt ihn. **½** und **2×** ändern die Länge von 1/32 bis 32 Beats.
- **IN** und **OUT** setzen einen manuellen Loop; **EXIT** verlässt ihn. In Perform wird er mit demselben Knopf neu gestartet.
- Ein Controller kann einen kurzen Loop rollen, der den Track darunter weiterlaufen lässt, wenn du loslässt.

Drücke **MEMORY +**, während ein Loop läuft, um ihn als Memory-Loop zu speichern, oder setze einen Hot Cue auf einen Loop. **AKTIV** im Cue-Editor macht aus einem gespeicherten Loop einen aktiven: Er greift von selbst, wenn die Wiedergabe ihn erreicht.

## Cues wie bei deinen anderen Tracks

Zeilenmenü: **Cues wie bei meinen anderen setzen**. OpenDJDecks schaut, wo du auf vorbereiteten Tracks Hot Cues und Memory Cues gesetzt hast, gemessen in Takten ab dem ersten Beat, den Phrasen und dem Ende, und füllt freie Pads der gewählten Tracks mit denselben Namen, Farben und Loop-Längen. Dafür braucht es mindestens drei analysierte Tracks mit Cues.

Unter **Einstellungen › Analyse › Cue-Punkte** passiert das auf Wunsch bei neu analysierten Tracks ohne Cues. Standardmäßig ist das aus. Jedes Setzen ist ein Rückgängig-Schritt.

## Auto Cue beim Laden

Ein geladener Track wartet an seinem ersten Ton, oder am ersten Memory Cue, falls er einen hat. Abschalten kannst du das unter **Einstellungen › Controller › Decks › Auto Cue**.
