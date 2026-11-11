---
title: Preparing a set
description: A set is a playlist with a plan. See the flow of tempo, key and energy, add transition notes, keep backup tracks ready and plan the start time.
keywords: [set, sets, set list, setlist, plan, planning, flow, transition, transitions, notes, backup, backups, order, energy, key, tempo, harmonic]
features: [prepare-preview-shift, browser-sets-flow, browser-set-flow-chart, browser-set-backups, shell-prepare-layouts]
---

A set is an ordered list with extras for playing it live: it shows how tempo, key and energy move from track to track, and you can leave yourself notes for each transition.

![A set as a chart of energy and tempo, with the transitions marked and two warnings named under it](../img/set-flow.png)

## Build a set

1. Press **+ SET** under the tree (or `⇧⌘N`) and give it a name by clicking the title.
2. Select tracks in **All tracks** and press **+** next to the set, or drag them over.
3. Drag rows inside the list to change the order. Use the column **#** to keep it in play order.

## Read the flow

**SET FLOW** is a chart of the set from left to right, one column per track:

- the **bar** is the track's energy, scaled to the set so the steps show;
- the **line** with a dot on each track is the tempo, with the lowest and highest BPM of the set on the axis at the left;
- under each column stand the key, the BPM, the position and the time it starts.

Between two tracks a marker shows how the transition fits:

- **solid**: it fits;
- **dashed**: worth rehearsing, a tempo step of more than 3 % or a key change that lifts or lowers the energy;
- **half**: tempo or key is not known yet;
- **struck** in red with a red dot at the top: a warning.

A transition warns when the tempo jumps by more than 6 % (double and half time count as the same tempo), when the keys clash (the same rule as the key line on the decks, see [Perform basics](perform-basics.md)), or when the energy drops or jumps steeply. Each warning is listed under the chart with its reason, for example `2 → 3` **TEMPO +9.1 %** or `5 → 6` **KEY CLASH 12B → 6B**; **ENERGY DROP** and **ENERGY JUMP** name the energy steps. Without warnings the line under the chart says so. Hold the pointer over a marker to read the whole transition in a sentence.

Click a warning or a marker to select the transition: both tracks are selected in the list, so you can hear them in turn, and the transition note belongs to it. Click a column to select one position. Notes, backups and the track you hear all relate to the selected position.

## Notes and backups

- Type in **TRANSITION NOTE** and press Return to keep a note for that transition.
- **PLANNED START**, for example `23:30`, adds real clock times to the flow.
- **BACKUPS FROM TAG LIST** turns the tracks of the tag list into backups for the selected position, in case the crowd goes elsewhere. The button next to it clears them.

## Find the right order

- Sort or read the **MATCH** column while a candidate plays, or open **SIMILAR** for suggestions; see [Library](library.md#the-match-column).
- Hot cues and memory cues you set while preparing appear on the stick and in Perform automatically.

## Hear a track at another tempo or key (from version 0.6)

Before you commit to a transition you can listen to a track as if it were pitched: in the Prepare player open the **TEMPO / KEY** tab.

- The **TEMPO** arrows change the speed of the preview (in percent); the key stays the same.
- The **KEY** arrows move it up or down by semitones.
- Next to the arrows you can read the tempo and key the preview comes to, and **RESET** brings it back to the original.

This is only for listening: the stored track, its beatgrid and its analysis do not change, and when you load it on a deck it plays as usual. The shift ends by itself as soon as you preview another track.

## Layouts for preparing

The layout button at the top of Prepare cycles through **PLAYER** (one player above the browser), **2 PLAYERS** (a full player and a compact second one), **DUAL** (two compact players side by side), **SIMPLE** (one compact player) and **FULL BROWSER**. `⌥⌘4` to `⌥⌘8` choose one directly. Player 2 takes the selected track with **LOAD** or a dragged row; it is handy for checking a transition.
