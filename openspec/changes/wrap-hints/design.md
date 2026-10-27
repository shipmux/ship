# Design

## Context

`status_line` in `crates/ship-client/src/ui/draw.rs` builds one `Line`: the mode pill, the exit status, the disconnected notice, the first line of a message, then each hint as a bold cyan chord and a gray label. The caller draws it into a one-row `Rect` at the bottom of the pane area, clears that row first, and drops the pane cursor if it falls inside. Anything past the row's width is clipped by ratatui.

## Goals / Non-Goals

**Goals:** every hint readable whole whenever the pane area has room, without changing anything when one row is enough.

**Non-goals:** a which-key popup with groups or delays, hints in normal mode, or changes to what the hints say.

## Decisions

1. **One row when everything fits.** Most of the time the status line is short, such as `exited (3)` or a narrow custom mode, and it should look exactly as it does now. Rejected: always using the column layout in a mode, which adds rows for nothing.
2. **Hints in aligned columns below a first row.** The first row holds the pill and the messages, and the hints sit below it in a grid whose cells are as wide as the widest hint, filled left to right so config order reads like text. Chords line up down the columns, the way which-key lays them out. Rejected: wrapping the hints as plain flowing text, which leaves ragged rows that are harder to scan.
3. **Grow upward, capped at the pane area's height.** The overlay stays anchored to the bottom, so it covers the same rows that it covers today, and more only when needed. If the pane area is too short, whole hints are cut from the end.

## Risks / Trade-offs

- [A tall overlay hides more of the pane while in a mode] → It's only shown outside normal mode or while there's a message, as before, and it uses the fewest rows that fit.
- [One very wide label widens every column] → Labels are short words by default. A user who sets a long `label` gets fewer columns, not a broken layout.
