"""The rooms below deck and in the hold, redrawn: bigger props, and one beat each.

The first version (directions/ansi/tools/rooms.py) was a stirring, tapping, lever-pulling loop. These are the
same five rooms (the galley is the server, the chart is tabs and panes, the telegraph is the cli, the handoff
is agents driving agents, the hold is the install) drawn on one bigger stage, 84 by 56 art pixels, each with a
beat that reads as anticipation, action and reaction:

    galley     the pot boils over: lid rattles, pops, foam and a burst of steam, the cook jumps, wipes his brow
    chart      a pane splits: the navigator winds up and taps, a line cuts the pane, the new pane lights up
    telegraph  the lever slams: the lookout hauls it back and brings it down, the bell flashes, the needle jumps
    handoff    a note crosses the room: the captain winds up, the deckhand catches it, salutes, files it
    hold       a crate is stacked: the hoist lowers it onto the pile, the cook ticks the list, it is stowed

Frame 0 is the room at rest and the last frame is the room at rest again, so the sheet can be shown as a still,
and a press plays it once (.act1 in landing.css). The crew are the neutral crewmates: they carry no state, and
their heads never move, so the page's blink, glance and eye-follow patches still line up. (The patch is hidden
while a beat plays.)

Same palette and same rules as the rest of the art: whole art pixels, the sixteen colours, no gradients. Steam and
foam are bright white with a grey outline, the one outline that reads against the tan wall.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ANSI = os.path.join(HERE, "..", "directions", "ansi", "tools")
sys.path.insert(0, ANSI)
from pixel import Canvas, PALETTE  # noqa: E402
from crew import crewmate, BX, BY, SY, R0, L0  # noqa: E402
import rooms as ansi  # noqa: E402  (the first rooms: the wall, the floor, the porthole)
import scenery as sc  # noqa: E402

W, H = 84, 56
FY = H - 6            # the row a crewmate's feet stand on: the floor's ink line is the row below
SEAM = {"s": "#c2baa9"}


def shell():
    return ansi.room(W, H)


# ---- people -------------------------------------------------------------------------------------------

# an arm is the points from the shoulder, relative to it; the left arm is the right one mirrored
ARMS = {
    "down": [(0, 0), (1, 1), (1, 2), (1, 3)],
    "reach": [(0, 0), (1, 0), (2, 0), (3, 0)],
    "up": [(0, 0), (1, -1), (1, -2), (1, -3), (1, -4)],
    "high": [(0, 0), (1, -1), (2, -2), (2, -3), (2, -4), (2, -5)],
    "out": [(0, 0), (1, 0), (2, -1), (3, -1), (4, -1)],
    "back": [(0, 0), (0, 1), (-1, 2), (-2, 2)],
    "salute": [(0, 0), (1, -1), (1, -2), (1, -3), (1, -4), (0, -5), (-1, -6), (-2, -7)],
    "wipe": [(0, 0), (1, -1), (1, -2), (1, -3), (0, -4), (-1, -5), (-2, -5)],
    "tick": [(0, 0), (1, -1), (1, -2), (1, -3)],
}
LEFT_ARM = [(L0, SY), (L0 - 1, SY + 1), (L0 - 1, SY + 2), (L0 - 1, SY + 3)]


def arm_pts(name, side):
    sx = R0 if side == "r" else L0
    sgn = 1 if side == "r" else -1
    return [(sx + sgn * dx, SY + dy) for dx, dy in ARMS[name]]


def person(c, hat, x, left="down", right="reach", dx=0, dy=0, flip=False, ox=0):
    """Stand a neutral crewmate with its sprite's left edge at x. Returns the hand points ((lx, ly), (rx, ry)) in
    room coordinates, for props. x is where the unflipped sprite would stand; a flipped one stands in the same box."""
    s = crewmate(hat, None, pose="bare")
    for p in LEFT_ARM:
        s.px.pop(p, None)
    hands = {}
    for side, name in (("l", left), ("r", right)):
        if name is None:
            continue
        pts = arm_pts(name, side)
        for p in pts:
            s.set(p[0], p[1], "0")
        hands[side] = pts[-1]
    top = FY - 20 + dy
    if flip:
        s = s.flipped()
        hands = {k: (W0 - 1 - v[0], v[1]) for k, v in hands.items()}
    c.blit(s, x + dx, top)
    return {k: (x + dx + v[0], top + v[1]) for k, v in hands.items()}


W0 = 28  # the crewmate sprite's width, for flipping


def face_xy(x, dy=0):
    """Where the page's eye patch goes for a crewmate standing at x: the screen, in room pixels."""
    return x + 9, FY - 20 + 10 + dy


# ---- small things ---------------------------------------------------------------------------------------

def blobs(c, circles, fill="f", edge="8"):
    """Steam or foam: bright discs with a grey edge, drawn on their own layer so the edge goes round the lot."""
    layer = Canvas(c.w, c.h)
    for cx, cy, r in circles:
        layer.ellipse(cx, cy, r, r, fill)
    layer.outline(edge)
    c.blit(layer, 0, 0)


def star(c, x, y, k="b", big=False):
    """A little burst of rays, for an impact."""
    for dx, dy in ((0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)):
        c.set(x + dx, y + dy, k)
    if big:
        for dx, dy in ((2, 2), (-2, 2), (2, -2), (-2, -2)):
            c.set(x + dx, y + dy, k)


def dither_rect(c, x, y, w, h, k, phase):
    for yy in range(y, y + h):
        for xx in range(x, x + w):
            if (xx + yy + phase) % 2 == 0:
                c.set(xx, yy, k)


# ---- the galley: the server -------------------------------------------------------------------------------

POT = (44, 26, 22, 12)     # x, y, w, h of the pot's body
STOVE = (38, 38, 36, 13)


def flame(c, level):
    x, y = 47, 42
    c.rect(44, 40, 14, 10, "0")                       # the fire door, open on the fire
    base = [1, 2, 3, 4][level]
    for i, (fx, h) in enumerate(((46, base), (50, base + 2), (54, base + 1))):
        c.rect(fx, 49 - h, 3, h, "3")
        c.rect(fx + 1, 49 - max(h - 1, 1), 1, max(h - 1, 1), "b")
        if h > 3:
            c.set(fx + 1, 49 - h, "b")


def galley_stove(c, level):
    sx, sy, sw, sh = STOVE
    c.rect(sx, sy, sw, sh, "0")
    c.rect(sx + 1, sy + 1, sw - 2, sh - 2, "8")
    c.rect(sx + 1, sy + 1, sw - 2, 1, "7")
    for x in range(sx + 3, sx + sw - 3, 7):                 # rivets
        c.set(x, sy + 3, "0")
    flame(c, level)
    c.rect(sx - 1, sy + sh - 2, 3, 2, "0"); c.rect(sx + sw - 2, sy + sh - 2, 3, 2, "0")


def galley_pipe(c):
    c.rect(68, 0, 5, 38, "0")
    c.rect(69, 0, 3, 38, "8")
    c.rect(69, 0, 1, 38, "7")
    c.rect(66, 14, 9, 2, "0"); c.rect(66, 15, 9, 1, "3")         # a bracket


def galley_wall(c):
    c.rect(4, 8, 30, 2, "0"); c.rect(4, 8, 30, 1, "3")              # a rail with the cook's tools
    for hx in (8, 18, 28):
        c.rect(hx, 4, 1, 4, "0")
    c.rect(7, 10, 3, 9, "0"); c.rect(8, 11, 1, 7, "3")                # a ladle
    c.ellipse(8.5, 21, 2, 2, "0"); c.ellipse(8.5, 21, 1, 1, "3")
    c.ellipse(19, 17, 4, 4, "0"); c.ellipse(19, 17, 3, 3, "8")        # a pan
    c.rect(16, 11, 7, 1, "0"); c.rect(19, 10, 1, 2, "0")
    c.rect(27, 10, 3, 7, "0"); c.rect(28, 11, 1, 5, "f")              # a jar
    # a porthole between the rail and the pipe
    c.blit(ansi.porthole(12), 46, 4)


def galley_pot(c, jig=0, shake=0):
    px, py, pw, ph = POT
    x = px + shake
    c.rect(x, py, pw, ph, "0")
    c.rect(x + 1, py + 1, pw - 2, ph - 2, "8")
    c.rect(x + 1, py + 1, pw - 2, 2, "7")
    c.rect(x + 2, py + 3, 1, ph - 5, "7")
    c.rect(x - 3, py + 3, 3, 4, "0"); c.rect(x - 2, py + 4, 2, 2, "8")           # handles
    c.rect(x + pw, py + 3, 3, 4, "0"); c.rect(x + pw, py + 4, 2, 2, "8")


def galley_lid(c, dx, dy, tilt=0):
    """The lid, a dome with a knob, sitting on the rim (rim row 26) and lifted by dy."""
    x, y = 44 + dx + 1, 22 + dy
    c.rect(x, y + 2, 20, 2, "0"); c.rect(x + 1, y + 2, 18, 1, "7")
    c.rect(x + 2, y, 16, 2, "0"); c.rect(x + 3, y + 1, 14, 1, "8")
    c.rect(x + 8, y - 2, 4, 2, "0"); c.rect(x + 9, y - 1, 2, 1, "7")
    if tilt:
        c.rect(x + 17, y + 2 + (1 if tilt > 0 else -1), 3, 1, "0")


def galley_frame(f, level, lid, foam, spill, steam, cook, shake=0, clang=False):
    c = shell()
    galley_wall(c)
    galley_pipe(c)
    galley_stove(c, level)
    # the cook, then the pot in front of nothing: the cook stands left of the stove
    cx = 4
    pose = {"rest": ("down", "up"), "startle": ("up", "high"), "hop": ("high", "high"),
            "wipe": ("down", "wipe"), "wipe2": ("down", "salute")}[cook[0]]
    hands = person(c, "cook", cx, left=pose[0], right=pose[1], dx=cook[1], dy=cook[2])
    # the ladle: upright in the right hand
    hx, hy = hands["r"]
    if cook[0] in ("rest", "startle", "hop"):
        c.rect(hx, hy - 10, 1, 11, "0")
        c.rect(hx - 1, hy - 14, 3, 4, "0"); c.rect(hx, hy - 13, 1, 2, "3")
    galley_pot(c, shake=shake)
    px, py, pw, ph = POT
    # foam over the rim and down the sides
    if foam:
        circles = [(px + 4 + shake, py - 1, 2 + foam // 3), (px + 9 + shake, py - foam // 2, 2 + foam // 2),
                   (px + 15 + shake, py - foam // 2 - 1, 2 + foam // 2), (px + 19 + shake, py - 1, 2 + foam // 3)]
        blobs(c, [(a, b, r) for a, b, r in circles if r > 0])
    for i in range(spill):
        # drips run down the pot's two sides onto the stove
        for sx in (px - 1 + shake, px + pw + shake):
            c.rect(sx, py + 2, 1, 3 + 3 * i, "f")
        c.set(px + 4 + shake, py + 6 + 3 * i, "f"); c.set(px + pw - 5 + shake, py + 5 + 3 * i, "f")
    if lid is not None:
        galley_lid(c, *lid)
    if clang:
        star(c, 54, 18, "b", True)
    puffs = [(55, 20, 0, 3), (49, 20, 0, 2), (61, 20, 1, 2), (55, 15, 1, 4), (47, 14, 2, 3), (63, 14, 2, 3), (55, 10, 3, 4),
             (42, 10, 3, 2), (67, 11, 4, 2)]
    circles = []
    for x, y, t0, rmax in puffs:
        k = steam - t0
        if 0 <= k < 7:
            r = [1, max(rmax - 1, 1), rmax, rmax, max(rmax - 1, 1), 1, 1][k]
            circles.append((x, y - k, r))
    if circles:
        blobs(c, circles)
    return c


GALLEY_EYES = face_xy(4)

# (level, lid (dx, dy) or None, foam, spill, steam clock (-1 none), cook (pose, dx, dy), shake, clang)
G_REST = (0, (0, 0), 0, 0, -1, ("rest", 0, 0), 0, False)
GALLEY = [
    G_REST,
    (1, (0, -1), 0, 0, -1, ("rest", 0, 0), 0, False),
    (2, (0, 0), 0, 0, -1, ("rest", 0, 0), 0, False),
    (3, (0, -1), 0, 0, -1, ("rest", 0, 0), 1, False),
    (3, (0, 0), 1, 0, -1, ("rest", 0, 0), -1, False),
    (3, (0, -2), 2, 0, -1, ("rest", 0, 0), 1, False),
    (3, (0, -9), 4, 0, 0, ("startle", -1, 0), 0, False),
    (3, (0, -15), 6, 1, 1, ("hop", -1, -2), 0, False),
    (3, (0, -18), 7, 2, 2, ("hop", -1, -3), 0, False),
    (3, (0, -15), 7, 3, 3, ("hop", -1, -2), 0, False),
    (3, (0, -8), 6, 3, 4, ("startle", -1, -1), 0, False),
    (2, (0, 0), 4, 3, 5, ("startle", 0, 0), 0, True),
    (2, (0, 0), 3, 2, 6, ("wipe", 0, 0), 0, False),
    (1, (0, 0), 2, 1, 7, ("wipe2", 0, 0), 0, False),
    (1, (0, 0), 1, 0, 8, ("wipe2", 0, 0), 0, False),
    (1, (0, 0), 0, 0, 9, ("wipe", 0, 0), 0, False),
    (1, (0, 0), 0, 0, 10, ("rest", 0, 0), 0, False),
    (0, (0, 0), 0, 0, -1, ("rest", 0, 0), 0, False),
]


def galley():
    return [galley_frame(i, *g) for i, g in enumerate(GALLEY)]


# ---- the chart: tabs and panes -------------------------------------------------------------------------

BOARD = (30, 3, 52, 38)
LEFT_PANE = (44, 5, 18, 34)
RIGHT_PANE = (63, 5, 18, 34)


def pane(c, x, y, w, h, border, lines=(), fill="f", hide_text=False):
    c.rect(x, y, w, h, border)
    c.rect(x + 1, y + 1, w - 2, h - 2, fill)
    if hide_text:
        return
    for i, ln in enumerate(lines):
        c.rect(x + 3, y + 3 + i * 3, ln, 1, "8")


def chart_board(c):
    bx, by, bw, bh = BOARD
    c.rect(bx, by, bw, bh, "0")
    c.rect(bx + 1, by + 1, bw - 2, bh - 2, "f")
    c.rect(bx + 1, by + 1, bw - 2, 1, "7")
    # the easel legs
    c.rect(bx + 4, by + bh, 2, 51 - (by + bh), "0"); c.rect(bx + bw - 6, by + bh, 2, 51 - (by + bh), "0")
    c.rect(bx + 4, 47, 2, 1, "3"); c.rect(bx + bw - 6, 47, 2, 1, "3")
    c.rect(bx + 1, by + bh, bw - 2, 1, "3")
    # the sidebar: three tabs and the agents under them
    c.rect(bx + 12, by + 2, 1, bh - 4, "8")
    for i, w_ in enumerate((7, 5, 6)):
        c.rect(bx + 3, by + 4 + i * 3, w_, 1, "8")
    c.rect(bx + 3, by + 15, 8, 1, "8")
    for i in range(3):
        c.rect(bx + 3, by + 18 + i * 3, 3 if i % 2 else 5, 1, "8")


def chart_frame(sel, split, newpane, nav, tip, spark, dy=0):
    c = shell()
    chart_board(c)
    lx, ly, lw, lh = LEFT_PANE
    pane(c, lx, ly, lw, lh, "6" if sel == "left" else "0", (10, 7, 9, 6, 8))
    rx, ry, rw, rh = RIGHT_PANE
    if split < 4:
        pane(c, rx, ry, rw, rh, "6" if sel == "right" else "0", (8, 11, 5), fill="e" if split == 0 and sel == "right" and spark else "f")
        if split:
            y = ry + 17
            c.rect(rx + 1, y, [0, 5, 11, 16][split], 1, "0")
    else:
        pane(c, rx, ry, rw, 16, "0", (8, 6))
        by_ = ry + 18
        if newpane == "lit":
            pane(c, rx, by_, rw, 16, "6", (9,), fill="e")
        elif newpane == "pulse":
            pane(c, rx, by_, rw, 16, "6", (9,))
        elif newpane == "d1":
            pane(c, rx, by_, rw, 16, "f", (), hide_text=True)
            dither_rect(c, rx, by_, rw, 16, "6", 0)
            c.rect(rx + 2, by_ + 2, rw - 4, 12, "f")
        elif newpane == "d2":
            dither_rect(c, rx, by_, rw, 16, "7", 1)
            for yy in range(by_, by_ + 16):
                for xx in range(rx, rx + rw):
                    if (xx % 3 or yy % 3):
                        c.set(xx, yy, "f") if (xx + yy) % 2 else None
    # the navigator, and the pointer he taps with
    dx_, dyy = nav
    hands = person(c, "navigator", 4, left="down", right="out", dx=dx_, dy=dyy)
    hx, hy = hands["r"]
    c.line(hx, hy, tip[0], tip[1], "0")
    c.line(hx, hy - 1, tip[0], tip[1] - 1, "0") if abs(tip[1] - hy) < 3 and False else None
    c.rect(tip[0] - 1, tip[1] - 1, 3, 3, "0"); c.rect(tip[0], tip[1], 1, 1, "3")
    for sx, sy, k in spark:
        star(c, sx, sy, k, True)
    return c


CHART = [
    # sel, split, newpane, nav (dx, dy), pointer tip, sparks
    ("left", 0, None, (0, 0), (50, 30), []),
    ("left", 0, None, (0, 1), (38, 38), []),
    ("right", 0, None, (0, 0), (60, 22), []),
    ("right", 0, None, (1, -1), (66, 20), []),
    ("right", 0, None, (1, 0), (68, 21), [(68, 21, "b")]),
    ("right", 1, None, (1, 0), (67, 23), []),
    ("right", 2, None, (1, 0), (70, 24), []),
    ("right", 3, None, (1, 0), (73, 23), []),
    ("none", 4, "lit", (0, -2), (62, 14), [(63, 22, "b"), (80, 22, "b")]),
    ("none", 4, "lit", (0, -3), (58, 8), [(63, 38, "f"), (80, 38, "f")]),
    ("none", 4, "pulse", (0, -1), (58, 8), []),
    ("none", 4, "lit", (0, 0), (60, 12), []),
    ("none", 4, "pulse", (0, 1), (56, 18), []),
    ("none", 4, "d1", (0, 0), (52, 24), []),
    ("none", 4, "d2", (0, 0), (50, 28), []),
    ("left", 0, None, (0, 0), (50, 30), []),
    ("left", 0, None, (0, 0), (50, 30), []),
]


def chart():
    return [chart_frame(*a) for a in CHART]


CHART_EYES = face_xy(4)

# ---- the telegraph: the cli ----------------------------------------------------------------------------

import math  # noqa: E402

DIAL = (52, 22)
BELL = (72, 11)          # the top of the bell, hung from a bracket on the wall
PIVOT = (53, 41)         # where the lever is pinned to the post


def needle_tip(deg, r=7):
    a = math.radians(deg)
    return DIAL[0] + round(r * math.cos(a)), DIAL[1] + round(r * math.sin(a))


def telegraph_post(c):
    cx, cy = DIAL
    c.rect(cx - 2, cy + 10, 5, 41 - cy - 10 + 8, "0"); c.rect(cx - 1, cy + 10, 3, 41 - cy - 10 + 8, "8")
    c.rect(cx - 8, 47, 17, 4, "0"); c.rect(cx - 7, 48, 15, 2, "8"); c.rect(cx - 7, 48, 15, 1, "7")
    c.rect(PIVOT[0] - 2, PIVOT[1] - 2, 5, 5, "0"); c.rect(PIVOT[0] - 1, PIVOT[1] - 1, 3, 3, "b")


def telegraph_dial(c, deg, flash=False):
    cx, cy = DIAL
    c.ellipse(cx, cy, 11, 11, "0")
    c.ellipse(cx, cy, 10, 10, "b" if flash else "3")
    c.ellipse(cx, cy, 8, 8, "0")
    c.ellipse(cx, cy, 7, 7, "f")
    for a in (-150, -120, -90, -60, -30):
        x, y = needle_tip(a, 6)
        c.set(x, y, "8")
    tx, ty = needle_tip(deg, 6)
    c.line(cx, cy, tx, ty, "0")
    c.rect(cx - 1, cy - 1, 3, 3, "0")


BELL_ROWS = [1, 2, 3, 3, 4, 4, 5, 6]            # half widths from the crown down to the lip


def telegraph_bell(c, sway, flash):
    """The bell hangs from a hook and swings a pixel or two at the lip. A flash turns it bright yellow."""
    bx, by = BELL
    c.rect(bx, 1, 1, by - 1, "0")                                   # the chain it hangs from, up to the beam
    c.rect(bx - 2, 1, 5, 2, "0")
    for i, hw in enumerate(BELL_ROWS):
        x0 = bx + (sway if i > 3 else 0)
        c.rect(x0 - hw, by + i, 2 * hw + 1, 1, "0")
        c.rect(x0 - hw + 1, by + i, 2 * hw - 1, 1, "b" if flash else "3")
        if hw > 1:
            c.rect(x0 - hw + 1, by + i, 1 if not flash else 2, 1, "f")
    c.rect(bx - 7 + sway, by + 8, 15, 1, "0")                       # the lip
    c.rect(bx - 1 - sway, by + 9, 3, 2, "0")                        # the clapper swings the other way


def telegraph_rays(c, stage):
    """The ding: short ink rays round the bell, moving out as the sound goes."""
    if not stage:
        return
    bx, by = BELL
    cy = by + 4
    r0 = 8 + 2 * (stage - 1)
    for deg in (-150, -120, -90, -60, -30):
        a = math.radians(deg)
        x1, y1 = bx + round(r0 * math.cos(a)), cy + round(r0 * math.sin(a))
        x2, y2 = bx + round((r0 + 3) * math.cos(a)), cy + round((r0 + 3) * math.sin(a))
        c.line(x1, y1, x2, y2, "0")


def telegraph_lever(c, hy):
    px, py = PIVOT
    x1 = 31
    for x in range(x1, px):
        y = py + round((hy - py) * (px - x) / (px - x1))
        c.set(x, y - 1, "0"); c.set(x, y + 1, "0")
        c.set(x, y, "3" if x < px - 2 else "0")
    c.rect(x1 - 3, hy - 3, 4, 7, "0"); c.rect(x1 - 2, hy - 2, 2, 5, "b")


def telegraph_frame(h, deg, bell, flash, rings, look, speed=False, impact=False, cheer=False):
    c = shell()
    c.blit(ansi.porthole(12), 8, 6)
    telegraph_post(c)
    telegraph_dial(c, deg, flash=bool(flash))
    telegraph_bell(c, bell, flash)
    telegraph_rays(c, rings)
    telegraph_lever(c, h)
    dx, dy = look
    lx = 6
    hands = person(c, "lookout", lx, left="up" if cheer else "down", right=None, dx=dx, dy=dy)
    sx, sy = lx + dx + R0, FY - 20 + dy + SY
    c.line(sx, sy, 29 + dx, h, "0"); c.line(sx, sy + 1, 29 + dx, h, "0")
    c.rect(29 + dx, h - 1, 2, 3, "0")
    if speed:
        for yy in (35, 38, 41):
            c.rect(36, yy, 12, 1, "8")
            c.rect(40, yy + 1, 7, 1, "7")
    if impact:
        star(c, 33, 46, "0", True)
        c.set(33, 46, "b")
    return c


NEEDLE0, NEEDLE1 = -145, -40
TELEGRAPH = [
    # lever handle y, needle, bell sway, flash, ray stage, lookout (dx, dy), speed lines, impact, cheer
    (43, NEEDLE0, 0, 0, 0, (0, 0)),
    (40, NEEDLE0, 0, 0, 0, (0, 0)),
    (37, NEEDLE0, 0, 0, 0, (-1, 0)),
    (35, NEEDLE0, 0, 0, 0, (-1, 0)),
    (35, NEEDLE0, 0, 0, 0, (-1, 0)),
    (47, NEEDLE0, 0, 0, 0, (1, 1), True, True),
    (48, NEEDLE1 - 15, 1, 2, 1, (1, 1)),
    (46, NEEDLE1 + 12, -1, 2, 2, (0, 0)),
    (44, NEEDLE1, 2, 1, 3, (0, 0), False, False, True),
    (43, NEEDLE1 - 6, -1, 0, 3, (0, -1), False, False, True),
    (43, NEEDLE1, 1, 0, 2, (0, -1), False, False, True),
    (43, NEEDLE1, -1, 0, 1, (0, 0), False, False, True),
    (43, NEEDLE1 - 40, 0, 0, 0, (0, 0)),
    (43, NEEDLE0 + 40, 0, 0, 0, (0, 0)),
    (43, NEEDLE0 + 12, 0, 0, 0, (0, 0)),
    (43, NEEDLE0, 0, 0, 0, (0, 0)),
]


def telegraph():
    return [telegraph_frame(*t) for t in TELEGRAPH]


TELEGRAPH_EYES = face_xy(6)


# ---- the handoff: agents driving agents ----------------------------------------------------------------

CAP_X, DECK_X = 3, 48


def note(c, x, y, edge=False, stamp=False, dissolve=0):
    """A note, 8 by 10: an ink edge, white paper, three lines. Edge-on it is a 3 wide sliver, mid-spin."""
    if edge:                                                 # mid-turn: narrower, one line of text left
        c.rect(x + 1, y, 6, 10, "0")
        c.rect(x + 2, y + 1, 4, 8, "f")
        c.rect(x + 3, y + 3, 2, 1, "8"); c.rect(x + 3, y + 5, 2, 1, "8")
        return
    layer = Canvas(c.w, c.h)
    layer.rect(x, y, 8, 10, "0")
    layer.rect(x + 1, y + 1, 6, 8, "f")
    for i, ln in enumerate((4, 5, 3)):
        layer.rect(x + 2, y + 2 + i * 2, ln, 1, "8")
    if stamp:
        for dx, dy in ((0, 2), (1, 3), (2, 2), (3, 1), (4, 0)):         # a tick, in ink
            layer.set(x + 2 + dx, y + 4 + dy - 2, "0")
    if dissolve:
        for (px_, py_) in list(layer.px):
            if (px_ + py_) % 2 == dissolve % 2 or (dissolve > 1 and px_ % 2 == 0):
                del layer.px[(px_, py_)]
    c.blit(layer, 0, 0)


def corkboard(c):
    c.rect(62, 8, 17, 14, "0")
    c.rect(63, 9, 15, 12, "3")
    c.rect(63, 9, 15, 1, "b")
    for nx_, ny_ in ((64, 11), (70, 10), (73, 14)):
        c.rect(nx_, ny_, 4, 5, "0")
        c.rect(nx_ + 1, ny_ + 1, 2, 3, "f")


def handoff_frame(cap, deck, held):
    """cap and deck are (left arm, right arm, dx, dy); held lists what is drawn besides the two: notes and marks."""
    c = shell()
    c.blit(ansi.porthole(12), 28, 4)
    corkboard(c)
    cl, cr, cdx, cdy = cap
    dl, dr, ddx, ddy = deck
    ch = person(c, "captain", CAP_X, left=cl, right=cr, dx=cdx, dy=cdy)
    dh = person(c, "deckhand", DECK_X, left=dl, right=dr, dx=ddx, dy=ddy, flip=True)
    for h in held:
        kind = h[0]
        if kind == "cap":                                    # a note in the captain's right hand
            hx, hy = ch["r"]
            note(c, hx + 1, hy - 9)
        elif kind == "deck":                                 # a note in the deckhand's right hand, held out toward the captain
            hx, hy = dh["r"]
            mode = h[1] if len(h) > 1 else None
            note(c, hx - 8, hy - 9, stamp=mode == "stamp", dissolve=mode if isinstance(mode, int) else 0)
        elif kind == "fly":
            note(c, h[1], h[2], edge=h[3])
        elif kind == "trail":
            c.rect(h[1] - 7, h[2] + 3, 5, 1, "8")
            c.rect(h[1] - 9, h[2] + 6, 6, 1, "8")
        elif kind == "smack":
            star(c, h[1], h[2], "0", True)
            c.set(h[1], h[2], "b")
    return c


Z = ("down", "reach", 0, 0)
HANDOFF = [
    (Z, Z, [("cap",)]),
    (("down", "up", 0, 0), ("down", "reach", 0, -1), [("cap",)]),
    (("down", "high", -1, 0), ("down", "reach", 0, 1), [("cap",)]),
    (("down", "out", 1, 0), Z, [("fly", 33, 33, False), ("trail", 33, 33)]),
    (("down", "out", 1, 0), Z, [("fly", 36, 30, False), ("trail", 36, 30)]),
    (Z, Z, [("fly", 39, 28, True), ("trail", 39, 28)]),
    (Z, ("down", "out", 0, 0), [("fly", 42, 28, False)]),
    (Z, ("down", "out", 0, 0), [("fly", 45, 31, False)]),
    (Z, ("down", "reach", 0, -1), [("deck",), ("smack", 51, 38)]),
    (("up", "reach", 0, 1), ("salute", "reach", 0, 0), [("deck",)]),
    (("up", "reach", 0, 1), ("salute", "reach", 0, 0), [("deck", "stamp")]),
    (("down", "wipe", 0, 0), ("salute", "reach", 0, 0), [("deck", 1)]),
    (("down", "up", 0, 0), ("down", "reach", 0, 0), [("deck", 2), ("cap",)]),
    (("down", "reach", 0, 0), Z, [("cap",)]),
    (Z, Z, [("cap",)]),
    (Z, Z, [("cap",)]),
]


def handoff():
    return [handoff_frame(*a) for a in HANDOFF]


HANDOFF_EYES = face_xy(CAP_X)


# ---- the hold: the install ---------------------------------------------------------------------------------

CRATE = 12
D_X, B_X = 32, 70          # the loose crate on the floor, and the one on the pile it is stacked on
RAIL_Y = 4


def crate_at(c, x, y, keep=1.0, w=CRATE, h=CRATE):
    """A crate, whole or part dissolved: dithering is how a finished thing leaves the page (the chart's new pane does it too)."""
    layer = Canvas(c.w, c.h)
    layer.blit(sc.crate(w, h), x, y)
    if keep < 1:
        for (px_, py_) in list(layer.px):
            if keep >= 0.5 and (px_ + py_) % 2:
                del layer.px[(px_, py_)]
            elif keep < 0.5 and (px_ % 2 or py_ % 2):
                del layer.px[(px_, py_)]
    c.blit(layer, 0, 0)


def hold_pile(c):
    c.blit(sc.crate(22, 16, "cargo"), 46, FY - 15)                 # the stencilled one
    c.blit(sc.crate(CRATE, CRATE), 51, FY - 15 - CRATE)            # one on top of it
    c.blit(sc.crate(CRATE, CRATE), B_X, FY - CRATE + 1)            # the one the new crate goes on


def hold_rail(c, tx, hy, sling, crate_top, crate_x):
    c.rect(28, RAIL_Y - 1, 55, 4, "0")
    c.rect(28, RAIL_Y, 55, 2, "3")
    c.rect(28, RAIL_Y, 55, 1, "b")
    for x in range(32, 82, 8):
        c.set(x, RAIL_Y + 1, "0")
    c.rect(tx - 4, RAIL_Y + 3, 9, 4, "0")                          # the trolley on the rail
    c.rect(tx - 3, RAIL_Y + 4, 7, 2, "8")
    c.rect(tx, RAIL_Y + 7, 1, hy - RAIL_Y - 7, "0")                # the rope
    c.rect(tx - 1, hy, 3, 2, "0")                                  # the hook
    c.set(tx - 2, hy + 2, "0"); c.set(tx + 2, hy + 2, "0")
    if sling:
        c.line(tx, hy + 1, crate_x + 1, crate_top, "0")
        c.line(tx, hy + 1, crate_x + CRATE - 2, crate_top, "0")


def hold_frame(tx, hy, sling, crate, cook, stacked=None, dust=0, new=1.0, glint=0):
    """crate: (x, y); stacked: how much of the crate on the pile is left (None: not there); new: the next crate on the floor."""
    c = shell()
    c.blit(ansi.lantern(), 12, 1)
    hold_pile(c)
    if new:
        crate_at(c, D_X, FY - CRATE + 1, keep=new)
    if stacked is not None:
        crate_at(c, B_X, FY - 2 * CRATE + 1 - 0, keep=stacked)
    if crate:
        crate_at(c, crate[0], crate[1])
    hold_rail(c, tx, hy, sling, crate[1] if crate else FY - CRATE + 1, crate[0] if crate else D_X)
    ticks, dip, dy = cook
    c.blit(crewmate("cook", None, pose="clipboard", fx={"ticks": ticks, "up": dip}), 3, FY - 20 + dy)
    if dust:                                        # the crate lands: a puff off its bottom corner, thinning out
        y0 = FY - CRATE - 1
        puffs = {2: [(B_X - 3, y0), (B_X - 7, y0 - 2), (B_X - 4, y0 - 5)], 3: [(B_X - 5, y0 - 3), (B_X - 9, y0 - 6)]}[dust]
        blobs(c, [(px_, py_, 1) for px_, py_ in puffs])
    return c


HOLD_REST_Y = FY - CRATE + 1
CARRY_Y = HOLD_REST_Y - 17
TX0, TX1 = 38, 76
HOLD = [
    # trolley x, hook y, sling, crate (x, y) or None, cook (ticks, pen dipped, dy), stacked, dust, new
    (TX0, 10, False, None, (0, 0, 0), None, 0, 1.0),
    (TX0, 18, False, None, (0, 0, 0), None, 0, 1.0),
    (TX0, 31, True, (D_X, HOLD_REST_Y), (0, 0, -1), None, 0, 0),
    (TX0, 29, True, (D_X, HOLD_REST_Y - 2), (0, 0, 0), None, 0, 0),
    (TX0, 22, True, (D_X, HOLD_REST_Y - 9), (0, 0, 0), None, 0, 0),
    (TX0, 14, True, (D_X, CARRY_Y), (0, 0, 0), None, 0, 0),
    (TX0 + 8, 14, True, (D_X + 8, CARRY_Y), (0, 0, 0), None, 0, 0),
    (TX0 + 16, 14, True, (D_X + 16, CARRY_Y), (0, 0, 0), None, 0, 0),
    (TX0 + 24, 14, True, (D_X + 24, CARRY_Y), (0, 0, 0), None, 0, 0),
    (TX0 + 32, 14, True, (D_X + 32, CARRY_Y), (0, 0, 0), None, 0, 0),
    (TX1, 14, True, (B_X, CARRY_Y), (0, 0, 0), None, 0, 0),
    (TX1, 17, True, (B_X, CARRY_Y + 3), (0, 0, 0), None, 0, 0),
    (TX1, 18, False, None, (0, 0, 0), 1.0, 2, 0),
    (TX1, 12, False, None, (1, 0, 0), 1.0, 3, 0),
    (TX1, 8, False, None, (1, 1, 0), 1.0, 0, 0),
    (TX1 - 8, 8, False, None, (2, 0, 0), 0.5, 0, 0.25),
    (TX1 - 16, 8, False, None, (2, 1, 0), 0.25, 0, 0.5),
    (TX1 - 24, 8, False, None, (2, 0, 0), None, 0, 1.0),
    (TX1 - 32, 10, False, None, (0, 0, 0), None, 0, 1.0),
    (TX0, 10, False, None, (0, 0, 0), None, 0, 1.0),
]


def hold():
    return [hold_frame(*a) for a in HOLD]


HOLD_EYES = face_xy(3)

ROOMS = {
    "room-galley": (galley, [GALLEY_EYES]),
    "room-chart": (chart, [CHART_EYES]),
    "room-telegraph": (telegraph, [TELEGRAPH_EYES]),
    "room-handoff": (handoff, [HANDOFF_EYES]),
    "room-hold": (hold, [HOLD_EYES]),
}


def build(out, only=None):
    written = []
    for name, (fn, _eyes) in ROOMS.items():
        if only and name not in only:
            continue
        frames = fn()
        colors = {**PALETTE, **SEAM}
        with open(os.path.join(out, name + ".svg"), "w") as f:
            f.write(Canvas.sheet(frames).svg(unit=1, decorative=True, colors=colors))
        written.append(name + ".svg")
    return written


if __name__ == "__main__":
    for n in build(os.path.join(HERE, "..", "public", "art")):
        print(n)
