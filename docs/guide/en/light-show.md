---
title: The DMX light show
description: A light show that follows the music from phrases, beat and energy, set up and played in two modes, fixtures placed on a floor plan and tuned one by one, your own looks and chases on pads, light cues stored with tracks, AUTO you can steer, an easy and a pro mode, a lighting panel in Perform and Club, a patch table, venues with named rig and show files, connecting interfaces with a guided setup, output status with a failsafe, safety limits, DJ events as triggers, music from an audio input or another computer, a second controller and OSC for a lighting operator, and an unattended mode for a fixed venue.
keywords: [trigger, triggers, dj event, drop, breakdown, crossfader, audio input, line-in, microphone, network, osc, second controller, lighting operator, lighting controller, unattended, autostart, calm look, setup, show, floor plan, stage plan, rotate, height, pan range, tilt range, invert, colour calibration, dimmer curve, light cue, cue list, cue lane, venue, venue profile, rig file, show file, import, export, output test, pads, look, looks, chase, chases, group master, palette, hold, release, test pattern, node discovery, artpoll, port, light, lights, lighting, dmx, light show, art-net, artnet, sacn, usb dmx, fixture, fixtures, moving head, par, strobe, laser, fog, smoke, blackout, stage, cue stack, ofl, patch, universe, address, collision, identify, failsafe, safety, epilepsy, photosensitive, pin]
features: [lighting-operator, lighting-triggers, lighting-music-sources, lighting-unattended, lighting-setup-show, lighting-floor-plan, lighting-cue-list, lighting-venues, lighting-pads, lighting-panel, lighting-auto-steer, lighting-connect, lighting-outputs, lighting-fixtures, lighting-patch-table, lighting-failsafe, lighting-safety, lighting-auto, lighting-easy, lighting-pro, lighting-views]
---

OpenDJDecks can run the lights itself. The **LIGHTING** tab (`⌥⌘3`) works in two modes, chosen at the top of its left column:

- **SETUP** is for getting the rig ready: **RIG**, **OUTPUT** and **SAFETY** on the left with the **VENUE** in use, **REMOTE** (a second controller and OSC) and its **FILES**, and in the middle **STAGE** (the floor plan), **PATCH** and **CHANNELS**. **TEST** beside them starts the output test (each fixture in turn) and stops it again, so a new connection is one key away.
- **SHOW** is for playing it: AUTO, EASY or PRO, the master, **FLASH**, **STROBE**, **FOG** and **TAP** on the left with a large **BLACKOUT** at the foot of the column, always in view however far the column is scrolled, and in the middle **STAGE** (the live preview), **PADS**, **CUES** and **TRIGGERS** (DJ events and where the music comes from).

An empty rig opens on SETUP, a rig with fixtures on SHOW.

![The LIGHTING view in SHOW with the patched rig](../img/lighting.png)

## Patch your lights

In **RIG**, choose a built-in type (PAR, mover, bar, strobe, blinder, laser, fog), give a **Name** and a **Group**, and press **ADD**:

- **COUNT** adds several fixtures of the type one after another, named "Name 1", "Name 2" and so on.
- **UNIVERSE** and **START** set where the first one goes. Left empty, the first free addresses are used. Fixtures that would not fit, or would overlap one already patched, are not added at all; the status line names the fixture in the way.
- **IMPORT FIXTURE FILE…** patches a fixture definition from the Open Fixture Library (`.json`), with all its modes.

**PATCH** in the middle lists every fixture. Change **UNIV**, **ADDRESS** and **CH** (channel count) by typing a value and pressing Enter; **MODE** steps through the modes of the type. A channel count that matches a mode switches to it; any other count keeps the layout and fills the extra channels with zero ("Custom"). Nothing has to be edited in files.

An edit that makes two fixtures share channels is kept, not refused, so you can re-address a rig step by step. Both rows get a struck line and say which fixture they overlap; a fixture whose channels run past 512 says so too.

**ID** makes one fixture blink at 1.5 per second so you can find it on the truss. Press it again to stop.

![The patch table with a collision and one fixture identifying](../img/lighting-patch.png)

## Place the fixtures on the floor plan

**STAGE** in SETUP is a floor plan seen from above, the audience at the bottom. Each fixture stands where its kind goes until you place it: movers on the back truss, bars and effects in the middle, washes at the front.

- Drag a fixture to where it hangs. With **GRID** on it snaps to a grid of twentieths of the stage.
- Click a fixture to open it beside the plan: **TURN** points it in steps of 15° (0° towards the audience), **HEIGHT** sets how high it hangs, **GROUP** moves it to another group (type the name and press Enter). **DEFAULT SPOT** puts it back where its kind goes.
- The positions drive the stage preview and the effects that run across the stage: a chase steps from left to right as the fixtures stand, a sweep fans out by position, the phase spread of an effect follows the same order, and a bar turned to face the back runs its cells the other way.

Under **PROGRAMMING** each fixture can be tuned on its own:

- **PAN FROM** / **PAN TO** and **TILT FROM** / **TILT TO** (movers) set the part of the travel the looks use, in steps of 5 %, for a mover that should not shine into the audience or onto a wall.
- **INVERT PAN** and **INVERT TILT** mirror the movement for a mover hung the other way round or upside down.
- **RED**, **GREEN** and **BLUE** raise or lower a colour by up to 50 %, so fixtures of different makes give the same white and the same red.
- **DIMMER CURVE** sets how the light follows the level: **LINEAR**, **SQUARE** (a darker low end, for fixtures that jump up early), **ROOT** (a brighter low end) or **SOFT**.

The tuning is applied when a look becomes DMX, before the safety limits: a raised colour or a brighter curve still cannot pass a group's level cap or the strobe cap. Places and tuning are saved with the rig.

![The floor plan with a fixture placed, turned and open for programming](../img/lighting-floor-plan.png)

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
- **SETUP PIN**: 4 to 8 digits. With a PIN set, **LOCK** closes the setup: patch, outputs, limits, the floor plan and the files need the PIN, and SHOW stays free for guests. The PIN is stored in `lighting.json` next to the library; it keeps guests out of the setup, it is not a password against someone with access to the computer.

![SAFETY with the strobe cap, lockout, level caps and the setup PIN](../img/lighting-safety.png)

## Unattended at a fixed venue

For a bar, a shop or a club room where the lights should run without anyone at the computer, **UNATTENDED** at the end of **SAFETY** locks the show and lets it run by itself:

- **RUN UNATTENDED** needs a setup PIN first; the PIN is what takes the show back. Clearing the PIN ends unattended mode.
- Once you press **LOCK**, SHOW and the lighting panel in Perform show **UNATTENDED** with what the show does now and a PIN field instead of the controls. **BLACKOUT** stays, as the one key anyone may press. The lighting controller, OSC, the deck pads in LIGHTING mode and the live commands of the MCP server do nothing. **TAKE OVER** with the PIN opens the show and the setup again.
- The show follows the music the way it is set (AUTO, EASY or PRO, your pads, light cues and triggers, the music source). The FLASH, STROBE and FOG keys are let go, and fog and lasers stay locked out whatever tries to arm them. Every safety limit holds.
- **START WITH THE APP**: each start of the app opens LIGHTING in SHOW with **DMX OUT** on. Everything is saved in `lighting.json`, so after a power cut or a restart the show comes back as it was, still locked. The app itself has to be started by the computer: add OpenDJDecks to the login items (macOS: System Settings › General › Login Items; Windows: the Startup folder; Linux: your desktop's autostart). OpenDJDecks does not do that for you.
- **CALM AFTER**: when no music has played for this long (10 s to 5 min), the rig fades to a calm warm wash at 30 % in four seconds, with no strobe, no fog and no lasers. It fades back to the show within a second when music returns.

![SHOW locked while the show runs unattended](../img/lighting-unattended.png)

Unattended mode is tested with a simulated output and a restart in UI tests; it has not run for days at a real venue.

## Run the show

**DMX OUT** sends the show to your outputs. Choose how it runs:

- **AUTO** follows phrases, beat and energy. Intros breathe, build-ups chase faster and end in a strobe, choruses sweep the movers on the beat, breakdowns calm down and outros fade. During a mix the looks of both tracks blend by how loud each is, and phrase changes crossfade over a bar.
- **EASY** is one **LOOK** for the whole rig, locked to the beat, with colour swatches, a master dimmer, **BLACKOUT**, and **FLASH**, **STROBE** and **FOG** while held. **TAP** sets the tempo when no deck plays.
- **PRO** sets per group a program (**MUSIC** follows the music, or one of the looks), a colour and an attribute effect (sine, saw, square, random on dimmer, pan, tilt; phase spread; beat-synced), an XY programmer for pan and tilt, and a **CUE STACK** with **GO**, **BACK** and **STORE** with fades counted in beats.

On the decks the **LIGHTING** pad mode gives you your own pads of the current page, or looks A to F, strobe (G) and flash (H) where a pad is empty. Every lighting control can also be mapped to MIDI.

## Your own looks and chases

**PADS** in the middle of SHOW holds your own looks on four pages of eight pads (**PAGE 1** to **4**). A pad shows its number (page and pad, such as 1.3), its name, whether it is a chase or a flash pad, and a line of the colours it sets, one per group. The pad in control is filled green.

- Press a pad to play it. A **LATCH** pad stays on until you press it again or press **RELEASE**; a **FLASH** pad is on while you hold it.
- **EDIT** turns the pads into an editor: click a pad to open it. An empty pad offers **NEW LOOK** (every group white and steady to start from), **NEW CHASE** and **FROM PRO** (the group settings you made in PRO mode).
- A look sets groups: for each group a program (**MUSIC** follows the phrase), a colour (**TRACK** takes it from AUTO's palette or the track) and a level (**—** leaves it to the program). **×** leaves a group out; groups a look leaves out keep what runs underneath. The pad's colour line follows every change at once.
- A chase steps through looks on the beat. **ADD STEPS**, then click the looks in order; the stepper sets how long each step lasts (a quarter beat to two bars). A chase follows the deck you hear, or **TAP** without music.
- **GROUP MASTERS** beside the pads set the level of each group on top of everything, the master dimmer included.

Looks, chases, pages and group masters are saved with the rig in `lighting.json`. Which pad is latched or held is not saved.

![PADS with a look open in the editor, AUTO held in the SHOW column](../img/lighting-pads.png)

On the decks, the **LIGHTING** pad mode plays the eight pads of the current page with pads A to H; a pad of the page without a look keeps the built-in looks A to F, strobe (G) and flash (H). In a mapping, `lighting.pad`, `lighting.page`, `lighting.page_next`, `lighting.page_prev` and `lighting.group_master` reach them from a controller, and MIDI learn can learn them (choose the pad, page or group under **Pad, page or group**).

## Light cues of a track

A light cue is stored with a track and starts at a bar of it, every time the track plays, in every mode: it plays a cue of the stack, a pad's look or a chase until the next light cue. **CUES** in SHOW lists the light cues of the track you hear (with no deck playing, of the track in the Prepare player):

- **CUE AT PLAYHEAD** adds one at the bar the playhead is in, playing the latched pad, else the cue the stack shows, else the first cue or pad.
- **AT** moves it a bar earlier or later, **NAME** renames it (type and press Enter), **PLAYS** steps through the cues of the stack, the pads with a look and the chases, and **FADE** sets the fade into it, from at once to 16 beats (a stack cue starts with its own fade, a pad with one beat). **×** removes it.
- The cue in effect carries the sodium tick.

In Prepare the light cues of the track sit on a lane under the phrases, each a sodium line with its name at its bar. Drag one to another bar; it lands on the nearest bar line. The lane shows while the track has light cues.

A pad or chase of a light cue takes only the groups its look sets, as a latched pad does; a pad you latch or hold still wins over it. Light cues pass through the group masters and the safety limits like everything else. Light cues are saved with the show.

![CUES with the light cues of a track](../img/lighting-cues.png)

![The light cue lane under the waveform in Prepare](../img/lighting-cue-lane.png)

## DJ events as triggers

**TRIGGERS** in SHOW ties moments of the mix to your pads and cues. One row per event:

- **TRACK LOADED**: a track is loaded onto a deck.
- **PLAY STARTS**: a deck starts playing.
- **PHRASE CHANGES**: the deck you hear moves into another phrase.
- **DROP** and **BREAKDOWN**: it moves into a chorus or drop, or into a breakdown or bridge. They win over **PHRASE CHANGES** when both are set.
- **CROSSFADER LEFT** and **CROSSFADER RIGHT**: the crossfader reaches a side. It has to come back towards the middle before the same side fires again.
- **TRACK END NEAR**: a playing track has 30 seconds or less left.

**PLAYS** steps through nothing, **RELEASE** (ends the trigger that plays), the cues of the stack and your pads with a look or chase. **FOR** sets how long it plays: until the next event, or 4, 8, 16, 32 or 64 beats. The row of the trigger playing carries the sodium tick, **LAST** names the last event, and **IN CONTROL** shows it as "TRIGGER · DROP · PAD 1.5 …".

A trigger plays over AUTO, EASY, PRO, HOLD and the light cues of the track, under a pad you latch or hold, and takes only the groups its look sets. It fades in over one beat and passes the group masters and the safety limits like everything else. Triggers are saved with the show.

![TRIGGERS with DJ events tied to pads, MUSIC FROM DECKS](../img/lighting-triggers.png)

## Music from outside the decks

**MUSIC FROM** at the top of **TRIGGERS** chooses what the show follows:

- **DECKS**: the decks of this app (the default).
- **AUDIO INPUT**: the computer's default audio input, line-in or a microphone, for a band, another DJ's mixer or music from another computer. The show finds the beat (70 to 180 BPM) and takes the energy from the level; half a second below −50 dBFS counts as silence. It settles in a few seconds and has no phrases, so AUTO plays its verse look and the phrase triggers do not fire. The line beside it shows the input and the tempo found.
- **NETWORK**: another OpenDJDecks on the local network (see below). The beat, phrase and energy of the deck heard there drive this show; after two seconds without a message it counts as silence.

The tempo finding is tested with generated kicks between 96 and 140 BPM. It has not been tried with a real microphone in a room or a club mixer.

## Venues, rigs and shows

**FILES** beside the **VENUE** in SETUP keeps named files in the `lighting` folder next to the library:

- **RIGS**: the patch and the outputs.
- **SHOWS**: the looks, pads, chases, cue stack, light cues, the AUTO table and the palette.
- **VENUES**: a venue binds a rig to a show, together with the safety limits and the failsafe. On arrival, choose the venue and press **LOAD**: its rig, show and limits come in at once.

Choose a row and press **LOAD**, **SAVE** (what runs now, under that name), **DUPLICATE** or **DELETE**; type a name and press **RENAME** or **SAVE AS NEW**. A new venue keeps the rig and show in use when they are saved as they are, and otherwise gets a rig and show of its own, so saving a venue never changes another one. **IN USE** marks what runs now. **IMPORT…** opens a venue with its rig and show from one file, **EXPORT…** writes the venue, rig and show in use to one file, for another computer or a backup.

The working rig stays in `lighting.json`; the first start after this step makes it the venue, rig and show "Default". Loading needs the setup PIN like the rest of SETUP.

![FILES with the venues saved, one in use](../img/lighting-files.png)

## A second lighting operator

When someone else runs the lights, **REMOTE** in SETUP gives them their own controls:

- **LIGHTING CONTROLLER** lists the MIDI controllers connected. **USE FOR LIGHTS** gives one to the lights: it then plays them with the lighting layout while another controller keeps the decks. **GIVE BACK** returns it to its own mapping. The layout (`mappings/lighting/lighting-operator.toml`) uses generic MIDI numbers on channel 1, so most pad controllers and fader boxes can be set to it in their own editor:

| MIDI | Plays |
|---|---|
| Notes 36–43 | pads 1–8 of the current page |
| Notes 44–47 | pages 1–4 |
| Notes 48, 49 | page back, page on |
| Notes 50–53 | BLACKOUT, FLASH, STROBE, FOG |
| Notes 54–56 | RELEASE, HOLD, TAP |
| Notes 57, 58 | cue BACK, cue GO |
| Notes 60–62 | AUTO, EASY, PRO |
| Controllers 20–27 | group masters 1–8 |
| Controller 28 | master dimmer |

- **OSC IN** (off by default) listens on a UDP port, 9300 unless you set another. While it is on, any device on the local network can play the lights, so turn it on only on a network you trust. The addresses mirror the lighting actions; numbers count from 1, and a missing "on" value means on:

| Address | Values | Does |
|---|---|---|
| `/opendjdecks/light/pad` | pad 1–8, on | plays a pad of the current page |
| `/opendjdecks/light/page` | page 1–4 | shows a page |
| `/opendjdecks/light/page/next`, `…/page/prev` | | next or previous page |
| `/opendjdecks/light/group` | group, level 0–1 | a group master |
| `/opendjdecks/light/master` | level 0–1 | the master dimmer |
| `/opendjdecks/light/blackout` | on | blackout on or off; without a value it toggles |
| `/opendjdecks/light/flash`, `…/strobe`, `…/fog` | on | the hold keys |
| `/opendjdecks/light/look` | look 1–6 | an EASY look |
| `/opendjdecks/light/mode` | `auto`, `easy`, `pro` | the mode |
| `/opendjdecks/light/cue/go`, `…/cue/back` | | the cue stack |
| `/opendjdecks/light/release`, `…/hold`, `…/tap` | | RELEASE, HOLD, TAP |
| `/opendjdecks/music` | bpm, beat, playing, energy, phrase, hue, seconds left | the music another instance sends |

- **SEND THE MUSIC** sends the tempo, beat, phrase and energy of the deck you hear 20 times a second to the address you type (`host:port`). On a second computer running OpenDJDecks for the lights, turn **OSC IN** on and choose **NETWORK** under **MUSIC FROM**: its show follows your mix while its operator plays pads, cues and triggers there.

![REMOTE with a controller given to the lights, OSC in and the music sent out](../img/lighting-remote.png)

The lighting layout is tested with simulated MIDI, OSC and the second instance with a sender on the loopback address of the same computer. No real second controller and no second computer on a real network have been tried yet. There is no special lighting-only start mode and no automatic discovery of the other computer: you type its address.

## Steer AUTO

With **AUTO** chosen, the SHOW column steers it:

- **PALETTE** chooses AUTO's colours: **TRACK KEY** (from the key of the track, as before), **WARM**, **COOL**, **SUNSET**, **OCEAN**, **WHITE**, or **OWN** with the swatches you tick. The colour moves on every eight bars, and to the next colour every other bar pair in high-energy phrases.
- **PHRASE → LOOK** says what AUTO plays for intro, verse, build-up, drop (chorus), breakdown and outro: a built-in program or one of your pads. The phrase heard now is marked.
- **HOLD** keeps the look AUTO plays now through phrase changes, until you press it again or **RELEASE**.

**Who wins.** From the top:

1. **BLACKOUT**.
2. The **FLASH** and **STROBE** keys while held.
3. A flash pad while held (the last one pressed).
4. The latched pad.
5. A trigger of a DJ event.
6. **HOLD**.
7. AUTO (or EASY or PRO), with the light cues of the track.

A pad takes only the groups its look sets; the others keep following the layer below. Group masters and the master dimmer then scale the result, and the safety limits apply to every frame last. **RELEASE** hands back to AUTO: the latched pad and HOLD end, with a fade over one beat; a flash pad ends at once when let go. The order is shown under **WHO WINS, TOP FIRST** with the layer in control lit, and **IN CONTROL** names it, such as "PAD 1.1 · RED WASH · LATCHED".

## Where to see it

- The **LIGHT STRIP** button (`⌥⌘K`, **STRIP** in narrower windows) opens the lighting panel in Perform and Club. It takes the browser's place, so the decks, waveforms and mixer keep their size; **LIGHT STRIP** again brings the browser back. It holds the stage preview (from 1360 px wide), the pages and pads, the master and group faders, AUTO / EASY / PRO, DMX OUT, HOLD and RELEASE, who is in control (or a lost link), and BLACKOUT, FLASH, STROBE and FOG. Every key, pad and fader is at least 44 px for touch, with dark surfaces and light ink for a dark booth.
- **WINDOW** opens the lighting view in its own window, for a second screen.

![The lighting panel in Perform, one look latched](../img/lighting-perform.png)

![The lighting panel in Club mode](../img/lighting-club.png)

The panel has been tried with a mouse and in UI tests at 1440 × 900 and 1280 × 800; it has not been tried on a real touch screen yet.

The floor plan, the fixture tuning, light cues and venue files are tested in UI tests and with a simulated output. **None of it has been tried on a real rig yet**: whether a pan range or a colour calibration matches a particular fixture can only be seen on that fixture.
