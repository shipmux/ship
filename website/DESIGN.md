---
name: Ship website
description: The ship as a place, drawn in the terminal's sixteen colours on one pixel grid.
colors:
  ink: "#1c1a27"
  blocked-red: "#e65a48"
  idle-green: "#3fae5a"
  working-yellow: "#e2a52b"
  sea-blue: "#2f63cc"
  error-magenta: "#c45cc4"
  unseen-cyan: "#25b2c0"
  bezel-cream: "#d4ccb8"
  muted-ink: "#5f5a75"
  bright-red: "#ff7a68"
  bright-green: "#86d873"
  bright-yellow: "#ffd462"
  bright-blue: "#79a4ff"
  bright-magenta: "#f590e8"
  bright-cyan: "#7fe3ec"
  paper: "#fbf8f1"
  room-seam: "#c2baa9"
  sky-high: "#a9d3ea"
  sky: "#cfe7f3"
  haze: "#e4f2f8"
  sea: "#1f4fa8"
  deep: "#14306b"
  wall: "#252232"
  hold: "#1c1a27"
  seabed: "#0f1a38"
  muted-light: "#4a4660"
  muted-dark: "#b9b3c9"
typography:
  pixel-display:
    fontFamily: "Departure Mono, ui-monospace, monospace"
    fontSize: "66px"
    fontWeight: 400
    lineHeight: "77px"
  pixel-headline:
    fontFamily: "Departure Mono, ui-monospace, monospace"
    fontSize: "33px"
    fontWeight: 400
    lineHeight: "44px"
  pixel-title:
    fontFamily: "Departure Mono, ui-monospace, monospace"
    fontSize: "22px"
    fontWeight: 400
    lineHeight: "33px"
  pixel-label:
    fontFamily: "Departure Mono, ui-monospace, monospace"
    fontSize: "11px"
    fontWeight: 400
    lineHeight: "22px"
  body:
    fontFamily: "Atkinson Hyperlegible Next, system-ui, sans-serif"
    fontSize: "18px"
    fontWeight: 400
    lineHeight: 1.55
  code:
    fontFamily: "ui-monospace, SF Mono, Menlo, Cascadia Mono, DejaVu Sans Mono, monospace"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.45
rounded:
  none: "0px"
spacing:
  art-pixel: "4px"
  art-pixel-phone: "3px"
  gutter: "56px"
  gutter-phone: "20px"
  column: "1328px"
components:
  terminal:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.bezel-cream}"
    typography: "{typography.code}"
    rounded: "{rounded.none}"
  copy-button:
    backgroundColor: "{colors.unseen-cyan}"
    textColor: "{colors.ink}"
    typography: "{typography.pixel-label}"
    padding: "2px 10px 3px"
  keycap:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
    typography: "{typography.code}"
    padding: "2px 7px 3px"
  plate:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
    typography: "{typography.code}"
    padding: "2px 12px 3px"
---

# Design system: Ship website

This records the art system as it ships on the `website-polish` branch. Values come from `public/css/site.css`, `landing.css`, `mode.css`, the generators in `tools/` and the frozen `directions/ansi/tools/`, and from the SVGs in `public/art/` they produce. Where a rule here and the code disagree, the code is right and this file is stale. The brief is `PRODUCT.md`; this file covers only how things look and move.

## Overview

**Creative North Star: "the ship as a place, drawn in terminal material"** (the phrase is `PRODUCT.md`'s and the ansi direction's, not new).

The landing page is one tall cross-section of a ship: sky, deck, waterline, below deck, the hold, seabed, read top to bottom with native scrolling. Everything is drawn on one pixel grid in the sixteen colours of Ship's own terminal theme, so the art looks like the thing it describes. Crewmates are small walking terminals. They supply the warmth, and the copy stays dry.

Light comes from the sky down. The upper grounds are pale and repaint at night. From the waterline down the page is already dark, and the only way to keep ink outlines readable there is a lit room (a tan wall inside an ink frame) or a bright-blue outline.

**Key characteristics:**

- One art pixel is 4 css px (3 on a phone), and every sprite is a whole number of them.
- Sixteen colours plus one seam grey in the art. Sprites have no gradients; depth is a dither.
- Crew carry state only through colour plus a glyph and a word. In scenes they are neutral.
- Motion is ambient or one-shot. Frame 0 of every sheet is the still picture.
- All art is decorative and `aria-hidden`.

## Colors

The palette is the terminal's sixteen colours ("Ship 16", mirrored in `directions/ansi/tools/pixel.py` and the `--c0` to `--c15` custom properties) plus one art-only seam grey. The state colours mean something in Ship, so they are spent carefully.

### State colours (crew and glyphs)

- **Working yellow** (#e2a52b): an agent that is working, and wood and brass in the art.
- **Blocked red** (#e65a48): blocked. Never used on scenery.
- **Unseen cyan** (#25b2c0): finished while you looked elsewhere. Also the selection colour (the selected pane's border, a lit menu row, the copy button).
- **Idle green** (#3fae5a): idle. **Error magenta** (#c45cc4): error.

Each has a bright twin (9 to e) used for outlines on dark water, flashes and text on dark grounds. The glyph and word for a state always travel with its colour: `●` working, `▲` blocked, `◆` unseen, `○` idle, `✖` error.

### Neutrals

- **Ink** (#1c1a27): outlines, screens, the hold's ground, the bar. **Muted ink** (#5f5a75): quiet ink, dividers, the footer.
- **Bezel cream** (#d4ccb8): terminal bezels, the lit rooms' wall. **Paper** (#fbf8f1): sails, caps, cards, keycap faces.
- **Room seam** (#c2baa9): the plank seams on a lit room's wall. Art only, and the one colour outside the sixteen.

### Grounds

Sky high #a9d3ea, sky #cfe7f3, haze #e4f2f8, then sea #1f4fa8, deep #14306b, wall #252232, hold #1c1a27 and seabed #0f1a38 as the page descends. The seabed's own background runs from deep to seabed. At night the first three become navy and nothing from the waterline down changes.

**The No State Colour On Props Rule.** Scenery does not spend red, green, cyan or magenta on things that are not crew or glyphs (`scenery.py`). Scenes break it in three named places: cyan for a selected pane or a lit menu row or keycap (the chart room, the mouse scene, the keys scene), and the muster board in the agents scene, where the five state colours are the content. Each glyph there sits beside its word.

**The Focus Colour Rule.** The focus ring is ink on light grounds and bright cyan on dark ones (`--ring`).

## Typography

**Pixel face:** Departure Mono, loaded from a CDN with a monospace fallback. **Reading:** Atkinson Hyperlegible Next. **Code:** the system monospace stack.

Departure Mono is set only at multiples of 11px so it lands on its own grid: 11, 22, 33, 44 and 66px (`.p11` to `.p66`), each with a line height 11px larger. It carries headings, place names, the "why ship" labels and the copy button. Reading text is a five-step scale, 14, 16, 18, 20 and 22px (`--t-sm` to `--t-xl`), with 18px body at 1.55. Code and terminals use 13 and 14px.

Copy is lowercase and flowing. Links are underlined (2px, offset 0.22em) and bold in the hold and seabed. Inline `code` is a translucent chip with no radius; `kbd` is a keycap with a 3px ink-and-muted underside.

**The One Pixel Face Rule.** The pixel face is for names and labels, never for paragraphs.

## Layout

A page column of 1328px (`min(1328px, 100% - 2 * gutter)`), gutter 56px, 20px on a phone. A sticky bar of 60px (92px on a phone, two rows). Section anchors scroll clear of the bar.

Breakpoints that carry layout, found in `landing.css`:

- **1360px and up:** the hatch, ladder and lantern at the top of the hold.
- **1280px:** the hold's two columns settle at 0.82fr and 1.18fr, and the keys card sets its words beside its scene.
- **1250px:** the waterline's flotsam row has room for the dinghy and two more things. Below it the dinghy floats between the words and the recording.
- **1200px:** the agents, mouse and saved tabs cards, and config, go side by side. Each needs a 336px words column for its scene.
- **1099px, 899px, 760px:** below-deck floors stack, the hero stacks and the seabed makes room, then rooms sit above their words.
- **600px:** the phone layout: art pixel 3px, gutter 20px, the keys table becomes a list.

The hold's cards each open with a scene, then the heading and words, then the recording. A scene is 336px wide (252px on a phone). The config card sets scene, heading and first paragraph as a head row, with the rest of the words beside the code they describe.

## Elevation & Depth

Flat. Surfaces are ink or cream blocks with hard edges. Depth is shown by ground (light to dark down the page), by a 1 art pixel ink outline, by a dithered step between two grounds (2 art pixel checker cells, `--deep` against `--sea`) and by a dithered halo around light. The few shadows are hard: a keycap's underside (`0 3px 0`), a plate's 2px ink ring, the copy button's pressed edge.

**The Dither Not Gradient Rule.** Sprites contain no gradients. A lamp's glow is a checker of bright yellow. The only translucent art is the wall's seams and the hatch's glow (`wall.svg` and `glow.svg`, from the first generation), which let the section's colour show through. Sections use flat grounds and dither steps; the seabed is the one section with a linear background.

## Shapes

Square. Radius is zero everywhere; corners are cut by one art pixel where a plate needs it (`clip-path` on the screen bezel). Borders are 2px ink on terminals. Every curve in the art is stepped on the grid (rings and domes are ellipses on whole pixels).

## Components

### Terminal and command block

- **Shape:** square, 2px ink border, ink ground, cream text in the code face (14px).
- **Command blocks** carry a copy button at the top right. They keep a line whole and scroll sideways (with a dithered edge on the side with more to see) where the page is narrow. The seabed's closing command is the exception: its block is as wide as the command, and on a phone it wraps at slashes and spaces and never scrolls.
- **State glyphs** are drawn only inside terminals and on the deck plates.

### Copy button and keycap

- **Copy button:** a cyan keycap in the pixel face (11px), 2px darker underside, 44px touch target on coarse pointers. It turns green when it worked.
- **Keycap (`kbd`):** ink with a paper label, nowrap, a 3px muted underside.

### Plate (the deck crew's label)

Ink strip with the glyph, the state word and the role, under each deck crewmate. The mood pose is never shown without it.

### Screen bezel (`.mount`)

Every recording and figure terminal gets the crewmate's own bezel: 1 art pixel ink outline, a 3 art pixel cream plate with a muted screw in each corner, then the ink screen, corners notched by one art pixel.

### Lit room (the scene stage)

The stage for every scene below deck and in the hold (see Art system). An 84 by 56 art pixel box: bezel-cream wall with room-seam plank seams, a 1 art pixel ink frame, a yellow floor with rivets. It reads on the dark page because it carries its own light.

## Art system

All art is generated. `directions/ansi/tools/` is the frozen first generation (the palette, the crewmate, the deck, the first rooms); it is read from and never edited. Everything since comes from `tools/art.py`, which imports it and writes straight into `public/art/`. Run `python3 tools/art.py` and never edit an SVG.

### The grid and the palette

- An art pixel is `--px`: 4px, 3px at 600px and under. A sprite's size is set per element in art pixels (`--w`, `--h`) and drawn with `image-rendering: pixelated`.
- All 161 SVGs in `public/art/` use only the sixteen colours plus the room seam. All but two (`wall.svg`, `glow.svg`) are whole-pixel rectangles at full opacity; those two are translucent.
- Sheets are frames laid side by side, each `--w` by `--h` art pixels, so a sheet is exactly `--n` frames wide.

### What is generated

| Family                                                                      | Size (art pixels)                             | Source                       |
| --------------------------------------------------------------------------- | --------------------------------------------- | ---------------------------- |
| Crewmate: 5 hats, a diver, 5 moods, blink and glance faces, at-work poses   | 28 by 24                                      | `crew.py`                    |
| Deck crew loops and the three docs-note crew                                | 36 by 28, 6 to 41 frames; 28 by 24            | `loops.py`                   |
| Deck crew beats and the props played over them                              | 36 by 28; 44 by 40                            | `beats.py`, `fx.py`          |
| Rooms below deck (galley, chart, telegraph, handoff) and the hold           | 84 by 56, 16 to 20 frames                     | `crewrooms.py`               |
| Hold card scenes (keys, config, agents, mouse, saved tabs)                  | 84 by 56, 12 to 16 frames                     | `holdscenes.py`              |
| Hanging lantern, ambient                                                    | 13 by 15, 10 frames                           | `holdscenes.py`              |
| Background crew: a barrel roller, a sleeper in a hammock, a mopper          | 46 by 25, 8 frames; 56 by 32, 12; 42 by 26, 8 | `backcrew.py`                |
| Seabed: lantern diver and chest                                             | 64 by 38, 8 frames                            | `seabed.py`                  |
| Waterline: the cook's dinghy (a beat)                                       | 78 by 38, 13 frames                           | `waterline.py`               |
| Fish: blue, yellow, a school of three                                       | 12 by 6 and 20 by 9, 2 frames                 | `scenery.py`, `seabed.py`    |
| Flotsam: sign, buoy, plank, bottle, "pr" crate, "refactor everything" plank | 18 by 12 down to 7 by 10; 14 by 14; 43 by 18  | `scenery.py`, `waterline.py` |
| Tiles: wave, planks, hull, wall, sand, keel, beam, bulkhead                 | 16 by 5 to 64 by 24                           | `scenery.py`, `rooms.py`     |

### Crew

- A crewmate is the bot mark's head on a small body, 28 by 24. Hats tell them apart (deckhand's cap, captain, cook, lookout, navigator; the diver's brass helmet is seabed only). Colour on the hat never means state; the face does.
- A mood is a face in that state's colour plus a pose. Deck crew show their mood with their plate. Crew in scenes are neutral, and so are the background crew.
- **The Head Stays Put Rule.** In room frames the head never moves, so the page's blink and glance patch (a 10 by 6 overlay on the screen) always lines up. A flipped crewmate has no patch, because the mouth would jump two pixels.

### Framing

- **Lit rooms** for anything that must read on a dark ground: ink outlines on a tan wall.
- **Bright-blue outlines** for crew in dark water (the divers), because ink vanishes there. The second diver has a white helmet band where the first has a blue one.
- **Muted grey limbs** for the background crew on the dark wall and hold, where ink vanishes too and a lit room would be too loud for a figure that is part of the background.
- **Plates** under deck crew; **bezels** around recordings.
- A scene has one prop that says what the card is about (a rack of keycaps, a gauge, a muster board, a boathook with an arrow for a tip, a rack of layout cards), one crewmate, one beat, and the light.

### Scenes, as shipped

- **Hold, install:** the cook stacking cargo (the original hold room), now at the head of the install card. **Keys:** the deckhand taps the `alt` keycap on a rack and the rest jingle. **Config:** the captain slides a lever from 23 to 34 and a mini sidebar widens. **Agents:** the lookout aims a spyglass down a board of the five states in the hero's order, each glyph beside its word, and the cursor row hops. **Mouse:** the deckhand, flipped, drags a pane border and opens a right-click menu with a boathook tipped by the arrow pointer. **Saved tabs:** the navigator takes a layout card from a rack labelled "tabs", shows its fresh prompts, and puts it back.
- **Waterline:** a "pr" crate and a "refactor everything" plank among the flotsam, and the cook in a dinghy netting a "10x" board. The dinghy floats at the surface from 1250px and in the flow under the words below that.
- **Seabed:** a second diver holds a lantern over a chest. Its glow flickers, a glint passes over the coins, and bubbles rise. A yellow fish swims through the light and a small school passes above. The reef around it moves over to make room; on 899px and narrower the rocks, starfish, crab and two kelp step aside.

### Background crew

Three neutral crewmates at odd jobs in the dark behind the copy, the way the deck's navigator rides the ratlines. They come from `tools/backcrew.py` and are the one part of the art that can be dropped whole: remove the block in `landing.css` and the three `.bgcrew` elements in `body.html`.

- **Roller (the hold):** a deckhand rolls a barrel along the floor and back, 46 by 25, 8 frames, with a puff of dust at the heels. It stands on the cargo piles' floor line (`.bg-roll`, from the left gutter) and the whole sprite walks.
- **Sleeper (the chart room's floor, the second below deck):** the lookout, off watch, asleep in a hammock slung from two rings in the wall. 56 by 32, 12 frames: the cloth sways a pixel, the chest rises and z's float up. It hangs on the blank wall above the recording (`.bg-hammock`, 60px from the right and 36px down) and stays where it is hung.
- **Mopper (the galley's floor, the first):** a cook mopping the beam, the mop sliding out and back, a streak of wet behind it and a drip at the far end. 42 by 26, 8 frames. It stands on the beam under the floor (`.bg-mop`, 86px from the right, on the floor's bottom edge) and stays where it stands.

Each is an `.act` loop in an absolutely positioned `.bgcrew` box (`aria-hidden`, `z-index: 0`, `pointer-events: none`), so it sits behind the copy and never takes a click. The two below deck are placed against their `.floor`, which is `position: relative` for that reason. The roller is placed against `.hold`, which already was.

**The Roll Matches The Walk Rule.** The barrel lies on its side and is seen end on, a disc 15 art pixels across. The page walks the sprite 3 art pixels a frame (`pace`) and each frame the barrel turns 22.5 degrees, which on a disc of radius 7.5 is the distance it has travelled to within half a degree, so it rolls without slipping. Change the stride or the barrel's size and the turn per frame has to change with them (`STRIDE`, `DIA` and `ROLL_STEP` in `backcrew.py`). The walk runs on the sheet's own clock: `pace` takes one step per frame, `--rs` steps each way, and flips the sprite with `scaleX(-1)` at each end.

Where they show:

- **900px and up:** the roller. Below it the hold's floor is full of cargo and `.bg-roll` is `display: none`.
- **1100px and up:** the sleeper and the mopper. Below it the floors stack and have no blank wall, and both are `display: none`.
- **The roller's walk** is `--rs` frames each way. It is 20 (60 art pixels) from 1360px, 13 at 1359px and under, and 6 at 1099px and under, because the small pile stands at 38% of the width and the walk shortens with the floor it has.

**Neutral and muted.** On the dark wall and hold a crewmate's ink outline, arms and legs would be invisible, so they are drawn in muted ink (#5f5a75): quieter than the lit rooms' crew and than the divers' bright blue. No state colour anywhere, so no red, green, cyan or magenta, and the face is the plain one. Yellow is only wood (the staves, the mop pole), the hammock is sea blue and the wet is bright blue.

**Motion.** Each has its own `--dt` (0.24s for the roller, 0.5s for the sleeper, 0.28s for the mopper) and `--lag`, so nothing moves in time with another. `site.js` pauses a sprite while it is off screen (`.act.off`), and the roller's `pace` pauses with its frames because they share an element. Under `prefers-reduced-motion: reduce` every animation is off, so each shows frame 0, the still pose, and the roller stands unflipped at the left end of its walk.

### Motion

Three mechanisms, all already in `site.css`, `landing.css`, `crew.js` and `site.js`:

- **Loop (`.act`):** a sheet stepped one frame per `--dt`, with its own length and pace so nothing moves in time with another. `site.js` pauses it while it is off screen.
- **Beat (`.act1`):** a one-shot sheet. `crew.js` plays it when its room is 70% in view (after 400ms) and again when pressed. The room's face patch is hidden while it plays.
- **Ambient CSS:** bob (flotsam, one art pixel, stepped), swell (waves), slosh (slop words), swim and tail (fish), sway (kelp), scuttle (crab), rise (bubbles), lamp (the hatch glow).

**The Frame Zero Rule.** Frame 0 of every sheet is the still picture, and a beat's last frame equals its first, so it rests where it started. Under `prefers-reduced-motion: reduce` the site turns every animation off and `crew.js` does nothing, so each sheet shows frame 0.

### Phones

Scenes keep the 3px grid, so a scene is 252px wide and the hold grows. The seabed and waterline additions have their own phone rules (above), and the background crew are gone by then. The pixel grid is never scaled to fit.

## Do's and Don'ts

### Do:

- **Do** change a generator and run `python3 tools/art.py`. Look at the sheet at 4px, 3px and 8px before it ships.
- **Do** make frame 0 the still pose and the last frame equal to it for a beat.
- **Do** put the glyph and the state word beside any mood pose or state colour.
- **Do** outline crew in ink on light and tan grounds, and in bright blue on dark water.
- **Do** mark art `aria-hidden` with empty `alt`, and keep every meaning in text.
- **Do** keep sprite widths and heights whole numbers of art pixels, and check a new sheet is exactly `n` frames wide.

### Don't:

- **Don't** edit an SVG in `public/art/` or anything in `directions/`.
- **Don't** add a colour, a gradient or a translucent pixel to a sprite (a new translucent sheet needs the reason written here).
- **Don't** spend a state colour on a prop without writing the exception down here.
- **Don't** scale a sprite to a pixel size other than `--px`.
- **Don't** move a crewmate's head in a room frame.
- **Don't** draw a background crewmate's limbs in ink or give it a state colour. Ink vanishes on the dark ground, so the limbs are muted grey.
- **Don't** let a decorative element take a tab stop, or move on a scroll event.
