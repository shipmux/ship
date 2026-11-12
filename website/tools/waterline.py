"""The waterline's additions: a cook in a dinghy netting slop words, and two more things adrift.

    dinghy         the one-shot beat: the cook swings a net down to a floating "10x", scoops it, lifts it over the boat
                   and the word dissolves into the bucket; the next one drifts in. 13 frames, 78 by 46. Frame 0 and the
                   last frame are the boat at rest, so a press plays it once (.act1, crew.js) and the page reads the same
                   without it. The boat bobs with the swell: that is the page's own bob animation, not drawn here.
    flot-pr        a crate stencilled "pr", adrift with the rest of the flotsam.
    flot-refactor  a plank with a pull request's title on it, in the same stencil as the "10x" sign.

Same palette and rules as the rest of the art: whole art pixels, the sixteen colours. The crew is neutral, and the cook's
head never moves, so the blink and glance patches line up. Nothing here carries a state.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import crewrooms as cr  # noqa: E402
from crewrooms import Canvas, PALETTE  # noqa: E402
import scenery as sc  # noqa: E402

W, H = 78, 38
COOK_X, COOK_TOP = 8, 4                 # the cook's sprite: left edge, and the row its top is on
BOAT_Y = 20                             # the boat's top row, just under the cook's face; his legs are behind its side
SURFACE = 26                            # the row the water's surface is on, where the page's wave line meets the sprite
TILE0 = (58, 28)                        # where the next slop word floats: the centre of its board
BUCKET = (37, 14)                       # the bucket in the boat, top left


def board(word, w=None):
    """A plank a word is stencilled on: ink edge, yellow wood, the word in ink. 9 high."""
    tw = len(word) * 4 - 1
    w = w or tw + 4
    c = Canvas(w, 9)
    c.rect(0, 0, w, 9, "0")
    c.rect(1, 1, w - 2, 7, "3")
    c.rect(1, 1, w - 2, 1, "b")
    c.text((w - tw) // 2, 2, word, "0")
    return c


def hull(c):
    """A dinghy seen from the side: a raised bow and stern, ink planking, yellow strakes, a bright rail."""
    insets = [0, 0, 1, 2, 3, 5, 7, 10]
    for r, inset in enumerate(insets):
        y = BOAT_Y + r
        x0, x1 = 3 + inset, 44 - inset
        c.rect(x0, y, x1 - x0, 1, "0")
        if r not in (0, 7):
            c.rect(x0 + 1, y, x1 - x0 - 2, 1, "3")
    c.rect(4, BOAT_Y, 40, 1, "0")
    c.rect(5, BOAT_Y + 1, 38, 1, "b")                                       # the rail
    c.rect(8, BOAT_Y + 4, 32, 1, "0")                                       # a strake
    for x in (12, 21, 30, 38):
        c.set(x, BOAT_Y + 2, "0")
    c.rect(2, BOAT_Y - 2, 3, 2, "0"); c.rect(43, BOAT_Y - 2, 3, 2, "0")     # the stem and the sternpost


def bucket(c):
    x, y = BUCKET
    c.rect(x, y, 10, 7, "0")
    c.rect(x + 1, y + 1, 8, 5, "8")
    c.rect(x + 1, y + 1, 8, 1, "7")
    c.rect(x + 3, y - 1, 4, 1, "0")                                          # the handle


def net(c, hx, hy, cx, cy):
    """The net on its pole: a pole from the cook's hand to the ring, the ring in brass, and a bag of mesh under it."""
    pole = Canvas(c.w, c.h)
    pole.line(hx, hy, cx, cy, "7")
    pole.line(hx + 1, hy, cx + 1, cy, "7")
    pole.outline("0")
    c.blit(pole, 0, 0)
    ring = Canvas(c.w, c.h)
    ring.ellipse(cx, cy, 8, 4, "3")
    for yy in range(cy - 3, cy + 6):                                         # the mesh: a checker, holes between
        for xx in range(cx - 7, cx + 8):
            if ((xx - cx) / 7.5) ** 2 + ((yy - cy - 1) / 5.5) ** 2 <= 1 and (xx + yy) % 2 == 0:
                ring.set(xx, yy, "f")
    ring.ellipse(cx, cy - 1, 6, 2, None)
    ring.outline("0")
    c.blit(ring, 0, 0)


def dissolve(layer, k):
    """Dithering is how a finished thing leaves the page: k 1 keeps half the pixels, k 2 keeps a quarter."""
    for (x, y) in list(layer.px):
        if k == 1 and (x + y) % 2:
            del layer.px[(x, y)]
        elif k == 2 and (x % 2 or y % 2):
            del layer.px[(x, y)]


def dinghy_frame(ring, tile=None, gone=0, arm="out"):
    """ring: the net's ring (cx, cy). tile: where the "10x" board is (its centre), or None. gone: 0 whole, 1 and 2 more
    dithered away (dissolving into the bucket, or arriving again)."""
    c = Canvas(W, H)
    bucket(c)
    hands = cr.person(c, "cook", COOK_X, left="down", right=arm, dy=COOK_TOP - (cr.FY - 20))
    hull(c)
    hx, hy = hands["r"]
    if tile is not None:
        layer = Canvas(W, H)
        layer.blit(board("10x"), tile[0] - 6, tile[1] - 4)
        dissolve(layer, gone)
        c.blit(layer, 0, 0)
    net(c, hx, hy, *ring)
    return c


# net ring centre, board centre or None, how dithered the board is
DINGHY = [
    ((52, 5), TILE0),
    ((55, 12), TILE0),
    ((58, 22), TILE0),
    ((58, 28), (58, 29)),
    ((56, 22), (56, 23)),
    ((50, 12), (50, 13)),
    ((43, 9), (43, 10)),
    ((43, 9), (41, 16), 1),
    ((46, 8), (41, 18), 2),
    ((50, 6), None),
    ((52, 5), TILE0, 2),
    ((52, 5), TILE0, 1),
    ((52, 5), TILE0),
]


def dinghy():
    return [dinghy_frame(*f) for f in DINGHY]


DINGHY_EYES = cr.face_xy(COOK_X, dy=COOK_TOP - (cr.FY - 20))


# ---- two more things adrift --------------------------------------------------------------------------------

def flot_pr():
    """A crate with "pr" on it, adrift like any other: the pull request nobody read to the end."""
    return sc.crate(14, 14, "pr")


def flot_refactor():
    """A pull request's title on a plank, two lines, on a post. Same wood and stencil as the "10x" sign."""
    c = Canvas(43, 18)
    c.rect(0, 0, 43, 15, "0")
    c.rect(1, 1, 41, 13, "3")
    c.rect(1, 1, 41, 1, "b")
    c.text(2, 2, "refactor", "0")
    c.text(2, 8, "everything", "0")
    c.rect(20, 15, 3, 3, "0")
    return c


SPRITES = {"flot-pr": flot_pr, "flot-refactor": flot_refactor}
ROOMS = {"beat-dinghy": dinghy}


def build(out):
    written = []
    for name, fn in ROOMS.items():
        with open(os.path.join(out, name + ".svg"), "w") as f:
            f.write(Canvas.sheet(fn()).svg(unit=1, decorative=True))
        written.append(name + ".svg")
    for name, fn in SPRITES.items():
        with open(os.path.join(out, name + ".svg"), "w") as f:
            f.write(fn().svg(unit=1, decorative=True))
        written.append(name + ".svg")
    return written


if __name__ == "__main__":
    for n in build(os.path.join(HERE, "..", "public", "art")):
        print(n)
