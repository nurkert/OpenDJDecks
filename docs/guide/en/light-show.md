---
title: The DMX light show
description: A light show that follows the music from phrases, beat and energy, your own looks and chases on pads, AUTO you can steer, an easy and a pro mode, a lighting panel in Perform and Club, a patch table, connecting interfaces with a guided setup, output status with a failsafe, and safety limits.
keywords: [pads, look, looks, chase, chases, group master, palette, hold, release, test pattern, node discovery, artpoll, port, light, lights, lighting, dmx, light show, art-net, artnet, sacn, usb dmx, fixture, fixtures, moving head, par, strobe, laser, fog, smoke, blackout, stage, cue stack, ofl, patch, universe, address, collision, identify, failsafe, safety, epilepsy, photosensitive, pin]
features: [lighting-pads, lighting-panel, lighting-auto-steer, lighting-connect, lighting-outputs, lighting-fixtures, lighting-patch-table, lighting-failsafe, lighting-safety, lighting-auto, lighting-easy, lighting-pro, lighting-views]
---

OpenDJDecks can run the lights itself. The **LIGHTING** tab (`⌥⌘3`) has three columns: setup on the left (**RIG**, **OUTPUT**, **SAFETY**), the middle (**STAGE** preview, **PADS** for your own looks, **PATCH** table, **CHANNELS**), and **SHOW** on the right to run it.

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

## Connect an interface

**GUIDED SETUP** in **OUTPUT** takes you through a connection in four short steps:

1. How the lights are connected: **ART-NET NODE**, **SACN RECEIVER** or **USB-DMX INTERFACE**.
2. Which one: for Art-Net the app asks the network which nodes are there and lists them by name, address and universes, or you type an address; for USB-DMX you choose the serial port; sACN needs no choice.
3. Which universe goes there (1 for a small rig; the patch table shows your fixtures' universe).
4. **SET AND TEST** sets the output, turns DMX OUT on and starts the test pattern. **YES, THEY FOLLOW** stops the test and closes the guide; **NOTHING HAPPENS** lists what to check, in order.

![The guided setup with an Art-Net node found](../img/lighting-guide.png)

The same parts work on their own in **OUTPUT**:

- **FIND NODES** (with ART-NET chosen) sends an ArtPoll, the "who is there?" message of the Art-Net protocol, to the broadcast address of the local network and lists every node that answers within about 1.5 seconds. **USE** sends the universe typed beside it to that node. A node with several output ports gets the universe on the port with the same number, otherwise its first port.
- With **USB-DMX** chosen, **PORTS** lists the serial devices that look like USB adapters (on macOS `/dev/cu.usbserial…` and `/dev/cu.usbmodem…`, on Linux `/dev/ttyUSB…`, `/dev/ttyACM…` and `/dev/serial/by-id`, on Windows the COM ports that exist). **REFRESH** looks again after plugging in; **USE** sends the universe to that port.
- **Several universes**: give each universe its own line. Sent to the same Art-Net node, universe 1 goes to its Port-Address 0, universe 2 to 1, and so on; sACN without a number sends universe 1 as sACN universe 1, universe 2 as 2, and so on.
- **TEST PATTERN**: **EACH** lights one fixture after another, a second each; **COLOURS** steps the whole rig through red, green, blue and white; **FULL** lights everything. It replaces the show until **OFF**, at 80 %, and passes through the safety limits like everything else: it cannot flash faster than the strobe cap, exceed a group's level cap or open fog or lasers.

![OUTPUT with a node found, the test pattern running](../img/lighting-connect.png)

Discovery, the port picker and the guide are tested with a stand-in node on the computer's own loopback address and listed test devices. **No real Art-Net node, sACN receiver or USB-DMX interface has been tried with them yet.** Nodes that only answer on port 6454 while another lighting program holds that port on the same computer may not show up; type their address instead.

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

- **STROBE AND FLASH CAP**: at most this many flashes per second, 3 by default, 0 for no strobe at all. Fast flashing light can trigger seizures in people with photosensitive epilepsy; guidance for broadcast and events puts the risk from about 3 flashes a second upwards, so 3 or less is the safe default. The cap works in two ways. Strobe channels are lowered to the capped rate; this assumes a fixture's fastest strobe value is 25 flashes a second, the common range, so a fixture that strobes faster than that can still flash a little faster than the cap. And every rise in a fixture's level of about a fifth of full or more counts as a flash, however dim the look and whatever the group's level cap. A fixture may not rise like that again until one cap period has passed since its last flash. This covers fast chases and effects on fixtures without a strobe channel.
- **LOCKOUT**: fog machines and lasers stay at zero until you press **ARM FOG** or **ARM LASERS**. Arming is not saved: each start of the app begins locked. **FOG** in SHOW does nothing while fog is locked.
- **LEVEL CAP PER GROUP**: the highest level a group may reach, in steps of 10 %.
- **SETUP PIN**: 4 to 8 digits. With a PIN set, **LOCK** closes the setup: patch, outputs and limits need the PIN, and the show controls on the right stay free for guests. The PIN is stored in `lighting.json` next to the library; it keeps guests out of the setup, it is not a password against someone with access to the computer.

![SAFETY with the strobe cap, lockout, level caps and the setup PIN](../img/lighting-safety.png)

## Run the show

**DMX OUT** sends the show to your outputs. Choose how it runs:

- **AUTO** follows phrases, beat and energy. Intros breathe, build-ups chase faster and end in a strobe, choruses sweep the movers on the beat, breakdowns calm down and outros fade. During a mix the looks of both tracks blend by how loud each is, and phrase changes crossfade over a bar.
- **EASY** is one **LOOK** for the whole rig, locked to the beat, with colour swatches, a master dimmer, **BLACKOUT**, and **FLASH**, **STROBE** and **FOG** while held. **TAP** sets the tempo when no deck plays.
- **PRO** sets per group a program (**MUSIC** follows the music, or one of the looks), a colour and an attribute effect (sine, saw, square, random on dimmer, pan, tilt; phase spread; beat-synced), an XY programmer for pan and tilt, and a **CUE STACK** with **GO**, **BACK** and **STORE** with fades counted in beats. **TRACK CUES** stores a cue at a bar of the track you hear, recalled whenever that track plays.

On the decks the **LIGHTING** pad mode gives you your own pads of the current page, or looks A to F, strobe (G) and flash (H) where a pad is empty. Every lighting control can also be mapped to MIDI.

## Your own looks and chases

**PADS** in the middle holds your own looks on four pages of eight pads (**PAGE 1** to **4**). A pad shows its number (page and pad, such as 1.3), its name, whether it is a chase or a flash pad, and a line of the colours it sets, one per group. The pad in control is filled green.

- Press a pad to play it. A **LATCH** pad stays on until you press it again or press **RELEASE**; a **FLASH** pad is on while you hold it.
- **EDIT** turns the pads into an editor: click a pad to open it. An empty pad offers **NEW LOOK** (every group white and steady to start from), **NEW CHASE** and **FROM PRO** (the group settings you made in PRO mode).
- A look sets groups: for each group a program (**MUSIC** follows the phrase), a colour (**TRACK** takes it from AUTO's palette or the track) and a level (**—** leaves it to the program). **×** leaves a group out; groups a look leaves out keep what runs underneath. The pad's colour line follows every change at once.
- A chase steps through looks on the beat. **ADD STEPS**, then click the looks in order; the stepper sets how long each step lasts (a quarter beat to two bars). A chase follows the deck you hear, or **TAP** without music.
- **GROUP MASTERS** beside the pads set the level of each group on top of everything, the master dimmer included.

Looks, chases, pages and group masters are saved with the rig in `lighting.json`. Which pad is latched or held is not saved.

![PADS with a look open in the editor, AUTO held on the right](../img/lighting-pads.png)

On the decks, the **LIGHTING** pad mode plays the eight pads of the current page with pads A to H; a pad of the page without a look keeps the built-in looks A to F, strobe (G) and flash (H). In a mapping, `lighting.pad`, `lighting.page`, `lighting.page_next`, `lighting.page_prev` and `lighting.group_master` reach them from a controller, and MIDI learn can learn them (choose the pad, page or group under **Pad, page or group**).

## Steer AUTO

With **AUTO** chosen, the right column steers it:

- **PALETTE** chooses AUTO's colours: **TRACK KEY** (from the key of the track, as before), **WARM**, **COOL**, **SUNSET**, **OCEAN**, **WHITE**, or **OWN** with the swatches you tick. The colour moves on every eight bars, and to the next colour every other bar pair in high-energy phrases.
- **PHRASE → LOOK** says what AUTO plays for intro, verse, build-up, drop (chorus), breakdown and outro: a built-in program or one of your pads. The phrase heard now is marked.
- **HOLD** keeps the look AUTO plays now through phrase changes, until you press it again or **RELEASE**.

**Who wins.** From the top:

1. **BLACKOUT**.
2. The **FLASH** and **STROBE** keys while held.
3. A flash pad while held (the last one pressed).
4. The latched pad.
5. **HOLD**.
6. AUTO (or EASY or PRO).

A pad takes only the groups its look sets; the others keep following the layer below. Group masters and the master dimmer then scale the result, and the safety limits apply to every frame last. **RELEASE** hands back to AUTO: the latched pad and HOLD end, with a fade over one beat; a flash pad ends at once when let go. The order is shown under **WHO WINS, TOP FIRST** with the layer in control lit, and **IN CONTROL** names it, such as "PAD 1.1 · RED WASH · LATCHED".

## Where to see it

- The **LIGHT STRIP** button (`⌥⌘K`, **STRIP** in narrower windows) opens the lighting panel in Perform and Club. It takes the browser's place, so the decks, waveforms and mixer keep their size; **LIGHT STRIP** again brings the browser back. It holds the stage preview (from 1360 px wide), the pages and pads, the master and group faders, AUTO / EASY / PRO, DMX OUT, HOLD and RELEASE, who is in control (or a lost link), and BLACKOUT, FLASH, STROBE and FOG. Every key, pad and fader is at least 44 px for touch, with dark surfaces and light ink for a dark booth.
- **WINDOW** opens the lighting view in its own window, for a second screen.

![The lighting panel in Perform, one look latched](../img/lighting-perform.png)

![The lighting panel in Club mode](../img/lighting-club.png)

The panel has been tried with a mouse and in UI tests at 1440 × 900 and 1280 × 800; it has not been tried on a real touch screen yet.
