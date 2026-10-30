# Run actions: architecture

Status: Locked again on 2026-10-30 by Cyan in chat, without a Plannotator pass, after the product paper dropped the string form. `CommandLine` and decision 3 are withdrawn, decision 2 now says commands are argv only, and the slices go from three to two. Before that, locked on 2026-10-30 in Plannotator review ("LGTM"). Reopened on 2026-10-29 by the product paper's change to FR-013 and SC-003: commands are fire-and-forget and must outlive the client or server that started them. The first lock (2026-10-29, Plannotator, "LGTM", with the lane risk checked afterwards) is superseded in decision 5, the new decision 10, the runner's description and the risks. Shape A still wins. Built on the relocked `product.md`. Cyan picked shape A in chat on 2026-10-29 ("A sounds good to me"), accepting a process mechanism in `ship-core` behind a cargo feature. The deciding outcome is that both actions behave the same wherever they run, apart from which machine runs them.

Diagrams are code blocks rather than Mermaid, following the pane-stacks papers.

## Context and constraints

- **`server.` actions are `Command`.** `ship-core/src/command.rs` defines `enum Command { Tab, Pane }`. It's both the config's `server.*` namespace (`Action::Server(Command)` in `ship-client/src/keymap.rs`) and the `ship tab` / `ship pane` clap subcommands. The client's `execute` (`ship-client/src/execute.rs`) turns each into an HTTP request. A `server.` request may depend only on its arguments and the caller's context: attachment, selected tab or pane, current directory (`docs/agents/code.md`).
- **`client.` actions are `ClientAction`** in `keymap.rs`. The UI's lane runs them in turn order (`ship-client/src/ui/lane.rs`).
- **The status line already has a slot for messages.** `Local.message` in `ship-client/src/ui/mod.rs` is drawn by `status_line` in `draw.rs`. Action errors already land there.
- **Panes already have the environment and directory rules FR-008 and FR-009 ask for.** `PaneEnv::apply` (`ship-server/src/pane.rs`) sets `TERM`, `SHIP_*`, and removes other multiplexers' and agents' variables, but only on portable-pty's `CommandBuilder`. A create with no `cwd` takes the anchor pane's `dir`, else its start `cwd` (`ship-server/src/state.rs`, around line 656), else home (`pane_cwd`).
- **Both sides already write a log file.** `RunLog` (`crates/ship/src/runlog.rs`) is set up by `diagnostics::init` in the `ship` binary, one per server and one per client process. It writes 5 MB parts and keeps the log directory under 200 MB. `ship-server` reaches it only through the `app.log` closure; `ship-client` doesn't reach it at all.
- **A pipe ties a command to its reader.** If a run's output goes through a pipe the owner reads, a command that keeps writing after the owner exits gets `SIGPIPE` or `EPIPE`, and most programs exit on that. FR-013 rules this out.
- **Terminal signals reach a whole process group.** Ctrl-C on a `ship server` running in the foreground, and the hangup when a client's terminal closes, go to every process in the owner's group. A child started normally is in that group.
- **What reference programs do.** tmux `run-shell -b` keeps jobs in the server's group and sends SIGTERM to the direct child when the server exits. kitty `launch --type=background`, WezTerm `background_child_process` and Zellij's `RunInBackground` never stop their commands. None stops grandchildren.
- **`ship-core` has optional features** (`axum`, `clap`, `kameo`) and already depends on `tokio` and, on Unix, `nix`.
- **clap can leave a variant out.** clap_derive 4.6.7 supports `#[command(skip)]` on subcommand variants.
- **A pane's `command` is argv on the wire and in `get` output.** Each entity field has one type everywhere (`docs/agents/code.md`). `server.run` and `client.run` use the same argv type.

## Candidate shapes

### A. One shared runner in `ship-core` (winner)

`ship-core` gains a runner behind a `run` feature. The runner starts a command in its own process group with empty stdin and its output written straight into a file the owner hands it, then reports how the command exited and never stops it. The server holds one runner behind a new route, feeding it the pane environment and directory rules and its log file. The client's UI loop holds the other, and its exits come back as status-line messages. Cost: one new module in `ship-core`, one route, and passing a log file handle from the `ship` binary to both sides. Failure mode: the runner is shared, so a bug in it shows up on both sides, but it also gets fixed on both sides at once.

```text
config ─► argv
                         │
      client.run ────────┼──────────────► Runner (client) ──exit──► status line
                         │                    │ output ─► client log file
      server.run ── execute ── POST /run ► Runner (server) ──exit──► server log
                                  ▲               │ output ─► server log file
                         PaneEnv + anchor dir
```

### B. Each side writes its own runner

Same flow, but `ship-server` and `ship-client` each spawn, route output and report exits themselves. `ship-core` stays types-only. Cost: spawning, the process-group setting, output handling and exit reporting are written twice. Failure mode: the two drift, so `client.run` and `server.run` differ in how they log or detach for no product reason.

```text
client.run ─► client spawn/log/exit      server.run ─► POST /run ─► server spawn/log/exit
```

### C. `server.run` as an invisible pane

Reuse pane spawning: PTY, environment and teardown come for free. Cost: a PTY and a Ghostty terminal per run, output goes into a terminal instead of the log, and hidden panes must either sit in the model or need a second registry. Failure mode: a run becomes a pane in all but name, and every pane rule has to learn to skip it. It also contradicts the product paper, where a pane is the answer for a command that should end with Ship. Rejected.

```text
server.run ─► POST /pane (hidden) ─► PTY + Ghostty ─► invisible pane in the model
```

## Decision

Shape A. What matters most is that `client.run` and `server.run` are the same thing apart from where they run. A defines a run once. B only gets that by being careful twice, and C turns a run into something else.

## Structure

```text
ship
└─ RunLog               hands each side a clone of its current log file for run output

ship-core
└─ run (feature "run")
   └─ Runner            spawn detached, output to the given file, report exit

ship-client
├─ keymap: ClientAction::Run { command: Vec<String> }
├─ execute: Command::Run ─► POST /run { command, pane, attachment }
└─ ui: Runner + exit channel ─► Local.message

ship-server
├─ route POST /run      resolves cwd from the anchor pane, env from PaneEnv
├─ PaneEnv              applies to a pane's CommandBuilder or a run's Command
└─ Runner               owned by the server
```

| Box | Owns |
| --- | --- |
| `Runner` | Starting runs on one side. Each run gets its own process group, empty stdin, and stdout and stderr set to the owner's log file. A log line names the run and its process id when it starts. Exit or signal is reported to the owner while the owner lives. Nothing stops a run: when the owner exits, the run carries on. |
| `RunLog` | Already exists. Gains a way to hand out a cloned handle to its current part. |
| `ClientAction::Run` | `client.run`. It spawns on the client's runner and returns at once. A spawn error is the action's error. An exit is a later message. |
| `Command::Run` | `server.run`. Skipped by clap, so there's no CLI form. `execute` sends argv, the selected pane and the attachment. |
| `POST /run` | It spawns on the server's runner with the pane environment and the anchor pane's directory, and replies once the process has started. A spawn error is the reply. An exit is only logged. |
| `PaneEnv` | One description of a child's environment, applied to both pane and run processes. |

## Key decisions

1. **`server.run` is a `Command` variant skipped by clap.** It's a `server.` action, so it belongs in `Command` and goes through `execute` like the others. *Rejected:* a separate `Action::ServerRun` outside `Command`, which breaks the rule that `server.*` is `Command`; and keeping the CLI form, which the product paper rules out.
2. **Commands are argv only, in the config and on the wire.** A shell runs only when the command names one, as in Herdr, WezTerm, kitty and Zellij. The `/run` body and `NewPane.command` are argv, and a string form can still be added later without breaking configs. *Rejected:* a string run by `/bin/sh -c` (withdrawn 2026-10-30), which can't be taken back once configs use it and has no meaning on Windows; and sending strings over the wire, which leaves the shell to whichever side resolves it.
3. **Withdrawn on 2026-10-30 with the string form.** It made `CommandLine` a serde untagged enum.
4. **The runner lives in `ship-core` behind a `run` feature.** Both binaries need the same mechanism, and `ship-core` already gates mechanisms behind features. *Rejected:* a new crate, a layer that only forwards; and per-side copies (shape B).
5. **Each run gets its own process group, and nothing stops it.** The group keeps Ctrl-C on a foreground server and the hangup of a client's terminal from reaching the run (FR-013). On Unix this is one setting on the command; elsewhere there's nothing to set. *Rejected:* staying in the owner's group, which lets terminal signals kill runs; tokio's `kill_on_drop` or tmux-style SIGTERM at exit, which only reach the direct child and contradict FR-013; and `setsid`, which is more than FR-013 needs.
6. **The server replies to `/run` once the process has started.** That's what lets a spawn error reach the status line through the normal error path (FR-011) without the lane waiting for the run to finish (FR-005). *Rejected:* replying before spawning, which loses spawn errors; and waiting for exit, which blocks the lane.
7. **`client.run` exits reach the UI through a channel into the existing `select!` loop**, setting `Local.message`. *Rejected:* polling runs on the frame tick.
8. **`PaneEnv` applies to either process builder.** The environment rule is written once and applied to portable-pty's `CommandBuilder` and tokio's `Command`. *Rejected:* a second copy of the variable list for runs.
9. **A run's directory comes from the anchor-pane rule that `pane create` already uses**, factored out so both call it. *Rejected:* resolving the directory in the client, which can't know the server's paths.
10. **A run writes into its side's log file directly, through a cloned file handle.** The file stays open in the run after the owner exits, so a chatty command keeps going (FR-013), and its output still lands in the server or client log (FR-010). *Rejected:* pipes read by the owner, which kill chatty runs once the owner exits; discarding output, which breaks FR-010; and a separate file per run, which adds files the product doesn't ask for.

## Risks and unknowns

- **The lane and a request that changes nothing (checked, no risk).** The lane moves on when the replica's revision reaches the highest revision a response reported (`Lane::next`, `Observer::caught_up`, `Client::wrote`). `app::stamp` (`ship-server/src/app.rs`) stamps every response with the current revision after the handler runs, so a `/run` reply carries a revision the client's replica already has, or will have once another client's change arrives. `server.run` doesn't stall the lane.
- **`PaneEnv` refactor.** The pane code calls `apply` on portable-pty's builder. Making it apply to both may need a small trait or closure over "set" and "remove". This is a program-paper detail, but it touches pane spawning, which must not change behavior.
- **Raw output in the log.** A run's bytes go into the log file without timestamps or a label, between tracing lines. The start line with the run's name and process id is what ties them together. Accepted: the log is for debugging, and the alternatives break FR-013.
- **Log size and rotation.** `RunLog` doesn't count a run's bytes, so a part can grow past 5 MB, and a run started before a rotation keeps writing into the old part. A chatty run can push the directory past its cap until the next rotation prunes it. Accepted for now. A cap can come later if it bites.
- **Windows.** There are no process groups to set, so a run may still get Ctrl-C from a console it shares with its owner. Unverified, best effort.

## Slices

1. The runner, the log handle from `RunLog`, `client.run`, and the `alt-e` pane name `scrollback`.
2. `server.run`: the `Command` variant, `/run` route, `PaneEnv` and cwd sharing.
