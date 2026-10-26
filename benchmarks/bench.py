#!/usr/bin/env python3
"""Compare terminal multiplexers through a PTY the harness owns.

The harness plays the outer terminal: it spawns each multiplexer's client on
a PTY, answers the few terminal queries the clients send, types into it, and
counts and searches every byte the client writes. Each run gets a fresh,
private HOME/XDG/TMPDIR tree, its own socket or port, and the tool's defaults.
See README.md for the workloads and metrics.
"""

import argparse
import fcntl
import hashlib
import json
import os
import platform
import pty
import random
import re
import select
import shutil
import signal
import statistics
import struct
import subprocess
import sys
import termios
import threading
import time
import urllib.request
from pathlib import Path

from pane import marker

HERE = Path(__file__).resolve().parent
PANE = HERE / "pane.py"
PY = sys.executable
COLS, ROWS = 200, 50
SHIP_BIN = os.environ.get("SHIP_BENCH_BIN", str(HERE.parent / "target/release/ship"))
HERDR_BIN = "/opt/homebrew/bin/herdr"
ZELLIJ_BIN = "/opt/homebrew/bin/zellij"
TMUX_BIN = "/opt/homebrew/bin/tmux"
SHIP_PORT = 43291  # never Ship's default 43179
ALPHA = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
ESC_RE = re.compile(rb"\x1b\[[0-9;?<=>!$ ]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1bP[^\x1b]*\x1b\\|\x1b[()][0-9A-Za-z]|\x1b[=>78DEHMc]")

# Replies a minimal xterm-like terminal gives. Everything else goes unanswered.
REPLIES = [
    (b"\x1b[c", b"\x1b[?62;22c"),
    (b"\x1b[0c", b"\x1b[?62;22c"),
    (b"\x1b[6n", b"\x1b[1;1R"),
    (b"\x1b]10;?", b"\x1b]10;rgb:d0d0/d0d0/d0d0\x1b\\"),
    (b"\x1b]11;?", b"\x1b]11;rgb:1010/1010/1010\x1b\\"),
]


def token(index):
    return "".join(ALPHA[(index + j) % 26] for j in range(8))


TOKENS = [token(i).encode() for i in range(26)]
TOKEN_RE = re.compile(b"|".join(TOKENS))


CSI = rb"(?:\x1b\[[0-9;?]*[ -/]*[@-~])*"


def painted(literal):
    """Regex for `literal` with any CSI sequences between its bytes."""
    return re.compile(CSI.join(re.escape(bytes([c])) for c in literal))


def text(data):
    """Visible text with escape sequences replaced by spaces."""
    return ESC_RE.sub(b" ", data).decode("utf-8", "replace")


def cpu_seconds(field):
    total = 0.0
    for part in field.split(":"):
        total = total * 60 + float(part)
    return total


class Env:
    """A private HOME/XDG/TMPDIR tree for one run."""

    def __init__(self, root):
        self.root = Path(root)
        shutil.rmtree(self.root, ignore_errors=True)
        for d in ("home", "config", "data", "state", "cache", "run", "tmp"):
            (self.root / d).mkdir(parents=True, mode=0o700)
        keep = ("PATH", "USER", "LOGNAME")
        self.vars = {k: os.environ[k] for k in keep if k in os.environ}
        self.vars.update(
            HOME=str(self.root / "home"),
            XDG_CONFIG_HOME=str(self.root / "config"),
            XDG_DATA_HOME=str(self.root / "data"),
            XDG_STATE_HOME=str(self.root / "state"),
            XDG_CACHE_HOME=str(self.root / "cache"),
            XDG_RUNTIME_DIR=str(self.root / "run"),
            TMPDIR=str(self.root / "tmp") + "/",
            TERM="xterm-256color",
            COLORTERM="truecolor",
            LANG="en_US.UTF-8",
            LC_ALL="en_US.UTF-8",
            SHELL="/bin/sh",
            PS1="$ ",
        )
        self.home = self.vars["HOME"]


def mux_processes(home, names):
    """(pid, rss_kib, cpu_s) for every process named in `names` whose
    environment carries this run's HOME. The live sessions never match."""
    res = subprocess.run(
        ["ps", "-x", "-E", "-ww", "-U", str(os.getuid()), "-o", "pid=,rss=,time=,command="],
        capture_output=True, text=True,
    )
    marker = f"HOME={home}"
    found = []
    for line in res.stdout.splitlines():
        if marker not in line:
            continue
        pid, rss, cpu, cmd = line.split(None, 3)
        if os.path.basename(cmd.split()[0]) in names:
            found.append((int(pid), int(rss), cpu_seconds(cpu)))
    return found


def net_bytes_in(pid):
    """Bytes `pid` has received over the network so far, from nettop. Only
    Ship's client talks to its server over TCP; the others use Unix sockets,
    which nettop doesn't count."""
    res = subprocess.run(["nettop", "-P", "-L", "1", "-x", "-p", str(pid), "-J", "bytes_in"],
                         capture_output=True, text=True)
    rows = [l.split(",") for l in res.stdout.splitlines()[1:] if l.strip()]
    return sum(int(r[1]) for r in rows if len(r) > 1 and r[1].isdigit()) if rows else None


def run_pids(home):
    """Every process, of any name, started under this run's HOME."""
    res = subprocess.run(["ps", "-x", "-E", "-ww", "-U", str(os.getuid()), "-o", "pid=,command="],
                         capture_output=True, text=True)
    return [int(l.split(None, 1)[0]) for l in res.stdout.splitlines() if f"HOME={home} " in l + " "]


class Session:
    """One multiplexer client on a harness-owned PTY."""

    def __init__(self, mux, env):
        self.mux, self.env = mux, env
        self.out = bytearray()
        self.scanned = 0
        self.peak_rss = 0
        self.samples = []
        self.stop_sampling = threading.Event()
        mux.before_client(env)
        argv = mux.client_argv(env)
        pid, fd = pty.fork()
        if pid == 0:
            os.chdir(env.home)
            os.execve(argv[0], argv, env.vars)
        self.pid, self.fd = pid, fd
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
        self.sampler = threading.Thread(target=self._sample, daemon=True)
        self.sampler.start()

    def _sample(self):
        while not self.stop_sampling.wait(1.0):
            procs = mux_processes(self.env.home, self.mux.names)
            self.peak_rss = max(self.peak_rss, sum(p[1] for p in procs))

    def cpu(self):
        procs = mux_processes(self.env.home, self.mux.names)
        self.peak_rss = max(self.peak_rss, sum(p[1] for p in procs))
        return sum(p[2] for p in procs), len(procs)

    def wire(self):
        """Bytes the client has received from its server, where that can be
        measured (Ship only); None otherwise."""
        return net_bytes_in(self.pid) if self.mux.name == "ship" else None

    def pump(self, timeout):
        """Read whatever arrives within `timeout`, answering queries."""
        r, _, _ = select.select([self.fd], [], [], timeout)
        if not r:
            return 0
        try:
            chunk = os.read(self.fd, 1 << 18)
        except OSError:
            return -1
        self.out += chunk
        lo = max(0, self.scanned - 16)
        window = bytes(self.out[lo:])
        for query, reply in REPLIES:
            if query in window[max(0, self.scanned - lo - len(query) + 1):]:
                os.write(self.fd, reply)
        self.scanned = len(self.out)
        return len(chunk)

    def wait(self, pattern, timeout, since=0):
        """Pump until `pattern` is painted after offset `since`. Escape
        sequences may sit between its characters: a renderer can reposition
        the cursor mid-word, for example after a wide character. Searches
        only new bytes (plus an overlap), so a long flood stays linear."""
        rx = painted(pattern)
        deadline = time.monotonic() + timeout
        pos = since
        while True:
            if rx.search(self.out, pos):
                return True
            pos = max(since, len(self.out) - 4096)
            left = deadline - time.monotonic()
            if left <= 0:
                return False
            self.pump(min(left, 0.05))

    def drain(self, seconds):
        deadline = time.monotonic() + seconds
        while (left := deadline - time.monotonic()) > 0:
            self.pump(min(left, 0.05))

    def send(self, data):
        os.write(self.fd, data)

    def ready(self, timeout=30):
        """Dismiss first-run dialogs the way a user would, then prove a shell
        answers. Returns the seconds it took."""
        start = time.monotonic()
        mark = 0
        last_try = 0.0
        while time.monotonic() - start < timeout:
            self.drain(0.3)
            fresh = text(bytes(self.out[mark:]))
            for pattern, keys in self.mux.dialogs:
                if re.search(pattern, fresh):
                    mark = len(self.out)
                    self.send(keys)
                    self.drain(0.6)
                    break
            else:
                if b"RDY-42" in self.out:
                    return time.monotonic() - start
                if time.monotonic() - last_try > 1.0:
                    last_try = time.monotonic()
                    self.send(b"echo RDY-$((6*7))\r")
        raise RuntimeError("shell never answered: " + text(bytes(self.out[-1500:]))[-600:])

    def close(self):
        self.stop_sampling.set()
        try:
            self.mux.stop(self.env)
        finally:
            # On macOS a process exiting on a PTY waits for its output to
            # drain, so keep reading until it is reaped, and close the
            # master before any blocking wait.
            try:
                os.kill(self.pid, signal.SIGHUP)
            except ProcessLookupError:
                pass
            reaped = False
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline:
                if os.waitpid(self.pid, os.WNOHANG)[0]:
                    reaped = True
                    break
                if self.pump(0.05) < 0:
                    time.sleep(0.05)
            os.close(self.fd)
            if not reaped:
                try:
                    os.kill(self.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                os.waitpid(self.pid, 0)
            sweep(self.env.home)


def sweep(home):
    """Kill anything still running under this run's HOME; return what was left."""
    left = run_pids(home)
    for sig in (signal.SIGTERM, signal.SIGKILL):
        for pid in left:
            try:
                os.kill(pid, sig)
            except ProcessLookupError:
                pass
        time.sleep(0.5)
        left = run_pids(home)
        if not left:
            break
    return left


def cli(argv, env, extra=None, timeout=15):
    vars = dict(env.vars, **(extra or {}))
    res = subprocess.run(argv, env=vars, capture_output=True, text=True, timeout=timeout, cwd=env.home)
    if res.returncode:
        raise RuntimeError(f"{argv} failed: {res.stderr.strip() or res.stdout.strip()}")
    return res.stdout


class Ship:
    name = "ship"
    names = {"ship"}
    # Ship's client, attached to a server it didn't launch, opens on no tab.
    dialogs = [(r"alt-right\s+to\s+open", b"\x1b[1;3C")]

    def url(self):
        return f"http://127.0.0.1:{SHIP_PORT}"

    def before_client(self, env):
        cfg = env.root / "config/ship/config.toml"
        cfg.parent.mkdir(parents=True, exist_ok=True)
        cfg.touch()
        env.vars["SHIP_CONFIG"] = str(cfg)
        log = open(env.root / "ship-server.log", "wb")
        self.server = subprocess.Popen([SHIP_BIN, "server", "--port", str(SHIP_PORT), "--starter"],
                                       env=env.vars, stdout=log, stderr=log, stdin=subprocess.DEVNULL,
                                       cwd=env.home, start_new_session=True)
        for _ in range(100):
            try:
                urllib.request.urlopen(self.url() + "/health", timeout=0.5)
                return
            except Exception:
                if self.server.poll() is not None:
                    raise RuntimeError("ship server exited; see " + str(env.root / "ship-server.log"))
                time.sleep(0.1)
        raise RuntimeError("ship server never became healthy")

    def client_argv(self, env):
        return [SHIP_BIN, "--server-url", self.url()]

    def side_pane(self, env, cmd):
        tabs = json.loads(cli([SHIP_BIN, "tab", "list", "--server-url", self.url()], env))
        tab = next(iter(tabs))
        cli([SHIP_BIN, "pane", "create", "--server-url", self.url(), "--tab", tab, "--", *cmd], env)

    def stop(self, env):
        try:
            cli([SHIP_BIN, "server", "stop", "--server-url", self.url()], env, timeout=10)
        except Exception:
            pass
        try:
            self.server.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.server.kill()
            self.server.wait()


class Herdr:
    name = "herdr"
    names = {"herdr"}
    # First run opens the settings/integrations panel; esc closes it.
    dialogs = [(r"esc\s+close", b"\x1b")]

    def before_client(self, env):
        run = env.root / "run"
        env.vars.update(HERDR_HOME=str(env.root / "herdr"), HERDR_SOCKET_PATH=str(run / "herdr.sock"),
                        HERDR_CLIENT_SOCKET_PATH=str(run / "herdr-client.sock"))

    def client_argv(self, env):
        return [HERDR_BIN]

    def side_pane(self, env, cmd):
        panes = json.loads(cli([HERDR_BIN, "pane", "list"], env))
        first = find_pane_id(panes)
        made = json.loads(cli([HERDR_BIN, "pane", "split", first, "--direction", "right"], env))
        new = find_pane_id(made)
        cli([HERDR_BIN, "pane", "run", new, " ".join(cmd)], env)
        # Splitting focuses the new pane; give focus back to the probe's pane.
        cli([HERDR_BIN, "pane", "focus", "--pane", new, "--direction", "left"], env)

    def stop(self, env):
        try:
            cli([HERDR_BIN, "server", "stop"], env, timeout=10)
        except Exception:
            pass


def find_pane_id(obj):
    """First `pane_id` in Herdr's JSON, whatever the envelope."""
    if isinstance(obj, dict):
        if isinstance(obj.get("pane_id"), str):
            return obj["pane_id"]
        for v in obj.values():
            if (found := find_pane_id(v)):
                return found
    if isinstance(obj, list):
        for v in obj:
            if (found := find_pane_id(v)):
                return found
    return None


class Zellij:
    name = "zellij"
    names = {"zellij"}
    # First run of a version shows release notes, then possibly startup tips.
    dialogs = [(r"Release Notes|ESC\S* - Dismiss|Startup Tips|<ESC>", b"\x1b")]

    def before_client(self, env):
        env.vars["ZELLIJ_SOCKET_DIR"] = str(env.root / "run/zj")

    def args(self, env):
        return ["--config-dir", str(env.root / "config/zellij"), "--data-dir", str(env.root / "data/zellij")]

    def client_argv(self, env):
        return [ZELLIJ_BIN, *self.args(env), "-s", "bench"]

    def side_pane(self, env, cmd):
        cli([ZELLIJ_BIN, *self.args(env), "-s", "bench", "run", "--no-focus", "-d", "right", "--", *cmd], env)

    def stop(self, env):
        try:
            cli([ZELLIJ_BIN, "kill-session", "bench"], env, timeout=10)
        except Exception:
            pass


class Tmux:
    name = "tmux"
    names = {"tmux"}
    dialogs = []

    def args(self, env):
        # No config file: tmux's own defaults, like the other muxes get.
        return ["-L", "bench", "-f", "/dev/null"]

    def before_client(self, env):
        env.vars["TMUX_TMPDIR"] = str(env.root / "run")

    def client_argv(self, env):
        return [TMUX_BIN, *self.args(env), "new-session", "-s", "bench"]

    def side_pane(self, env, cmd):
        cli([TMUX_BIN, *self.args(env), "split-window", "-h", "-d", "-t", "bench", "--", *cmd], env)

    def stop(self, env):
        try:
            cli([TMUX_BIN, *self.args(env), "kill-server"], env, timeout=10)
        except Exception:
            pass


MUXES = {"ship": Ship, "herdr": Herdr, "zellij": Zellij, "tmux": Tmux}


# ---------------------------------------------------------------- workloads

def pane_cmd(kind, *args):
    return [PY, "-I", str(PANE), kind, *map(str, args)]


def exec_in_shell(sess, cmd):
    sess.send(("exec " + " ".join(cmd) + "\r").encode())


def read_stamp(path, timeout=5):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            return json.loads(Path(path).read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            time.sleep(0.05)
    return None


def window(sess, w0, b0, dt):
    """Byte rates for a measured window: what the client wrote to the
    outer terminal and, for Ship, what it received from its server."""
    out = len(sess.out) - b0
    w1 = sess.wire()
    wire = w1 - w0 if w0 is not None and w1 is not None else None
    return dict(bytes=out, bytes_s=out / dt, wire_bytes=wire, wire_bytes_s=wire / dt if wire is not None else None)


def w_idle(sess, cfg):
    w0 = sess.wire()
    t0, b0 = time.monotonic(), len(sess.out)
    c0, n = sess.cpu()
    sess.drain(cfg.idle_s)
    c1, _ = sess.cpu()
    dt = time.monotonic() - t0
    return dict(cpu_pct=100 * (c1 - c0) / dt, **window(sess, w0, b0, dt), procs=n,
                ok=True, check="prompt answered RDY-42 before the window")


def w_agent(sess, cfg):
    nonce = random.randrange(10**5, 10**6)
    st = sess.env.root / "stamp.json"
    w0 = sess.wire()
    c0, n = sess.cpu()
    b0 = len(sess.out)
    exec_in_shell(sess, pane_cmd("agent", "--seconds", cfg.agent_s, "--nonce", nonce, "--stamp", st))
    t0 = time.monotonic()
    seen_tokens = sess.wait(b"tokens", 10, b0)
    done = sess.wait(marker("agent", nonce).encode(), cfg.agent_s + 15, b0)
    t1 = time.monotonic()
    c1, _ = sess.cpu()
    return dict(cpu_pct=100 * (c1 - c0) / (t1 - t0), **window(sess, w0, b0, t1 - t0), procs=n,
                ok=seen_tokens and done, check=f"status line seen={seen_tokens} end marker seen={done}")


def flood(kind, mib):
    def run(sess, cfg):
        nonce = random.randrange(10**5, 10**6)
        st = sess.env.root / "stamp.json"
        w0 = sess.wire()
        c0, n = sess.cpu()
        b0 = len(sess.out)
        exec_in_shell(sess, pane_cmd(kind, "--mib", mib, "--nonce", nonce, "--stamp", st))
        done = sess.wait(marker("flood", nonce).encode(), cfg.flood_timeout, b0)
        t_seen = time.monotonic_ns()
        stamp = read_stamp(st)
        sess.drain(1.0)
        c1, _ = sess.cpu()
        w1 = sess.wire()
        if not (done and stamp):
            return dict(ok=False, check=f"end marker seen={done} stamp={bool(stamp)}", bytes=len(sess.out) - b0)
        wall = (t_seen - stamp["start_ns"]) / 1e9
        return dict(wall_ms=wall * 1e3, mb_s=stamp["bytes"] / 1e6 / wall,
                    writer_ms=(stamp["written_ns"] - stamp["start_ns"]) / 1e6, cpu_s=c1 - c0,
                    bytes=len(sess.out) - b0, wire_bytes=w1 - w0 if w0 is not None and w1 is not None else None,
                    input_bytes=stamp["bytes"], procs=n, ok=True,
                    check="end marker rendered")
    return run


def animation(kind):
    def run(sess, cfg):
        st = sess.env.root / "stamp.json"
        w0 = sess.wire()
        c0, n = sess.cpu()
        b0 = len(sess.out)
        exec_in_shell(sess, pane_cmd(kind, "--seconds", cfg.anim_s, "--fps", 60, "--stamp", st))
        t0 = time.monotonic()
        done = sess.wait(b"ANIMFINISHED", cfg.anim_s + 20, b0)
        t1 = time.monotonic()
        c1, _ = sess.cpu()
        stamp = read_stamp(st) or {}
        painted = len(TOKEN_RE.findall(bytes(sess.out[b0:])))
        secs = stamp.get("end_ns", 0) - stamp.get("start_ns", 0)
        return dict(cpu_pct=100 * (c1 - c0) / (t1 - t0), **window(sess, w0, b0, t1 - t0), procs=n,
                    frames_offered=stamp.get("frames"), painted_fps=painted / (secs / 1e9) if secs else None,
                    ok=done and painted > 0, check=f"finished={done} frame labels painted={painted}")
    return run


SIDE_TEXT = {"floodloop": b"status=", "agent": b"tokens"}


def split_latency(marks, keys_path, warmup):
    """Median of each leg of a key's trip: harness to probe (`in`), the
    probe itself (`program`), probe to harness (`out`). The probe logs one
    line per key it echoes, so lines pair with keys by order; any lost key
    breaks that pairing and the split is skipped."""
    try:
        probe = [tuple(map(int, l.split())) for l in keys_path.read_text().splitlines()]
    except FileNotFoundError:
        return {}
    if len(probe) != len(marks) or not all(found for *_, found in marks):
        return {}
    legs = [((r - t0) / 1e6, (w - r) / 1e6, (t1 - w) / 1e6)
            for (t0, t1, _), (r, w) in list(zip(marks, probe))[warmup:]]
    return {f"{name}_p50_ms": statistics.median(col) for name, col in zip(("in", "program", "out"), zip(*legs))}


def typing(side=None):
    def run(sess, cfg):
        if side:
            sess.mux.side_pane(sess.env, pane_cmd(*side))
            sess.drain(1.5)
        st = sess.env.root / "probe.json"
        b0 = len(sess.out)
        exec_in_shell(sess, pane_cmd("probe", "--stamp", st))
        if not sess.wait(b"probe-ready", 10, b0):
            return dict(ok=False, check="probe never started")
        sess.drain(1.0)
        rng = random.Random(1)
        lat, lost, marks = [], 0, []
        w0 = sess.wire()
        c0, n = sess.cpu()
        t_start, b1 = time.monotonic(), len(sess.out)
        for i in range(cfg.warmup + cfg.keys):
            want = TOKENS[i % 26]
            mark = len(sess.out)
            # The probe stamps with the same clock, so the gaps line up.
            t0 = time.monotonic_ns()
            sess.send(bytes([97 + i % 26]))
            # Search only new bytes so an earlier echo can't satisfy this key.
            found = False
            deadline = time.monotonic() + cfg.key_timeout
            pos = mark
            while time.monotonic() < deadline:
                if sess.out.find(want, pos) >= 0:
                    found = True
                    break
                pos = max(mark, len(sess.out) - 7)
                sess.pump(0.01)
            t1 = time.monotonic_ns()
            marks.append((t0, t1, found))
            if i >= cfg.warmup:
                if found:
                    lat.append((t1 - t0) / 1e6)
                else:
                    lost += 1
            sess.drain(rng.uniform(0.02, 0.06))
        c1, _ = sess.cpu()
        dt = time.monotonic() - t_start
        # The side pane must have kept painting during the probe window.
        side_seen = side is None or sess.out.find(SIDE_TEXT[side[0]], b1) >= 0
        ok = len(lat) >= cfg.keys * 0.95 and side_seen
        q = statistics.quantiles(lat, n=100, method="inclusive") if len(lat) > 2 else [None] * 99
        split = split_latency(marks, Path(f"{st}.keys"), cfg.warmup)
        (sess.env.root / "keys.json").write_text(json.dumps(marks))
        rates = window(sess, w0, b1, dt)
        sent = cfg.warmup + cfg.keys
        return dict(p50_ms=q[49], p95_ms=q[94], p99_ms=q[98], max_ms=max(lat, default=None), echoed=len(lat), **split,
                    lost=lost, cpu_pct=100 * (c1 - c0) / dt, **rates, bytes_per_key=rates["bytes"] / sent,
                    wire_bytes_per_key=rates["wire_bytes"] / sent if rates["wire_bytes"] is not None else None,
                    lat_ms=[round(x, 4) for x in lat], procs=n, ok=ok,
                    check=f"{len(lat)}/{cfg.keys} keys echoed, {lost} lost, side pane painting={side_seen}")
    return run


WORKLOADS = {
    "idle": w_idle,
    "agent": w_agent,
    "logflood": flood("logflood", 16),
    "uniflood": flood("uniflood", 8),
    "anim": animation("anim"),
    "noise": animation("noise"),
    "type_idle": typing(),
    "type_flood": typing(side=("floodloop", "--seconds", 600)),
    "type_agent": typing(side=("agent", "--seconds", 600)),
}


# ---------------------------------------------------------------- driver

def one(mux_name, workload, rep, cfg, scratch):
    mux = MUXES[mux_name]()
    env = Env(Path(scratch) / f"{mux_name}-{workload}-{rep}")
    if cfg.trace and mux_name == "ship":
        env.vars["SHIP_TRACE"] = str(env.root / "trace")
    rec = dict(mux=mux_name, workload=workload, rep=rep, started=time.strftime("%H:%M:%S"))
    # Wall and monotonic time read together, so timeline.py can place the
    # harness and probe stamps on Ship's timelines.
    (env.root / "clock.json").write_text(json.dumps(dict(unix_ns=time.time_ns(), monotonic_ns=time.monotonic_ns())))
    sess = None
    try:
        sess = Session(mux, env)
        rec["ready_s"] = sess.ready()
        sess.drain(cfg.settle)
        rec.update(WORKLOADS[workload](sess, cfg))
    except Exception as e:  # recorded, not fatal: other runs continue
        rec.update(ok=False, error=f"{type(e).__name__}: {e}")
    finally:
        if sess:
            rec["peak_rss_mb"] = sess.peak_rss / 1024
            rec["total_bytes"] = len(sess.out)
            tail = text(bytes(sess.out[-6000:]))
            rec["screen_tail"] = re.sub(r"\s+", " ", tail)[-300:]
            if cfg.keep_raw:
                (env.root / "outer.bin").write_bytes(bytes(sess.out))
            sess.close()
        left = sweep(env.home)
        rec["leftover_pids"] = left
    return rec


TYPING = ["p50_ms", "in_p50_ms", "program_p50_ms", "out_p50_ms", "p95_ms", "p99_ms", "lost", "cpu_pct",
          "bytes_per_key", "wire_bytes_per_key"]
STEADY = ["cpu_pct", "bytes_s", "wire_bytes_s", "peak_rss_mb"]
FLOOD = ["wall_ms", "mb_s", "cpu_s", "bytes", "wire_bytes", "peak_rss_mb"]
METRICS = {
    "idle": STEADY,
    "agent": STEADY,
    "logflood": FLOOD,
    "uniflood": FLOOD,
    "anim": ["cpu_pct", "painted_fps", "bytes_s", "wire_bytes_s", "peak_rss_mb"],
    "noise": ["cpu_pct", "painted_fps", "bytes_s", "wire_bytes_s", "peak_rss_mb"],
    "type_idle": TYPING,
    "type_flood": TYPING,
    "type_agent": TYPING,
}
POOLED = {"p95_ms": 95, "p99_ms": 99}  # tails from every key of every run, not a median of per-run tails


def fmt(v):
    return f"{v:,.0f}" if abs(v) >= 1000 else f"{v:.3g}" if abs(v) < 10 else f"{v:.1f}"


def per_run(records, mux, w, m):
    return [r[m] for r in records if r["mux"] == mux and r["workload"] == w and r.get(m) is not None]


def cell(records, mux, w, m):
    if m in POOLED:
        keys = [x for r in records if r["mux"] == mux and r["workload"] == w for x in r.get("lat_ms") or []]
        if len(keys) > 2:
            return statistics.quantiles(keys, n=100, method="inclusive")[POOLED[m] - 1]
    vals = per_run(records, mux, w, m)
    return statistics.median(vals) if vals else None


def bootstrap_diff(a, b, rng, n=2000):
    """95% interval for median(b) - median(a), resampling whole runs, so the
    spread between runs counts and not just the spread between keys."""
    diffs = sorted(statistics.median(rng.choices(b, k=len(b))) - statistics.median(rng.choices(a, k=len(a)))
                   for _ in range(n))
    return diffs[int(0.025 * n)], diffs[int(0.975 * n) - 1]


def summarize(records):
    flagged = [r for r in records if r.get("ok") and r.get("load_high")]
    good = [r for r in records if r.get("ok") and not r.get("load_high")]
    muxes = [m for m in MUXES if any(r["mux"] == m for r in records)]
    head = ["| workload metric | " + " | ".join(muxes) + " |", "|---|" + "---|" * len(muxes)]

    lines = ["Medians of per-run values; p95 and p99 pool every key across runs.", ""] + head
    for w, ms in METRICS.items():
        for m in ms:
            cells = [cell(good, mux, w, m) for mux in muxes]
            if any(c is not None for c in cells):
                lines.append(f"| {w} {m} | " + " | ".join("-" if c is None else fmt(c) for c in cells) + " |")

    others = [m for m in muxes if m != "ship"]
    if "ship" in muxes and others:
        rng = random.Random(0)
        lines += ["", "Each tool minus Ship: median difference [95% interval, bootstrap over runs].", "",
                  "| workload metric | " + " | ".join(others) + " |", "|---|" + "---|" * len(others)]
        for w, ms in METRICS.items():
            for m in ms:
                if m in POOLED or m == "lost":
                    continue
                base = per_run(good, "ship", w, m)
                cells = []
                for mux in others:
                    vals = per_run(good, mux, w, m)
                    if len(base) < 2 or len(vals) < 2:
                        cells.append("-")
                        continue
                    lo, hi = bootstrap_diff(base, vals, rng)
                    d = statistics.median(vals) - statistics.median(base)
                    cells.append(f"{d:+.3g} [{lo:+.3g}, {hi:+.3g}]")
                if any(c != "-" for c in cells):
                    lines.append(f"| {w} {m} | " + " | ".join(cells) + " |")

    lines += ["", "Per-run p50 latency (ms), in run order:"]
    for w in ("type_idle", "type_flood", "type_agent"):
        for mux in muxes:
            vals = per_run(good, mux, w, "p50_ms")
            if vals:
                lines.append(f"  {w:10s} {mux:6s} " + " ".join(f"{v:.2f}" for v in vals))

    fails = [r for r in records if not r.get("ok")]
    lines += ["", f"runs: {len(records)}, failed: {len(fails)}, over the load limit: {len(flagged)} (left out)"]
    for r in fails:
        lines.append(f"  FAIL {r['mux']} {r['workload']} rep{r['rep']}: {r.get('error') or r.get('check')}")
    for r in flagged:
        lines.append(f"  LOAD {r['mux']} {r['workload']} rep{r['rep']}: load1 {r['load_before']:.2f}")
    return "\n".join(lines)


def sha256(path):
    try:
        return hashlib.sha256(Path(path).read_bytes()).hexdigest()
    except OSError:
        return None


def capture(*argv):
    try:
        res = subprocess.run(argv, capture_output=True, text=True, timeout=30)
        return (res.stdout + res.stderr).strip()
    except (OSError, subprocess.TimeoutExpired) as e:
        return f"{type(e).__name__}: {e}"


def machine():
    """What the machine was doing, so a result can be judged later: power
    and thermal settings, sleep assertions, Spotlight and Time Machine."""
    return dict(
        python=platform.python_version(),
        hw_model=capture("sysctl", "-n", "hw.model"),
        os_build=capture("sw_vers"),
        ncpu=os.cpu_count(),
        pmset_custom=capture("pmset", "-g", "custom"),
        pmset_therm=capture("pmset", "-g", "therm"),
        pmset_assertions=capture("pmset", "-g", "assertions"),
        spotlight=capture("mdutil", "-s", "/"),
        time_machine=capture("tmutil", "status"),
    )


def wait_for_load(cfg):
    """Wait up to --load-wait for the 1-minute load average to fall to
    --max-load. Returns the load seen and whether it was still too high."""
    deadline = time.monotonic() + cfg.load_wait
    while (load := os.getloadavg()[0]) > cfg.max_load and time.monotonic() < deadline:
        time.sleep(5)
    return load, load > cfg.max_load


def interrupt(*_):
    raise KeyboardInterrupt


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--mux", nargs="+", default=list(MUXES), choices=list(MUXES))
    p.add_argument("--workload", nargs="+", default=list(WORKLOADS), choices=list(WORKLOADS))
    p.add_argument("--reps", type=int, default=5)
    p.add_argument("--seed", type=int, help="seed for the run order (default: random, recorded in the results)")
    p.add_argument("--cooldown", type=float, default=1.0, help="seconds between runs")
    p.add_argument("--max-load", type=float, default=4.0,
                   help="1-minute load average above which a run waits, and is left out if it stays high")
    p.add_argument("--load-wait", type=float, default=120, help="seconds to wait for the load to fall")
    p.add_argument("--idle-s", type=float, default=10)
    p.add_argument("--agent-s", type=float, default=15)
    p.add_argument("--anim-s", type=float, default=5)
    p.add_argument("--keys", type=int, default=100)
    p.add_argument("--warmup", type=int, default=10)
    p.add_argument("--key-timeout", type=float, default=2.0)
    p.add_argument("--flood-timeout", type=float, default=120)
    p.add_argument("--settle", type=float, default=1.0)
    p.add_argument("--scratch", default="/tmp/ship-bench-runs")
    p.add_argument("--keep-raw", action=argparse.BooleanOptionalAction, default=True,
                   help="save every byte the client wrote as <scratch>/<run>/outer.bin")
    p.add_argument("--trace", action="store_true",
                   help="record Ship timelines to <scratch>/<run>/trace (slows Ship; profiling only)")
    p.add_argument("--out", default=str(HERE / "results"))
    p.add_argument("--summarize", metavar="JSONL", help="print the summary for a results file and exit")
    cfg = p.parse_args()
    # SIGTERM unwinds like Ctrl-C, so the current run's teardown still runs.
    signal.signal(signal.SIGTERM, interrupt)
    if cfg.summarize:
        records = [json.loads(l) for l in open(cfg.summarize)]
        print(summarize([r for r in records if r.get("kind") != "meta"]))
        return
    load, high = wait_for_load(cfg)
    if high:
        sys.exit(f"load average {load:.2f} is above --max-load {cfg.max_load}; quit what's busy and try again")
    seed = cfg.seed if cfg.seed is not None else random.randrange(1 << 32)
    order_rng = random.Random(seed)
    Path(cfg.out).mkdir(exist_ok=True)
    path = Path(cfg.out) / time.strftime("results-%Y%m%d-%H%M%S.jsonl")
    repo = str(HERE.parent)
    bins = {"ship": SHIP_BIN, "herdr": HERDR_BIN, "zellij": ZELLIJ_BIN, "tmux": TMUX_BIN}
    meta = dict(kind="meta", ship_bin=SHIP_BIN, cols=COLS, rows=ROWS, argv=sys.argv, seed=seed,
                ship_rev=capture("git", "-C", repo, "rev-parse", "HEAD"),
                git_status_sha256=hashlib.sha256(capture("git", "-C", repo, "status", "--porcelain").encode()).hexdigest(),
                versions={m: capture(b, "-V" if m == "tmux" else "--version") for m, b in bins.items()},
                binary_sha256={m: sha256(b) for m, b in bins.items()},
                machine=machine(), load=os.getloadavg())
    records = []
    with open(path, "w") as f:
        f.write(json.dumps(meta) + "\n")
        for rep in range(cfg.reps):
            for w in cfg.workload:
                # A fresh random order in every block, so no tool always runs
                # right after another one's flood.
                order = order_rng.sample(cfg.mux, len(cfg.mux))
                for m in order:
                    load, high = wait_for_load(cfg)
                    rec = one(m, w, rep, cfg, cfg.scratch)
                    rec.update(load_before=load, load_high=high, load1=os.getloadavg()[0])
                    records.append(rec)
                    f.write(json.dumps(rec) + "\n")
                    f.flush()
                    status = "ok" if rec.get("ok") else "FAIL " + (rec.get("error") or rec.get("check", ""))
                    if high:
                        status += f" (load {load:.2f}, left out)"
                    print(f"[{rec['started']}] rep{rep} {w:10s} {m:6s} {status}", flush=True)
                    time.sleep(cfg.cooldown)
    print(f"\nresults: {path}\n")
    print(summarize(records))


if __name__ == "__main__":
    main()
