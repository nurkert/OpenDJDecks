---
title: Sampler and sequencer
description: Sixteen slots in four banks, loops and one-shots, and a step sequencer that stays locked to the beat.
keywords: [sampler, sample, samples, one shot, loop, gate, bank, slots, pads, sequencer, sequence, pattern, drum machine, pattern recall, quantize, sample sync]
features: [sampler-slots, sampler-load-library, sampler-banks, sampler-volume-cue, sampler-sequencer, sampler-panel, sampler-modes-sync, pad-mode-sampler, pad-mode-pattern-recall]
---

Press **SAMPLER** in the top bar (`⌥⌘S`) to open the panel between the mixer and the browser.

![The sampler panel with slots, the selected slot and the sequencer row](../img/sampler.png)

## Load samples

Select a track in the browser and press **LOAD SELECTED** on a slot, or drag a track onto a pad. Slots are kept with your library and come back when the audio starts.

There are four banks of 16 slots; **BANK** switches them (`⇧R` and `⇧T`).

## Play

Click a pad, or press the keys `X C V B M , . /` for slots 1 to 8. **STOP ALL** (`⇧P`) stops everything.

Select a slot to set its mode:

- **1-SHOT** plays to the end once.
- **LOOP** repeats until stopped.
- **GATE** plays only while the pad is held.

**QUANT** starts pads on the next beat of the master, **SYNC** follows the master tempo (with **÷2** and **×2** to correct a slot that was detected at the wrong speed). **GAIN** is per slot, **VOL** is the whole sampler. **CUE** sends it to the headphones.

From the deck, the **SAMPLER** pad mode plays slots: odd decks pads 1 to 8, even decks 9 to 16.

## Sequencer

The row at the bottom of the panel draws the pattern of the selected slot in 16 steps. Click steps to set them. **PLAY** (`⇧O`) starts it, locked to the master beat, and **REC** records pads into the pattern while it runs. **CLR** clears the selected slot's pattern.

**SAVE** stores the patterns of all slots on the next free pad of the **PATTERN** pad mode. In that mode, a stored pad recalls the patterns and starts the sequencer; **Shift** plus the pad clears it.
