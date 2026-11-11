#!/usr/bin/env python3
"""Fold several bench runs into one compact table per case.

usage: summarize.py run1.md run2.md run3.md
Cells show the median over runs; "added" also shows the min..max over runs.
"""
import re, statistics, sys

def parse(path):
    out, case = {}, None
    for line in open(path):
        if line.startswith("### "):
            case = line[4:].split(",")[0].strip() if "side note" not in line else line[4:].split(":")[0].strip().replace("side note", "(window_log 24)")
            case = case.replace("  ", " ")
        elif line.startswith("| ") and case and not line.startswith("| variant") and not line.startswith("| image"):
            cells = [c.strip() for c in line.strip().strip("|").split("|")]
            if len(cells) == 16:
                num = lambda s: float(re.match(r"[-+]?[\d.]+", s).group())
                out[(case, cells[0])] = dict(
                    server=num(cells[1]), client=num(cells[2]), total=num(cells[3]),
                    added=num(cells[4]), p99add=num(cells[6]), wmed=num(cells[8]),
                    wmean=num(cells[9]), first_us=num(cells[10]), wfirst=num(cells[11]),
                    over1k=num(cells[13]), over100k=num(cells[14]))
    return out

runs = [parse(p) for p in sys.argv[1:]]
cases = []
for k in runs[0]:
    if k[0] not in cases:
        cases.append(k[0])
for case in cases:
    print(f"\n#### {case}\n")
    print("| variant | server us | client us | added us vs A (median, range over runs) | added us p99 | wire B/frame median | wire B/frame mean | first frame: us / wire B | frames > 1 KB / > 100 KB (of 299) |")
    print("|---|---|---|---|---|---|---|---|---|")
    for (c, v) in runs[0]:
        if c != case:
            continue
        g = lambda f: [r[(c, v)][f] for r in runs]
        m = lambda f: statistics.median(g(f))
        a = g("added")
        print(f"| {v} | {m('server'):.0f} | {m('client'):.0f} | {m('added'):+.0f} ({min(a):+.0f}..{max(a):+.0f}) | {m('p99add'):+.0f} | {m('wmed'):,.0f} | {m('wmean'):,.0f} | {m('first_us'):,.0f} / {m('wfirst'):,.0f} | {m('over1k'):.0f} / {m('over100k'):.0f} |")
