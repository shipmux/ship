"""The sprites for the Optimal-ish post's link preview (public/og-optimal-ish.png, see og_optimal_ish.py): two sailboats
racing, and the finish. Same canvas, palette and rules as the rest of the art (the crewmate comes from crewrooms.py, which
imports the frozen ansi tools; none of them is edited). Change this file and run it, don't edit the SVGs.

    python3 tools/race.py

    race-ship.svg     the boat that is behind: a yellow hull, a paper sail with a yellow pennant and "ship" stencilled
                      on the jib, the captain under the boom. 64 by 58.
    race-rival.svg    the boat ahead: the same lines in muted ink, a cream sail and nobody on it, and no name. 64 by 58.
    race-pole.svg     the finish: a yellow pole with a chequered flag, standing on a float. 24 by 54.

Frame 0 only, nothing moves. Props spend no state colour: ink, paper, cream, muted ink and yellow wood.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import crewrooms as cr  # noqa: E402
from crewrooms import Canvas  # noqa: E402

OUT = os.path.join(HERE, "..", "public", "art")
W, H = 64, 58
HULL_Y = 44               # the hull's top row
MAST_X = 30               # the mast's left column (it is 3 wide)
BOOM_Y = 38               # the row the sails' feet are on


def tri_main(top, foot, x_mast, x_tail):
    """The mainsail: a right triangle hung behind the mast, apex at the masthead, widest at the boom."""
    c = Canvas(W, H)
    for y in range(top, foot + 1):
        w = round((x_mast - x_tail) * (y - top) / (foot - top))
        c.rect(x_mast - w, y, w, 1, "f")
    return c


def tri_jib(top, foot, x_mast, x_bow):
    """The jib: a right triangle in front of the mast, apex at the masthead, tack at the bow."""
    c = Canvas(W, H)
    for y in range(top, foot + 1):
        w = round((x_bow - x_mast) * (y - top) / (foot - top))
        c.rect(x_mast, y, w, 1, "f")
    return c


def hull(c, body, rail):
    """The cook's dinghy hull, drawn longer: raised bow and stern, ink planking, a rail and one strake."""
    insets = [0, 0, 1, 2, 3, 5, 7, 10]
    x_l, x_r = 2, 62
    for r, inset in enumerate(insets):
        y = HULL_Y + r
        x0, x1 = x_l + inset, x_r - inset
        c.rect(x0, y, x1 - x0, 1, "0")
        if r not in (0, 7):
            c.rect(x0 + 1, y, x1 - x0 - 2, 1, body)
    c.rect(x_l + 1, HULL_Y, x_r - x_l - 2, 1, "0")
    c.rect(x_l + 2, HULL_Y + 1, x_r - x_l - 4, 1, rail)
    c.rect(x_l + 6, HULL_Y + 4, x_r - x_l - 12, 1, "0")
    for x in range(x_l + 10, x_r - 8, 9):
        c.set(x, HULL_Y + 2, "0")
    c.rect(x_l - 1, HULL_Y - 2, 3, 2, "0")          # the stem and the sternpost
    c.rect(x_r - 2, HULL_Y - 2, 3, 2, "0")


def rig(sail, flag, name=None):
    c = Canvas(W, H)
    sails = Canvas(W, H)
    main = tri_main(3, BOOM_Y, MAST_X, 4)
    jib = tri_jib(8, BOOM_Y, MAST_X + 3, 58)
    sails.blit(main, 0, 0)
    sails.blit(jib, 0, 0)
    sails = sails.mapped({"f": sail})
    sails.outline("0")
    c.blit(sails, 0, 0)
    pennant = Canvas(W, H)                           # a pennant streaming aft from the masthead, tapering to a point
    for r, w in enumerate((7, 5, 3)):
        pennant.rect(MAST_X - w, 2 + r, w, 1, flag)
    pennant.outline("0")
    c.blit(pennant, 0, 0)
    c.rect(MAST_X, 1, 3, HULL_Y - 1, "0")           # the mast: ink with a stripe of wood
    c.rect(MAST_X + 1, 2, 1, HULL_Y - 2, "3")
    c.rect(MAST_X - 8, BOOM_Y + 1, 9, 1, "0")       # the boom reaches aft of the sail's foot
    c.rect(2, BOOM_Y + 1, MAST_X - 1, 1, "0")
    c.rect(MAST_X + 3, BOOM_Y + 1, 26, 1, "0")      # the jib's foot
    if name:
        c.text(MAST_X + 5, BOOM_Y - 7, name, "0")     # on the jib, where the captain is not in front of it
    return c


def ship():
    c = Canvas(W, H)
    c.blit(rig("f", "b", "ship"), 0, 0)
    cr.person(c, "captain", 6, left=None, right=None, dy=HULL_Y + 2 - (cr.FY - 20) - 20)
    hull(c, "3", "b")
    return c


def rival():
    c = Canvas(W, H)
    c.blit(rig("7", "8"), 0, 0)
    hull(c, "8", "7")
    return c


def pole():
    """The finish pole: a yellow pole in ink, a chequered flag streaming right in a stepped wave, a float at the foot."""
    w, h = 24, 54
    c = Canvas(w, h)
    c.rect(2, 2, 3, h - 6, "0")
    c.rect(3, 2, 1, h - 6, "3")
    c.rect(0, h - 5, 9, 4, "0")                     # the float
    c.rect(1, h - 4, 7, 2, "3")
    c.rect(1, h - 4, 7, 1, "b")
    flag = Canvas(w, h)
    fw, fh = 18, 12
    for y in range(fh):
        for x in range(fw):
            dy = 1 if (x // 6) % 2 else 0           # a stepped ripple along the flag
            col = "f" if ((x // 3) + (y // 3)) % 2 == 0 else "0"
            flag.set(5 + x, 3 + y + dy, col)
    flag.outline("0")
    c.blit(flag, 0, 0)
    return c


def build(out=OUT):
    sprites = {"race-ship.svg": ship(), "race-rival.svg": rival(), "race-pole.svg": pole()}
    for name, canvas in sprites.items():
        with open(os.path.join(out, name), "w") as f:
            f.write(canvas.svg(decorative=True))
    return list(sprites)


if __name__ == "__main__":
    for name in build():
        print(name)
