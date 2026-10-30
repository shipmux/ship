# Tasks

These are checkpoints for implementing the locked plan, one slice at a time. The papers in `design/` are the contract, and `design/program.md` holds the file map, signatures and checks for each slice. Complete and verify one slice, then stop for Cyan's review before starting the next. Record surprises in the program paper's deviation log.

Verification uses real workflows and temporary probes, not permanent authored tests. For each slice, run these checks:
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo deny check`;
- debug and release builds;
- `openspec validate run-actions --type change --strict`.

Use an owned temporary server on a scratch name, with `XDG_RUNTIME_DIR`, `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories. Don't touch existing user work, including the `default` server. Kill any `tick` loops and `sleep` processes the checks leave behind. Checks run on macOS. Linux counts only when run there and is otherwise reported unverified.

## 1. Runner, `client.run` and the scrollback name

- [x] 1.1 Add `Run` with the non-empty argv check in `ship-core/src/command.rs`, and verify with `ship config check` that `client.run = { command = [] }`, `command = "ls"` and an unknown field each fail with the field's line and column while an argv binding passes.
- [x] 1.2 Add the `run` feature and `ship-core/src/run.rs` (`Runner`, `OnExit`, `Ended`, `detach`), and verify the workspace builds with and without the feature.
- [x] 1.3 Add `RunLog::output` and `Diagnostics::run_output`, pass it from `main.rs` to `ui::run`, and verify a run's output lands in the client's current log part.
- [x] 1.4 Add `ClientAction::Run`, the client runner and exit channel in `ui/mod.rs`, and `ui/run.rs` (`start`, `describe`), and verify the `client.run` scenarios in P1, P2 and P3 in a running client: the directory, `SHIP_*` variables, start failure and `exit 3` messages, and output kept off the screen.
- [x] 1.5 Verify SC-003 for the client: bind the `tick` loop under `client.run`, quit the client, and confirm with `pgrep -f "echo tick"` after 5 seconds that it's running and the client log is still growing; repeat by closing the client's terminal window.
- [x] 1.6 Add `name = "scrollback"` to both `alt-e` defaults in `defaults.toml`, and verify `ship config defaults` shows it and pressing `alt-e` opens a pane labeled `scrollback` (P4).
- [x] 1.7 Document `client.run` in `docs/site/configuration.md`, with an argv example and a `["sh", "-c", ...]` example, where output goes, how failures show, and that Ship never stops a run; verify the documented binding works as written.

### Slice 1 verification

Verified on macOS on 2026-10-30. Linux and Windows are unverified.

- Passed workspace debug and release builds, core builds with no default features and with `run`, formatting, Clippy with `-D warnings`, `cargo deny check`, and strict OpenSpec validation.
- A temporary probe drove a real client in an isolated tmux terminal against the scratch `run-slice1` server, with all three XDG directories under `/tmp/ship-slice1.lOwbqe/`. Evidence is in that directory's `evidence.json`, config error results, status-line captures and client logs. No permanent tests were added.
- Checked config rejection with line/column; argv and explicit shells; both documented bindings; actual menu execution; three concurrent runs and responsive typing; selected and absent pane environment (including stale inherited IDs); Ship callback; starting directory; empty stdin; output in the current client log and absent from the screen; named start, non-zero and signal errors; and the real `alt-e` editor pane's `scrollback` name.
- After both `alt-q` and closing the terminal PTY, `pgrep -f "echo tick"` still found the run after five seconds and its client log grew. The probe stopped its own server and process groups afterward; a process inspection found no remaining scratch server, tick loops or sleep runs.

## 2. `server.run`

- [x] 2.1 Add `Command::Run` skipped by clap, `RunCommand` and `PROTOCOL_VERSION` 21, and verify `ship --help` lists no `run` command and `ship server status` reports `protocol_version` 21.
- [x] 2.2 Add `ChildEnv` and change `PaneEnv::apply` to take it, and verify `env` in a new pane is unchanged against a pane from before the change.
- [x] 2.3 Factor `anchor_dir` out of pane creation, add `Message<RunCommand>` with the server runner and `run_output` from `serve`, and the `/api/v0/runs` route, and verify `server.run` starts in the selected pane's directory with the pane environment and `SHIP_PANE_ID` (P2).
- [x] 2.4 Add `Outcome::Started`, `Client::run` and the `execute` branch, and verify the `server.run` scenarios in P1 and P3: start failure in the status line, output and `exit 3` only in the server log.
- [x] 2.5 Verify SC-001 by typing into a pane straight after a `["sleep", "30"]` run, with no delay compared to typing without it.
- [x] 2.6 Verify SC-003 for the server: run the `tick` loop under `server.run`, run `ship server stop`, and confirm with `pgrep` after 5 seconds that it's still running; repeat with a foreground `ship server` stopped by Ctrl-C.
- [x] 2.7 Document `server.run` in `docs/site/configuration.md`, including its directory, environment and that its exits are only logged, and verify the documented binding works as written.

### Slice 2 verification

Verified on macOS on 2026-10-30. Linux, Windows and physical-terminal interaction are unverified.

- Passed debug and release workspace builds, a workspace build without default features, core feature checks, a standalone server check without default features, formatting, Clippy with `-D warnings`, `cargo deny check`, strict OpenSpec validation and `git diff --check`. The complete 12-checkpoint slice 1 client probe passed again.
- Passed 18 live checkpoints in both debug and release builds, using isolated tmux terminals and the scratch `run-slice2` and `run-slice2-fg` servers. All XDG paths are under `/tmp/ship-slice2.kKVfPw/`. The successful evidence files are `debug-2y26eqd2/evidence.json` and `release-phdzfltr/evidence.json` there, alongside HTTP errors, status-line captures, configs and logs. No permanent tests were added.
- Checked config rejection with line/column, absent CLI command, protocol 21, HTTP 202 with no body, HTTP 400 for empty argv, null context and spawn errors, and 404 for a missing pane. Tested unset and tab-only selections, inherited-ID removal, tracked pane directory and `PWD`, empty stdin, full pane/run environment parity, the documented Ship callback, argv without expansion, explicit shells, server-only output and exits, named start errors, no tab/pane state changes, and real right-click menu execution.
- Compared the complete environment of a new pane before and after the refactor with `pane_env.py`; it is unchanged after normalizing its fresh pane ID and the copied pre-change binary's path. Checked that key-created panes use the same anchor directory as runs.
- Three independent `sleep 30` runs stayed alive while typing latency remained comparable to the baseline. A separate five-second command did not delay typing. After normal server stop and real Ctrl-C on a foreground server, `pgrep -f "echo tick"` still found each loop after five seconds and its server log kept growing. The probe stopped its owned run groups, servers and terminals afterward.

## Workflow follow-up

- Archive the change after Cyan's review of the last slice.
- Verify the archived specs: `run-actions` created, `keymap` and `health-exchange` updated.
