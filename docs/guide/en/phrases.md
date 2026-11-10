---
title: Phrases
description: Intro, chorus, up, down, outro. Phrases are detected automatically; see them on every waveform and correct them when you need to.
keywords: [phrase, phrases, phrasing, drop, breakdown, phrase start, phrase sync, blend, transition, incoming track, overlay, structure, intro, chorus, outro, up, down, verse, sections, track structure, edit phrases]
features: [analysis-phrase, analysis-phrase-edit, deck-phrase-display, mix-phrase-ahead, mix-phrase-start, mix-phrase-sync, mix-incoming-overlay]
---

Phrases are the sections of a track: intro, chorus, up, down, outro. The analysis finds them for you and draws them as a coloured bar under the overview in Prepare and on every deck, so you can see at a glance what comes next.

![The phrase bar and the PHRASE tab of the Prepare player](../img/phrases.png)

## Correct a phrase

Open the track in Prepare and pick the **PHRASE** tab.

- **Change the type:** click a phrase on the bar and choose a type (or press `⇧P`). In a gap, this creates a new phrase.
- **Move a boundary:** drag the edge between two phrases; it snaps to bars. **BOUNDARY TO CURSOR** (`⇧B`) moves the nearest boundary to the cursor.
- **Split a phrase:** **CUT** (`⇧X`) at the cursor, on the nearest bar.
- **Join two phrases:** **MERGE** (`⇧J`).
- **CLEAR** removes all phrases of the track.

A new analysis of the track replaces your edits. Every change is one undo step and reaches your sticks.

## How phrases are shown

Under **Preferences › View** you switch the phrases on the enlarged waveforms and the bar under the overviews separately, and choose **NAMES** or **COLOURS ONLY** for the types. The platter shows the current phrase with a countdown; see [Perform](perform-basics.md#deck-readouts).

## Mix on the phrase

![Deck 2 waits for deck 1's next phrase; its outline is drawn over deck 1's waveform from that phrase on](../img/phrase-blend.png)

- **Bars to the drop and the break:** under the tempo on each platter, **DROP 8 · BREAK 24** counts whole bars to the next drop (a chorus after something else) and the next breakdown (a down or bridge phrase), nearest first. The bars to the next phrase stay on the platter's top line and on the waveform.
- **Start on the next phrase:** cue the incoming track, then **right-click PLAY**. The key gets the armed outline and the platter counts **START -bars.beats**; the deck starts when the master deck reaches its next phrase (the next 16-bar mark if it has no phrases). Right-click again, **PLAY** or **CUE** cancel. On a controller: `deck.phrase_start`.
- **Phrase-snapped sync:** under **Preferences › Controller › Decks › Sync** choose **PHRASE**. When you press **SYNC** on a playing deck, it also jumps whole beats so it stands as far before one of its phrases as the master deck before its next one. Beat sync keeps the phase as before.
- **The incoming track over the playing one:** the master's lane draws the other deck's waveform as an outline in that deck's colour, on the same time scale. While it plays you see where its beats fall; while it is cued, the outline starts at its cue, placed on the master's next phrase, with a mark carrying the deck number. **Preferences › View › Incoming track** switches it off.

The start lands within one audio block of the phrase boundary (a few milliseconds) in tests with generated tracks. It has not been tried at a gig or on a hardware controller yet, and it can only be as right as the phrases and the grid: check them in Prepare first.
