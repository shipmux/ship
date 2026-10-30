# Run actions

Status: Locked again on 2026-10-30 by Cyan in chat, without a Plannotator pass ("the change is trivial. just change everything and don't make me reread"). That lock drops the string form: `command` is argv only, as in Herdr, WezTerm, kitty and Zellij, because a string form can be added later without breaking configs but can't be taken back. FR-002, FR-003, P1 to P4, the examples, the non-goals and SC-003 changed with it. Before that, locked on 2026-10-29 in Plannotator review ("LGTM"). The first lock (2026-10-29, Plannotator, "LGTM") is superseded by one change: commands are fire-and-forget. FR-013 used to stop a command when the client or server that started it ended, and SC-003 checked that nothing was left running. Both now say the opposite. Cyan chose this in chat after comparing reference programs. tmux `run-shell -b` sends SIGTERM only to the direct child when its server exits. kitty, WezTerm and Zellij's `RunInBackground` don't stop their commands. None of them stops grandchildren. Cyan's reason: "if you didn't want fire and forget, why not just run in a pane?" Also after the first lock, at Cyan's request: the array example used `~`, which only a shell expands, so the two examples swapped forms. Written from a design conversation with Cyan on 2026-10-29 for [#22](https://github.com/shipmux/ship/issues/22). Cyan chose this over renaming from the UI ([#84](https://github.com/shipmux/ship/issues/84)), which stays deferred for its open prompt questions. Decided in that conversation: commands run in the background, both actions take a struct with a `command` field so fields can be added later, a command is a string run by `/bin/sh -c` or an argv array (since withdrawn: argv only), `server.run` has no CLI form, and a `server.run` command that exits non-zero after it started is reported in the server log only.

## Goal

Key bindings and menu entries can only send Ship's own actions today. This change lets them run any command, so a script can do what a plugin would: call `ship` back, open a pane, or do something outside Ship entirely. `server.run` runs the command on the machine the server is on, and `client.run` runs it on the machine the client is on. On a local setup that's the same machine; they differ once a client talks to a remote server ([#88](https://github.com/shipmux/ship/issues/88)).

```toml
"alt-g" = { action = { client.run = { command = ["gh", "pr", "view", "--web"] } } }                # argv, no shell
"alt-b" = { action = { server.run = { command = ["sh", "-c", "~/.config/ship/report-branch"] } } }  # a shell only when you name one
```

The edit-scrollback pane opened by `alt-e` gets the name `scrollback`, so it stops showing `sh` as its label.

## User stories

### P1: Run a command from a key

- **Given** a binding `client.run = { command = ["touch", "/tmp/hit"] }`, **when** the user presses its key, **then** `/tmp/hit` exists on the client's machine and the client keeps responding while the command runs.
- **Given** a binding `server.run = { command = ["sh", "-c", "sleep 5; touch /tmp/hit"] }`, **when** the user presses its key and then types in a pane straight away, **then** the typing reaches the pane at once, and `/tmp/hit` appears about 5 seconds later.
- **Given** the same key is pressed three times quickly, **then** three runs start.
- **Given** a menu entry with `client.run` or `server.run`, **when** the user picks it, **then** the command runs as if from a key.

Tested independently by binding each action to commands that touch files and checking the files and the client's responsiveness.

### P2: The command can call Ship back

- **Given** pane `X` is selected, **when** a bound `server.run = { command = ["sh", "-c", "\"$SHIP_BIN\" pane set --name picked"] }` runs, **then** pane `X` is named `picked`.
- **Given** pane `X` is selected and its shell is in `~/code/app`, **when** a bound `server.run = { command = ["sh", "-c", "pwd > /tmp/where"] }` runs, **then** `/tmp/where` holds the directory a new pane created from `X` would start in.
- **Given** the client was started from `~/notes`, **when** a bound `client.run = { command = ["sh", "-c", "pwd > /tmp/where"] }` runs, **then** `/tmp/where` holds `~/notes`.
- **Given** nothing is selected, **when** either action runs, **then** `SHIP_PANE_ID` is unset and the command still runs.

Tested independently by binding commands that print their environment and directory to a file.

### P3: Failures are visible

- **Given** a binding `client.run = { command = ["no-such-program"] }`, **when** the user presses its key, **then** the status line says the command couldn't start and names it.
- **Given** a binding `client.run = { command = ["sh", "-c", "exit 3"] }`, **when** it exits, **then** the status line says the command exited with 3 and names it.
- **Given** a binding `server.run = { command = ["no-such-program"] }`, **when** the user presses its key, **then** the status line says the command couldn't start and names it.
- **Given** a binding `server.run = { command = ["sh", "-c", "echo out; echo err >&2; exit 3"] }`, **when** it exits, **then** the server log holds `out`, `err` and the exit status, and the status line shows nothing.
- **Given** a `client.run` command prints output, **then** the output is in the client log and never drawn over the client's screen.

Tested independently by binding failing commands and reading the status line and both logs.

### P4: The scrollback pane's label

- **Given** the user presses `alt-e`, **then** the new pane's label is `scrollback`.

Tested independently by pressing `alt-e` on a pane with output.

## Non-goals

- A CLI form of `server.run`. A shell or `ssh` already runs commands.
- Waiting for a command before the binding's next action, or cancelling the rest of the binding when a command fails.
- Reporting a `server.run` command's exit status to clients. That needs a new event on the attach stream; it can be added later without breaking anything.
- Showing a command's output in a pane or on the screen. A command that needs to show something or ask for input opens its own pane.
- Passing client-only context such as the text selection or the clicked row.
- A string form for `command`. A command is argv, and a shell runs only when the command names one. A string form can be added later without breaking configs.
- Limiting how many runs of one binding can be alive at once.
- Stopping a command. Ship starts it and leaves it alone. A command that should be watched, stopped, or end with Ship belongs in a pane (`server.pane.create`).
- Choosing a shell per platform. On Windows a command is argv too, best effort like the rest of Ship there.

## Functional requirements

- **FR-001:** The config MUST accept `server.run = { command = ... }` and `client.run = { command = ... }` as actions wherever key bindings and menu entries accept actions. Unknown fields MUST be rejected when the config loads, as for other actions.
- **FR-002:** `command` MUST be required and written as an array of strings. It MUST run as argv with its first element as the program and no shell involved. An empty array MUST be rejected when the config loads.
- **FR-003:** Withdrawn on 2026-10-30 with the string form. `server.pane.create`'s `command` stays as it is.
- **FR-004:** `server.run` MUST start its command on the server's machine, and `client.run` on the client's machine.
- **FR-005:** Running a command MUST NOT wait for it to exit. The binding's later actions and the user's next keys MUST proceed while it runs.
- **FR-006:** Each trigger MUST start a new run, whether or not an earlier run of the same binding is still alive.
- **FR-007:** A command MUST get `SHIP_BIN`, `SHIP_SERVER_URL`, `SHIP_ATTACHMENT_ID` for the triggering client, and `SHIP_PANE_ID` for its selected pane. `SHIP_PANE_ID` MUST be unset when nothing is selected.
- **FR-008:** A `server.run` command MUST start in the directory a new pane created from the selected pane would start in. A `client.run` command MUST start in the directory the client was started from.
- **FR-009:** A `server.run` command MUST get the same environment as a new pane, apart from `SHIP_PANE_ID`, which names the selected pane.
- **FR-010:** A command's standard input MUST be empty, and its standard output and error MUST go to the log of the side that ran it: the server log for `server.run` and the client log for `client.run`. Output MUST NOT reach the client's terminal.
- **FR-011:** When a command can't start, the triggering client's status line MUST say so and name the command.
- **FR-012:** When a `client.run` command exits non-zero or is killed by a signal, the client's status line MUST say so and name the command. When a `server.run` command does, the server log MUST record it, and clients MUST NOT be told.
- **FR-013:** Ship MUST NOT stop a command it started. A `server.run` command MUST keep running after the server stops, and a `client.run` command MUST keep running after its client quits or detaches, including when the command goes on writing output.
- **FR-014:** The default `alt-e` binding and its menu entry MUST name the new pane `scrollback`.

## Success criteria

- **SC-001:** With a `server.run` binding to `sleep 30`, typing in a pane right after pressing the key shows no delay compared with typing without pressing it.
- **SC-002:** Every acceptance scenario in P1 to P4 passes in a running client on macOS. Linux is checked separately or reported as unverified.
- **SC-003:** With bindings to `["sh", "-c", "while true; do echo tick; sleep 1; done"]` under both actions, the `client.run` command is still running 5 seconds after the client quits, and the `server.run` command is still running 5 seconds after the server stops.

## Open unknowns

None.
