#!/usr/bin/env python3
"""Walk one traced typing run key by key: where did each slow key wait?

    uv run bench.py --mux ship --workload type_flood --reps 1 --trace
    python3 keytrace.py /tmp/ship-bench-runs/ship-type_flood-0 [--slow 2] [--key N]

Puts Ship's client and server timelines, the harness's per-key stamps and the
probe's read/write stamps on one clock (as timeline.py does), then, for each
key, finds the hops Ship owns:

    sent      harness wrote the key to the client's PTY
    out       client: "input sent" (the key leaves the client)
    route     server: `route` span (input request handled)
    pty       server: "pty write" span of 1 byte (the key reaches the PTY)
    read      probe read the key
    echo      probe wrote the echo
    parse     server: first `vt_write` on the probe pane's session thread after the echo
    publish   server: the probe pane's `publish` span (the one after `parse`)
    decode    client: first small "decode" (a probe-sized screen) after `publish`
    drawn     client: end of the first `redraw` after `decode`
    seen      harness saw the echo

and prints, for slow keys, the gap before each hop. With --key, it also
dumps every client and server event in that key's window, so a gap can be
read event by event. The probe pane's session thread is the one that writes
1-byte keys to its PTY; probe screens are told apart from flood screens by
their size (`--small` bytes).
"""

import argparse
import json
import statistics
import sys
from pathlib import Path

from timeline import events


def load(run):
    clock = json.loads((run / "clock.json").read_text())
    base_us, mono0 = clock["unix_ns"] // 1000, clock["monotonic_ns"]
    tracks = {}
    for path in sorted((run / "trace").glob("*.json")):
        evs = events(path)
        anchor = next((e for e in evs if "unix_us" in e.get("args", {})), None)
        if anchor is None:
            sys.exit(f"{path}: no timeline anchor")
        shift = int(anchor["args"]["unix_us"]) - anchor["ts"] - base_us
        role = path.stem.split("-")[0]
        out = []
        for e in evs:
            if "ts" not in e:
                continue
            e = dict(e)
            e["t"] = (e["ts"] + shift) / 1e3  # ms since the harness's clock zero
            if e.get("ph") == "i" and "message" in e.get("args", {}):
                e["name"] = e["args"]["message"]
            out.append(e)
        out.sort(key=lambda e: e["t"])
        tracks[role] = out
    keys = json.loads((run / "keys.json").read_text())
    probe = [tuple(map(int, l.split())) for l in (run / "probe.json.keys").read_text().splitlines()]
    ms = lambda ns: (ns - mono0) / 1e6
    return tracks, [(ms(a), ms(b), f) for a, b, f in keys], [(ms(a), ms(b)) for a, b in probe]


def spans(track, name):
    """(start, end, args, tid) for B/E pairs or X events named `name`."""
    open_, out = {}, []
    for e in track:
        if e.get("name") != name:
            continue
        ph = e.get("ph")
        if ph == "X":
            out.append((e["t"], e["t"] + e.get("dur", 0) / 1e3, e.get("args", {}), e.get("tid")))
        elif ph == "B":
            open_.setdefault(e.get("tid"), []).append(e)
        elif ph == "E" and open_.get(e.get("tid")):
            b = open_[e.get("tid")].pop()
            out.append((b["t"], e["t"], b.get("args", {}), e.get("tid")))
    out.sort(key=lambda s: (s[0], s[1]))
    return out


def first(items, after, pred=lambda x: True):
    for x in items:
        if x[0] >= after and pred(x):
            return x
    return None


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("run", type=Path)
    p.add_argument("--slow", type=float, default=2.0)
    p.add_argument("--small", type=int, default=3000, help="decode bytes at or under this are a probe screen")
    p.add_argument("--key", type=int, action="append", default=[], help="dump every event in this key's window")
    p.add_argument("--warmup", type=int, default=10)
    a = p.parse_args()
    tracks, keys, probe = load(a.run)
    client, server = tracks.get("client", []), tracks.get("server", [])
    sent_ev = [(e["t"], e) for e in client if e.get("name") == "input sent"]
    route = spans(server, "route")
    ptyw = spans(server, "pty write")
    probe_tids = {s[3] for s in ptyw if str(s[2].get("bytes")) == "1"}
    vtw = [s for s in spans(server, "vt_write") if s[3] in probe_tids]
    publish = spans(server, "publish")
    decode = spans(client, "decode")
    redraw = spans(client, "redraw")
    rows = []
    for i, ((t0, t1, found), (r, w)) in enumerate(zip(keys, probe)):
        if i < a.warmup or not found:
            continue
        hop = {"sent": t0}
        x = first(sent_ev, t0)
        hop["out"] = x[0] if x else None
        x = first(route, t0)
        hop["route"] = x[0] if x else None
        x = first(ptyw, t0, lambda s: s[3] in probe_tids)
        hop["pty"] = x[0] if x else None
        hop["read"], hop["echo"] = r, w
        x = first(vtw, w)
        hop["parse"] = x[0] if x else None
        x = first(publish, hop["parse"] or w)
        hop["publish"] = x[1] if x else None
        x = first(decode, hop["publish"] or w, lambda s: int(s[2].get("bytes", 1 << 30)) <= a.small)
        hop["decode"] = x[0] if x else None
        x = first(redraw, hop["decode"] or w)
        hop["drawn"] = x[1] if x else None
        hop["seen"] = t1
        rows.append((i, t1 - t0, hop))
    names = ["out", "route", "pty", "read", "echo", "parse", "publish", "decode", "drawn", "seen"]
    gaps = {n: [] for n in names}
    slow = [row for row in rows if row[1] > a.slow]
    print(f"{len(rows)} keys, {len(slow)} over {a.slow} ms")
    print("gap before each hop, ms (median over all keys | median over slow keys)")
    order = ["sent"] + names

    def gap(hop, n):
        if hop.get(n) is None:
            return None
        before = [hop[m] for m in order[: order.index(n)] if hop.get(m) is not None]
        return hop[n] - before[-1]

    for n in names:
        allg = [g for _, _, h in rows if (g := gap(h, n)) is not None]
        sg = [g for _, _, h in slow if (g := gap(h, n)) is not None]
        print(f"  {n:8s} {statistics.median(allg) if allg else float('nan'):7.3f} | "
              f"{statistics.median(sg) if sg else float('nan'):7.3f}")
    for i, total, hop in sorted(slow, key=lambda r: -r[1]):
        parts = []
        prev = hop["sent"]
        for n in names:
            v = hop.get(n)
            parts.append(f"{n} +{v - prev:.2f}" if v is not None else f"{n} ?")
            if v is not None:
                prev = v
        print(f"key {i:3d} total {total:6.2f}: " + "  ".join(parts))
    for k in a.key:
        t0, t1, _ = keys[k]
        print(f"\n--- key {k}: {t0:.3f} .. {t1:.3f}")
        for role, tr in tracks.items():
            for e in tr:
                if t0 - 0.5 <= e["t"] <= t1 + 0.5 and e.get("name") not in ("pty read",):
                    args = {kk: v for kk, v in e.get("args", {}).items() if kk not in ("message",)}
                    print(f"  {e['t'] - t0:8.3f} {role:6s} tid {e.get('tid')!s:>6} {e.get('ph')} {e.get('name')} {args}")


if __name__ == "__main__":
    main()
