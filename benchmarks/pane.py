"""Programs that run inside a multiplexer pane during a benchmark.

Each one is started by `bench.py` with `exec python3 -I pane.py <kind> ...`.
They write only to their own terminal, so the multiplexer is the only thing
between them and the harness PTY. Timing facts go to a side file (`--stamp`)
as monotonic nanoseconds, which share a clock with the harness on macOS.
Flood programs print their end marker and then sleep, so the pane stays open
until the harness tears the session down.
"""

import argparse
import fcntl
import json
import os
import random
import struct
import sys
import termios
import time
import tty

ALPHA = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"


def token(index):
    """Eight consecutive capitals starting at `index`. Neighbouring indices
    differ in every cell, so a diffing renderer must repaint all eight."""
    return "".join(ALPHA[(index + j) % 26] for j in range(8))


def marker(kind, nonce):
    """An end marker the command line never contains: capitals spelled from
    the nonce's digits. Pane content is lowercase, so a diffing renderer has
    to repaint every cell of it."""
    return kind.upper() + "".join(ALPHA[10 + int(d)] for d in str(nonce))


def size():
    rows, cols, _, _ = struct.unpack("HHHH", fcntl.ioctl(1, termios.TIOCGWINSZ, b"\0" * 8))
    return max(cols, 20), max(rows, 5)


def out(data):
    if isinstance(data, str):
        data = data.encode()
    view = memoryview(data)
    while view:
        n = os.write(1, view)
        view = view[n:]


def stamp(path, **facts):
    if path:
        with open(path, "w") as f:
            json.dump(facts, f)


def park():
    while True:
        time.sleep(3600)


def log_lines(count, seed=7):
    rng = random.Random(seed)
    paths = ["/api/v1/items", "/api/v1/users", "/healthz", "/api/v2/search", "/static/app.js"]
    lines = []
    for i in range(count):
        lines.append(
            f"2026-10-24t18:{i // 60000 % 60:02d}:{i // 1000 % 60:02d}.{i % 1000:03d} info worker-{rng.randrange(32):02d} "
            f"req={rng.getrandbits(48):012x} status={rng.choice((200, 200, 200, 204, 404, 500))} "
            f"ms={rng.randrange(1, 900):3d} path={rng.choice(paths)}/{rng.randrange(100000)}\n"
        )
    return "".join(lines).encode()


def unicode_lines(count, seed=11):
    rng = random.Random(seed)
    pieces = ["漢字かなカナ", "한국어 텍스트", "émojis 🚀🧪✅🔥", "éàô", "αβγδε ζηθ", "кириллица",
              "→ ⇒ ∀ ∃ ∑ √", "▁▂▃▄▅▆▇█", "┌─┬─┐│", "日本語の文章です", "表情 😀😃😄😁", "x⃝ y⃣"]
    lines = []
    for i in range(count):
        lines.append(f"{i:07d} " + " ".join(rng.choice(pieces) for _ in range(6)) + "\n")
    return "".join(lines).encode()


def flood(kind, mib, nonce, stamp_path):
    make = log_lines if kind == "log" else unicode_lines
    block = make(4000)
    reps = max(1, int(mib * 1024 * 1024 / len(block)))
    data = block * reps
    out("\x1b[H\x1b[2J")
    start = time.monotonic_ns()
    out(data)
    out(f"\n{marker('flood', nonce)}\n")
    end = time.monotonic_ns()
    stamp(stamp_path, start_ns=start, written_ns=end, bytes=len(data))
    park()


def flood_loop(seconds):
    """Continuous log output for the side pane of a typing test."""
    block = log_lines(2000, seed=3)
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        out(block)
    out("\nside-flood-finished\n")
    park()


def agent(seconds, nonce, stamp_path):
    """A coding-agent-like screen: a static transcript, a 10 Hz spinner and
    token counter on the status line, and a new transcript line every 1.5 s
    scrolled inside a region, the way an agent TUI appends output."""
    cols, rows = size()
    rng = random.Random(5)
    words = "the build reads config parses tokens checks layout writes files runs tests and reports".split()
    out("\x1b[?25l\x1b[H\x1b[2J")
    out("\x1b[1;1H\x1b[1m agent session\x1b[0m")
    for r in range(3, rows - 2):
        out(f"\x1b[{r};1H  " + " ".join(rng.choice(words) for _ in range(cols // 8))[: cols - 4])
    out(f"\x1b[{rows - 1};1H" + "─" * (cols - 1))
    out(f"\x1b[3;{rows - 3}r")  # scroll region for the transcript
    spinner = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"
    start = time.monotonic()
    start_ns = time.monotonic_ns()
    tick = 0
    next_line = start + 1.5
    while True:
        now = time.monotonic()
        elapsed = now - start
        if elapsed >= seconds:
            break
        if now >= next_line:
            line = " ".join(rng.choice(words) for _ in range(cols // 8))[: cols - 4]
            out(f"\x1b[{rows - 3};1H\n  {line}")
            next_line += 1.5
        out(f"\x1b[{rows};2H\x1b[36m{spinner[tick % 10]}\x1b[0m thinking {elapsed:5.1f}s · {tick * 37:6d} tokens  ")
        tick += 1
        time.sleep(max(0.0, start + tick * 0.1 - time.monotonic()))
    out(f"\x1b[r\x1b[{rows};2H{marker('agent', nonce)}\x1b[K")
    stamp(stamp_path, start_ns=start_ns, end_ns=time.monotonic_ns(), ticks=tick)
    park()


def anim(seconds, fps, stamp_path, noise=False):
    """Full-screen truecolor frames. Every cell gets a new background each
    frame; the top-left label is a frame token the harness counts. The
    default is a smooth gradient, which repeats within a frame and so
    compresses well; `noise` gives every cell a random colour, which
    leaves a compressor little to find within one frame."""
    cols, rows = size()
    out("\x1b[?25l")
    start = time.monotonic()
    start_ns = time.monotonic_ns()
    frame = 0
    while time.monotonic() - start < seconds:
        t = frame * 0.07
        parts = ["\x1b[H"]
        rgb = os.urandom(rows * cols * 3) if noise else None
        for r in range(rows):
            parts.append(f"\x1b[{r + 1};1H")
            row = []
            for c in range(cols):
                if noise:
                    i = (r * cols + c) * 3
                    row.append(f"\x1b[48;2;{rgb[i]};{rgb[i + 1]};{rgb[i + 2]}m ")
                    continue
                v = (c * 3 + r * 5 + frame * 4) & 255
                row.append(f"\x1b[48;2;{v};{(v + int(t * 40)) & 255};{(255 - v) & 255}m ")
            parts.append("".join(row))
        parts.append(f"\x1b[0m\x1b[1;1Hf{token(frame)}")
        out("".join(parts))
        frame += 1
        time.sleep(max(0.0, start + frame / fps - time.monotonic()))
    out("\x1b[0m\x1b[2J\x1b[3;1HANIMFINISHED")
    stamp(stamp_path, start_ns=start_ns, end_ns=time.monotonic_ns(), frames=frame)
    park()


def probe(stamp_path):
    """Echo probe for latency: each received lowercase letter repaints one
    fixed spot with its token, so one keypress gives one recognisable echo."""
    tty.setraw(0)
    out("\x1b[2J\x1b[1;1Hprobe-ready ")
    stamp(stamp_path, ready_ns=time.monotonic_ns())
    # One "read_ns write_ns" line per echoed key, so the harness can split
    # its latency at the program.
    keys = open(stamp_path + ".keys", "w", buffering=1) if stamp_path else None
    while True:
        data = os.read(0, 64)
        got = time.monotonic_ns()
        if not data:
            return
        for b in data:
            if 97 <= b <= 122:
                out(f"\x1b[3;1H{token(b - 97)}")
                if keys:
                    keys.write(f"{got} {time.monotonic_ns()}\n")


def main():
    p = argparse.ArgumentParser()
    p.add_argument("kind", choices=["logflood", "uniflood", "floodloop", "agent", "anim", "noise", "probe"])
    p.add_argument("--seconds", type=float, default=60)
    p.add_argument("--mib", type=float, default=16)
    p.add_argument("--fps", type=float, default=60)
    p.add_argument("--nonce", type=int, default=0)
    p.add_argument("--stamp")
    a = p.parse_args()
    if a.kind in ("logflood", "uniflood"):
        flood("log" if a.kind == "logflood" else "uni", a.mib, a.nonce, a.stamp)
    elif a.kind == "floodloop":
        flood_loop(a.seconds)
    elif a.kind == "agent":
        agent(a.seconds, a.nonce, a.stamp)
    elif a.kind in ("anim", "noise"):
        anim(a.seconds, a.fps, a.stamp, noise=a.kind == "noise")
    else:
        probe(a.stamp)


if __name__ == "__main__":
    sys.exit(main())
