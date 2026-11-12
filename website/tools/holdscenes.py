"""The hold's card scenes: one small lit room beside each card, the way the rooms below deck sit beside theirs.

Each is the same stage as the rooms in crewrooms.py (84 by 56 art pixels, the same tan wall, floor and ink frame, so
ink outlines read although the hold around them is dark) with one beat: frame 0 is the room at rest, the last frame is
the room at rest again, and a press plays it once (.act1 in landing.css, crew.js). The crew in them are neutral, and
their heads never move, so the page's blink and glance patches always line up.

    keys           the key rack: keycaps hang from pegs; the deckhand taps one, it lights, and they all jingle
    lantern-sway   the lamp each scene hangs from its cord: a separate ambient loop (.act), so it swings whether or not
                   the beat is playing, and a still page shows it plumb. The page places it over the scene's own lantern spot.

Same palette and rules as the rest of the art: whole art pixels, the sixteen colours, no gradients.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import crewrooms as cr  # noqa: E402  (the stage, the neutral crew, and the small things a beat is made of)
from crewrooms import Canvas, PALETTE, SEAM  # noqa: E402
from pixel import FONT3x5  # noqa: E402
import rooms as ansi  # noqa: E402

cr.ARMS["poke"] = [(0, 0), (1, 0), (2, -1), (3, -1), (4, -1), (5, -1)]   # a hand held out to touch something


# ---- the keys: a rack of keycaps on pegs ---------------------------------------------------------------

def keycap(label, lit=False):
    """A keycap with its label in the 3 by 5 face: ink edge, a bright face, a shaded lip. Lit, the face is cyan."""
    tw = len(label) * 4 - 1
    w, h = max(tw + 4, 9), 10
    c = Canvas(w, h)
    c.rect(0, 0, w, h, "0")
    c.rect(1, 1, w - 2, h - 3, "e" if lit else "f")
    c.rect(1, h - 2, w - 2, 1, "6" if lit else "7")
    c.text((w - tw) // 2, 2, label, "0")
    for x, y in ((0, 0), (w - 1, 0)):                          # rounded top corners
        c.set(x, y, None)
    return c


BOARD = (32, 10, 50, 10)             # x, y, w, h of the rack's board
PEG_Y = BOARD[1] + BOARD[3]         # the row under the board where the pegs start
# label, peg x, strap length: the caps hang from their pegs, a little different in length so the row is not a ruler
CAPS = [("alt", 38, 11), ("n", 53, 7), ("q", 64, 4), ("e", 76, 6)]


def keys_rack(c, sway=(0, 0, 0, 0), lit=0):
    """sway is a sideways shift in pixels for each keycap's lower end: the strap leans, the cap follows."""
    bx, by, bw, bh = BOARD
    c.rect(bx, by, bw, bh, "0")
    c.rect(bx + 1, by + 1, bw - 2, bh - 2, "3")
    c.rect(bx + 1, by + 1, bw - 2, 1, "b")
    c.text(bx + (bw - 15) // 2, by + 3, "keys", "0")
    for nx in (bx + 2, bx + bw - 3):
        c.set(nx, by + bh - 2, "0")                                  # nails
    for i, (label, px, strap) in enumerate(CAPS):
        cap = keycap(label, lit=(lit and i == 0))
        dx = sway[i]
        c.rect(px - 1, PEG_Y, 3, 2, "0"); c.set(px, PEG_Y, "3")       # the peg
        top = PEG_Y + 1 + strap
        c.line(px, PEG_Y + 2, px + dx, top, "0")                      # the strap, leaning
        c.blit(cap, px - cap.w // 2 + dx, top)


def keys_frame(right, sway, lit=0):
    c = cr.shell()
    keys_rack(c, sway, lit)
    hands = cr.person(c, "deckhand", 4, left="down", right=right)
    if lit == 1:
        cr.star(c, hands["r"][0] + 3, hands["r"][1] - 1, "b", True)
    return c


# right arm, how far each cap swings, whether the first cap is lit
KEYS = [
    ("down", (0, 0, 0, 0)),
    ("reach", (0, 0, 0, 0)),
    ("poke", (0, 0, 0, 0)),
    ("poke", (1, 0, 0, 0), 1),
    ("poke", (1, -1, 1, 0), 1),
    ("reach", (1, 1, -1, 1), 1),
    ("down", (-1, -1, 1, -1), 1),
    ("down", (-1, 1, -1, 1), 0),
    ("down", (1, -1, 1, -1), 0),
    ("down", (0, 1, 0, 1)),
    ("down", (0, 0, 0, 0)),
    ("down", (0, 0, 0, 0)),
]


def keys():
    return [keys_frame(*k) for k in KEYS]


# ---- the lantern: one ambient loop for every hold scene ---------------------------------------------------

LANTERN_W = 13                                  # the lantern is 9 wide; two pixels either side for the swing
SWAY = [0, 1, 2, 2, 1, 0, -1, -2, -2, -1]       # how far the lantern's foot is from plumb, one step at a time


def lantern_sway():
    """The hanging lantern swinging from its cord: the cord leans from a fixed top, the lantern keeps its shape.
    Frame 0 is plumb, so a still page shows the lantern the way lantern.svg does."""
    base = ansi.lantern()
    frames = []
    for lean in SWAY:
        c = Canvas(LANTERN_W, base.h)
        for (x, y), k in base.px.items():
            if y >= 4:
                c.set(x + 2 + lean, y, k)
        for y in range(4):
            c.set(4 + 2 + round(lean * (y + 1) / 4), y, "8")
        frames.append(c)
    return frames


# ---- config: a lever on a gauge, and the window that follows ----------------------------------------------

# the 3 by 5 face has no 2 to 9; these are the same size and weight
FONT3x5.update({
    "2": "###|..#|###|#..|###", "3": "###|..#|###|..#|###", "4": "#.#|#.#|###|..#|..#",
    "5": "###|#..|###|..#|###", "6": "###|#..|###|#.#|###", "7": "###|..#|..#|..#|..#",
    "8": "###|#.#|###|#.#|###", "9": "###|#.#|###|..#|###",
})


def term_screen(c, x, y, w, h, side, lines=True):
    """A small terminal: an ink screen with Ship's sidebar down the left (the selected tab in the muted purple) and
    a pane beside it. side is the sidebar's width in pixels."""
    c.rect(x, y, w, h, "0")
    c.rect(x + 1, y + 1, w - 2, h - 2, "0")
    c.rect(x + 1, y + 1, side, h - 2, "0")
    c.rect(x + 1 + side, y + 1, 1, h - 2, "8")                       # the divider
    if lines:
        for i, ln in enumerate((5, 4, 6, 3)):
            if i == 2:
                c.rect(x + 1, y + 3 + i * 3 - 1, side, 3, "8")      # the selected tab
                c.rect(x + 3, y + 3 + i * 3, min(ln, side - 3), 1, "f")
            else:
                c.rect(x + 3, y + 3 + i * 3, min(ln, side - 3), 1, "8")
        mx = x + side + 4
        c.rect(mx, y + 3, 10, 1, "e")                                  # the file's comment line
        c.rect(mx, y + 6, 7, 1, "f")                                   # [client]
        c.rect(mx, y + 9, 12, 1, "e")                                  # sidebar_width = 34
        c.rect(mx + 13, y + 9, 2, 1, "b")


def config_frame(arm, hx, value, grab=False, spark=False):
    c = cr.shell()
    side = round(9 + (value - 23) * 7 / 11)
    term_screen(c, 34, 5, 46, 27, side)
    # the rail the lever's knob slides on, with its ticks
    c.rect(34, 43, 46, 2, "0")
    for x in range(36, 80, 4):
        c.set(x, 41, "0")
    # the readout: a small ink box with the number in yellow
    c.rect(66, 34, 14, 9, "0")
    c.text(69, 36, str(value), "b")
    # the knob
    c.rect(hx - 3, 39, 7, 10, "0")
    c.rect(hx - 2, 40, 5, 8, "3")
    c.rect(hx - 2, 40, 5, 1, "b")
    c.rect(hx - 1, 43, 3, 2, "0")
    hands = cr.person(c, "captain", 4, left="down", right=None if arm == "line" else arm)
    if arm == "line":                                                 # the arm runs out along the rail to the knob
        sx, sy = 4 + cr.R0, cr.FY - 20 + cr.SY
        c.line(sx, sy, hx - 4, 43, "0"); c.line(sx, sy + 1, hx - 4, 44, "0")
        c.rect(hx - 5, 42, 2, 3, "0")
    if spark:
        cr.star(c, 33 + side, 17, "b", True)
    return c


# arm ("down", "poke" or "line" out to the knob), knob x, the number it reads
CONFIG = [
    ("down", 36, 23),
    ("poke", 36, 23),
    ("line", 36, 23),
    ("line", 39, 25),
    ("line", 42, 27),
    ("line", 45, 29),
    ("line", 48, 31),
    ("line", 52, 34, False, True),
    ("line", 52, 34, False, True),
    ("line", 48, 31),
    ("line", 42, 27),
    ("line", 37, 24),
    ("poke", 36, 23),
    ("down", 36, 23),
]


def config():
    return [config_frame(*f) for f in CONFIG]


CONFIG_EYES = cr.face_xy(4)


# ---- agents: the muster board, with the glyph and the word for each state --------------------------------

GLYPHS = {
    "working": (".###.|#####|#####|#####|.###.", "3"),                       # the real glyphs, 5 by 5
    "blocked": ("..#..|.###.|.###.|#####|#####", "1"),
    "unseen": ("..#..|.###.|#####|.###.|..#..", "6"),
    "idle": (".###.|#...#|#...#|#...#|.###.", "2"),
    "error": ("##.##|.###.|..#..|.###.|##.##", "5"),
}
# the hero's agents list, in Ship's own order, so the board says what the sidebar says
MUSTER = ["blocked", "error", "unseen", "working", "idle"]
BX0, BY0, BW = 33, 14, 48           # the list panel: x, y, width. Each row is 6 high.


def glyph(c, name, x, y, bright=False):
    rows, key = GLYPHS[name]
    for dy, row in enumerate(rows.split("|")):
        for dx, v in enumerate(row):
            if v == "#":
                c.set(x + dx, y + dy, {"3": "b", "1": "9", "6": "e", "2": "a", "5": "d"}[key] if bright else key)


def muster_board(c, sel=None, flash=None):
    """A wall board, the sidebar's agents list as a thing you could nail up. The header is a brass plank; each row has
    the state's glyph and its word, in the state's colour, on the terminal's ink. sel is the row the cursor is on
    (a muted purple bar, as in ship); flash is the row whose glyph is lit."""
    c.rect(BX0, 3, BW, 11, "0"); c.rect(BX0 + 1, 4, BW - 2, 9, "3"); c.rect(BX0 + 1, 4, BW - 2, 1, "b")
    c.text(BX0 + (BW - 23) // 2, 6, "agents", "0")
    c.rect(BX0, BY0 - 1, BW, 5 * 6 + 5, "0")
    for i, name in enumerate(MUSTER):
        y = BY0 + 1 + i * 6
        if sel == i:
            c.rect(BX0 + 1, y - 1, BW - 2, 6, "8")
        glyph(c, name, BX0 + 4, y, bright=(flash == i))
        c.text(BX0 + 12, y, name, GLYPHS[name][1])
    for nx in (BX0 + 2, BX0 + BW - 3):
        c.set(nx, 8, "0")


def spyglass(c, hx, hy, tx, ty, length=9):
    """The lookout's glass, held at (hx, hy) and aimed at (tx, ty): a brass tube with an ink edge."""
    n = max(abs(tx - hx), abs(ty - hy))
    ex, ey = hx + round((tx - hx) * length / n), hy + round((ty - hy) * length / n)
    layer = Canvas(c.w, c.h)
    layer.line(hx, hy, ex, ey, "3")
    layer.line(hx + 1, hy, ex + 1, ey, "3")
    layer.outline("0")
    c.blit(layer, 0, 0)
    c.set(ex, ey, "b")


def agents_frame(arm, sel=None, flash=None, spark=None):
    c = cr.shell()
    muster_board(c, sel, flash)
    aim = isinstance(arm, int)
    hands = cr.person(c, "lookout", 4, left="down", right="up" if aim else arm)
    if aim:                                                           # the glass is raised and aimed at the row
        spyglass(c, hands["r"][0], hands["r"][1], BX0 + 6, BY0 + 3 + arm * 6)
    if spark is not None:
        cr.star(c, BX0 - 3, BY0 + 3 + spark * 6, "b", True)
    return c


# arm (a pose, or the row it points at), the row the cursor is on, the row whose glyph is lit, a spark at a row
AGENTS = [
    ("down",),
    ("reach",),
    (0, None),
    (0, 0, None, 0),
    (0, 0, 0),
    (0, 0),
    (2, 2, None, 2),
    (2, 2, 2),
    (2, 2),
    ("reach", None),
    ("down",),
    ("down",),
]


def agents():
    return [agents_frame(*f) for f in AGENTS]


AGENTS_EYES = cr.face_xy(4)


# ---- mouse: a boathook whose tip is the pointer, dragging a pane's border ---------------------------------

CURSOR = """
#.......
##......
#f#.....
#ff#....
#fff#...
#ffff#..
#fffff#.
#ffff###
#ff#f#..
#f#.#f#.
##..#f#.
....##..
"""
MX0 = 56                                    # the deckhand stands at the right, facing the window
WIN = (4, 6, 50, 33)                        # x, y, w, h of the window's screen
MENU = ["split right", "split down", "stack", "close"]


def pane_box(c, x, y, w, h, sel, cursor=False):
    k = "6" if sel else "8"
    c.rect(x, y, w, 1, k); c.rect(x, y + h - 1, w, 1, k); c.rect(x, y, 1, h, k); c.rect(x + w - 1, y, 1, h, k)
    c.rect(x + 2, y, 5, 1, "0"); c.rect(x + 3, y, 3, 1, "f" if sel else "7")           # the pane's name on its border
    c.rect(x + 2, y + 3, 5, 1, "8")
    if cursor:
        c.rect(x + 8, y + 3, 2, 2, "b")


def mouse_frame(tip, wl, grab=False, menu=None, right_sel=False, spark=False, upright=False):
    c = cr.shell()
    wx, wy, ww, wh = WIN
    c.rect(wx, wy, ww, wh, "0")
    div = wx + 2 + wl                                                  # the border between the panes
    pane_box(c, wx + 2, wy + 2, wl + 1, wh - 4, not right_sel, cursor=not right_sel)
    pane_box(c, div, wy + 2, ww - 4 - wl, wh - 4, right_sel, cursor=right_sel)
    if grab:                                                           # the border being held takes the selected colour
        c.rect(div, wy + 3, 1, wh - 6, "6")
    if menu is not None:                                               # the right-click menu: ink box, one row lit
        mx, my = 36, 18
        c.rect(mx, my, 15, 15, "8"); c.rect(mx + 1, my + 1, 13, 13, "0")
        for i, label in enumerate(MENU):
            ry = my + 2 + i * 3
            if i == menu:
                c.rect(mx + 1, ry - 1, 13, 3, "6")
            c.rect(mx + 3, ry, 3 + (i * 3) % 5, 1, "0" if i == menu else "f")
    hands = cr.person(c, "deckhand", MX0, left="down", right="out", flip=True)
    hx, hy = hands["r"]
    tx, ty = tip
    ex, ey = tx + 4, ty + 8                                            # where the cursor's back meets the pole
    if upright:
        ex, ey = hx - 1, 24
        tx, ty = ex - 4, ey - 8
    pole = Canvas(c.w, c.h)
    pole.line(hx, hy, ex, ey, "0")
    pole.line(hx + 1, hy, ex + 1, ey, "0")
    c.blit(pole, 0, 0)
    for k in range(0, 10, 4):                                          # brass bands along the pole
        bx = hx + round((ex - hx) * (k + 3) / 10); by = hy + round((ey - hy) * (k + 3) / 10)
        c.rect(bx, by, 2, 2, "3")
    c.blit_map(CURSOR, tx, ty, {"#": "0"})
    if spark:
        cr.star(c, tx, ty, "b", True)
    return c


# cursor tip, the pane width on the left, grabbing the border, which menu row is lit, the right pane selected, a spark
D = 21
MOUSE = [
    ((52, 12), D, False, None, False, False, True),
    ((47, 12), D),
    ((D + 6, 20), D, True),
    ((D + 8, 20), 23, True),
    ((D + 11, 20), 26, True),
    ((D + 13, 20), 28, True, None, False, True),
    ((44, 24), 28),
    ((40, 22), 28, False, 0, True),
    ((40, 25), 28, False, 1, True),
    ((40, 25), 28, False, 1, True, True),
    ((D + 13, 20), 28, True),
    ((D + 10, 20), 25, True),
    ((D + 7, 20), 22, True),
    ((D + 6, 20), D, True, None, False, True),
    ((52, 12), D),
    ((52, 12), D, False, None, False, False, True),
]


def mouse():
    return [mouse_frame(*f) for f in MOUSE]


MOUSE_EYES = cr.face_xy(MX0)


# ---- saved tabs: a layout card out of the chart rack, fresh shells on it ----------------------------------

RACK = (55, 8, 27)                           # x, y, width of the rack that holds the saved layouts
SLOT = lambda col, row: (RACK[0] + 1 + col * 8, RACK[1] + 11 + row * 12)     # a cubby's top left; each is 7 by 11
PICK = (1, 0)                                # the cubby the navigator takes a layout from
HAND = (38, 26)                              # where the held card sits, at the navigator's raised hand


def layout_card(c, x, y):
    """A layout card, 16 by 12: the tab's panes drawn small, each holding a fresh prompt (a dash and the cursor), because
    what comes back is the layout and not what was running."""
    c.rect(x, y, 16, 12, "0")
    c.rect(x + 1, y + 1, 14, 10, "f")
    for px, py, pw, ph in ((2, 2, 6, 8), (9, 2, 5, 4), (9, 7, 5, 3)):
        c.rect(x + px, y + py, pw, ph, "8")
        c.rect(x + px + 1, y + py + 1, pw - 2, ph - 2, "f")
        c.set(x + px + 1, y + py + 1, "8")
    c.rect(x + 11, y + 3, 1, 1, "b"); c.rect(x + 11, y + 8, 1, 1, "b"); c.rect(x + 5, y + 3, 1, 2, "b")


def small_card(c, x, y, rise=0):
    """A saved layout standing in its cubby: a white card with its panes as three strokes. rise lifts it out a few rows."""
    c.rect(x + 1, y + 1 - rise, 5, 10 + rise, "f")
    c.rect(x + 1, y + 1 - rise, 5, 1, "8")
    c.rect(x + 3, y + 3 - rise, 1, 6, "8")
    c.rect(x + 4, y + 4 - rise, 1, 1, "b")


def saved_frame(arm, rise=0, card=None, star=None):
    """rise: how far the picked layout peeks out of its cubby; card: the full-size card (x, y) once it is out and in the
    navigator's hand (the cubby is empty meanwhile)."""
    c = cr.shell()
    rx, ry, rw = RACK
    c.rect(rx, ry, rw, 35, "0"); c.rect(rx + 1, ry + 1, rw - 2, 9, "3"); c.rect(rx + 1, ry + 1, rw - 2, 1, "b")
    c.text(rx + (rw - 15) // 2, ry + 3, "tabs", "0")
    c.rect(rx + 1, ry + 10, rw - 2, 25, "3")                                       # the wood between the cubbies
    for row in (0, 1):
        for col in (0, 1, 2):
            sx, sy = SLOT(col, row)
            c.rect(sx, sy, 7, 11, "0")
            if (col, row) == PICK and card is not None:
                continue
            if (col, row) == PICK:
                small_card(c, sx, sy, rise)
            else:
                small_card(c, sx, sy)
    cr.person(c, "navigator", 14, left="down", right=arm)
    if card is not None:
        layout_card(c, *card)
    if star:
        cr.star(c, *star)
    return c


# arm, how far the picked layout peeks out, then the card (x, y) in the navigator's hand, and a spark
SAVED = [
    ("down", 0),
    ("reach", 1),
    ("poke", 2),
    ("up", 0, (60, 18), (68, 17, "b", True)),
    ("high", 0, (50, 22)),
    ("high", 0, HAND, (HAND[0] + 17, HAND[1] + 1, "b", True)),
    ("high", 0, HAND),
    ("high", 0, HAND, (HAND[0] + 17, HAND[1] + 1, "b", True)),
    ("high", 0, (50, 22)),
    ("up", 0, (60, 18)),
    ("poke", 2),
    ("reach", 1, None, (SLOT(*PICK)[0] + 3, SLOT(*PICK)[1] - 2, "f", True)),
    ("down", 0),
    ("down", 0),
]


def saved():
    return [saved_frame(*f) for f in SAVED]


SAVED_EYES = cr.face_xy(14)


KEYS_EYES = cr.face_xy(4)

ROOMS = {
    "room-keys": (keys, [KEYS_EYES]),
    "room-config": (config, [CONFIG_EYES]),
    "room-agents": (agents, [AGENTS_EYES]),
    "room-mouse": (mouse, [MOUSE_EYES]),
    "room-saved-tabs": (saved, [SAVED_EYES]),
}


LOOPS = {"lantern-sway": lantern_sway}


def build(out):
    written = []
    colors = {**PALETTE, **SEAM}
    for name, (fn, _eyes) in ROOMS.items():
        with open(os.path.join(out, name + ".svg"), "w") as f:
            f.write(Canvas.sheet(fn()).svg(unit=1, decorative=True, colors=colors))
        written.append(name + ".svg")
    for name, fn in LOOPS.items():
        with open(os.path.join(out, name + ".svg"), "w") as f:
            f.write(Canvas.sheet(fn()).svg(unit=1, decorative=True))
        written.append(name + ".svg")
    return written
