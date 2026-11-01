"""One-shot beats for the crew, and the props that play over the crew who carry a state.

Drawn with the ansi direction's generators (imported from directions/ansi/tools, never edited), written
straight into public/art/ by tools/art.py. Run that, don't edit the SVGs.

A beat sheet is a row of 36 by 28 frames, drawn like the deck crew's loops (the 28 by 24 crewmate stands at (4, 4)).
Frame 0 is the resting pose and the last frame is close to it, so the page reads the same with motion off, and
a press plays the sheet once (.act1 in landing.css). Every beat has the same shape: a squat to get ready, the thing
the role does, and a settle back into the rest pose. Each sheet also has a night twin whose outline is bright blue,
as the diver's is, because ink on the night deck is lost.

The fx sheets are the props that play over the five crew who stand in for agents. They never replace the crewmate's
face or pose, so the glyph, the colour and the word beside it keep saying the state: the lookout's spyglass sweeps, the
navigator points, the deckhand's mop foams, the cook's parcel steams and the captain checks his clipboard. Each is 44 by
40 with the crew's own 36 by 28 frame at (4, 8), so there is room above the hat. Pressing a crewmate plays its sheet
once and lights its plate (landing.css, crew.js).

lost-lookout: the 404's lookout, standing on the buoy: the neutral lookout with the nest's spyglass loop, in one sheet.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "directions", "ansi", "tools"))
from pixel import Canvas, PALETTE  # noqa: E402
from crew import crewmate  # noqa: E402
import scene  # noqa: E402

FW, FH, OX, OY = 36, 28, 4, 4
XW, XH, XX, XY = 44, 40, 4, 8        # an fx frame, and where the crew's 36 by 28 frame sits in it
CX, CY = XX + OX, XY + OY            # where the 28 by 24 crewmate's corner lands in an fx frame
NIGHT = "c"  # the outline at night: bright blue


def stand(hat, pose=None, dx=0, dy=0, fx=None, outline=None):
    c = Canvas(FW, FH)
    c.blit(crewmate(hat, None, pose=pose, fx=fx or {}, outline=outline), OX + dx, OY + dy)
    return c


def beat_captain(outline=None):
    """The captain puts the lantern down and checks the clipboard, a line at a time, and nods."""
    steps = [("lantern", {}, 0), ("lantern", {}, 1),
             ("clipboard", {"ticks": 0, "up": 0}, -1), ("clipboard", {"ticks": 0, "up": 1}, 0),
             ("clipboard", {"ticks": 1, "up": 0}, 0), ("clipboard", {"ticks": 1, "up": 1}, 0),
             ("clipboard", {"ticks": 2, "up": 0}, 0), ("clipboard", {"ticks": 2, "up": 1}, 0),
             ("clipboard", {"ticks": 2, "up": 0}, 1), ("lantern", {}, 0)]
    return [stand("captain", pose, 0, dy, fx, outline) for pose, fx, dy in steps]


def beat_lookout(outline=None):
    """The lookout raises the spyglass and sweeps it down toward the list and up to the horizon, glinting at each end."""
    frames = [stand("lookout", "point", 0, 0, None, outline), stand("lookout", None, 0, 1, None, outline)]
    sweep = [(0, False, -1), (1, False, 0), (2, True, 0), (2, False, 0), (1, False, 0), (0, False, 0),
             (-1, False, 0), (-2, True, 0), (-1, False, 0), (0, False, 0)]
    for tilt, glint, dy in sweep:
        c = stand("lookout", None, 0, dy, None, outline)
        glass = scene.spyglass(tilt, glint=glint)
        c.blit(glass.mapped({"0": outline}) if outline else glass, OX - 4, OY + 2 + dy)
        frames.append(c)
    frames[-1] = stand("lookout", "point", 0, 0, None, outline)
    return frames


def beat_deckhand(outline=None):
    """The deckhand gives up on the clipboard and waves you over."""
    steps = [("clipboard", {"ticks": 0}, 0), ("reach", {"up": 0}, 1), ("reach", {"up": 0}, -1),
             ("reach", {"up": 1}, 0), ("reach", {"up": 2}, 0), ("reach", {"up": 1}, 0), ("reach", {"up": 2}, 0),
             ("reach", {"up": 1}, 0), ("reach", {"up": 0}, 0), ("clipboard", {"ticks": 0}, 0)]
    return [stand("deckhand", pose, 0, dy, fx, outline) for pose, fx, dy in steps]


def log_captain(outline=None):
    """The blog's captain: the docs notes' deckhand loop (long rests, then a few lines written), drawn for the captain.
    He stands on the page itself, so at night he gets the bright blue outline."""
    steps = [(0, 0)] * 6 + [(0, 1), (1, 0), (1, 1), (2, 0), (2, 1), (2, 0), (2, 0), (2, 0)]
    return [crewmate("captain", None, pose="clipboard", fx={"ticks": t, "up": d}, outline=outline) for t, d in steps]


# ---- the props over the deck crew -----------------------------------------------------------------------

def _puff(c, circles, fill="f", edge="8"):
    """Steam and foam: bright discs with a grey edge, outlined together."""
    layer = Canvas(c.w, c.h)
    for cx, cy, r in circles:
        layer.ellipse(cx, cy, r, r, fill)
    layer.outline(edge)
    c.blit(layer, 0, 0)


def _star(c, x, y, k="b", big=False):
    for dx, dy in ((0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)):
        c.set(x + dx, y + dy, k)
    if big:
        for dx, dy in ((2, 2), (-2, 2), (2, -2), (-2, -2)):
            c.set(x + dx, y + dy, k)


def fx_lookout():
    """The spyglass comes out beside the head, sweeps down and up, and glints at each end."""
    steps = [None, (0, 0), (1, 0), (2, 1), (2, 0), (0, 0), (-2, 0), (-2, 1), (-1, 0), None]
    frames = []
    for st in steps:
        c = Canvas(XW, XH)
        if st:
            c.blit(scene.spyglass(st[0], glint=bool(st[1])), CX - 4, CY + 2)
        frames.append(c)
    return frames


def fx_navigator():
    """A pointer comes out of the raised hand, taps up at the sky, then down toward the plate, and goes."""
    hx, hy = CX + 25, CY + 7                                      # the raised hand
    tips = [None, (hx + 3, hy - 3), (hx + 6, hy - 7), (hx + 6, hy - 7), (hx + 5, hy - 4), (hx + 6, hy + 3),
            (hx + 6, hy + 9), (hx + 6, hy + 9), (hx + 3, hy + 5), None]
    taps = {3, 7}
    frames = []
    for i, tip in enumerate(tips):
        c = Canvas(XW, XH)
        if tip:
            c.line(hx, hy, tip[0], tip[1], "0")
            c.line(hx, hy + 1, tip[0], tip[1] + 1, "0") if abs(tip[1] - hy) < 5 else c.line(hx + 1, hy, tip[0] + 1, tip[1], "0")
            c.rect(tip[0] - 1, tip[1] - 1, 3, 3, "0")
            c.set(tip[0], tip[1], "3")
            if i in taps:
                _star(c, tip[0] + 3, tip[1] - 2, "b")
        frames.append(c)
    return frames


def fx_deckhand():
    """The mop kicks up foam that grows, glints and pops."""
    mx, my = CX + 22, CY + 16                                      # the mop head
    grow = [
        [], [(mx + 2, my - 1, 1)], [(mx + 2, my - 3, 1), (mx + 5, my - 1, 1)],
        [(mx + 2, my - 4, 2), (mx + 6, my - 2, 1), (mx - 2, my - 2, 1)],
        [(mx + 2, my - 6, 2), (mx + 6, my - 4, 2), (mx - 2, my - 3, 1), (mx + 8, my - 1, 1)],
        [(mx + 2, my - 8, 2), (mx + 6, my - 6, 2), (mx - 2, my - 4, 1), (mx + 8, my - 3, 1)],
        None, None, None, [],
    ]
    frames = []
    for i, blobs in enumerate(grow):
        c = Canvas(XW, XH)
        if blobs:
            _puff(c, blobs)
        elif i in (6, 7, 8):                                      # the foam pops into a few bright sparks
            pts = [(mx + 2, my - 8), (mx + 7, my - 6), (mx - 2, my - 4), (mx + 9, my - 3)]
            for n, (px_, py_) in enumerate(pts):
                if i == 6:
                    _star(c, px_, py_, "b", True)
                elif i == 7:
                    _star(c, px_, py_ - 1, "b")
                elif n % 2 == 0:
                    c.set(px_, py_ - 2, "b")
        frames.append(c)
    return frames


def fx_cook():
    """Steam off the parcel: three puffs, one after another, each growing as it climbs and thinning out."""
    px_, py_ = CX + 25, CY + 7                                     # over the parcel
    frames = []
    for i in range(10):
        c = Canvas(XW, XH)
        discs = []
        for born, dx in ((1, 0), (3, 3), (5, -2)):
            k = i - born
            if 0 <= k < 5:
                r = (1, 2, 2, 2, 1)[k]
                discs.append((px_ + dx + (k // 2) * (1 if dx >= 0 else -1), py_ - 2 * k, r))
        if discs:
            _puff(c, discs)
        frames.append(c)
    return frames


def fx_captain():
    """A clipboard pops up over the hat, a line at a time is ticked, and it is put away."""
    x0, y0 = CX + 9, 1
    tick = ((2, 0), (0, 1), (2, 1), (1, 2))                        # a 3 by 3 check mark
    frames = []
    for i, ticks in enumerate([None, 0, 0, 1, 1, 2, 2, 3, 3, None]):
        c = Canvas(XW, XH)
        if ticks is not None:
            h = 11 if i > 1 else 7
            top = y0 + (11 - h)
            c.rect(x0, top, 11, h, "0")
            c.rect(x0 + 1, top + 1, 9, h - 2, "f")
            c.rect(x0 + 3, top - 1, 5, 2, "0")
            c.rect(x0 + 4, top - 1, 3, 1, "3")
            if i > 1:
                for n in range(3):
                    c.rect(x0 + 5, y0 + 2 + n * 3, 4, 1, "8")
                    if n < ticks:
                        for dx, dy in tick:
                            c.set(x0 + 1 + dx, y0 + 1 + n * 3 + dy, "0")
        frames.append(c)
    return frames


FX = {"fx-lookout": fx_lookout, "fx-navigator": fx_navigator, "fx-deckhand": fx_deckhand, "fx-cook": fx_cook,
      "fx-captain": fx_captain}


def lost_lookout(outline=None):
    """The 404's lookout: the neutral lookout at his glass, sweeping it as the nest's does (the same tilts)."""
    tilts = [0, 0, 1, 2, 2, 2, 1, 0, -1, -2, -2, -1, 0, 0]
    frames = []
    for i, t in enumerate(tilts):
        c = stand("lookout", None, 0, 0, None, outline)
        glass = scene.spyglass(t, glint=abs(t) == 2 and i % 2 == 0)
        c.blit(glass.mapped({"0": outline}) if outline else glass, OX - 4, OY + 2)
        frames.append(c)
    return frames


BEATS = {"beat-captain": beat_captain, "beat-lookout": beat_lookout, "beat-deckhand": beat_deckhand}


def build(out):
    written = []
    for suffix, outline in (("", None), ("-night", NIGHT)):
        name = "crew-captain-clipboard-loop" + suffix + ".svg"
        with open(os.path.join(out, name), "w") as f:
            f.write(Canvas.sheet(log_captain(outline)).svg(unit=1, decorative=True, colors=PALETTE))
        written.append(name)
    for name, fn in BEATS.items():
        for suffix, outline in (("", None), ("-night", NIGHT)):
            with open(os.path.join(out, name + suffix + ".svg"), "w") as f:
                f.write(Canvas.sheet(fn(outline)).svg(unit=1, decorative=True, colors=PALETTE))
            written.append(name + suffix + ".svg")
    for suffix, outline in (("", None), ("-night", NIGHT)):
        name = "lost-lookout" + suffix + ".svg"
        with open(os.path.join(out, name), "w") as f:
            f.write(Canvas.sheet(lost_lookout(outline)).svg(unit=1, decorative=True, colors=PALETTE))
        written.append(name)
    for name, fn in FX.items():
        for suffix, outline in (("", None), ("-night", NIGHT)):
            frames = fn()
            if outline:
                frames = [f.mapped({"0": outline}) for f in frames]
            with open(os.path.join(out, name + suffix + ".svg"), "w") as f:
                f.write(Canvas.sheet(frames).svg(unit=1, decorative=True, colors=PALETTE))
            written.append(name + suffix + ".svg")
    return written
