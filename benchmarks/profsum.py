#!/usr/bin/env python3
"""Summarise a samply profile (from profile.py) without opening a browser.

    python3 profsum.py /tmp/ship-prof/scale/server.json.gz
    python3 profsum.py client.json.gz --threads tokio-rt-worker --top 30
    python3 profsum.py server.json.gz --callers serde_json --top 10

A sample taken while a thread is parked (cvwait, kevent, ...) is dropped.
Each remaining sample counts once: a sampler that stops a running thread
finds it in proportion to the CPU it uses, so shares are shares of CPU time.
(--cpu-weighted weights by samply's threadCPUDelta instead, which charges the
CPU a thread burned before parking to the park call and so skews the shares.)

Reads `<name>.json.gz` plus the `<name>.json.syms.json` that
`--unstable-presymbolicate` wrote beside it. Only threads of the process the
profile was recorded for (the one running the binary named by --binary, `ship`
by default) are counted, so panes' shells and Python programs are left out.
Prints, per thread name, the share of CPU time, then the top frames by self
time and by inclusive time. Waiting is invisible here (use the timeline trace for that).

Check that symbols resolved before quoting a profile: the "unresolved" line
counts samples whose leaf frame has no name, and --check lists the
Ghostty (Zig) frames found.
"""

import argparse
import bisect
import collections
import gzip
import json
import re
from pathlib import Path


def load(path):
    prof = json.load(gzip.open(path))
    syms_path = Path(str(path).replace(".json.gz", ".json.syms.json"))
    syms = json.loads(syms_path.read_text())
    table = syms["string_table"]
    by_lib = {}
    for d in syms["data"]:
        rows = sorted((r["rva"], r["size"], table[r["symbol"]]) for r in d["symbol_table"])
        by_lib[(d["debug_name"], d["debug_id"].replace("-", "").upper())] = (
            [r[0] for r in rows], rows)
    return prof, by_lib


def resolver(prof, by_lib):
    libs = prof["libs"]

    def lib_key(i):
        lib = libs[i]
        return (lib["debugName"], lib["breakpadId"][:32].upper())

    def resolve_thread(t):
        strings = t["stringArray"]
        res_lib = t["resourceTable"]["lib"]
        func_res = t["funcTable"]["resource"]
        func_name = t["funcTable"]["name"]
        names = []
        for f in range(t["frameTable"]["length"]):
            func = t["frameTable"]["func"][f]
            rva = t["frameTable"]["address"][f]
            r = func_res[func]
            lib = res_lib[r] if r is not None and r >= 0 else None
            name = None
            libname = "?"
            if lib is not None:
                libname = libs[lib]["name"]
                starts, rows = by_lib.get(lib_key(lib), ([], []))
                i = bisect.bisect_right(starts, rva) - 1
                if i >= 0 and rows[i][0] <= rva < rows[i][0] + max(rows[i][1], 1):
                    name = rows[i][2]
                elif i >= 0 and rva - rows[i][0] < 4096 and rows[i][1] == 0:
                    name = rows[i][2]
            if name is None:
                raw = strings[func_name[func]]
                name = raw if raw and not re.fullmatch(r"0x[0-9a-f]+", raw) else None
            names.append((name or f"unresolved {libname}+{rva:#x}", libname, name is not None))
        return names

    return resolve_thread


# Leaves where a thread is parked. The CPU it used just before parking is
# charged to these by the sampler, so they are dropped from the shares and
# reported separately.
WAITS = {"__psynch_cvwait", "kevent", "kevent64", "semaphore_wait_trap", "__wait4", "__psynch_mutexwait",
         "mach_msg2_trap", "mach_msg_trap", "__ulock_wait", "__ulock_wait2", "select", "poll", "__select",
         "__semwait_signal", "_pthread_cond_wait", "read", "__read_nocancel"}


def short(name, width=110):
    name = re.sub(r"<[^<>]*>", "<>", name)
    name = re.sub(r"<[^<>]*>", "<>", name)
    return name if len(name) <= width else name[: width - 1] + "..."


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("profile", type=Path)
    p.add_argument("--binary", default="ship", help="process name to keep (default: ship)")
    p.add_argument("--threads", help="regex on thread names; default all of the process, merged by name")
    p.add_argument("--top", type=int, default=25)
    p.add_argument("--callers", metavar="REGEX", help="for frames matching REGEX, list who calls them")
    p.add_argument("--cpu-weighted", action="store_true",
                   help="weight samples by threadCPUDelta (skews toward the call a thread parks in)")
    p.add_argument("--with-waits", action="store_true", help="keep samples parked in a wait call")
    p.add_argument("--check", action="store_true", help="print symbol-resolution checks")
    a = p.parse_args()
    prof, by_lib = load(a.profile)
    resolve = resolver(prof, by_lib)
    want = re.compile(a.threads) if a.threads else None
    per_thread = collections.Counter()
    self_t, incl_t = collections.Counter(), collections.Counter()
    lib_self = collections.Counter()
    caller_c = collections.Counter()
    total = unresolved = parked = 0
    zig = collections.Counter()
    for t in prof["threads"]:
        if t.get("processName") != a.binary or t["samples"]["length"] == 0:
            continue
        if want and not want.search(t["name"]):
            continue
        names = resolve(t)
        st = t["stackTable"]
        frame_of, prefix = st["frame"], st["prefix"]
        # threadCPUDelta is the CPU microseconds the thread used since the
        # previous sample; `weight` only counts repeated idle samples.
        weights = t["samples"].get("threadCPUDelta")
        for k, stack in enumerate(t["samples"]["stack"]):
            if stack is None:
                continue
            if a.cpu_weighted:
                w = weights[k] if weights and weights[k] is not None else 0
                if not w:
                    continue
            else:
                w = 1
            leaf = names[frame_of[stack]]
            if leaf[0] in WAITS and not a.with_waits:
                parked += w
                continue
            total += w
            per_thread[t["name"]] += w
            self_t[leaf[0]] += w
            lib_self[leaf[1]] += w
            if not leaf[2]:
                unresolved += w
            seen, s, below = set(), stack, None
            while s is not None:
                nm = names[frame_of[s]][0]
                if nm not in seen:
                    seen.add(nm)
                    incl_t[nm] += w
                if a.callers and re.search(a.callers, nm) and below is not None:
                    caller_c[(nm, below)] += w
                below = nm
                s = prefix[s]
            if a.check:
                for nm in seen:
                    if re.search(r"ghostty|terminal::|\bzig\b|Terminal|PageList|stream\.", nm):
                        zig[nm] += 1
    total = max(total, 1)
    unit = "us" if a.cpu_weighted else "samples"
    print(f"{a.profile.name}: {total if not a.cpu_weighted else total / 1e3:.0f} {'ms of CPU' if a.cpu_weighted else 'running samples'} in {a.binary} threads")
    if a.cpu_weighted:
        print(f"(left out: {parked / 1e3:.0f} ms charged to parked threads)")
    print("threads (share of that CPU):")
    for n, c in per_thread.most_common(8):
        print(f"  {100 * c / total:5.1f}%  {n}")
    print(f"unresolved leaf frames: {100 * unresolved / max(total, 1):.1f}%")
    print("by library (self):", ", ".join(f"{n} {100 * c / total:.0f}%" for n, c in lib_self.most_common(6)))
    print(f"\ntop {a.top} by self time")
    for n, c in self_t.most_common(a.top):
        print(f"  {100 * c / total:5.1f}%  {short(n)}")
    print(f"\ntop {a.top} by inclusive time")
    for n, c in incl_t.most_common(a.top):
        print(f"  {100 * c / total:5.1f}%  {short(n)}")
    if a.callers:
        print(f"\nframes matching {a.callers!r} and their immediate callees on the path to the leaf")
        for (n, b), c in caller_c.most_common(a.top):
            print(f"  {100 * c / total:5.1f}%  {short(n, 70)}  ->  {short(b, 60)}")
    if a.check:
        print("\nGhostty/Zig-looking frames found:", len(zig))
        for n, c in zig.most_common(8):
            print(f"  {c:6d}  {short(n)}")


if __name__ == "__main__":
    main()
