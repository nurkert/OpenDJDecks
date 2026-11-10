---
title: Phrasen
description: Intro, Chorus, Up, Down, Outro. Phrasen werden automatisch erkannt; du siehst sie auf jeder Wellenform und korrigierst sie bei Bedarf.
keywords: [phrase, phrasen, drop, breakdown, phrasenstart, phrasen sync, übergang, blend, kommender track, overlay, struktur, intro, chorus, outro, up, down, verse, abschnitte, track struktur, phrasen bearbeiten]
features: [analysis-phrase, analysis-phrase-edit, deck-phrase-display, mix-phrase-ahead, mix-phrase-start, mix-phrase-sync, mix-incoming-overlay]
---

Phrasen sind die Abschnitte eines Tracks: Intro, Chorus, Up, Down, Outro. Die Analyse findet sie für dich und zeichnet sie als farbigen Balken unter die Übersicht in Prepare und auf jedem Deck, damit du auf einen Blick siehst, was kommt.

![Die Phrasenleiste und der Tab PHRASE im Prepare-Player](../img/phrases.png)

## Eine Phrase korrigieren

Öffne den Track in Prepare und wähle den Tab **PHRASE**.

- **Typ ändern:** Klicke eine Phrase auf der Leiste an und wähle einen Typ (oder `⇧P`). In einer Lücke entsteht eine neue Phrase.
- **Grenze verschieben:** Ziehe die Kante zwischen zwei Phrasen; sie rastet auf Takten ein. **GRENZE ZUM CURSOR** (`⇧B`) verschiebt die nächste Grenze zum Cursor.
- **Phrase teilen:** **TEILEN** (`⇧X`) am Cursor, auf dem nächsten Takt.
- **Zwei Phrasen verbinden:** **VERBINDEN** (`⇧J`).
- **LÖSCHEN** entfernt alle Phrasen des Tracks.

Eine neue Analyse ersetzt deine Änderungen. Jede Änderung ist ein Rückgängig-Schritt und erreicht deine Sticks.

## Wie Phrasen angezeigt werden

Unter **Einstellungen › Ansicht** schaltest du Phrasen auf den vergrößerten Wellenformen und die Leiste unter den Übersichten getrennt, und wählst **NAMEN** oder **NUR FARBEN** für die Typen. Der Platter zeigt die aktuelle Phrase mit Countdown; siehe [Perform](perform-basics.md#deck-anzeigen).

## Auf der Phrase mixen

![Deck 2 wartet auf die nächste Phrase von Deck 1; seine Kontur liegt ab dieser Phrase über der Wellenform von Deck 1](../img/phrase-blend.png)

- **Takte bis zum Drop und Break:** Unter dem Tempo auf jedem Platter zählt **DROP 8 · BREAK 24** ganze Takte bis zum nächsten Drop (ein Chorus nach etwas anderem) und zum nächsten Breakdown (eine Down- oder Bridge-Phrase), das nähere zuerst. Die Takte bis zur nächsten Phrase stehen weiter oben auf dem Platter und auf der Wellenform.
- **Auf der nächsten Phrase starten:** Cue den kommenden Track und **klicke rechts auf PLAY**. Die Taste bekommt den Bereit-Rahmen, der Platter zählt **START -Takte.Beats**; das Deck startet, wenn das Master-Deck seine nächste Phrase erreicht (ohne Phrasen die nächste 16-Takt-Marke). Ein zweiter Rechtsklick, **PLAY** oder **CUE** brechen ab. Am Controller: `deck.phrase_start`.
- **Sync auf die Phrase:** Unter **Einstellungen › Controller › Decks › Sync** wählst du **PHRASE**. Drückst du **SYNC** an einem laufenden Deck, springt es zusätzlich um ganze Beats, sodass es so weit vor einer seiner Phrasen steht wie das Master-Deck vor seiner nächsten. Beat-Sync hält die Phase wie bisher.
- **Der kommende Track über dem laufenden:** Die Spur des Masters zeichnet die Wellenform des anderen Decks als Kontur in dessen Deckfarbe, im selben Zeitmaßstab. Läuft es, siehst du, wo seine Beats liegen; ist es gecuet, beginnt die Kontur an seinem Cue, gesetzt auf die nächste Phrase des Masters, mit einer Marke mit der Decknummer. **Einstellungen › Ansicht › Kommender Track** schaltet sie aus.

Der Start landet in Tests mit erzeugten Tracks innerhalb eines Audioblocks (wenige Millisekunden) an der Phrasengrenze. Auf einem Gig oder mit einem Hardware-Controller ist das noch nicht ausprobiert, und es ist nur so richtig wie die Phrasen und das Grid: Prüfe sie vorher in Prepare.
