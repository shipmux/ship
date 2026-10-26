# Multiplexer benchmarks

Compares Ship, Herdr, Zellij and tmux on one machine. The harness plays the outer terminal: it starts each multiplexer's client on a PTY it owns, types into it, and counts and searches every byte the client writes back. All four tools are measured the same way.

## Running it

```sh
cargo build --release           # from the repo root; the harness uses target/release/ship
cd benchmarks
uv run bench.py                 # every tool, every workload, 5 reps (about 35 min)
uv run bench.py --reps 3        # a quicker check (about 20 min)
uv run bench.py --mux ship --workload type_idle --reps 1
uv run bench.py --summarize results/results-<stamp>.jsonl
```

Quit your own Ship, Herdr and Zellij sessions before a run you mean to quote; they add background load.

## Profiling Ship

`--trace` sets `SHIP_TRACE` for Ship's server and client, so each run also records their timelines under `<scratch>/<run>/trace/`. Tracing slows Ship and the files are large (about 250 MB for a 10 s `type_flood`), so keep traced runs out of comparisons. For a typing run, `timeline.py` merges the client, server, harness and probe onto one clock:

```sh
uv run bench.py --mux ship --workload type_idle --reps 1 --keys 60 --trace
python3 timeline.py /tmp/ship-bench-runs/ship-type_idle-0   # writes merged.json for https://ui.perfetto.dev
```

Runs on macOS and Linux. Needs [uv](https://docs.astral.sh/uv/), which reads the Python version and the one dependency (psutil) from the top of `bench.py`, and `herdr`, `zellij` and `tmux` on `PATH`. Set `SHIP_BENCH_BIN`, `HERDR_BENCH_BIN`, `ZELLIJ_BENCH_BIN` or `TMUX_BENCH_BIN` to measure a different binary, or leave a tool out with `--mux`. `uv run bench.py --help` lists the duration and key-count knobs.

On Linux, `taskset` limits every tool, pane and the harness itself to a set of cores, which stands in for a smaller machine; the CPUs a run was allowed are recorded:

```sh
taskset -c 0-3 uv run bench.py
```

Results go to `results/results-<stamp>.jsonl`: a meta line, then one line per run with its metrics, an `ok` flag, the check that decided it, the load average before and after, and the end of the screen text. The meta line records the tool versions and the sha256 of each binary, the Ship commit and a hash of `git status`, the run-order seed, and the machine: Python version, platform, CPU model and count, memory, and whether it ran on battery. On macOS it adds `hw.model`, `sw_vers`, and the output of `pmset -g custom`, `pmset -g therm`, `pmset -g assertions`, `mdutil -s /` and `tmutil status`. On Linux it adds `/etc/os-release`, the CPUs the run was allowed, the frequency governor, turbo or boost state, and `systemd-detect-virt`. Each run's raw outer-terminal bytes are kept as `/tmp/ship-bench-runs/<tool>-<workload>-<rep>/outer.bin` (`--no-keep-raw` turns that off).

## Isolation

Every run gets a fresh tree under `/tmp/ship-bench-runs/<run>/` and points `HOME`, `XDG_*_HOME`, `XDG_RUNTIME_DIR` and `TMPDIR` into it. On top of that:

- Ship: a foreground `ship server --port 43291 --starter` (never the default 43179), an empty `SHIP_CONFIG`, and the client attached with `--server-url`.
- Herdr: `HERDR_HOME`, `HERDR_SOCKET_PATH` and `HERDR_CLIENT_SOCKET_PATH` inside the run tree.
- Zellij: `--config-dir`, `--data-dir`, `ZELLIJ_SOCKET_DIR` and session name `bench`.
- tmux: `TMUX_TMPDIR` inside the run tree, socket name `bench` (`-L`), `-f /dev/null` and session name `bench`.

The inherited environment is dropped except `PATH`, `USER` and `LOGNAME`, so `HERDR_*`, `ZELLIJ_*`, `TMUX*` and `SHIP_*` variables from the calling shell never leak in. `SHELL=/bin/sh`. No tool gets a config file, so each runs its defaults, including Zellij's tab bar and status bar plugins and Ship's sidebar.

First-run screens are dismissed the way a user would: `alt-right` to open Ship's starter tab, `esc` on Herdr's settings panel and Zellij's release notes. A run only starts once `echo RDY-$((6*7))` typed into the pane prints `RDY-42`. Teardown uses each tool's own stop command, then kills anything still running with the run's `HOME`.

The outer terminal is 200x50. The harness answers DA1, cursor-position and OSC 10/11 colour queries like a minimal xterm and leaves the rest unanswered.

## Run order and load

Each rep runs every workload once per tool, in a fresh random tool order for every workload, with `--cooldown` seconds (1) between runs. `--seed` repeats an order; the seed used is in the meta line.

The harness won't start while the 1-minute load average is above `--max-load` (4). Before each run it waits up to `--load-wait` seconds (120) for the load to fall; a run that starts above the limit anyway is kept in the file, marked `load_high`, and left out of the summary.

## Workloads

Pane programs live in `pane.py`. Each one is started by typing `exec python3 -I pane.py ...` into the pane's shell.

| workload | what runs | duration |
|---|---|---|
| `idle` | a shell prompt doing nothing | 10 s |
| `agent` | an agent-like screen: static transcript, a 10 Hz spinner and token counter, a new scrolled line every 1.5 s | 15 s |
| `logflood` | 16 MiB of ASCII log lines written as fast as the pane takes them | until rendered |
| `uniflood` | 8 MiB of CJK, emoji, combining marks and box drawing | until rendered |
| `anim` | full-screen truecolor frames at 60 fps, every cell a new background each frame, in a smooth gradient | 5 s |
| `noise` | the same, but every cell a random colour, which leaves a compressor little to find within a frame | 5 s |
| `type_idle` | 100 keypresses (after 10 warmup) into an echo probe, 20-60 ms apart | about 5 s |
| `type_flood` | the same probe beside a pane that floods log lines nonstop | about 5 s |
| `type_agent` | the same probe beside the `agent` screen | about 5 s |

## Metrics

- `cpu_pct`, `cpu_s`: user plus system CPU of every process named `ship`, `herdr`, `zellij` or `tmux` that carries the run's `HOME` (server and client), from `ps` before and after the window. Pane programs and shells are excluded. Floods report CPU seconds; steady workloads report percent of one core.
- `peak_rss_mb`: highest sum of those processes' RSS, sampled once a second over the whole run.
- `bytes`, `bytes_s`: bytes the client wrote to the outer terminal during the window, and per second.
- `wire_bytes`, `wire_bytes_s`: bytes Ship's client received from its server during the window, compressed, from `nettop`. Ship only: the other tools talk over Unix sockets, which `nettop` doesn't count. macOS only: Linux has no per-process counter for it.
- `bytes_per_key`, `wire_bytes_per_key`: the same, divided by the keys sent in a typing run.
- `wall_ms`, `mb_s`: from the moment the flood program starts writing until the harness sees the end marker in the client's output, so it includes rendering. `mb_s` is input bytes over that time. `writer_ms` (raw file only) is how long the program's own write took.
- `painted_fps`: frame labels the client actually painted, divided by the animation's length. Frames a tool coalesces don't count.
- `p50_ms`, `p95_ms`, `p99_ms`: time from writing a key to the client's PTY until that key's echo appears in the client's output. `lost`: keys with no echo within 2 s.
- `in_p50_ms`, `program_p50_ms`, `out_p50_ms`: the median key split at the probe, which stamps when it reads each key and when it writes the echo. `in` is harness to probe, through the multiplexer's input path. `program` is the probe itself and works as a control: it changes only with the machine. `out` is probe to harness, through the output path. Reported only when no key was lost, since stamps pair with keys by order.

## Summary

`--summarize` prints three things. The medians table takes the median of the per-run values, except p95 and p99, which pool every key from every run (a p99 of one run's 100 keys is about its 2nd-worst key). The differences table gives each tool's median minus Ship's, with a 95% interval from a bootstrap that resamples whole runs, so the spread between runs counts. The per-run p50 list shows how much a tool's latency moved from run to run. With 3 reps the intervals are rough; 5 is enough for differences of a millisecond or more, and `--reps` buys more precision when a claim needs it.

## How a result is checked

End markers are capitals spelled from a random nonce. The command line never contains them and pane content is lowercase, so a diffing renderer has to repaint every marker cell and a match means the screen really showed it. The match allows escape sequences between the marker's characters, because Herdr repositions the cursor mid-word after wide characters. Each probe key is a letter that the probe answers by repainting one spot with eight consecutive capitals (`a` gives `ABCDEFGH`, `b` gives `BCDEFGHI`). Neighbouring answers differ in every cell, and the harness searches only bytes that arrived after the key was sent. A typing run fails if under 95% of keys echo, or if the side pane stopped painting during the window. A run that fails its check is kept in the file and left out of the medians.

## Limits

- Run on macOS so far. Linux is supported but unverified. The machine may be busy; `load1` is recorded per run.
- One outer-terminal reader in Python. It drains 256 KiB at a time, which kept up in testing, but a real terminal parses and draws where this one only counts.
- Latency is to the bytes leaving the client, not to pixels.
- `ps` CPU time has 10 ms resolution, so idle CPU near zero is noise.
