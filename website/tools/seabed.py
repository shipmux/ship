"""The seabed's additions: a second diver with a lantern, and the chest it has found.

    seabed-chest   a diver holds a lantern over a sunken chest; the lantern's light flickers, the chest glints and
                   a few bubbles rise from the helmet. 8 frames, 64 by 38, an ambient loop (.act): frame 0 is the
                   still scene, so with motion off the page reads the same.

Same palette and rules as the rest of the art, with the seabed's own rule from scenery.py: no state colour is spent
on scenery that is not a crewmate, so the chest is wood (yellow), iron (greys) and light, and its lining is grey.
Ink outlines vanish on dark water, so the diver's are bright blue, as the first diver's are. This diver has no hose: a
hose is a vertical line the full height of the section, and anywhere the copy is it would cross the words.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "directions", "ansi", "tools"))
from pixel import Canvas, PALETTE  # noqa: E402
from crew import crewmate  # noqa: E402
import scenery as sc  # noqa: E402

W, H = 64, 38
FLOOR = H - 3                 # the row the diver's feet stand on: the sand heaps over the two rows below
DX, DTOP = 2, FLOOR - 20      # where the diver sprite goes (its feet are 20 rows below its top)
CHEST_X, CHEST_Y = 35, FLOOR - 16   # the chest's top left; it is 28 by 17
LAMP = (DX + 25, DTOP + 12)   # the lantern's glass, in the diver's raised hand
HELMET = DTOP + 4             # the top row of the brass helmet: the sprite's body starts five rows down and the hat rises one


def chest():
    """An open sea chest seen from the front: iron-bound wood, the lid thrown back, light pouring out of it."""
    c = Canvas(28, 17)
    # the lid, thrown back: its inner face, with an iron band and the wood showing at the edges
    c.rect(3, 0, 22, 8, "0")
    c.rect(4, 1, 20, 6, "8")
    c.rect(4, 1, 20, 1, "7")
    c.rect(4, 6, 20, 1, "0")
    for x in (8, 19):
        c.rect(x, 1, 2, 5, "7")
    # the body: a front of yellow planks bound with two iron bands, a lock plate between
    c.rect(0, 8, 28, 9, "0")
    c.rect(1, 9, 26, 7, "3")
    c.rect(1, 9, 26, 1, "b")
    c.rect(1, 12, 26, 1, "0")
    for x in (4, 22):
        c.rect(x, 8, 3, 9, "0"); c.rect(x + 1, 9, 1, 7, "8")
    c.rect(12, 11, 4, 4, "0"); c.rect(13, 12, 2, 2, "b")
    # the mouth, heaped with coins: bright yellow with a white glint here and there
    for x, top in ((5, 6), (7, 5), (9, 4), (11, 4), (13, 3), (15, 4), (17, 4), (19, 5), (21, 6)):
        c.rect(x, top, 2, 8 - top, "b")
    c.rect(4, 7, 20, 1, "3")
    for x, y in ((9, 4), (13, 3), (17, 4), (11, 6), (20, 6)):
        c.set(x, y, "f")
    return c


def mound(c, x0, x1, y):
    """Sand heaped against something, in the seabed's own tan, with its dither on top: the page's sand is the same."""
    for x in range(x0, x1):
        top = y + (1 if (x - x0) % 4 in (0, 3) else 0)
        c.rect(x, top, 1, H - top, "7")
        if (x + top) % 2:
            c.set(x, top, "f")


def glow(c, cx, cy, r):
    """A dithered halo, dense near the lamp and thin at the edge, in the bright yellow."""
    for y in range(cy - r, cy + r + 1):
        for x in range(cx - r, cx + r + 1):
            d = ((x - cx) ** 2 + (y - cy) ** 2) ** 0.5
            if d < r * 0.4 and (x % 2 == 0 or y % 2 == 0):
                c.set(x, y, "b")
            elif d < r * 0.7 and (x + y) % 2 == 0:
                c.set(x, y, "b")
            elif d < r and x % 2 == 0 and y % 2 == 0:
                c.set(x, y, "b")


def diver():
    """The second diver: the first diver's brass helmet, with a white band where his is blue, so the two are not twins."""
    d = crewmate("diver", None, pose="lantern", outline="c")
    for x in range(8, 20):
        d.set(x, 8, "f")
    return d


def frame(radius, mouth, glint, bubbles):
    c = Canvas(W, H)
    glow(c, LAMP[0] + 1, LAMP[1] + 1, radius)
    c.blit(diver(), DX, DTOP)
    glow(c, CHEST_X + 14, CHEST_Y + 5, mouth)
    c.blit(chest(), CHEST_X, CHEST_Y)
    mound(c, CHEST_X - 6, W, FLOOR + 1)
    mound(c, 0, CHEST_X - 8, FLOOR + 2)
    if glint is not None:
        gx, gy = glint
        for dx, dy in ((0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)):
            c.set(gx + dx, gy + dy, "f")
    for bx, by, big in bubbles:
        b = sc.bubble() if big else Canvas(2, 2).rect(0, 0, 2, 2, "c")
        c.blit(b, bx, by)
    return c


# per frame: the lantern's glow radius, the chest's glow radius, where a glint shows (or None), and the bubbles (bubbles())
GLINTS = [None, None, (CHEST_X + 9, CHEST_Y - 5), (CHEST_X + 9, CHEST_Y - 5), None, (CHEST_X + 20, CHEST_Y - 4), (CHEST_X + 20, CHEST_Y - 4), None]
RADII = [13, 14, 14, 13, 12, 13, 14, 13]
MOUTH = [8, 8, 9, 9, 8, 7, 8, 9]


def bubbles(f):
    out = []
    for off, dx in ((0, 0), (4, 1)):
        life = (f + off) % 8
        if life < 5:
            out.append((DX + 16 + dx + (life % 2), HELMET - 3 - 3 * life, life < 3))
    return out


def seabed_chest():
    return [frame(RADII[f], MOUTH[f], GLINTS[f], bubbles(f)) for f in range(8)]


# ---- fish for the water around it ------------------------------------------------------------------------

def fish_glow():
    """A yellow fish that swims through the lantern's light, the one warm fish. Two frames, tail up and tail down, 12 by 6,
    the same frame as the blue fish so it can use the page's own tail loop."""
    frames = []
    for up in (True, False):
        c = Canvas(12, 6)
        c.ellipse(5, 3, 4, 1.5, "3")
        c.rect(3, 2, 5, 1, "b")
        c.set(9, 2 if up else 4, "b"); c.set(10, 1 if up else 5, "b"); c.set(10, 2 if up else 4, "b")
        c.set(10, 3, "b"); c.set(11, 1 if up else 5, "b")
        c.set(3, 2, "0")
        frames.append(c)
    return frames


def fish_school():
    """Three small fish that keep together, 20 by 9, two frames: each flicks its tail on the other beat."""
    spots = ((0, 3), (7, 0), (9, 5))
    frames = []
    for flick in (0, 1):
        c = Canvas(20, 9)
        for i, (x, y) in enumerate(spots):
            up = (i + flick) % 2
            c.rect(x + 2, y + 1, 5, 2, "c")
            c.set(x + 3, y, "c"); c.set(x + 3, y + 3, "c")
            c.set(x + 1 + 0, y + 1 + up, "c")
            c.set(x + 7, y + up, "c"); c.set(x + 7, y + 2 + (1 - up), "c")
            c.set(x + 5, y + 1, "f")
        frames.append(c)
    return frames


ART = {"seabed-chest": seabed_chest, "fish-glow": fish_glow, "fish-school": fish_school}


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
