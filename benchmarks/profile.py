#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["psutil>=7", "pyte>=0.8"]
# ///
"""Record on-CPU profiles of Ship's server and client under one workload.

    SHIP_BENCH_BIN=<binary with symbols> uv run profile.py scale --out /tmp/ship-prof/scale
    python3 profsum.py /tmp/ship-prof/scale/server.json.gz

Both processes run under `samply record --save-only --unstable-presymbolicate`,
so each profile has a .syms.json beside it and needs no debug files to read.
The binary needs symbols: build it with
`CARGO_PROFILE_RELEASE_STRIP=false CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`
(see README.md). Workloads:

- `scale`: `--tabs` tabs of `--panes` agent screens with a title spinner,
  the client attached on its own shell tab, for `--seconds`
- `flood`: bench.py's type_flood (a flooding side pane and the probe),
  typing for `--seconds`
- `type`: bench.py's type_idle (the probe alone on a quiet shell), typing
  for `--seconds`
- `idle`: a client on one quiet shell, for `--seconds`
"""

import argparse
import json
import os
import random
import shutil
import signal
import subprocess
import time
from pathlib import Path

import bench
from bench import SHIP_BIN, SHIP_NAME, Env, cli, pane_cmd, sweep
import extra


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("workload", choices=["scale", "flood", "type", "idle"])
    p.add_argument("--out", type=Path, required=True)
    p.add_argument("--tabs", type=int, default=20)
    p.add_argument("--panes", type=int, default=3)
    p.add_argument("--seconds", type=float, default=15)
    p.add_argument("--title", action=argparse.BooleanOptionalAction, default=True)
    a = p.parse_args()
    bench.RSS_SAMPLE_S = 0  # the 1 s sampler stalls the reader and skews latency
    a.out.mkdir(parents=True, exist_ok=True)
    env = Env(Path("/tmp/ship-prof-runs") / a.workload)
    cfgf = env.root / "config/ship/config.toml"
    cfgf.parent.mkdir(parents=True, exist_ok=True)
    cfgf.touch()
    env.vars["SHIP_CONFIG"] = str(cfgf)
    SAMPLY = shutil.which("samply") or "samply"
    sam = lambda name: [SAMPLY, "record", "--save-only", "--unstable-presymbolicate", "-o",
                        str(a.out / f"{name}.json.gz"), "--"]
    log = open(env.root / "ship-server.log", "wb")
    server = subprocess.Popen(sam("server") + [SHIP_BIN, "server", "--name", SHIP_NAME, "--starter"], env=env.vars,
                              stdout=log, stderr=log, stdin=subprocess.DEVNULL, cwd=env.home, start_new_session=True)
    for _ in range(200):
        try:
            url = extra.ship_url(env)
            break
        except Exception:
            time.sleep(0.1)
    if a.workload == "scale":
        agent = pane_cmd("agent", "--seconds", 900, *(["--title"] if a.title else []))
        for t in range(a.tabs):
            extra.ship_tab(env, url, [agent] * a.panes, place="down")
    time.sleep(1.0)

    class Wrapped(extra.PreStarted):
        def client_argv(self, env):
            return sam("client") + [SHIP_BIN, "--server-url", self.url]

    started = type("S", (), {"server": server, "url": url})()
    mux = Wrapped(started)
    sess = bench.Session(mux, env)
    info = {}
    try:
        try:
            sess.ready()
        except OSError:
            sess.pump(0.5)
            raise SystemExit("client exited during startup; its output: " + repr(bytes(sess.out[-600:])))
        if a.workload in ("flood", "type"):
            if a.workload == "flood":
                mux.side_pane(env, pane_cmd("floodloop", "--seconds", 600))
            sess.drain(1.5)
            tcfg = argparse.Namespace(warmup=10, keys=int(a.seconds / 0.045), key_timeout=2.0)
            r = bench.typing()(sess, tcfg)
            info = {k: r.get(k) for k in ("p50_ms", "p99_ms", "max_ms", "echoed", "lost")}
        else:
            sess.drain(a.seconds)
        info["tab_list_bytes"] = len(cli([SHIP_BIN, "tab", "list", "--server-url", url], env))
    finally:
        # Stop the client with a detach-like signal so samply writes its file.
        # The harness's child is samply; the Ship client is samply's child.
        import psutil
        for child in psutil.Process(sess.pid).children():
            child.send_signal(signal.SIGTERM)
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline and psutil.Process(sess.pid).status() != psutil.STATUS_ZOMBIE:
            sess.pump(0.1)
        try:
            cli([SHIP_BIN, "server", "stop", "--server-url", url], env, timeout=20)
        except Exception:
            pass
        try:
            server.wait(timeout=60)
        except subprocess.TimeoutExpired:
            server.kill()
        sess.stop_sampling.set()
        try:
            os.waitpid(sess.pid, 0)
        except ChildProcessError:
            pass
        sweep(env.home)
    (a.out / "info.json").write_text(json.dumps(info))
    print(json.dumps(info))


if __name__ == "__main__":
    main()
