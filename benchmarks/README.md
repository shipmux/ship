# Multiplexer benchmarks

Compares Ship, Herdr, Zellij and tmux on one machine. The harness plays the outer terminal: it starts each multiplexer's client on a PTY it owns, types into it, and counts and searches every byte the client writes back. All four tools are measured the same way.

## Running it

```sh
cargo build --release              # from the repo root; the harness uses target/release/ship
cd benchmarks
python3 bench.py                   # every tool, every workload, 3 reps (about 40 min)
python3 bench.py --mux ship --workload type_idle --reps 1
python3 bench.py --summarize results/results-<stamp>.jsonl
```

## Profiling Ship

`--trace` sets `SHIP_TRACE` for Ship's server and client, so each run also records their timelines under `<scratch>/<run>/trace/`. Tracing slows Ship and the files are large (about 250 MB for a 10 s `type_flood`), so keep traced runs out of comparisons. For a typing run, `timeline.py` merges the client, server, harness and probe onto one clock:

```sh
python3 bench.py --mux ship --workload type_idle --reps 1 --keys 60 --trace
python3 timeline.py /tmp/ship-bench-runs/ship-type_idle-0   # writes merged.json for https://ui.perfetto.dev
```

Needs Python 3 with no extra packages, and `herdr`, `zellij` and `tmux` at `/opt/homebrew/bin`. Set `SHIP_BENCH_BIN` to measure a different Ship binary. `python3 bench.py --help` lists the duration and key-count knobs.

Results go to `results/results-<stamp>.jsonl`: a meta line (versions, Ship rev, load average), then one line per run with its metrics, an `ok` flag, the check that decided it, and the end of the screen text. Each run's raw outer-terminal bytes are kept as `/tmp/ship-bench-runs/<tool>-<workload>-<rep>/outer.bin` (`--no-keep-raw` turns that off).

## Isolation

Every run gets a fresh tree under `/tmp/ship-bench-runs/<run>/` and points `HOME`, `XDG_*_HOME`, `XDG_RUNTIME_DIR` and `TMPDIR` into it. On top of that:

- Ship: a foreground `ship server --port 43291 --starter` (never the default 43179), an empty `SHIP_CONFIG`, and the client attached with `--server-url`.
- Herdr: `HERDR_HOME`, `HERDR_SOCKET_PATH` and `HERDR_CLIENT_SOCKET_PATH` inside the run tree.
- Zellij: `--config-dir`, `--data-dir`, `ZELLIJ_SOCKET_DIR` and session name `bench`.
- tmux: `TMUX_TMPDIR` inside the run tree, socket name `bench` (`-L`), `-f /dev/null` and session name `bench`.

The inherited environment is dropped except `PATH`, `USER` and `LOGNAME`, so `HERDR_*`, `ZELLIJ_*`, `TMUX*` and `SHIP_*` variables from the calling shell never leak in. `SHELL=/bin/sh`. No tool gets a config file, so each runs its defaults, including Zellij's tab bar and status bar plugins and Ship's sidebar.

First-run screens are dismissed the way a user would: `alt-right` to open Ship's starter tab, `esc` on Herdr's settings panel and Zellij's release notes. A run only starts once `echo RDY-$((6*7))` typed into the pane prints `RDY-42`. Teardown uses each tool's own stop command, then kills anything still running with the run's `HOME`.

The outer terminal is 200x50. The harness answers DA1, cursor-position and OSC 10/11 colour queries like a minimal xterm and leaves the rest unanswered.

## Workloads

Pane programs live in `pane.py`. Each one is started by typing `exec python3 -I pane.py ...` into the pane's shell.

| workload | what runs | duration |
|---|---|---|
| `idle` | a shell prompt doing nothing | 30 s |
| `agent` | an agent-like screen: static transcript, a 10 Hz spinner and token counter, a new scrolled line every 1.5 s | 60 s |
| `logflood` | 16 MiB of ASCII log lines written as fast as the pane takes them | until rendered |
| `uniflood` | 8 MiB of CJK, emoji, combining marks and box drawing | until rendered |
| `anim` | full-screen truecolor frames at 60 fps, every cell a new background each frame | 10 s |
| `type_idle` | 200 keypresses (after 10 warmup) into an echo probe, 20-60 ms apart | about 10 s |
| `type_flood` | the same probe beside a pane that floods log lines nonstop | about 10 s |
| `type_agent` | the same probe beside the `agent` screen | about 10 s |

## Metrics

- `cpu_pct`, `cpu_s`: user plus system CPU of every process named `ship`, `herdr`, `zellij` or `tmux` that carries the run's `HOME` (server and client), from `ps` before and after the window. Pane programs and shells are excluded. Floods report CPU seconds; steady workloads report percent of one core.
- `peak_rss_mb`: highest sum of those processes' RSS, sampled once a second over the whole run.
- `bytes`: bytes the client wrote to the outer terminal during the window.
- `wall_ms`, `mb_s`: from the moment the flood program starts writing until the harness sees the end marker in the client's output, so it includes rendering. `mb_s` is input bytes over that time. `writer_ms` (raw file only) is how long the program's own write took.
- `painted_fps`: frame labels the client actually painted, divided by the animation's length. Frames a tool coalesces don't count.
- `p50_ms`, `p95_ms`, `p99_ms`: time from writing a key to the client's PTY until that key's echo appears in the client's output. `lost`: keys with no echo within 2 s.
- `in_p50_ms`, `program_p50_ms`, `out_p50_ms`: the median key split at the probe, which stamps when it reads each key and when it writes the echo. `in` is harness to probe, through the multiplexer's input path. `program` is the probe itself and works as a control: it changes only with the machine. `out` is probe to harness, through the output path. Reported only when no key was lost, since stamps pair with keys by order.

## How a result is checked

End markers are capitals spelled from a random nonce. The command line never contains them and pane content is lowercase, so a diffing renderer has to repaint every marker cell and a match means the screen really showed it. The match allows escape sequences between the marker's characters, because Herdr repositions the cursor mid-word after wide characters. Each probe key is a letter that the probe answers by repainting one spot with eight consecutive capitals (`a` gives `ABCDEFGH`, `b` gives `BCDEFGHI`). Neighbouring answers differ in every cell, and the harness searches only bytes that arrived after the key was sent. A typing run fails if under 95% of keys echo, or if the side pane stopped painting during the window. A run that fails its check is kept in the file and left out of the medians.

## Limits

- macOS only so far, run on a machine that may be busy. `load1` is recorded per run.
- One outer-terminal reader in Python. It drains 256 KiB at a time, which kept up in testing, but a real terminal parses and draws where this one only counts.
- Latency is to the bytes leaving the client, not to pixels.
- `ps` CPU time has 10 ms resolution, so idle CPU near zero is noise.
