"""Sprites added to public/art/ since the ansi direction was frozen. Same canvas and palette as
directions/ansi/tools (imported from there, never edited), same rule: change this file and run it,
don't edit the SVG.

    python3 tools/art.py

face-look.svg: nine faces of a neutral crewmate on a 3 by 3 sheet, for the eyes that follow the pointer.
Column 0 looks left, 1 straight on, 2 right; row 0 looks up, 1 level, 2 down. The middle cell is the
plain neutral face, so a sheet can always rest on it.

beats.py: the crew's one-shot reactions (a press plays a sheet once) and the props that play over the deck crew.
crewrooms.py: the five rooms below deck and in the hold, each a sheet with one beat.
fx.py: the props that play over the deck crew when you press them, and the hold's other one-shot extras.
holdscenes.py: the hold's card scenes, one lit room at the head of each card (keys, config, agents, mouse, saved tabs;
install keeps crewrooms.py's hold room), and the swaying lantern they hang.
seabed.py: the seabed's additions: a second diver and the chest it has found, and two more fish.
waterline.py: the waterline's additions: the cook in a dinghy netting slop words, and a "pr" crate and a plank adrift.
backcrew.py: samples of background crew (a barrel rolled along the hold floor, a sleeper in a hammock, a mopper on a beam),
not on the page yet.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "directions", "ansi", "tools"))
from pixel import Canvas  # noqa: E402

OUT = os.path.join(HERE, "..", "public", "art")
FW, FH = 10, 6  # the screen of a crewmate: the same patch the blink and glance faces cover


def face(dx, dy):
    """The neutral face with its eyes turned (dx, dy) pixels: two 2 by 2 eyes and a small mouth.
    The mouth follows sideways, and drops a row when the eyes look down, so the face stays whole."""
    c = Canvas(FW, FH)
    c.rect(0, 0, FW, FH, "0")
    for ex in (2, 6):
        c.rect(ex + dx, 1 + dy, 2, 2, "f")
    c.rect(5 + dx, 4 + (1 if dy > 0 else 0), 2, 1, "f")
    return c


def look():
    sheet = Canvas(FW * 3, FH * 3)
    for row, dy in enumerate((-1, 0, 1)):
        for col, dx in enumerate((-1, 0, 1)):
            sheet.blit(face(dx, dy), col * FW, row * FH)
    return sheet


def build(out=OUT):
    with open(os.path.join(out, "face-look.svg"), "w") as f:
        f.write(look().svg(decorative=True))
    written = ["face-look.svg"]
    sys.path.insert(0, HERE)
    import beats  # noqa: E402
    import crewrooms  # noqa: E402
    import holdscenes  # noqa: E402
    import seabed  # noqa: E402
    import waterline  # noqa: E402
    import backcrew  # noqa: E402
    written += beats.build(out)
    written += crewrooms.build(out)
    written += holdscenes.build(out)
    written += seabed.build(out)
    written += waterline.build(out)
    written += backcrew.build(out)
    return written


if __name__ == "__main__":
    for name in build():
        print(name)
