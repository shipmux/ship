"""Background crew, samples: little neutral crewmates doing small jobs in the dark behind the copy, the way the deck's
navigator rides the ratlines. Not on the page yet: this file only draws the sheets, for review.

    crew-bg-barrel   the hold: a deckhand rolls a barrel along the floor. 8 frames, 46 by 25, a loop (.act). The barrel
                     lies on its side and is seen end on, a disc that turns 22.5 degrees a frame while the page walks
                     the whole sprite 3 art pixels a frame (3 / 7.5 radians: it rolls without slipping), to and fro
                     along the floor, flipped at each end. Frame 0 to 7 is two strides and a half turn of the planks.
    crew-bg-hammock  below deck: the lookout, off watch, asleep in a hammock slung from two rings in the wall, the cloth
                     swaying a pixel, the chest rising and z's floating up. 12 frames, 56 by 32, a loop (.act) that stays
                     where it is hung.
    crew-bg-mop      below deck: a cook mops a beam, the mop sliding out and back, a streak of wet behind it and a drip
                     at the far end. 8 frames, 42 by 26, a loop (.act) that stays where it stands.

Same palette and rules as the rest of the art: whole art pixels, the sixteen colours, no gradients, and the crew are
neutral (the plain face, and no red, green, cyan or magenta anywhere). Yellow is only wood, as on the cargo pile (the
staves, the mop pole); the hammock is sea blue and the wet is bright blue.

Ground: these stand on the dark wall and hold, where ink vanishes. A crewmate's ink outline, arms and legs would be
invisible there, so they are drawn in the muted grey ("8") instead: quieter than the lit rooms' crew and than the
divers' bright blue, which suits a figure that is part of the background. Frame 0 of every sheet is the still pose,
so with motion off (or before the sheet loads) the page reads the same.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import crewrooms as cr  # noqa: E402  (the neutral crew and the shared palette; it puts directions/ansi/tools on the path)
from crewrooms import Canvas  # noqa: E402
from crew import crewmate  # noqa: E402
from pixel import FONT3x5  # noqa: E402

LIMB = "8"  # the key for what is ink on a light ground: outline, arms, legs


def figure(hat, left="down", right="down", gait=0, face=None):
    """A neutral crewmate for a dark ground: 28 by 24, drawn the way crewrooms.person() does it, with the limbs in LIMB."""
    s = crewmate(hat, None, pose="bare", outline=LIMB, fx={"gait": gait})
    for p in cr.LEFT_ARM:
        s.px.pop(p, None)
    for side, name in (("l", left), ("r", right)):
        if name:
            for x, y in cr.arm_pts(name, side):
                s.set(x, y, LIMB)
    if face:
        screen = Canvas(10, 6)
        screen.blit_map(face, 0, 0, {"#": "f"})
        s.rect(9, 10, 10, 6, "0")
        s.blit(screen, 9, 10)
    return s


# ---- the hold: a barrel rolled along the floor ---------------------------------------------------------

BW, BH = 46, 25
FLOOR = BH - 1         # the row the feet stand on: the last row, so the page can set the sprite on the floor line
BARREL_X = 30          # the barrel's left edge: the hand's tip is at x 30, so the barrel is pushed, not reached for
CREW_X, CREW_Y = 6, FLOOR - 20
DIA = 15               # the barrel, lying on its side and seen end on, is a disc 15 pixels across with a middle pixel
ROLL_STEP = 22.5       # degrees it turns each frame. The head's three planks look the same every half turn, so eight
                       # frames are a whole loop of the sheet
STRIDE = 3             # art pixels the whole sprite travels each frame. Rolling without slipping turns a disc of radius
                       # r by STRIDE / r radians, and 3 / 7.5 radians is 22.5 degrees, so the page walks the sprite
                       # STRIDE pixels per frame and the barrel turns exactly as far as it has gone


def keg(turn):
    """The barrel end on, 15 across: iron-grey rim, yellow bilge, a cream hoop and the head of three planks. The planks
    are parallel seams that turn about the middle by `turn` degrees. The light (the bright arc, top left) does not turn,
    so the whole disc reads as a round thing rolling and not as a pattern spinning on a flat one."""
    import math
    c = Canvas(DIA, DIA)
    mid = DIA // 2
    th = math.radians(turn)
    for y in range(DIA):
        for x in range(DIA):
            dx, dy = x - mid, y - mid
            r = math.hypot(dx, dy)
            if r > 7.4:
                continue
            if r > 6.5:
                c.set(x, y, "8")
            elif r > 5.3:
                c.set(x, y, "3")
            elif r > 4.3:
                c.set(x, y, "7")
            else:
                u = -dx * math.sin(th) + dy * math.cos(th)
                c.set(x, y, "0" if abs(abs(u) - 1.6) < 0.55 else "b")
    for x, y in ((3, 3), (4, 2), (2, 4)):            # the light, on the bilge
        c.set(x, y, "b")
    return c


def dust(c, x, phase):
    """A puff kicked up behind the heels: bright specks that spread and rise."""
    spec = {
        1: ((0, 0), (1, 0)),
        2: ((-1, -1), (1, 0), (2, -1)),
        3: ((-2, -2), (0, -1), (3, -2)),
    }[phase]
    for dx, dy in spec:
        c.set(x + dx, FLOOR - 1 + dy, "7")


# gait (0 both feet down, 1 and 2 a foot lifted), dust phase. A walk cycle is four frames, so the sheet is two strides.
STEPS = [(0, 0), (1, 1), (0, 2), (2, 3), (0, 0), (1, 1), (0, 2), (2, 3)]


def barrel_frame(i):
    gait, puff = STEPS[i]
    c = Canvas(BW, BH)
    c.blit(figure("deckhand", left="back" if gait == 1 else "down", right="reach", gait=gait), CREW_X, CREW_Y)
    c.blit(keg(i * ROLL_STEP), BARREL_X, FLOOR - DIA + 1)
    if puff:
        dust(c, CREW_X - 2, puff)
    return c


def barrel():
    return [barrel_frame(i) for i in range(8)]


# ---- below deck: asleep in a hammock -----------------------------------------------------------------------

HW, HH = 56, 32
LX, RX, EY, SAG = 8, 47, 18, 9     # the cloth's two ends, their row, and how far it hangs between them
RING_Y = 10                        # the iron rings on the wall the ropes hang from, at x 3 and 52


def cloth_y(x):
    t = (x - (LX + RX) / 2.0) / ((RX - LX) / 2.0)
    return EY + int(round(SAG * (1 - t * t)))


def zed(c, x, y):
    """A z in bright white, in the 3 by 5 face."""
    for dy, row in enumerate(FONT3x5["z"].split("|")):
        for dx, v in enumerate(row):
            if v == "#":
                c.set(x + dx, y + dy, "f")


SLEEP_FACE = """
..........
..........
.###..###.
..........
....##....
..........
"""


def sleeper():
    """The crewmate's box and cap with its eyes shut, cropped to the 18 rows that show above the cloth: its arms and legs are
    in the hammock."""
    return figure("lookout", left=None, right=None, face=SLEEP_FACE).crop(5, 0, 18, 18)


def hammock_frame(sway, breath, zs):
    c = Canvas(HW, HH)
    # the ropes, from two rings on the wall down to the cloth's ends
    for rx, ex in ((3, LX + sway), (52, RX + sway)):
        c.rect(rx - 1, RING_Y - 1, 3, 3, "7")
        c.set(rx, RING_Y, None)
        c.line(rx, RING_Y + 2, ex, EY, "7")
    # the sleeper, sunk in the cloth so its box is covered from the chest down
    mid = (LX + RX) // 2
    c.blit(sleeper(), mid - 9 + sway, cloth_y(mid) - 15 - breath)
    # the cloth, in front: a lit top edge and the blue canvas under it
    for x in range(LX + sway, RX + sway + 1):
        y = cloth_y(x - sway)
        c.set(x, y, "c")
        c.set(x, y + 1, "4")
        c.set(x, y + 2, "4")
        c.set(x, y + 3, "4")
    # z's rising from the head, one every six frames, drifting up and to the right
    for start in (0, 6):
        age = (zs - start) % 12
        if age < 8:
            zed(c, mid + 10 + age // 3 + sway, 8 - age)
    return c


def hammock():
    frames = []
    for f in range(12):
        sway = (0, 0, 0, 1, 1, 1, 0, 0, 0, -1, -1, -1)[f]
        frames.append(hammock_frame(sway, 1 if f % 4 in (2, 3) else 0, f))
    return frames


# ---- below deck: mopping the beam ------------------------------------------------------------------------

MW, MH = 42, 26
MFLOOR = MH - 1


def mop_frame(stroke, lean, drip):
    """stroke 0 to 5 is how far the mop head has slid out; lean 1 puts the crewmate's weight on the front foot."""
    c = Canvas(MW, MH)
    cx = 2 + lean
    cy = MFLOOR - 20
    # the wet the mop leaves: a dither of bright blue along the floor behind the head
    head = 30 + stroke
    for x in range(24, head - 4):
        if x % 2 == 0:
            c.set(x, MFLOOR, "c")
    c.blit(figure("cook", left="down", right="reach", gait=1 if lean else 0), cx, cy)
    # the pole, from the hand to the floor, and the head: a bright pad with a grey fringe
    top = (cx + 25 + stroke // 2, cy + 8)
    c.line(top[0], top[1], head, MFLOOR - 2, "3")
    c.rect(head - 3, MFLOOR - 1, 7, 1, "f")
    c.rect(head - 4, MFLOOR, 9, 1, "7")
    for x in range(head - 4, head + 5, 2):
        c.set(x, MFLOOR, "8")
    if drip:
        c.set(head + 5, MFLOOR - 3 + drip, "c")
    return c


def mop():
    # stroke, lean, drip: the head slides out and back, the crewmate leans into the far end, a drip falls off the pad
    seq = [(1, 0, 0), (2, 0, 0), (4, 1, 0), (5, 1, 1), (4, 1, 2), (2, 0, 0), (0, 0, 0), (0, 0, 0)]
    return [mop_frame(*f) for f in seq]


ART = {"crew-bg-barrel": barrel, "crew-bg-hammock": hammock, "crew-bg-mop": mop}


def build(out):
    written = []
    for name, fn in ART.items():
        with open(os.path.join(out, name + ".svg"), "w") as f:
            f.write(Canvas.sheet(fn()).svg(unit=1, decorative=True))
        written.append(name + ".svg")
    return written


if __name__ == "__main__":
    for n in build(os.path.join(HERE, "..", "public", "art")):
        print(n)
