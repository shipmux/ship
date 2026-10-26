#!/usr/bin/env python3
"""Merge one traced typing run into a single timeline for Perfetto.

    uv run bench.py --mux ship --workload type_idle --reps 1 --trace
    python3 timeline.py /tmp/ship-bench-runs/ship-type_idle-0

Writes <run>/merged.json and prints its path. Open it at
https://ui.perfetto.dev. It holds the client and server timelines on one
clock, a harness track with one slice per key from send to echo seen, and a
probe track with one slice per key from the probe's read to its echo write.

Each Ship timeline starts its clock at zero and logs a "timeline anchor"
with the wall time; the harness wrote its wall and monotonic time together
to clock.json; the probe shares the harness's monotonic clock. Time zero is
the harness's clock reading.
"""

import json
import sys
from pathlib import Path

HARNESS_PID, PROBE_PID = 100, 101


def events(path):
    """A tracing-chrome file's events. A process killed before flushing
    leaves the array unclosed, so fall back to one event per line."""
    text = path.read_text()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        found = []
        for line in text.splitlines():
            line = line.strip().rstrip(",")
            if line.startswith("{"):
                try:
                    found.append(json.loads(line))
                except json.JSONDecodeError:
                    pass
        return found


def ship_track(path, pid, base_us):
    evs = events(path)
    anchor = next((e for e in evs if "unix_us" in e.get("args", {})), None)
    if anchor is None:
        sys.exit(f"{path}: no timeline anchor; was it recorded by this build?")
    shift = int(anchor["args"]["unix_us"]) - anchor["ts"] - base_us
    role = path.stem.split("-")[0]
    out = [dict(ph="M", pid=pid, name="process_name", args=dict(name=f"ship {role}"))]
    for e in evs:
        e = dict(e, pid=pid)
        # tracing-chrome names events by file and line; the message reads better.
        if e.get("ph") == "i" and "message" in e.get("args", {}):
            e["name"] = e["args"]["message"]
        if "ts" in e:
            e["ts"] += shift
        out.append(e)
    return out


def slices(pid, name, label, spans, mono0):
    out = [dict(ph="M", pid=pid, name="process_name", args=dict(name=name))]
    for i, (start, end) in enumerate(spans):
        out.append(dict(ph="X", pid=pid, tid=1, name=f"{label} {i}",
                        ts=(start - mono0) / 1e3, dur=(end - start) / 1e3))
    return out


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    run = Path(sys.argv[1])
    clock = json.loads((run / "clock.json").read_text())
    base_us, mono0 = clock["unix_ns"] // 1000, clock["monotonic_ns"]
    merged = []
    for pid, path in enumerate(sorted((run / "trace").glob("*.json")), start=1):
        merged += ship_track(path, pid, base_us)
    keys = json.loads((run / "keys.json").read_text())
    merged += slices(HARNESS_PID, "harness", "key", [(t0, t1) for t0, t1, found in keys if found], mono0)
    probe = [tuple(map(int, l.split())) for l in (run / "probe.json.keys").read_text().splitlines()]
    merged += slices(PROBE_PID, "probe", "echo", probe, mono0)
    dest = run / "merged.json"
    dest.write_text(json.dumps(dict(traceEvents=merged)))
    print(dest)


if __name__ == "__main__":
    main()
