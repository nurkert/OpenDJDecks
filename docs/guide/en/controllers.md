---
title: Controllers and MIDI learn
description: Plug in a compatible controller and play. For anything else, map controls with MIDI learn and see what your controller sends in the MIDI monitor.
keywords: [controller, controllers, midi, midi learn, mapping, ddj, ddj-flx10, ddj-flx4, flx10, flx4, pioneer, led, soft takeover, browse, encoder, midi monitor, hardware]
features: [ctrl-led-feedback, ctrl-soft-takeover, ctrl-midi-learn-monitor, ctrl-mapping-four-deck, ctrl-mapping-two-deck, browser-controller-browse]
---

OpenDJDecks ships with mappings compatible with the Pioneer DJ DDJ-FLX10 and DDJ-FLX4, with LED feedback for play, cue, sync, master, key lock, slip, loop and the pads, and the VU meters. The mappings are not yet fully confirmed on real devices, so individual controls or LEDs may not work: try them before a gig. Connect a controller by USB; it is picked up within a few seconds, and its audio interface is used automatically (**Preferences › Audio › Output device**).

When you move a knob after a preset jumped, soft takeover makes sure the value only changes once the physical control has reached it.

![The controller page in Preferences](../img/controllers.png)

## Browse from the controller

Turn the browse encoder to move through the list. **BACK** moves the focus into the tree, where turning steps through the lists (hold shift to jump by eight), and a press opens a folder or moves into the track list. In the list, a press loads into a free deck and shift-press adds to the tag list. **VIEW** toggles the full browser in Prepare and the Browse layout in Perform.

## MIDI learn

In **Preferences › Controller › MIDI learn**:

1. Choose an **Action** and, if it applies, a **Deck / channel**.
2. Press **LEARN**, then move the control.
3. The control is described and shown as a mapping line. Press **ADD TO MY MAPPING** to write it into your own mapping file, which is used from the next start.

**Own mapping file** adds a mapping on top of the built-in ones and wins for its device. `docs/controller-mappings.md` in the source describes the format; [the developer docs](../../controller-mappings.md) have the details.

## MIDI monitor

Turn on **Monitor** to see every incoming MIDI message and what it does, for example when you are working out what a control sends.
