# Handing work to another agent

<!--
Scope: The loop for one agent starting another, giving it a task, waiting and reading the answer.
Not here: each command's full options (`--help`).
-->

Start a worker, wait for it to be ready, then send a task and read the reply. This example needs Claude Code and `jq` installed:

```sh
TAB=$(ship tab create --name workers | jq -r .id)
WORKER=$(ship pane create --tab "$TAB" --name worker -- claude | jq -r .id)
ship pane wait --pane "$WORKER"                                     # until it's ready
ship pane send --pane "$WORKER" "Write a haiku about ships" --key enter --wait \
  | jq -r '.program[""].state'
ship pane read --pane "$WORKER" --lines 40
```

`--wait` prints the pane when the agent reports `done`, `idle`, `blocked` or `error`. Read the state before assuming success. `done` or `idle` means it's ready, `blocked` means it needs an answer, and `error` means something went wrong.

To answer a prompt, send the keys and say you expect it to be blocked:

```sh
ship pane send --pane "$WORKER" --key down --key enter --wait --if blocked
```

`--if idle,done` only sends if the agent is in one of those states, so you don't type into an agent that's still busy. Use `--status` to choose which outcomes end a wait:

```sh
ship pane wait --pane "$WORKER" --status done,blocked,error
```

Process exit is a separate outcome. Add `exited` to `--status` if that should end the wait too, and create the pane with `--exit-behavior keep` if you need to inspect it afterward. `send --wait` accepts `--status` as well.

## If a send stalls

After sending to an agent that isn't working, `send --wait` gives it five seconds to report a changed state. If it doesn't, the command fails even though the input was sent. Read the pane or use `pane wait`; don't send the task again just because the wait failed. [Command errors](reading-command-errors.md) shows how to tell whether input went out.

Once the state changes, there is no completion deadline. Plain `pane wait` has no deadline either. Use `timeout 600 ...` to limit a wait (`gtimeout` on macOS, from coreutils); this does not extend the initial five-second check.

Waiting uses program status reports, not text on screen. For a program that doesn't report status, use plain `send` and `read`, or [wait for its process to exit](reading-output.md#reading-after-a-program-exits).
