"""One-shot beats for the crew, and the props that play over the crew who carry a state.

Drawn with the ansi direction's generators (imported from directions/ansi/tools, never edited), written
straight into public/art/ by tools/art.py. Run that, don't edit the SVGs.

A beat sheet is a row of 36 by 28 frames, drawn like the deck crew's loops (the 28 by 24 crewmate stands at (4, 4)).
Frame 0 is the resting pose and the last frame is close to it, so the page reads the same with motion off, and
a press plays the sheet once (.act1 in landing.css). Every beat has the same shape: a squat to get ready, the thing
the role does, and a settle back into the rest pose. Each sheet also has a night twin whose outline is bright blue,
as the diver's is, because ink on the night deck is lost.

The fx sheets are the props that play over the five crew who stand in for agents (and the diver). They never replace the
crewmate's face, so the glyph, the colour and the word beside it keep saying the state.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "directions", "ansi", "tools"))
from pixel import Canvas, PALETTE  # noqa: E402
from crew import crewmate  # noqa: E402
import scene  # noqa: E402

FW, FH, OX, OY = 36, 28, 4, 4
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
    return written
