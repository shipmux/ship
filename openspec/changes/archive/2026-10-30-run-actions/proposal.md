# Proposal

This is the planning artifact for a locked plan. The three papers in `design/` are the source of truth; this proposal and the deltas under `specs/` are views of them. Cyan locked product and architecture on 2026-10-29 and 2026-10-30, and program on 2026-10-30. Approval does not start implementation.

## Why

Key bindings and menu entries can only send Ship's own actions, so a script can't be put on a key ([#22](https://github.com/shipmux/ship/issues/22)). Running any command from a key lets a script do what a plugin would: call `ship` back, open a pane, or act outside Ship. Separately, the pane `alt-e` opens is labeled `sh` instead of something useful.

## What Changes

- **`server.run = { command = [...] }`** starts a command on the server's machine, in the directory a new pane from the selected pane would start in, with the pane environment and `SHIP_PANE_ID` naming the selected pane. It has no CLI form. A failure to start shows in the status line; output and the exit go to the server log only.
- **`client.run = { command = [...] }`** starts a command on the client's machine, in the client's starting directory. Output goes to the client log, and a failure to start or a non-zero exit shows in the status line. It's the one to use for commands that need the user's current desktop or login session, such as opening a URL.
- **Argv only.** `command` is a non-empty array run without a shell. A shell runs only when the command names one, such as `["sh", "-c", "..."]`.
- **Fire-and-forget.** Neither action waits, and Ship never stops a command it started: it keeps running after its client quits or its server stops. A command that should end with Ship belongs in a pane.
- **New route.** `POST /api/v0/runs`, replying 202 once the command has started. The protocol version goes to 21.
- **Scrollback label.** The default `alt-e` binding and the pane menu's "edit scrollback" name the new pane `scrollback`.
- **Docs.** `docs/site/configuration.md` describes both actions.

Not in this change (the product paper lists them all): a string form for `command`, a CLI form of `server.run`, waiting for a command or reporting a `server.run` exit to clients, showing output on screen, client-only context such as the text selection, and limiting concurrent runs.

## Capabilities

### New Capabilities

- `run-actions`: the `server.run` and `client.run` actions: the command form, where and how a command starts, its environment and directory, output, failure reporting, and commands outliving the client or server that started them.

### Modified Capabilities

- `keymap`: `server.run` is a `server.` action without a CLI form, `client.run` is a `client.` action that runs a command rather than changing the view, and the default `alt-e` names its pane `scrollback`.
- `health-exchange`: protocol 21.

## Impact

- `ship-core`: `Run` and `Command::Run` (skipped by clap) in `command.rs`, `RunCommand` and `PROTOCOL_VERSION` 21 in `protocol.rs`, `name = "scrollback"` in `defaults.toml`, and a new `run` module behind a `run` cargo feature.
- `ship`: `RunLog` and `Diagnostics` hand out a clone of the current log part for run output.
- `ship-client`: `ClientAction::Run`, a runner and exit channel in the UI loop, and `Client::run` for the new route.
- `ship-server`: the `/api/v0/runs` route, a runner in the state actor, `PaneEnv` applying to tokio's `Command` as well as portable-pty's builder, and the anchor-pane directory rule shared with pane creation.
- Platforms: each run gets its own process group on Unix. Windows is best effort and unverified.
