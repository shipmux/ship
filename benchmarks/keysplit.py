#!/usr/bin/env python3
"""Split every key of typing runs at the probe, and show where the slow ones lost time.

    python3 keysplit.py /tmp/ship-bench-runs/ship-type_flood-*
    python3 keysplit.py --slow 2 /tmp/ship-bench-runs/*-type_flood-*

For each run directory (one `bench.py` typing run, kept with --keep-raw),
pairs the harness's per-key send/seen stamps (`keys.json`) with the probe's
read/write stamps (`probe.json.keys`) and reports, per tool:

- how many keys exceeded `--slow` ms and their share
- for those slow keys, the `in` leg (harness to probe, the input path),
  `program` (the probe itself) and `out` leg (probe to harness, the output path)
- the latency histogram of all keys in coarse buckets, so a quantized tail
  (one frame, two frames) shows up as a spike and a smeared tail does not

Warmup keys (the first `--warmup`, 10 by default as in bench.py) are skipped.
A run with a lost key has no pairing and is skipped.
"""

import argparse
import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path

BUCKETS = [0.5, 1, 2, 4, 6, 8, 10, 12, 16, 25, 50, 1e9]


def legs(run, warmup):
    keys = json.loads((run / "keys.json").read_text())
    probe = [tuple(map(int, l.split())) for l in (run / "probe.json.keys").read_text().splitlines()]
    if len(probe) != len(keys) or not all(found for *_, found in keys):
        return None
    out = []
    for i, ((t0, t1, _), (r, w)) in enumerate(zip(keys, probe)):
        if i < warmup:
            continue
        out.append(dict(i=i, total=(t1 - t0) / 1e6, inn=(r - t0) / 1e6, prog=(w - r) / 1e6,
                        out=(t1 - w) / 1e6, t0=t0))
    return out


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("runs", nargs="+", type=Path)
    p.add_argument("--slow", type=float, default=2.0, help="ms above which a key counts as slow")
    p.add_argument("--warmup", type=int, default=10)
    p.add_argument("--list", action="store_true", help="print every slow key")
    a = p.parse_args()
    by_tool = defaultdict(list)
    for run in a.runs:
        if not (run / "keys.json").exists():
            continue
        got = legs(run, a.warmup)
        if got is None:
            print(f"skip {run.name}: lost keys, no pairing", file=sys.stderr)
            continue
        tool = run.name.split("-")[0]
        by_tool[tool] += [dict(k, run=run.name) for k in got]
    for tool, ks in by_tool.items():
        slow = [k for k in ks if k["total"] > a.slow]
        tot = sorted(k["total"] for k in ks)
        q = statistics.quantiles(tot, n=100, method="inclusive")
        print(f"\n{tool}: {len(ks)} keys, p50 {q[49]:.2f} p95 {q[94]:.2f} p99 {q[98]:.2f} max {tot[-1]:.2f} ms;"
              f" slow (>{a.slow} ms): {len(slow)} ({100 * len(slow) / len(ks):.1f}%)")
        if slow:
            med = lambda f: statistics.median(k[f] for k in slow)
            print(f"  slow keys, median legs: in {med('inn'):.2f}  program {med('prog'):.3f}  out {med('out'):.2f} ms")
            inn_dom = sum(k["inn"] > k["out"] for k in slow)
            print(f"  slow keys where the input leg is the larger: {inn_dom}/{len(slow)}")
        hist, lo = [], 0
        for hi in BUCKETS:
            n = sum(lo < x <= hi for x in tot)
            if n:
                hist.append(f"{lo:g}-{hi:g}:{n}" if hi < 1e9 else f">{lo:g}:{n}")
            lo = hi
        print("  histogram (ms:count): " + "  ".join(hist))
        if a.list:
            for k in sorted(slow, key=lambda k: -k["total"]):
                print(f"    {k['run']} key {k['i']:3d}: total {k['total']:6.2f}  in {k['inn']:6.2f}"
                      f"  program {k['prog']:.3f}  out {k['out']:6.2f}")


if __name__ == "__main__":
    main()
