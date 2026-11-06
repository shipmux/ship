# Focus reports: architecture

Status: Locked on 2026-11-06, approved by merging #119 ("just pull and commit the spec updates straight on main").

## Summary

```text
terminal ──FocusGained/Lost──▶ client ──PUT /attach/view {area, focused}──▶ server
                                                                              │
                         commit → apply_focus: focused panes now vs before ◀──┘
                                                                              │
                         pane ──send_focus──▶ session ──\e[I / \e[O──▶ program (if mode 1004)
```

Each attachment's record gains `focused`. Which panes have focus is derived when the server publishes, and the server tells only the panes whose answer changed, the same way it already sends pane sizes.

## Context and constraints

- Every change to tabs or attachments goes through `ServerState::commit` (`crates/ship-server/src/state.rs:162`), which publishes a replica and then sends changed sizes with `apply_sizes`.
- Selection repair already moves selections to stack fronts, zoomed panes and successors (`state.rs:197`), and a dropped attach stream detaches (`crates/ship-server/src/attach.rs:53`).
- The vendored session already has `send_focus`, but it writes the report even when the program didn't ask, and its focus also hides the rendered cursor (`vendor/ratatui-ghostty/src/widget.rs:104`).
- Store what can't be derived. A pane's focus can be derived from the attachments, so it isn't stored.

## Candidate shapes

### A. Derive at publish and diff against the last replica (winner)

`focused_panes(replica)` gives the set of pane IDs; `apply_focus` compares it with the previous replica's set. Every path that can change focus already ends in `commit`, so none needs its own code, and the result can't drift from the published state.

### B. Track transitions in each handler (rejected)

`SetView`, `Select`, `Detach`, the cycle, zoom and close handlers would each work out what changed. Six places to keep right, and the stress test's crash and repair cases would each be a separate path.

### C. Most recently active window wins, as in tmux (rejected)

One window's focus decides. Herdr and tmux both count a pane as focused while any focused viewer shows it (Herdr's `another_focused_viewer` check in `src/server/headless.rs`), and Ship's per-attachment selections make that rule simple.

## Decision

Shape A.

## Key decisions

- **KD-1** `focused` lives on `ViewingRecord`, set at attach and by `PUT /attach/view`. It's published, so other clients and scripts can see it.
- **KD-2** `AttachRequest.focused` defaults to `false`, so scripts and older shapes never give focus. The terminal client attaches with `true`, since terminals report focus only on a change.
- **KD-3** `PROTOCOL_VERSION` goes from 22 to 23. An older client would leave `focused` out, and every pane would be told it had lost focus.
- **KD-4** The session reports focus only to a program that enabled mode 1004, and reports the current focus when the program enables it (FR-003).
- **KD-5** A session's focus no longer affects rendering. The client already decides which pane shows a cursor, and the outer terminal styles an unfocused one.

## Risks and unknowns

- A program that turns mode 1004 off and back on within one read of its output gets no report, because the check compares the mode before and after each batch.
- Each focus change in the outer terminal publishes a replica to every client. That's one small publish per window switch.
