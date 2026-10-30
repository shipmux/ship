# Design

Summary of the three locked papers in `design/`, which are the source of truth. Where this file and a paper disagree, the paper wins. Cyan locked product and architecture in Plannotator on 2026-10-29 and 2026-10-30, then approved the switch to argv-only commands and the program paper in chat on 2026-10-30 without rereading. Locking does not start implementation: each slice runs on Cyan's request and stops for review before the next.

- [`design/product.md`](design/product.md): what and why. User stories P1 to P4, FR-001 to FR-014 (FR-003 withdrawn), success criteria SC-001 to SC-003, no open unknowns.
- [`design/architecture.md`](design/architecture.md): shape A, one shared runner in `ship-core`, and key decisions 1 to 10 (3 withdrawn).
- [`design/program.md`](design/program.md): the touched files and signatures, slices 1 and 2, and the deviation log.

## Context

Key bindings and menu entries can only send Ship's own actions ([#22](https://github.com/shipmux/ship/issues/22)). The edit-scrollback pane also shows `sh` as its label. See the product paper's Goal.

## Goals / Non-Goals

**Goals:** see the product paper's user stories P1 (run a command from a key), P2 (the command can call Ship back), P3 (failures are visible) and P4 (the scrollback pane's label).

**Non-goals:** see the product paper's Non-goals, notably no string form, no CLI form of `server.run`, and Ship never stopping a command.

## Decisions

1. **`server.run` and `client.run` take `{ command = [argv] }`.** Argv only; a shell runs when the command names one, as in Herdr, WezTerm, kitty and Zellij (decision 2).
2. **`server.run` is a `Command` variant skipped by clap**, sent as `POST /api/v0/runs` with the selected pane and attachment (decisions 1 and 6). The new route takes the protocol to 21 (program paper).
3. **One runner in `ship-core` behind a `run` feature** serves both sides (decision 4).
4. **Runs are fire-and-forget.** Each gets its own process group so terminal signals aimed at its owner miss it, and nothing stops it (decision 5).
5. **Output goes into the side's log file through a cloned handle**, so a run that keeps writing outlives its owner (decision 10).
6. **`client.run` failures reach the status line** through a channel into the UI's `select!` loop; `server.run` exits are logged only (decision 7).
7. **`server.run` gets the pane environment and directory**, through `PaneEnv` applied to either builder and a shared anchor-directory rule (decisions 8 and 9).

## Risks / Trade-offs

See the architecture paper's Risks and unknowns: raw run output in the log, log rotation not counting run bytes, the `PaneEnv` refactor, and Windows being best effort.
