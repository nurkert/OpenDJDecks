---
title: The DMX light show
description: A light show that follows the music from phrases, beat and energy, an easy mode with looks, a pro mode with programs, effects and a cue stack, a patch table, output status with a failsafe, and safety limits.
keywords: [light, lights, lighting, dmx, light show, art-net, artnet, sacn, usb dmx, fixture, fixtures, moving head, par, strobe, laser, fog, smoke, blackout, stage, cue stack, ofl, patch, universe, address, collision, identify, failsafe, safety, epilepsy, photosensitive, pin]
features: [lighting-outputs, lighting-fixtures, lighting-patch-table, lighting-failsafe, lighting-safety, lighting-auto, lighting-easy, lighting-pro, lighting-views]
---

OpenDJDecks can run the lights itself. The **LIGHTING** tab (`⌥⌘3`) has three columns: setup on the left (**RIG**, **OUTPUT**, **SAFETY**), the middle (**STAGE** preview, **PATCH** table, **CHANNELS**), and **SHOW** on the right to run it.

![The LIGHTING view with the patched rig](../img/lighting.png)

## Patch your lights

In **RIG**, choose a built-in type (PAR, mover, bar, strobe, blinder, laser, fog), give a **Name** and a **Group**, and press **ADD**:

- **COUNT** adds several fixtures of the type one after another, named "Name 1", "Name 2" and so on.
- **UNIVERSE** and **START** set where the first one goes. Left empty, the first free addresses are used. Fixtures that would not fit, or would overlap one already patched, are not added at all; the status line names the fixture in the way.
- **IMPORT FIXTURE FILE…** patches a fixture definition from the Open Fixture Library (`.json`), with all its modes.

**PATCH** in the middle lists every fixture. Change **UNIV**, **ADDRESS** and **CH** (channel count) by typing a value and pressing Enter; **MODE** steps through the modes of the type. A channel count that matches a mode switches to it; any other count keeps the layout and fills the extra channels with zero ("Custom"). Nothing has to be edited in files.

An edit that makes two fixtures share channels is kept, not refused, so you can re-address a rig step by step. Both rows get a struck line and say which fixture they overlap; a fixture whose channels run past 512 says so too.

**ID** makes one fixture blink at 1.5 per second so you can find it on the truss. Press it again to stop.

![The patch table with a collision and one fixture identifying](../img/lighting-patch.png)

## Outputs and a lost link

**OUTPUT** chooses how each universe leaves the computer: Art-Net, sACN, or a USB-DMX interface (serial device). Type the universe (empty is universe 1), the target, and press **SET**; several universes are possible.

Each output has a status line: solid green while it sends, struck with **LOST** in red when the link is gone, dashed for 20 seconds after it came back. A link that fails is tried again every second, so an interface that is plugged back in, or plugged in after the app started, is picked up by itself. Perform shows the same state next to DMX in the lighting strip, and in the top bar while a link is lost and the strip is hidden.

**WHEN A LINK IS LOST** decides what the whole rig shows while any link is missing:

- **HOLD** keeps the last look.
- **SAFE SCENE** fades in two seconds to a calm warm wash at 30 %: no strobe, no fog, no lasers.
- **BLACKOUT** goes dark at once.

When every link is back, the rig fades back to the show in one second.

![OUTPUT with one universe sending and one USB link lost](../img/lighting-output.png)

**CHANNELS** shows the 512 values of a universe as they are sent, after the safety limits.

### What happens if the app crashes

- If the show itself fails inside the app, the lighting thread catches it and sends the failsafe scene from then on.
- When the app quits normally, **SAFE SCENE** and **BLACKOUT** send that scene as the last frame. **HOLD** sends nothing more.
- If the app is killed or the computer stops, the app can do nothing. What the lights do then is up to the hardware: most USB-DMX interfaces with their own processor and most Art-Net nodes keep repeating the last frame they got; sACN receivers usually hold for a few seconds and then follow their own setting. Set the "DMX loss" behaviour on your interface and fixtures if it matters.

Loss and recovery are tested with a simulated link. They have not been tried yet with a real interface being unplugged.

## Safety limits

**SAFETY** sets limits that every look must keep. They are applied to the finished DMX frame right before it is sent, so automatic looks, buttons, cues, track cues, the remote tools and identify all pass through them.

- **STROBE AND FLASH CAP**: at most this many flashes per second, 3 by default, 0 for no strobe at all. Fast flashing light can trigger seizures in people with photosensitive epilepsy; guidance for broadcast and events puts the risk from about 3 flashes a second upwards, so 3 or less is the safe default. The cap works in two ways. Strobe channels are lowered to the capped rate; this assumes a fixture's fastest strobe value is 25 flashes a second, the common range, so a fixture that strobes faster than that can still flash a little faster than the cap. And a fixture whose level jumps from dark to bright is held dark until one cap period has passed since its last flash, which covers fast chases and effects on fixtures without a strobe channel.
- **LOCKOUT**: fog machines and lasers stay at zero until you press **ARM FOG** or **ARM LASERS**. Arming is not saved: each start of the app begins locked. **FOG** in SHOW does nothing while fog is locked.
- **LEVEL CAP PER GROUP**: the highest level a group may reach, in steps of 10 %.
- **SETUP PIN**: 4 to 8 digits. With a PIN set, **LOCK** closes the setup: patch, outputs and limits need the PIN, and the show controls on the right stay free for guests. The PIN is stored in `lighting.json` next to the library; it keeps guests out of the setup, it is not a password against someone with access to the computer.

![SAFETY with the strobe cap, lockout, level caps and the setup PIN](../img/lighting-safety.png)

## Run the show

**DMX OUT** sends the show to your outputs. Choose how it runs:

- **AUTO** follows phrases, beat and energy. Intros breathe, build-ups chase faster and end in a strobe, choruses sweep the movers on the beat, breakdowns calm down and outros fade. During a mix the looks of both tracks blend by how loud each is, and phrase changes crossfade over a bar.
- **EASY** is one **LOOK** for the whole rig, locked to the beat, with colour swatches, a master dimmer, **BLACKOUT**, and **FLASH**, **STROBE** and **FOG** while held. **TAP** sets the tempo when no deck plays.
- **PRO** sets a **PROGRAM**, a **COLOUR** and an attribute effect (sine, saw, square, random on dimmer, pan, tilt; phase spread; beat-synced) per group, an XY programmer for pan and tilt, and a **CUE STACK** with **GO**, **BACK** and **STORE** with fades counted in beats. **TRACK CUES** stores a cue at a bar of the track you hear, recalled whenever that track plays.

On the decks the **LIGHTING** pad mode gives you looks A to F, strobe (G) and flash (H) from the pads. Every lighting control can also be mapped to MIDI.

## Where to see it

- The **LIGHT STRIP** button (`⌥⌘K`) in Perform shows a lighting strip with the output state.
- **WINDOW** opens the lighting view in its own window, for a second screen.

![The lighting strip in Perform with a lost link](../img/lighting-perform.png)
