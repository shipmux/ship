#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["psutil>=7", "pyte>=0.8"]
# ///
"""Workloads bench.py doesn't cover: cold start, restore, attach, tab and
pane switching, many busy tabs, CLI round trips and resize.

    uv run extra.py --mux ship tmux --workload switch --reps 3
    uv run extra.py --summarize results/extra-<stamp>.jsonl

It reuses bench.py's isolation (a fresh HOME/XDG tree per run, each tool's
defaults, the harness as the outer terminal). What it adds is a way to tell
when the outer terminal *shows* something, rather than when some bytes went
by: every chunk the client writes is stamped as it arrives, and afterwards
the chunks are replayed through pyte, a VT emulator, to find the first chunk
after which the screen satisfies a condition (a tab's name visible, a pane's
size label at its new size). Diffing renderers that repaint only part of a
word, and a screen that briefly shows the old size, are both handled,
because the condition is checked against the emulated screen.

Pane programs come from pane.py: `tag` paints a full static screen that names
itself and its size, `agent --title` is bench.py's agent screen plus a title
spinner, and `probe` is bench.py's echo probe.

Ship is measured on the binary bench.py uses (SHIP_BENCH_BIN or
target/release/ship). Quit your own sessions before a run you mean to quote.
"""

import argparse
import json
import os
import random
import re
import signal
import statistics
import struct
import subprocess
import sys
import termios
import fcntl
import time
from pathlib import Path

import psutil
import pyte

import bench
from bench import (COLS, ROWS, HERDR_BIN, MUXES, SHIP_BIN, SHIP_NAME, TMUX_BIN, ZELLIJ_BIN, Env, Session,
                   cli, mux_processes, pane_cmd, sweep, text)
from pane import sized

HERE = Path(__file__).resolve().parent
ALPHA = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
PROMPT = "PRMPT$ "
# macOS /bin/sh (bash) sets its own PS1 from /etc/bashrc, so any "$ " counts.
PROMPT_SEEN = re.compile(r"\$ ")


def letters(i, width=2):
    """Index as capitals: 0 -> AA, 1 -> AB."""
    s = ""
    for _ in range(width):
        s = ALPHA[i % 26] + s
        i //= 26
    return s


# ---------------------------------------------------------------- recording

class Rec(Session):
    """A Session that stamps every chunk, so the screen can be replayed."""

    def __init__(self, mux, env, cols=COLS, rows=ROWS):
        self.chunks = []  # (monotonic_ns, offset_after)
        self.events = []  # (monotonic_ns, kind, payload): resizes and keys, for the replay
        self.t_spawn = time.monotonic_ns()
        self.size = (cols, rows)
        super().__init__(mux, env)
        if (cols, rows) != (COLS, ROWS):
            self.resize(cols, rows)

    def pump(self, timeout):
        n = super().pump(timeout)
        if n and n > 0:
            self.chunks.append((time.monotonic_ns(), len(self.out)))
        return n

    def resize(self, cols, rows):
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
        self.size = (cols, rows)
        self.events.append((time.monotonic_ns(), "resize", (cols, rows), len(self.out)))

    def mark(self, kind, payload=None):
        t = time.monotonic_ns()
        self.events.append((t, kind, payload, len(self.out)))
        return t


class QuietScreen(pyte.Screen):
    """pyte's screen without replies (the live harness already answered) and
    tolerant of the private-mode status query newer pyte trips on."""

    def report_device_status(self, *args, **kwargs):
        pass

    def report_device_attributes(self, *args, **kwargs):
        pass

    def write_process_input(self, data):
        pass


class Replay:
    """The outer terminal's screen, advanced chunk by chunk."""

    def __init__(self, rec, start_offset=0, cols=COLS, rows=ROWS):
        self.rec = rec
        self.screen = QuietScreen(cols, rows)
        self.stream = pyte.ByteStream(self.screen)
        self.offset = 0
        self.ci = 0
        self.ei = 0
        # Feed everything before `start_offset` at once.
        self._feed_to(start_offset)

    def _feed_to(self, offset):
        # Apply resizes that happened before this offset first.
        while self.ei < len(self.rec.events) and self.rec.events[self.ei][3] <= offset:
            t, kind, payload, at = self.rec.events[self.ei]
            if at > self.offset:
                self.stream.feed(bytes(self.rec.out[self.offset:at]))
                self.offset = at
            if kind == "resize":
                cols, rows = payload
                self.screen.resize(rows, cols)
            self.ei += 1
        if offset > self.offset:
            self.stream.feed(bytes(self.rec.out[self.offset:offset]))
            self.offset = offset

    def text(self):
        return "\n".join(self.screen.display)

    def first(self, cond, after_ns, until_ns=None):
        """Monotonic ns of the first chunk at or after `after_ns` after which
        `cond(self)` holds, or None. Chunks before `after_ns` are fed
        silently, so the replay must move forward only."""
        while self.ci < len(self.rec.chunks):
            t, off = self.rec.chunks[self.ci]
            if until_ns is not None and t > until_ns:
                return None
            self.ci += 1
            self._feed_to(off)
            if t >= after_ns and cond(self):
                return t
        return None


# ---------------------------------------------------------------- tools

class ShipCold(bench.Ship):
    """Bare `ship --server-name bench`: the client starts its own server,
    the way a user's first `ship` of the day does."""

    def before_client(self, env):
        cfg = env.root / "config/ship/config.toml"
        cfg.parent.mkdir(parents=True, exist_ok=True)
        cfg.touch()
        env.vars["SHIP_CONFIG"] = str(cfg)

    def client_argv(self, env):
        return [SHIP_BIN, "--server-name", SHIP_NAME]

    def stop(self, env):
        try:
            cli([SHIP_BIN, "server", "stop", "--server-name", SHIP_NAME], env, timeout=10)
        except Exception:
            pass


def ship_id(obj, prefix):
    """First string in Ship's JSON that is an ID with `prefix`."""
    if isinstance(obj, str):
        return obj if obj.startswith(prefix) else None
    if isinstance(obj, dict):
        for k, v in obj.items():
            if isinstance(k, str) and k.startswith(prefix):
                return k
            if (found := ship_id(v, prefix)):
                return found
    if isinstance(obj, list):
        for v in obj:
            if (found := ship_id(v, prefix)):
                return found
    return None


def ship_url(env):
    status = json.loads(cli([SHIP_BIN, "server", "status", "--server-name", SHIP_NAME], env, timeout=5))
    return "unix://" + status["socket"]


def ship_tab(env, url, panes, place="right"):
    """A new top-level tab holding `panes` (argv lists); returns (tab, [pane ids])."""
    tab = ship_id(json.loads(cli([SHIP_BIN, "tab", "create", "--server-url", url], env)), "tab:")
    ids = []
    anchor = None
    for i, argv in enumerate(panes):
        args = [SHIP_BIN, "pane", "create", "--server-url", url]
        if anchor is None:
            args += ["--tab", tab]
        else:
            args += ["--pane", anchor, "--place", place]
        made = json.loads(cli(args + ["--", *argv], env))
        pid = ship_id(made, "pane:")
        ids.append(pid)
        anchor = anchor or pid
    return tab, ids


def tmux(env, *args):
    return cli([TMUX_BIN, "-L", "bench", "-f", "/dev/null", *args], env)


def tmux_window(env, panes, split="-h"):
    out = tmux(env, "new-window", "-d", "-P", "-F", "#{window_id}", "--", *panes[0]).strip()
    for argv in panes[1:]:
        tmux(env, "split-window", split, "-d", "-t", out, "--", *argv)
    return out


def tag_cmd(name):
    return pane_cmd("tag", "--name", name)


# ---------------------------------------------------------------- helpers

def proc_split(sess):
    """(server, client) CPU seconds and RSS KiB of this run's mux processes;
    the client is the process the harness spawned."""
    procs = mux_processes(sess.env.home, sess.mux.names)
    client = [p for p in procs if p[0] == sess.pid]
    server = [p for p in procs if p[0] != sess.pid]
    return (sum(p[2] for p in server), sum(p[2] for p in client),
            sum(p[1] for p in server), sum(p[1] for p in client), len(procs))


def q(vals, p):
    vals = sorted(vals)
    if not vals:
        return None
    if len(vals) < 3:
        return vals[-1] if p > 50 else statistics.median(vals)
    return statistics.quantiles(vals, n=100, method="inclusive")[p - 1]


def stats(vals):
    vals = [v for v in vals if v is not None]
    if not vals:
        return dict(n=0)
    return dict(n=len(vals), p50=q(vals, 50), p95=q(vals, 95), p99=q(vals, 99), max=max(vals))


def wait_text(sess, pattern, timeout):
    """Pump until `pattern` (bytes regex) shows in the visible text of new output."""
    deadline = time.monotonic() + timeout
    start = len(sess.out)
    while time.monotonic() < deadline:
        sess.pump(0.02)
        if re.search(pattern, text(bytes(sess.out[max(0, start - 200):])).encode()):
            return True
    return False


# ---------------------------------------------------------------- workloads

def w_cold(mux_name, env, cfg):
    """Client start with no server: until the shell's prompt is on screen,
    then until a typed command's output is."""
    env.vars["PS1"] = PROMPT
    mux = ShipCold() if mux_name == "ship" else MUXES[mux_name]()
    sess = Rec(mux, env)
    try:
        t0 = sess.t_spawn
        deadline = time.monotonic() + 20
        dismissed = 0
        while time.monotonic() < deadline:
            sess.pump(0.02)
            vis = text(bytes(sess.out[-20000:]))
            if PROMPT_SEEN.search(vis):
                break
            for pattern, keys in mux.dialogs:
                if mux_name != "ship" and re.search(pattern, vis) and dismissed < 3:
                    sess.send(keys)
                    dismissed += 1
                    sess.drain(0.3)
        sess.drain(1.0)
        # Herdr's and Zellij's first-run panels sit over a prompt that is
        # already painted; close them before typing.
        seen_from = 0
        for _ in range(3):
            vis = text(bytes(sess.out[seen_from:]))
            hit = [keys for pattern, keys in mux.dialogs if re.search(pattern, vis)]
            if mux_name == "ship" or not hit:
                break
            seen_from = len(sess.out)
            sess.send(hit[0])
            dismissed += 1
            sess.drain(0.6)
        # Prove the prompt is live.
        t_type = sess.mark("type")
        sess.send(b"echo RDY-$((6*7))\r")
        ok = wait_text(sess, rb"RDY-42", 10)
        sess.drain(0.3)
        rp = Replay(sess)
        t_first = sess.chunks[0][0] if sess.chunks else None
        t_paint = rp.first(lambda r: any(line.strip() for line in r.screen.display), t0)
        rp = Replay(sess)
        t_prompt = rp.first(lambda r: PROMPT_SEEN.search(r.text()) is not None, t0)
        rp = Replay(sess)
        t_echo = rp.first(lambda r: "RDY-42" in r.text(), t_type)
        ms = lambda t, base=t0: (t - base) / 1e6 if t else None
        final = Replay(sess)
        final.first(lambda r: False, 0)
        return dict(screen=re.sub(r"\s+", " ", final.text())[-400:], first_byte_ms=ms(t_first), first_paint_ms=ms(t_paint), prompt_ms=ms(t_prompt),
                    echo_ms=ms(t_echo, t_type), dialogs=dismissed, ok=ok and t_prompt is not None,
                    check=f"prompt seen={t_prompt is not None} echo={ok}")
    finally:
        sess.close()


def ship_populate(env, url, tabs, per_tab, kind="shell"):
    for t in range(tabs):
        if kind == "shell":
            panes = [["/bin/sh"]] * per_tab
        elif kind == "tag":
            panes = [tag_cmd(f"TAB{letters(t)}P{ALPHA[p]}") for p in range(per_tab)]
        else:
            panes = [pane_cmd("agent", "--seconds", 900, "--title")] * per_tab
        ship_tab(env, url, panes, place="right" if per_tab <= 2 else "down")


def w_restore(mux_name, env, cfg):
    """Ship only. A server with `--tabs` tabs of `--panes` shells is stopped,
    which saves them; then a bare `ship` restores them all."""
    assert mux_name == "ship"
    env.vars["PS1"] = PROMPT
    ShipCold().before_client(env)
    log = open(env.root / "seed-server.log", "wb")
    server = subprocess.Popen([SHIP_BIN, "server", "--name", SHIP_NAME, "--starter"], env=env.vars, stdout=log,
                              stderr=log, stdin=subprocess.DEVNULL, cwd=env.home, start_new_session=True)
    for _ in range(100):
        try:
            url = ship_url(env)
            break
        except Exception:
            time.sleep(0.1)
    ship_populate(env, url, cfg.tabs - 1, cfg.panes)
    time.sleep(1.0)
    cli([SHIP_BIN, "server", "stop", "--server-url", url], env, timeout=20)
    server.wait(timeout=20)
    saved = list((env.root / "state").rglob("bench.json"))
    r = w_cold(mux_name, env, cfg)
    r["saved_file_bytes"] = saved[0].stat().st_size if saved else None
    return r


class Started:
    """A running multiplexer with content, and a client attached to it."""

    def __init__(self, mux_name, env, cfg, kind, tabs, per_tab):
        self.mux_name = mux_name
        self.env = env
        self.tabs = tabs
        self.per_tab = per_tab
        env.vars["PS1"] = PROMPT
        self.mux = MUXES[mux_name]()


def ship_server(env):
    mux = bench.Ship()
    cfg = env.root / "config/ship/config.toml"
    cfg.parent.mkdir(parents=True, exist_ok=True)
    cfg.touch()
    env.vars["SHIP_CONFIG"] = str(cfg)
    log = open(env.root / "ship-server.log", "wb")
    mux.server = subprocess.Popen([SHIP_BIN, "server", "--name", SHIP_NAME, "--starter"], env=env.vars,
                                  stdout=log, stderr=log, stdin=subprocess.DEVNULL, cwd=env.home,
                                  start_new_session=True)
    for _ in range(100):
        try:
            mux.url = ship_url(env)
            return mux
        except Exception:
            time.sleep(0.1)
    raise RuntimeError("ship server never became healthy")


class PreStarted(bench.Ship):
    """bench.Ship, but the server is already running: before_client only
    remembers it."""

    def __init__(self, started):
        self.server, self.url = started.server, started.url

    def before_client(self, env):
        pass


def tmux_server(env):
    env.vars["TMUX_TMPDIR"] = str(env.root / "run")
    tmux(env, "new-session", "-d", "-s", "bench", "-x", str(COLS), "-y", str(ROWS))


class TmuxAttach(bench.Tmux):
    def before_client(self, env):
        env.vars["TMUX_TMPDIR"] = str(env.root / "run")

    def client_argv(self, env):
        return [TMUX_BIN, *self.args(env), "attach", "-t", "bench"]


def w_attach(mux_name, env, cfg):
    """A server with `--tabs` tabs of `--panes` full screens each; time from
    starting a client to the first paint, and (Ship, which opens on no tab
    when it didn't launch the server) from alt-right to the tab's screen."""
    env.vars["PS1"] = PROMPT
    if mux_name == "ship":
        started = ship_server(env)
        ids = [ship_tab(env, started.url, [tag_cmd(f"TAB{letters(t)}P{ALPHA[p]}") for p in range(cfg.panes)],
                        place="down")[0] for t in range(cfg.tabs)]
        tabs = json.loads(cli([SHIP_BIN, "tab", "list", "--server-url", started.url], env))
        for tid in list(tabs):
            if tid not in ids:
                cli([SHIP_BIN, "tab", "close", "--server-url", started.url, "--tab", tid], env)
        mux = PreStarted(started)
    elif mux_name == "tmux":
        tmux_server(env)
        for t in range(cfg.tabs):
            tmux_window(env, [tag_cmd(f"TAB{letters(t)}P{ALPHA[p]}") for p in range(cfg.panes)], split="-v")
        tmux(env, "kill-window", "-t", "bench:0")
        tmux(env, "select-window", "-t", "bench:1")
        mux = TmuxAttach()
    else:
        raise SystemExit(f"attach: {mux_name} not supported")
    time.sleep(1.5)
    sess = Rec(mux, env)
    try:
        t0 = sess.t_spawn
        sess.drain(1.5)
        t_open = None
        if mux_name == "ship":
            t_open = sess.mark("key")
            sess.send(b"\x1b[1;3C")  # alt-right: open the first tab
            sess.drain(1.0)
        want = "TABAAPA"
        rp = Replay(sess)
        t_paint = rp.first(lambda r: any(line.strip() for line in r.screen.display), t0)
        rp = Replay(sess)
        t_tab = rp.first(lambda r: want in r.text(), t_open or t0)
        ms = lambda t, base: (t - base) / 1e6 if t else None
        server_cpu, client_cpu, server_rss, client_rss, n = proc_split(sess)
        return dict(first_paint_ms=ms(t_paint, t0), tab_ms=ms(t_tab, t_open or t0),
                    server_rss_mb=server_rss / 1024, client_rss_mb=client_rss / 1024,
                    ok=t_tab is not None, check=f"tab screen seen={t_tab is not None}")
    finally:
        sess.close()


def w_switch(mux_name, env, cfg):
    """`--tabs` tabs, each of `--panes` `tag` screens. Press next-tab and
    time until the next tab's screen is on the outer terminal: its first
    pane's name (`shown`), and every pane's size label at the size it
    settles at (`full`). The first lap visits tabs no client has viewed yet;
    the second lap revisits them."""
    env.vars["PS1"] = PROMPT
    names = [[f"TAB{letters(t)}P{ALPHA[p]}" for p in range(cfg.panes)] for t in range(cfg.tabs)]
    if mux_name == "ship":
        started = ship_server(env)
        tab_ids = []
        for t in range(cfg.tabs):
            tab, _ = ship_tab(env, started.url, [tag_cmd(n) for n in names[t]], place="down")
            tab_ids.append(tab)
        # Close the starter tab, so the order is just ours.
        tabs = json.loads(cli([SHIP_BIN, "tab", "list", "--server-url", started.url], env))
        for tid in list(tabs):
            if tid not in tab_ids:
                cli([SHIP_BIN, "tab", "close", "--server-url", started.url, "--tab", tid], env)
        mux = PreStarted(started)
        key = b"\x1b[1;3C"  # alt-right
    elif mux_name == "tmux":
        tmux_server(env)
        for t in range(cfg.tabs):
            tmux_window(env, [tag_cmd(n) for n in names[t]], split="-v")
        tmux(env, "kill-window", "-t", "bench:0")
        tmux(env, "select-window", "-t", "bench:1")
        mux = TmuxAttach()
        key = b"\x02n"  # prefix n
    else:
        raise SystemExit(f"switch: {mux_name} not supported")
    time.sleep(1.5)
    sess = Rec(mux, env)
    try:
        sess.drain(1.5)
        if mux_name == "ship":
            # The client opens on no tab; the first alt-right opens tab 0.
            sess.mark("key", 0)
            sess.send(key)
            sess.drain(1.0)
        presses = []
        cur = 0
        for i in range(cfg.tabs * 2):
            nxt = (cur + 1) % cfg.tabs
            t = sess.mark("key", nxt)
            sess.send(key)
            sess.drain(random.uniform(0.25, 0.4))
            presses.append((t, nxt, i < cfg.tabs - 1))
            cur = nxt
        sess.drain(0.5)
        c = proc_split(sess)
        # Replay: for each press, when its tab's name shows, and when every
        # pane's size label shows the size it has just before the next press.
        sizes_re = {n: re.compile(re.escape(n) + r"X([K-T]+)Y([K-T]+)") for row in names for n in row}
        rp = Replay(sess)
        shown, full, first_lap, rev = [], [], [], []
        for j, (t, tab, first) in enumerate(presses):
            end = presses[j + 1][0] if j + 1 < len(presses) else None
            ts = rp.first(lambda r: names[tab][0] in r.text(), t, end)
            shown.append((ts - t) / 1e6 if ts else None)
        # A second pass finds the settled size labels: the screen just
        # before each next press.
        rp = Replay(sess)
        settled = []
        for j, (t, tab, first) in enumerate(presses):
            end = presses[j + 1][0] if j + 1 < len(presses) else sess.chunks[-1][0] + 1
            rp.first(lambda r: False, t, end - 1)
            txt = rp.text()
            labels = {}
            for n in names[tab]:
                m = sizes_re[n].findall(txt)
                labels[n] = m[-1] if m else None
            settled.append(labels)
        rp = Replay(sess)
        for j, (t, tab, first) in enumerate(presses):
            end = presses[j + 1][0] if j + 1 < len(presses) else None
            want = settled[j]
            if any(v is None for v in want.values()):
                full.append(None)
                rp.first(lambda r: False, t, (end or t) - 1)
                continue
            labels = [f"{n}X{w[0]}Y{w[1]}" for n, w in want.items()]
            tf = rp.first(lambda r: all(l in r.text() for l in labels), t, end)
            full.append((tf - t) / 1e6 if tf else None)
        lap1 = [full[j] for j, p in enumerate(presses) if p[2]]
        lap2 = [full[j] for j, p in enumerate(presses) if not p[2]]
        sh1 = [shown[j] for j, p in enumerate(presses) if p[2]]
        sh2 = [shown[j] for j, p in enumerate(presses) if not p[2]]
        missing = sum(v is None for v in full)
        return dict(shown_first_lap=stats(sh1), shown_revisit=stats(sh2), full_first_lap=stats(lap1),
                    full_revisit=stats(lap2), shown_ms=shown, full_ms=full, press_ns=[p[0] for p in presses], server_cpu_s=c[0], client_cpu_s=c[1],
                    ok=missing <= 2, check=f"{len(presses) - missing}/{len(presses)} switches fully drawn")
    finally:
        sess.close()


def w_stack(mux_name, env, cfg):
    """Ship only: one tab, a stack of `--panes` `tag` panes; alt-. cycles
    the stack, and each press needs a different pane's full screen."""
    assert mux_name == "ship"
    env.vars["PS1"] = PROMPT
    started = ship_server(env)
    names = [f"STK{ALPHA[p]}" for p in range(cfg.panes)]
    tab, ids = ship_tab(env, started.url, [tag_cmd(n) for n in names], place="stack")
    tabs = json.loads(cli([SHIP_BIN, "tab", "list", "--server-url", started.url], env))
    for tid in list(tabs):
        if tid != tab:
            cli([SHIP_BIN, "tab", "close", "--server-url", started.url, "--tab", tid], env)
    time.sleep(1.0)
    sess = Rec(PreStarted(started), env)
    try:
        sess.drain(1.0)
        sess.send(b"\x1b[1;3C")
        sess.drain(1.0)
        presses = []
        for i in range(cfg.panes * 6):
            t = sess.mark("key")
            sess.send(b"\x1b.")  # alt-.
            sess.drain(random.uniform(0.25, 0.4))
            presses.append(t)
        rp = Replay(sess)
        # Which stack pane is in front after each press: the one whose name
        # is visible at the end of its interval; then time to that.
        fronts = []
        for j, t in enumerate(presses):
            end = presses[j + 1] if j + 1 < len(presses) else sess.chunks[-1][0] + 1
            rp.first(lambda r: False, t, end - 1)
            txt = rp.text()
            fronts.append(next((n for n in names if n in txt and txt.index(n) < 20 * COLS), None))
        rp = Replay(sess)
        lat = []
        for j, t in enumerate(presses):
            end = presses[j + 1] if j + 1 < len(presses) else None
            n = fronts[j]
            prev = fronts[j - 1] if j else None
            if not n or n == prev:
                lat.append(None)
                rp.first(lambda r: False, t, (end or t) - 1)
                continue
            ts = rp.first(lambda r: n in r.text() and (prev is None or prev not in r.text()), t, end)
            lat.append((ts - t) / 1e6 if ts else None)
        return dict(cycle=stats(lat), cycle_ms=lat, ok=sum(v is not None for v in lat) >= len(lat) // 2,
                    check=f"{sum(v is not None for v in lat)}/{len(lat)} cycles timed")
    finally:
        sess.close()


def w_scale(mux_name, env, cfg):
    """`--tabs` tabs of `--panes` agent screens (with a title spinner unless
    --no-title) running at once; the probe types in a pane of its own tab.
    CPU, memory and the probe's echo latency."""
    env.vars["PS1"] = PROMPT
    agent = pane_cmd("agent", "--seconds", 900, *(["--title"] if cfg.title else []))
    if mux_name == "ship":
        started = ship_server(env)
        for t in range(cfg.tabs):
            ship_tab(env, started.url, [agent] * cfg.panes, place="down")
        mux = PreStarted(started)
    elif mux_name == "tmux":
        tmux_server(env)
        for t in range(cfg.tabs):
            tmux_window(env, [agent] * cfg.panes, split="-v")
        tmux(env, "select-window", "-t", "bench:0")
        mux = TmuxAttach()
    else:
        raise SystemExit(f"scale: {mux_name} not supported")
    time.sleep(2.0)
    sess = Rec(mux, env)
    try:
        sess.ready()
        sess.drain(2.0)
        # bench.py's typing workload, in the starter tab's shell.
        tcfg = argparse.Namespace(warmup=10, keys=cfg.keys, key_timeout=2.0)
        c0 = proc_split(sess)
        t0 = time.monotonic()
        r = bench.typing()(sess, tcfg)
        dt = time.monotonic() - t0
        c1 = proc_split(sess)
        r.update(server_cpu_pct=100 * (c1[0] - c0[0]) / dt, client_cpu_pct=100 * (c1[1] - c0[1]) / dt,
                 server_rss_mb=c1[2] / 1024, client_rss_mb=c1[3] / 1024, procs=c1[4])
        # Plus a quiet stretch with no typing, for steady CPU.
        c0 = proc_split(sess)
        t0 = time.monotonic()
        sess.drain(cfg.idle_s)
        dt = time.monotonic() - t0
        c1 = proc_split(sess)
        r.update(steady_server_cpu_pct=100 * (c1[0] - c0[0]) / dt, steady_client_cpu_pct=100 * (c1[1] - c0[1]) / dt,
                 steady_bytes_s=None)
        r.pop("lat_ms", None)
        return r
    finally:
        sess.close()


def timeit(fn, n):
    out = []
    for _ in range(n):
        t = time.perf_counter_ns()
        fn()
        out.append((time.perf_counter_ns() - t) / 1e6)
    return out


def w_cli(mux_name, env, cfg):
    """Round trips of the commands scripts and agents call, against a
    server with `--tabs` tabs of `--panes` shells. Each command is a fresh
    process, as from a script."""
    env.vars["PS1"] = PROMPT
    n = cfg.cli_n
    res = {}
    if mux_name == "ship":
        started = ship_server(env)
        ship_populate(env, started.url, cfg.tabs, cfg.panes)
        time.sleep(1.0)
        u = ["--server-url", started.url]
        nm = ["--server-name", SHIP_NAME]
        tabs = json.loads(cli([SHIP_BIN, "tab", "list", *u], env))
        pane = ship_id(tabs, "pane:")
        run = lambda *a: cli([SHIP_BIN, *a], env)
        res["version"] = timeit(lambda: run("--version"), n)
        res["server_status"] = timeit(lambda: run("server", "status", *nm), n)
        res["tab_list"] = timeit(lambda: run("tab", "list", *u), n)
        res["tab_list_by_name"] = timeit(lambda: run("tab", "list", *nm), n)
        res["pane_get"] = timeit(lambda: run("pane", "get", *u, "--pane", pane), n)
        res["pane_read"] = timeit(lambda: run("pane", "read", *u, "--pane", pane), n)
        res["pane_send_input"] = timeit(lambda: run("pane", "send", *u, "--pane", pane, "true", "--key", "enter"), n)

        def create_close():
            made = json.loads(run("pane", "create", *u, "--pane", pane, "--place", "stack", "--", "/bin/sh"))
            run("pane", "close", *u, "--pane", ship_id(made, "pane:"))
        res["pane_create_close"] = timeit(create_close, max(5, n // 3))
        started.server.send_signal(signal.SIGTERM)
        try:
            started.server.wait(timeout=10)
        except subprocess.TimeoutExpired:
            started.server.kill()
    elif mux_name == "tmux":
        tmux_server(env)
        for t in range(cfg.tabs):
            tmux_window(env, [["/bin/sh"]] * cfg.panes, split="-v")
        time.sleep(1.0)
        res["version"] = timeit(lambda: cli([TMUX_BIN, "-V"], env), n)
        res["tab_list"] = timeit(lambda: tmux(env, "list-windows", "-a"), n)
        res["pane_list"] = timeit(lambda: tmux(env, "list-panes", "-a"), n)
        res["pane_read"] = timeit(lambda: tmux(env, "capture-pane", "-p", "-t", "bench:1.0"), n)
        res["pane_send_input"] = timeit(lambda: tmux(env, "send-keys", "-t", "bench:1.0", "true", "Enter"), n)

        def create_close():
            wid = tmux(env, "new-window", "-d", "-P", "-F", "#{window_id}", "/bin/sh").strip()
            tmux(env, "kill-window", "-t", wid)
        res["pane_create_close"] = timeit(create_close, max(5, n // 3))
        tmux(env, "kill-server")
    elif mux_name == "herdr":
        herdr = bench.Herdr()
        herdr.before_client(env)
        sess = Session(herdr, env)
        try:
            sess.ready()
            res["version"] = timeit(lambda: cli([HERDR_BIN, "--version"], env), n)
            res["pane_list"] = timeit(lambda: cli([HERDR_BIN, "pane", "list"], env), n)
            panes = json.loads(cli([HERDR_BIN, "pane", "list"], env))
            first = bench.find_pane_id(panes)
            res["pane_read"] = timeit(lambda: cli([HERDR_BIN, "pane", "read", first], env), n)
        finally:
            sess.close()
    else:
        raise SystemExit(f"cli: {mux_name} not supported")
    return dict(**{k: stats(v) for k, v in res.items()}, raw={k: [round(x, 3) for x in v] for k, v in res.items()},
                ok=True, check="commands ran")


def w_resize(mux_name, env, cfg):
    """One tab of `--panes` `tag` screens. The outer terminal alternates
    between two sizes; time until every pane's size label shows its new
    size. Then a drag: one column per 8 ms for `--drag` steps, with CPU over
    the drag and the time from the last step until the screen settles."""
    env.vars["PS1"] = PROMPT
    names = [f"RSZ{ALPHA[p]}" for p in range(cfg.panes)]
    if mux_name == "ship":
        started = ship_server(env)
        tab, _ = ship_tab(env, started.url, [tag_cmd(n) for n in names], place="right")
        tabs = json.loads(cli([SHIP_BIN, "tab", "list", "--server-url", started.url], env))
        for tid in list(tabs):
            if tid != tab:
                cli([SHIP_BIN, "tab", "close", "--server-url", started.url, "--tab", tid], env)
        mux = PreStarted(started)
    elif mux_name == "tmux":
        tmux_server(env)
        tmux_window(env, [tag_cmd(n) for n in names], split="-h")
        tmux(env, "select-layout", "-t", "bench:1", "tiled")
        tmux(env, "kill-window", "-t", "bench:0")
        mux = TmuxAttach()
    else:
        raise SystemExit(f"resize: {mux_name} not supported")
    time.sleep(1.0)
    sess = Rec(mux, env)
    try:
        sess.drain(1.0)
        if mux_name == "ship":
            sess.send(b"\x1b[1;3C")
            sess.drain(1.0)
        lab = {n: re.compile(re.escape(n) + r"X([K-T]+)Y([K-T]+)") for n in names}

        def labels(txt):
            return {n: (m[-1] if (m := lab[n].findall(txt)) else None) for n in names}

        steps = []
        sizes = [(COLS, ROWS), (COLS - 37, ROWS - 9)]
        for i in range(cfg.resizes):
            cols, rows = sizes[(i + 1) % 2]
            t = sess.mark("resize-at")
            sess.resize(cols, rows)
            sess.drain(0.6)
            steps.append(t)
        # Drag.
        c0 = proc_split(sess)
        t_drag0 = time.monotonic()
        cols, rows = sizes[0]
        for i in range(cfg.drag):
            cols = cols - 1 if i < cfg.drag // 2 else cols + 1
            sess.resize(cols, rows)
            sess.drain(0.008)
        t_last = sess.mark("drag-end")
        drag_s = time.monotonic() - t_drag0
        sess.drain(1.5)
        c1 = proc_split(sess)
        # Replay.
        rp = Replay(sess)
        lat = []
        for j, t in enumerate(steps):
            end = steps[j + 1] if j + 1 < len(steps) else t_last
            rp.first(lambda r: False, t, end - 1)
            want = labels(rp.text())
            lat.append(want)
        rp = Replay(sess)
        out = []
        for j, t in enumerate(steps):
            end = steps[j + 1] if j + 1 < len(steps) else t_last
            want = lat[j]
            if any(v is None for v in want.values()):
                out.append(None)
                rp.first(lambda r: False, t, end - 1)
                continue
            ls = [f"{n}X{w[0]}Y{w[1]}" for n, w in want.items()]
            tf = rp.first(lambda r: all(l in r.text() for l in ls), t, end)
            out.append((tf - t) / 1e6 if tf else None)
        # Settle after the drag: the final labels, first time all shown.
        rp = Replay(sess)
        rp.first(lambda r: False, 0, sess.chunks[-1][0] + 1)
        final = labels(rp.text())
        # The screens are static, so the output going quiet is the settle.
        settle = max(0.0, (sess.chunks[-1][0] - t_last) / 1e6) if sess.chunks else None
        return dict(resize=stats(out), resize_ms=out, resize_ns=steps, step_labels=[list(w.values()) for w in lat], final_labels=list(final.values()), drag_steps=cfg.drag, drag_s=drag_s,
                    drag_server_cpu_s=c1[0] - c0[0], drag_client_cpu_s=c1[1] - c0[1],
                    drag_settle_ms=settle, ok=sum(v is not None for v in out) >= len(out) - 1,
                    check=f"{sum(v is not None for v in out)}/{len(out)} resizes fully drawn")
    finally:
        sess.close()


WORKLOADS = dict(cold=w_cold, restore=w_restore, attach=w_attach, switch=w_switch, stack=w_stack,
                 scale=w_scale, cli=w_cli, resize=w_resize)


def one(mux_name, workload, rep, cfg):
    env = Env(Path(cfg.scratch) / f"{mux_name}-{workload}-{rep}")
    if cfg.trace and mux_name == "ship":
        env.vars["SHIP_TRACE"] = str(env.root / "trace")
    env.root.mkdir(parents=True, exist_ok=True)
    # Wall and monotonic time read together, so a --trace run can be lined up with the harness (keytrace.py).
    (env.root / "clock.json").write_text(json.dumps(dict(unix_ns=time.time_ns(), monotonic_ns=time.monotonic_ns())))
    rec = dict(mux=mux_name, workload=workload, rep=rep, started=time.strftime("%H:%M:%S"),
               load_before=os.getloadavg()[0], tabs=cfg.tabs, panes=cfg.panes)
    try:
        rec.update(WORKLOADS[workload](mux_name, env, cfg))
    except Exception as e:
        import traceback
        rec.update(ok=False, error=f"{type(e).__name__}: {e}", tb=traceback.format_exc()[-1500:])
    finally:
        # Stop anything the workload started outside a Session.
        try:
            cli([SHIP_BIN, "server", "stop", "--server-name", SHIP_NAME], env, timeout=10)
        except Exception:
            pass
        rec["leftover_pids"] = sweep(env.home)
    rec["load1"] = os.getloadavg()[0]
    return rec


def summarize(records):
    lines = []
    for r in records:
        keep = {k: v for k, v in r.items() if k not in ("raw", "shown_ms", "full_ms", "cycle_ms", "resize_ms", "tb",
                                                          "screen_tail", "lat_ms", "press_ns", "resize_ns")}
        lines.append(json.dumps(keep, default=lambda x: round(x, 3) if isinstance(x, float) else str(x)))
    return "\n".join(lines)


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--mux", nargs="+", default=["ship", "tmux"], choices=list(MUXES))
    p.add_argument("--workload", nargs="+", default=list(WORKLOADS), choices=list(WORKLOADS))
    p.add_argument("--reps", type=int, default=3)
    p.add_argument("--tabs", type=int, default=20)
    p.add_argument("--panes", type=int, default=3)
    p.add_argument("--keys", type=int, default=100)
    p.add_argument("--idle-s", type=float, default=10)
    p.add_argument("--cli-n", type=int, default=30)
    p.add_argument("--resizes", type=int, default=12)
    p.add_argument("--drag", type=int, default=60)
    p.add_argument("--title", action=argparse.BooleanOptionalAction, default=True)
    p.add_argument("--trace", action="store_true",
                   help="record Ship timelines to <scratch>/<run>/trace (profiling only)")
    p.add_argument("--rss-sample-s", type=float, default=0,
                   help="seconds between peak-RSS samples (bench.py's 1 s sampler stalls the reader 10-20 ms)")
    p.add_argument("--scratch", default="/tmp/ship-extra-runs")
    p.add_argument("--out", default=str(HERE / "results"))
    cfg = p.parse_args()
    bench.RSS_SAMPLE_S = cfg.rss_sample_s
    signal.signal(signal.SIGTERM, bench.interrupt)
    Path(cfg.out).mkdir(exist_ok=True)
    path = Path(cfg.out) / time.strftime("extra-%Y%m%d-%H%M%S.jsonl")
    others = sorted({proc.info["name"] for proc in psutil.process_iter(["name"])
                     if (proc.info["name"] or "").split(":")[0] in ("ship", "herdr", "zellij", "tmux")})
    meta = dict(kind="meta", argv=sys.argv, ship_bin=SHIP_BIN, ship_sha256=bench.sha256(SHIP_BIN),
                ship_rev=bench.capture("git", "-C", str(HERE.parent), "rev-parse", "HEAD"),
                load=os.getloadavg(), other_mux_processes=bench.capture("pgrep", "-lf", "ship|herdr|zellij|tmux"),
                machine=bench.machine())
    with open(path, "w") as f:
        f.write(json.dumps(meta) + "\n")
        for rep in range(cfg.reps):
            for w in cfg.workload:
                for m in random.sample(cfg.mux, len(cfg.mux)):
                    if w in ("restore", "stack") and m != "ship":
                        continue
                    rec = one(m, w, rep, cfg)
                    f.write(json.dumps(rec) + "\n")
                    f.flush()
                    status = "ok" if rec.get("ok") else "FAIL " + (rec.get("error") or rec.get("check", ""))
                    print(f"[{rec['started']}] rep{rep} {w:8s} {m:6s} {status}", flush=True)
                    time.sleep(1.0)
    print(f"\nresults: {path}")


if __name__ == "__main__":
    main()
