"""Compare Ship's numbers in two bench.py results files as a Markdown table.

    python3 compare.py HEAD.jsonl [--base BASE.jsonl] [--size SIZE.json] [--base-size SIZE.json]

CI runs this on every pull request (.github/workflows/bench-report.yml), with
the latest run on main as the base. Shared runners are noisy, so only
metrics that come out the same run after run are compared; the timings are
shown for reference. Standard library only: the report job doesn't install
anything.
"""

import argparse
import json
import statistics

MARKER = "<!-- ship-bench -->"
# Flag a compared metric that moved by more than this fraction.
THRESHOLD = 0.05

# (workload, metric, label, unit, compared). Compared metrics moved about 1%
# between runs on one machine; the rest are timings and CPU, which follow
# whatever else the runner is doing, and the flood's bytes, which depend on
# how many screens the server published while it ran (16-26 KB).
METRICS = [
    ("type_idle", "bytes_per_key", "typing: bytes to the terminal per key", "B", True),
    ("type_idle", "wire_bytes_per_key", "typing: bytes from the server per key", "B", True),
    ("agent", "wire_bytes_s", "agent screen: bytes from the server", "B/s", True),
    ("idle", "wire_bytes_s", "idle: bytes from the server", "B/s", True),
    ("anim", "wire_bytes_s", "animation: bytes from the server", "B/s", True),
    ("type_idle", "p50_ms", "typing p50", "ms", False),
    ("logflood", "mb_s", "log flood throughput", "MB/s", False),
    ("anim", "painted_fps", "animation fps", "fps", False),
    ("agent", "cpu_pct", "agent screen CPU", "%", False),
    ("logflood", "wire_bytes", "16 MiB log flood: bytes from the server", "B", False),
]


def load(path):
    """The meta line and Ship's successful runs."""
    meta, runs = {}, []
    for line in open(path):
        rec = json.loads(line)
        if rec.get("kind") == "meta":
            meta = rec
        elif rec.get("mux") == "ship" and rec.get("ok") and not rec.get("load_high"):
            runs.append(rec)
    return meta, runs


def median(runs, workload, metric):
    values = [r[metric] for r in runs if r.get("workload") == workload and r.get(metric) is not None]
    return statistics.median(values) if values else None


def fmt(value, unit):
    if value is None:
        return "-"
    for limit, suffix in ((1e6, "M"), (1e3, "k")):
        if unit.startswith("B") and value >= limit:
            return f"{value / limit:.1f} {suffix}{unit}"
    return f"{value:.2f} {unit}" if value < 100 else f"{value:.0f} {unit}"


def change(base, head, compared):
    if base is None or head is None:
        return ""
    if base == 0:
        return "" if head == 0 else "new"
    delta = (head - base) / base
    flag = " ⚠️" if compared and delta > THRESHOLD else ""
    return f"{delta:+.1%}{flag}"


# (key, label) in bench.yml's size.json, which counts code lines with tokei.
SIZES = [
    ("implementation", "Implementation (`crates/`, budget about 20,000)"),
    ("tests", "Tests (`crates/`)"),
    ("vendored", "Vendored (`vendor/`)"),
]


def sizes(head_path, base_path):
    """The Rust line counts, or nothing when the run didn't record them."""
    if not head_path:
        return []
    head = json.load(open(head_path))
    base = json.load(open(base_path)) if base_path else {}
    lines = ["", "### Rust lines of code", "", "| | main | this PR | change |", "| --- | ---: | ---: | ---: |"]
    for key, label in SIZES:
        b, h = base.get(key), head.get(key)
        delta = "" if b is None or h is None or b == h else f"{h - b:+,}"
        lines.append(f"| {label} | {'-' if b is None else f'{b:,}'} | {h:,} | {delta} |")
    return lines


def report(head_path, base_path, size_path=None, base_size_path=None):
    head_meta, head = load(head_path)
    base_meta, base = load(base_path) if base_path else ({}, [])
    rev = lambda meta: (meta.get("ship_rev") or "?")[:7]
    lines = [MARKER, "### Ship benchmark", ""]
    if base_path:
        lines.append(f"This pull request at `{rev(head_meta)}` against main at `{rev(base_meta)}`.")
    else:
        lines.append(f"`{rev(head_meta)}`. No run on main to compare against yet.")
    lines += ["", "| | main | this PR | change |", "| --- | ---: | ---: | ---: |"]
    for compared in (True, False):
        if not compared:
            lines.append("| **For reference only** | | | |")
        for workload, metric, label, unit, flag in METRICS:
            if flag != compared:
                continue
            b, h = median(base, workload, metric), median(head, workload, metric)
            lines.append(f"| {label} | {fmt(b, unit)} | {fmt(h, unit)} | {change(b, h, compared)} |")
    lines += ["", f"Medians of {len(head)} Ship runs on a shared `ubuntu-24.04` runner. "
              f"⚠️ marks a byte count that grew more than {THRESHOLD:.0%}. Timings on shared runners "
              "move with whatever else the machine is doing, and so does how many screens a flood sends, so those are never flagged. "
              "Bytes from the server go through `wiretap.py`, which adds a hop to every timing here. "
              "See `benchmarks/README.md`."]
    lines += sizes(size_path, base_size_path)
    return "\n".join(lines)


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("head")
    p.add_argument("--base")
    p.add_argument("--size", help="this run's size.json")
    p.add_argument("--base-size", help="the base run's size.json")
    args = p.parse_args()
    print(report(args.head, args.base, args.size, args.base_size))


if __name__ == "__main__":
    main()
